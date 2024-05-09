use std::iter;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{DataStruct, Fields, FieldsNamed, Path};

use super::field::FieldVisitorTokenStream;
use crate::attr::{DeriveMeta, ProstAttr};
use crate::context::Context;
use crate::deserialize::field::FieldVisitorTokenGenerator;

static IDENT_VARIANT_UNKNOWN: &str = "_Unknown";

pub fn expand_struct(
    context: &Context,
    meta: &DeriveMeta,
    serde: &Path,
    deserializer: &Ident,
    ident: &Ident,
    data: &DataStruct,
) -> Result<TokenStream, ()> {
    match &data.fields {
        Fields::Named(f) => {
            NamedStructDeserializer::new(context, meta, serde, deserializer, ident, f).expand()
        }
        Fields::Unnamed(_) => {
            context.push_error_spanned_by(
                &data.fields,
                "Unit struct is not available for deserialization.",
            );
            Err(())
        }
        Fields::Unit => {
            context.push_error_spanned_by(
                &data.fields,
                "Unit struct is not available for deserialization.",
            );
            Err(())
        }
    }
}

struct NamedStructDeserializer<'a> {
    context: &'a Context,
    meta: &'a DeriveMeta,
    serde: &'a Path,
    deserializer: &'a Ident,
    ident: &'a Ident,
    fields: &'a FieldsNamed,
}

impl<'a> NamedStructDeserializer<'a> {
    pub fn new(
        context: &'a Context,
        meta: &'a DeriveMeta,
        serde: &'a Path,
        deserializer: &'a Ident,
        ident: &'a Ident,
        fields: &'a FieldsNamed,
    ) -> Self {
        Self {
            context,
            meta,
            serde,
            deserializer,
            ident,
            fields,
        }
    }

    #[inline]
    fn get_field_idents(&self) -> impl Iterator<Item = &Ident> {
        self.fields.named.iter().map(|v| v.ident.as_ref().unwrap())
    }

    fn expand_field_deserializer_impl(
        &self,
        ident_unknown: Option<&Ident>,
    ) -> (Ident, TokenStream, Vec<Ident>) {
        let serde = self.serde;

        let mut variants = vec![];
        variants.extend(
            self.get_field_idents()
                .map(|v| format_ident!("{}", v.unraw().to_string().to_case(Case::Pascal))),
        );

        let field_names = self
            .get_field_idents()
            .map(|v| format!("`{}`", v))
            .join(" or ");

        let ident_enum = format_ident!("Field");
        let ident_visitor = format_ident!("{}Visitor", ident_enum);

        let field_match_arms = iter::zip(
            self.get_field_idents()
                .map(IdentExt::unraw)
                .map(|i| i.to_string())
                .collect_vec(),
            variants.iter(),
        )
        .map(|(name, variant)| {
            quote! {
                #name => Ok(#ident_enum::#variant)
            }
        });

        let (unknown_variant, unknown_match_arm) = if let Some(unknown) = ident_unknown {
            (
                Some(quote! { #unknown, }),
                quote! { Ok(#ident_enum::#unknown) },
            )
        } else {
            (
                None,
                quote! { Err(#serde::de::Error::unknown_field(value, FIELDS)) },
            )
        };

        let expr = quote! {
            enum #ident_enum {
                #unknown_variant
                #(#variants),*
            }

            impl<'de> #serde::Deserialize<'de> for #ident_enum {
                fn deserialize<D>(deserializer: D) -> Result<#ident_enum, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    struct #ident_visitor;

                    impl<'de> #serde::de::Visitor<'de> for #ident_visitor {
                        type Value = #ident_enum;

                        fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            formatter.write_str(#field_names)
                        }

                        fn visit_str<E>(self, value: &str) -> Result<#ident_enum, E>
                        where
                            E: #serde::de::Error,
                        {
                            match value {
                                #(#field_match_arms,)*
                                _ => #unknown_match_arm,
                            }
                        }
                    }

                    deserializer.deserialize_identifier(#ident_visitor)
                }
            }
        };

        (ident_enum, expr, variants)
    }

    fn expand_visitor_impl(&self) -> Result<(Ident, TokenStream), ()> {
        let serde = self.serde;
        let unknown = format_ident!("{}", IDENT_VARIANT_UNKNOWN);

        let ident_self = self.ident;
        let expecting = format!("struct {}", ident_self);
        let ident_visitor = format_ident!("Visitor");

        let ident_unknown = if self.meta.ignore_unknown_fields {
            Some(&unknown)
        } else {
            None
        };

        let (ident_field_enum, field_deserializer, field_variants) =
            self.expand_field_deserializer_impl(ident_unknown);

        let field_visitor_token_generator =
            FieldVisitorTokenGenerator::new(self.context, self.meta, self.serde);

        let mut var_decls = vec![];
        let mut var_match_arms = vec![];
        let mut var_narrowings = vec![];
        let mut var_fields = vec![];

        if self.meta.ignore_unknown_fields {
            var_match_arms.push(quote! {
                #ident_field_enum::#unknown => {
                    map.next_value::<serde_json::Value>()?;
                }
            });
        }

        for (field, field_variant) in iter::zip(self.fields.named.iter(), field_variants.iter()) {
            let prost_attr = ProstAttr::from_ast(self.context, &field.attrs)?;

            let ident_field_var = format_ident!("psd_{}", field.ident.as_ref().unwrap().unraw());
            let ident_field = field.ident.as_ref().unwrap();
            let field_name = ident_field.to_string();
            var_decls.push(quote! { let mut #ident_field_var = None; });

            let FieldVisitorTokenStream {
                value_getter_expr,
                narrowing_expr,
            } = field_visitor_token_generator.expand(&prost_attr, &field_name, &ident_field_var);

            var_match_arms.push(quote! {
                #ident_field_enum::#field_variant => {
                    if #ident_field_var.is_some() {
                        return Err(#serde::de::Error::duplicate_field(#field_name));
                    }
                    #ident_field_var = #value_getter_expr;
                }
            });
            var_narrowings.push(narrowing_expr);

            var_fields.push(quote! {
                #ident_field: #ident_field_var
            });
        }

        let expr = quote! {
            #field_deserializer

            struct #ident_visitor;

            impl<'de> #serde::de::Visitor<'de> for #ident_visitor {
                type Value = #ident_self;

                fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    formatter.write_str(#expecting)
                }

                fn visit_map<V>(self, mut map: V) -> Result<#ident_self, V::Error>
                where
                    V: #serde::de::MapAccess<'de>,
                {
                    #(#var_decls)*
                    while let Some(key) = map.next_key::<#ident_field_enum>()? {
                        match key {
                            #(#var_match_arms),*
                        };
                    }
                    #(#var_narrowings)*

                    Ok(#ident_self {
                        #(#var_fields),*
                    })
                }
            }
        };

        Ok((ident_visitor, expr))
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let deserializer = self.deserializer;

        let name = self.ident.to_string();
        let fields = self
            .get_field_idents()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let (visitor_ident, visitor_impl) = self.expand_visitor_impl()?;

        Ok(quote! {
            #visitor_impl

            const FIELDS: &'static [&'static str] = &[ #(#fields), * ];
            #deserializer.deserialize_struct(#name, &FIELDS, #visitor_ident)
        })
    }
}
