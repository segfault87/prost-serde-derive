use std::iter;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{parse_quote, DataStruct, Fields, FieldsNamed, Path, Type};

use super::field::FieldVisitorTokenStream;
use crate::attr::{DeriveMeta, ProstAttr, ProtobufType};
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
            NamedStructDeserializer::new(context, meta, serde, deserializer, ident, f)?.expand()
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

struct SingleFieldVariant {
    ident: Ident,
    ty: Option<Type>,
}

impl SingleFieldVariant {
    pub fn new(ident: Ident, ty: Option<Type>) -> Self {
        Self { ident, ty }
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn def(&self) -> TokenStream {
        let ident = self.ident();
        match &self.ty {
            Some(ty) => {
                quote! { #ident(#ty) }
            }
            None => {
                quote! { #ident }
            }
        }
    }

    pub fn pat(&self) -> TokenStream {
        let ident = self.ident();
        match &self.ty {
            Some(_) => {
                quote! { #ident(name) }
            }
            None => {
                quote! { #ident }
            }
        }
    }

    pub fn gen(&self, value: Option<TokenStream>) -> TokenStream {
        let ident = self.ident();
        match &self.ty {
            Some(_) => {
                let value = value.unwrap();
                quote! { #ident(#value) }
            }
            None => {
                quote! { #ident }
            }
        }
    }
}

struct Field {
    ident: Ident,
    attr: ProstAttr,
}

struct NamedStructDeserializer<'a> {
    context: &'a Context,
    meta: &'a DeriveMeta,
    serde: &'a Path,
    deserializer: &'a Ident,
    ident: &'a Ident,
    fields: Vec<Field>,
}

impl<'a> NamedStructDeserializer<'a> {
    pub fn new(
        context: &'a Context,
        meta: &'a DeriveMeta,
        serde: &'a Path,
        deserializer: &'a Ident,
        ident: &'a Ident,
        fields: &'a FieldsNamed,
    ) -> Result<Self, ()> {
        let mut typed_fields = Vec::new();

        for field in fields.named.iter() {
            let prost_attr = ProstAttr::from_ast(context, &field.attrs)?;
            typed_fields.push(Field {
                ident: field.ident.as_ref().unwrap().clone(),
                attr: prost_attr,
            })
        }

        Ok(Self {
            context,
            meta,
            serde,
            deserializer,
            ident,
            fields: typed_fields,
        })
    }

    #[inline]
    fn get_field_idents(&self) -> impl Iterator<Item = &Ident> {
        self.fields.iter().map(|v| &v.ident)
    }

    fn expand_field_deserializer_impl(
        &self,
        ident_unknown: Option<&Ident>,
    ) -> (Ident, TokenStream, Vec<SingleFieldVariant>) {
        let serde = self.serde;

        let mut variants = vec![];
        for field in &self.fields {
            let ident = format_ident!("{}", field.ident.unraw().to_string().to_case(Case::Pascal));
            if let ProtobufType::OneOf(_) = field.attr.ty {
                // keep oneof field name inside
                let ty_string: Type = parse_quote!(String);
                variants.push(SingleFieldVariant::new(ident, Some(ty_string)));
            } else {
                variants.push(SingleFieldVariant::new(ident, None));
            }
        }
        // TODO: show oneof fields
        let field_names = self
            .get_field_idents()
            .map(|v| format!("`{}`", v))
            .join(" or ");

        let ident_enum = format_ident!("Field");
        let ident_visitor = format_ident!("{}Visitor", ident_enum);

        let mut oneof_field_if_exprs = Vec::new();
        let mut field_match_arms = Vec::new();

        for (field, variant) in iter::zip(self.fields.iter(), variants.iter()) {
            if let ProtobufType::OneOf(ref p) = field.attr.ty {
                let names = quote! { #p::field_names() };
                let variant_gen = variant.gen(Some(quote! { value.to_string() }));
                oneof_field_if_exprs.push(quote! {
                    if #names.contains(&value) {
                        return Ok(#ident_enum::#variant_gen);
                    }
                })
            } else {
                let name = field.ident.unraw().to_string();
                let variant = variant.ident();
                field_match_arms.push(quote! {
                    #name => return Ok(#ident_enum::#variant)
                });
            }
        }

        let oneof_field_if_exprs = if oneof_field_if_exprs.len() > 1 {
            oneof_field_if_exprs
                .into_iter()
                .reduce(|acc, i| quote! { #acc else #i})
                .unwrap()
        } else {
            quote! { #(#oneof_field_if_exprs)* }
        };

        let (unknown_variant, unknown_match) = if let Some(unknown) = ident_unknown {
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

        let variant_defs = variants.iter().map(SingleFieldVariant::def);

        let expr = quote! {
            enum #ident_enum {
                #unknown_variant
                #(#variant_defs),*
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
                            write!(formatter, "{}", #field_names)
                        }

                        fn visit_str<E>(self, value: &str) -> Result<#ident_enum, E>
                        where
                            E: #serde::de::Error,
                        {
                            match value {
                                #(#field_match_arms,)*
                                _ => {}
                            }

                            #oneof_field_if_exprs

                            #unknown_match
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

        for (field, field_variant) in iter::zip(self.fields.iter(), field_variants.iter()) {
            let ident_field_var = format_ident!("psd_{}", field.ident.unraw());
            let ident_field = &field.ident;
            let field_name = ident_field.to_string();
            var_decls.push(quote! { let mut #ident_field_var = None; });

            let FieldVisitorTokenStream {
                value_getter_expr,
                narrowing_expr,
            } = field_visitor_token_generator.expand(&field.attr, &field_name, &ident_field_var)?;

            let field_variant_pat = field_variant.pat();
            var_match_arms.push(quote! {
                #ident_field_enum::#field_variant_pat => {
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
