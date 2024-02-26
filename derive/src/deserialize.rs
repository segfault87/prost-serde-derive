use std::iter;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{parse_quote, Data, DataStruct, DeriveInput, Error, Fields, FieldsNamed, Path};

use crate::{
    attr::{DeriveMeta, FieldModifier, ProstAttr, ProtobufType},
    context::Context,
    util::{deraw, wrap_block},
};

struct NamedStructDeserializer<'a> {
    context: &'a Context,
    meta: &'a DeriveMeta,
    serde: &'a Path,
    ident: &'a Ident,
    fields: &'a FieldsNamed,
}

impl<'a> NamedStructDeserializer<'a> {
    pub fn new(
        context: &'a Context,
        meta: &'a DeriveMeta,
        serde: &'a Path,
        ident: &'a Ident,
        fields: &'a FieldsNamed,
    ) -> Self {
        Self {
            context,
            meta,
            serde,
            ident,
            fields,
        }
    }

    #[inline]
    fn get_field_idents(&self) -> impl Iterator<Item = &Ident> {
        self.fields.named.iter().map(|v| v.ident.as_ref().unwrap())
    }

    fn expand_field_deserializer_impl(&self) -> (Ident, TokenStream, Vec<Ident>) {
        let serde = self.serde;

        let variants = self
            .get_field_idents()
            .map(|v| Ident::new(&deraw(v).to_case(Case::Pascal), Span::call_site()))
            .collect::<Vec<_>>();
        let field_names =
            itertools::join(self.get_field_idents().map(|v| format!("`{}`", v)), " or ");

        let ident = Ident::new("Field", Span::call_site());
        let ident_visitor = Ident::new(&(ident.to_string() + "Visitor"), Span::call_site());

        let pat_fields = iter::zip(self.get_field_idents().map(deraw), variants.iter()).map(
            |(name, variant)| {
                quote! {
                    #name => Ok(#ident::#variant)
                }
            },
        );

        let expr = quote! {
            enum #ident {
                #(#variants),*
            }

            impl<'de> #serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<#ident, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    struct #ident_visitor;

                    impl<'de> #serde::de::Visitor<'de> for #ident_visitor {
                        type Value = #ident;

                        fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            formatter.write_str(#field_names)
                        }

                        fn visit_str<E>(self, value: &str) -> Result<#ident, E>
                        where
                            E: #serde::de::Error,
                        {
                            match value {
                                #(#pat_fields,)*
                                _ => Err(#serde::de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(#ident_visitor)
                }
            }
        };

        (ident, expr, variants)
    }

    fn expand_visitor_impl(&self) -> Result<(Ident, TokenStream), ()> {
        let serde = self.serde;

        let ident = self.ident;
        let expecting = format!("struct {}", ident);
        let visitor_ident = Ident::new("Visitor", Span::call_site());

        let (field_enum_ident, field_deserializer, field_variants) =
            self.expand_field_deserializer_impl();

        let mut var_decls = vec![];
        let mut var_pat_fields = vec![];
        let mut var_narrowings = vec![];
        let mut var_fields = vec![];

        fn value_getter(
            omit_type_errors: bool,
            type_sig: Option<TokenStream>,
            expr: TokenStream,
            default_value: &TokenStream,
        ) -> TokenStream {
            let getter = match type_sig {
                Some(v) => quote! { map.next_value::<#v>() },
                None => quote! { map.next_value() },
            };

            if omit_type_errors {
                quote! {
                    match #getter {
                        Ok(value) => #expr,
                        Err(_) => Some(#default_value)
                    }
                }
            } else {
                quote! {
                    {
                        let value = #getter?;
                        #expr
                    }
                }
            }
        }

        let omit_type_errors = self.meta.omit_type_errors;
        let use_default_for_missing_fields = self.meta.use_default_for_missing_fields;

        for (field, field_variant) in iter::zip(self.fields.named.iter(), field_variants.iter()) {
            let prost_attr = ProstAttr::from_ast(self.context, &field.attrs)?;

            let default_value = prost_attr.get_default_value();

            let field_var_ident = Ident::new(
                &format!("psd__{}", deraw(field.ident.as_ref().unwrap())),
                Span::call_site(),
            );
            let field_ident = field.ident.as_ref().unwrap();
            let field_name = field_ident.to_string();
            var_decls.push(quote! { let mut #field_var_ident = None; });

            let value_getter_expr = match (prost_attr.modifier, prost_attr.ty) {
                (FieldModifier::None, ProtobufType::Enumeration(path)) => value_getter(
                    omit_type_errors,
                    Some(quote! { String }),
                    quote! {
                        Some(match #path::from_str_name(&value) {
                            Some(v) => v.into(),
                            None => return Err(#serde::de::Error::unknown_variant(&value, &[])),
                        })
                    },
                    &default_value,
                ),
                (FieldModifier::None, ProtobufType::Bytes(_)) => value_getter(
                    omit_type_errors,
                    Some(quote! { String }),
                    quote! {
                        Some({
                            extern crate base64 as _base64;
                            match _base64::decode(&value) {
                                Ok(v) => v.into(),
                                Err(_) => return Err(#serde::de::Error::invalid_value(#serde::de::Unexpected::Str(&value), &"A base64 string")),
                            }
                        })
                    },
                    &default_value,
                ),
                (FieldModifier::None, _) => value_getter(
                    omit_type_errors,
                    None,
                    quote! { Some(value) },
                    &default_value,
                ),
                (FieldModifier::Repeated, ProtobufType::Enumeration(path)) => value_getter(
                    omit_type_errors,
                    Some(quote! { Vec<String> }),
                    quote! {
                        Some({
                            let mut result = vec![];
                            for value in value.iter() {
                                match #path::from_str_name(&value) {
                                    Some(v) => {
                                        result.push(v.into());
                                    }
                                    None => {
                                        return Err(#serde::de::Error::unknown_variant(&value, &[]));
                                    }
                                }
                            }
                            result
                        })
                    },
                    &default_value,
                ),
                (FieldModifier::Repeated, ProtobufType::Bytes(_)) => value_getter(
                    omit_type_errors,
                    Some(quote! { Vec<String> }),
                    quote! {
                        Some({
                            extern crate base64 as _base64;
                            let mut result = vec![];
                            for value in value.iter() {
                                match _base64::decode(value) {
                                    Ok(v) => {
                                        result.push(v.into());
                                    },
                                    Err(_) => {
                                        return Err(
                                            #serde::de::Error::invalid_value(
                                                #serde::de::Unexpected::Str(value),
                                                &"A base64 string",
                                            ),
                                        );
                                    }
                                }
                            }
                            result
                        })
                    },
                    &default_value,
                ),
                (FieldModifier::Repeated, _) => todo!(),
                (FieldModifier::Optional, ProtobufType::Enumeration(path)) => value_getter(
                    omit_type_errors,
                    Some(quote! { Option<String> }),
                    quote! {
                        match &value {
                            Some(v) => match #path::from_str_name(v) {
                                Some(v) => Some(v.into()),
                                None => return Err(#serde::de::Error::unknown_variant(v, &[])),
                            },
                            None => None,
                        }
                    },
                    &default_value,
                ),
                (FieldModifier::Optional, ProtobufType::Bytes(_)) => todo!(),
                (FieldModifier::Optional, _) => {
                    value_getter(omit_type_errors, None, quote! { value }, &default_value)
                }
            };

            var_pat_fields.push(quote! {
                #field_enum_ident::#field_variant => {
                    if #field_var_ident.is_some() {
                        return Err(#serde::de::Error::duplicate_field(#field_name));
                    }
                    #field_var_ident = #value_getter_expr;
                }
            });
            match prost_attr.modifier {
                FieldModifier::Repeated => {
                    var_narrowings.push(quote! {
                        let #field_var_ident = #field_var_ident.unwrap_or(vec![]);
                    });
                }
                FieldModifier::None => {
                    if use_default_for_missing_fields {
                        var_narrowings.push(quote! {
                            let #field_var_ident = #field_var_ident.unwrap_or(#default_value);
                        })
                    } else {
                        var_narrowings.push(quote! {
                            let #field_var_ident = #field_var_ident.ok_or_else(|| #serde::de::Error::missing_field(#field_name))?;
                        });
                    }
                }
                _ => {}
            }
            var_fields.push(quote! {
                #field_ident: #field_var_ident
            });
        }

        let expr = quote! {
            #field_deserializer

            struct #visitor_ident;

            impl<'de> #serde::de::Visitor<'de> for #visitor_ident {
                type Value = #ident;

                fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    formatter.write_str(#expecting)
                }

                fn visit_map<V>(self, mut map: V) -> Result<#ident, V::Error>
                where
                    V: #serde::de::MapAccess<'de>,
                {
                    #(#var_decls)*
                    while let Some(key) = map.next_key()? {
                        match key {
                            #(#var_pat_fields),*
                        };
                    }
                    #(#var_narrowings)*

                    Ok(#ident {
                        #(#var_fields),*
                    })
                }
            }
        };

        Ok((visitor_ident, expr))
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let ident_name = self.ident.to_string();
        let fields = self
            .get_field_idents()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let (visitor_ident, visitor_impl) = self.expand_visitor_impl()?;

        Ok(quote! {
            #visitor_impl

            const FIELDS: &'static [&'static str] = &[ #(#fields), * ];
            deserializer.deserialize_struct(#ident_name, &FIELDS, #visitor_ident)
        })
    }
}

fn expand_struct(
    context: &Context,
    meta: &DeriveMeta,
    serde: &Path,
    ident: &Ident,
    data: &DataStruct,
) -> Result<TokenStream, ()> {
    match &data.fields {
        Fields::Named(f) => NamedStructDeserializer::new(context, meta, serde, ident, f).expand(),
        Fields::Unnamed(_) => {
            context.error_spanned_by(
                &data.fields,
                "Unit struct is not available for deserialization.",
            );
            Err(())
        }
        Fields::Unit => {
            context.error_spanned_by(
                &data.fields,
                "Unit struct is not available for deserialization.",
            );
            Err(())
        }
    }
}

struct EnumDeserializer<'a> {
    serde: &'a Path,
    ident: &'a Ident,
}

impl<'a> EnumDeserializer<'a> {
    pub fn new(serde: &'a Path, ident: &'a Ident) -> Self {
        Self { serde, ident }
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let ident = self.ident;
        let serde = self.serde;
        Ok(quote! {
            let str = String::deserialize(deserializer)?;
            #ident::from_str_name(&str).ok_or(#serde::de::Error::unknown_variant(&str, &[]))
        })
    }
}

fn expand_enum(serde: &Path, ident: &Ident) -> Result<TokenStream, ()> {
    EnumDeserializer::new(serde, ident).expand()
}

pub fn expand_deserialize(input: DeriveInput) -> Result<TokenStream, Vec<Error>> {
    let context = Context::new();

    let derive_meta = match DeriveMeta::from_ast(&context, &input.attrs) {
        Ok(v) => v,
        Err(_) => {
            context.check()?;
            return Ok(quote! {});
        }
    };

    let ident = &input.ident;
    let data = &input.data;

    let serde: Path = parse_quote! { _serde };

    let deserialization_block = match data {
        Data::Struct(d) => expand_struct(&context, &derive_meta, &serde, ident, d),
        Data::Enum(_) => expand_enum(&serde, ident),
        Data::Union(d) => {
            context.error_spanned_by(
                d.union_token,
                "Union type is not available for deserialization.",
            );
            Err(())
        }
    }
    .unwrap_or_else(|_| quote! {});

    context.check()?;

    let impl_body = quote! {
        impl<'de> #serde::Deserialize<'de> for #ident {

            fn deserialize<D>(deserializer: D) -> Result<#ident, D::Error>
            where D: #serde::Deserializer<'de>,
            {
                #deserialization_block
            }

        }
    };

    Ok(wrap_block(impl_body))
}
