use std::iter;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{Data, DataEnum, Path};

use crate::attr::{DeriveMeta, ProstAttr, ProstType};
use crate::context::Context;

pub fn expand_enum(
    context: &Context,
    meta: &DeriveMeta,
    serde: &Path,
    deserializer: &Ident,
    ident: &Ident,
    d: &DataEnum,
) -> Result<TokenStream, ()> {
    match meta.prost_type {
        ProstType::Enum => Ok(quote! {
            let s = String::deserialize(#deserializer)?;
            #ident::from_str_name(&s).ok_or(#serde::de::Error::unknown_variant(&s, &[]))
        }),
        ProstType::Oneof => OneofDeserializer::new(
            context,
            meta,
            serde,
            deserializer,
            ident,
            d.variants.iter().collect_vec(),
        )?
        .expand(),
        _ => Err(()),
    }
}

#[allow(unused)]
struct Variant {
    ident: Ident,
    attr: ProstAttr,
}

#[allow(unused)]
struct OneofDeserializer<'a> {
    context: &'a Context,
    meta: &'a DeriveMeta,
    serde: &'a Path,
    deserializer: &'a Ident,
    ident: &'a Ident,
    variants: Vec<Variant>,
}

impl<'a> OneofDeserializer<'a> {
    pub fn new(
        context: &'a Context,
        meta: &'a DeriveMeta,
        serde: &'a Path,
        deserializer: &'a Ident,
        ident: &'a Ident,
        variants: Vec<&syn::Variant>,
    ) -> Result<Self, ()> {
        let mut typed_variants = Vec::new();
        for variant in variants {
            let attr = ProstAttr::from_ast(context, &variant.attrs)?;
            typed_variants.push(Variant {
                ident: variant.ident.clone(),
                attr,
            });
        }

        Ok(Self {
            context,
            meta,
            serde,
            deserializer,
            ident,
            variants: typed_variants,
        })
    }

    #[inline]
    fn get_variant_idents(&self) -> impl Iterator<Item = &Ident> {
        self.variants.iter().map(|v| &v.ident)
    }

    pub fn expand_variant_deserializer_impl(&self) -> (Ident, TokenStream, Vec<Ident>) {
        let serde = self.serde;

        let variants = self.get_variant_idents().cloned().collect_vec();

        let names = variants
            .iter()
            .map(|v| v.to_string().to_case(Case::Snake))
            .collect_vec();

        let ident_enum = format_ident!("Variant");
        let ident_visitor = format_ident!("{}Visitor", ident_enum);

        let mut match_arms = Vec::new();
        for (variant, name) in iter::zip(variants.iter(), names.iter()) {
            match_arms.push(quote! {
                #name => Ok(#ident_enum::#variant)
            })
        }

        let expecting_names = names.iter().map(|v| format!("`{}`", v)).join(" or ");
        let expr = quote! {
            enum #ident_enum {
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
                            formatter.write_str(#expecting_names)
                        }

                        fn visit_str<E>(self, value: &str) -> Result<#ident_enum, E>
                        where
                            E: #serde::de::Error,
                        {
                            match value {
                                #(#match_arms),*,
                                _ => Err(#serde::de::Error::unknown_field(value, VARIANTS))
                            }
                        }
                    }

                    deserializer.deserialize_identifier(#ident_visitor)
                }
            }
        };

        (ident_enum, expr, variants)
    }

    pub fn expand_visitor_impl(&self) -> Result<(Ident, TokenStream), ()> {
        let serde = self.serde;

        let ident_self = self.ident;
        let expecting = format!("enum {}", ident_self);
        let ident_visitor = format_ident!("Visitor");

        let (ident_variant_enum, variant_deserializer, variants) =
            self.expand_variant_deserializer_impl();

        let mut variant_match_arms = Vec::new();

        for variant in variants {
            variant_match_arms.push(quote! {
                (#ident_variant_enum::#variant, variant) => {
                    let value = #serde::de::VariantAccess::newtype_variant(variant)?;
                    Ok(#ident_self::#variant(value))
                }
            });
        }

        let expr = quote! {
            #variant_deserializer

            struct #ident_visitor;

            impl<'de> #serde::de::Visitor<'de> for #ident_visitor {
                type Value = #ident_self;

                fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    formatter.write_str(#expecting)
                }

                fn visit_enum<A>(self, data: A) -> Result<#ident_self, A::Error>
                where
                    A: #serde::de::EnumAccess<'de>,
                {
                    match data.variant()? {
                        #(#variant_match_arms)*
                    }
                }
            }

        };

        Ok((ident_visitor, expr))
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let deserializer = self.deserializer;

        let name = self.ident.to_string();
        let fields = self
            .get_variant_idents()
            .map(ToString::to_string)
            .map(|s| s.to_case(Case::Snake))
            .collect_vec();

        let (visitor_ident, visitor_impl) = self.expand_visitor_impl()?;

        Ok(quote! {
            #visitor_impl

            const VARIANTS: &'static [&'static str] = &[ #(#fields), * ];
            #deserializer.deserialize_enum(#name, &VARIANTS, #visitor_ident)
        })
    }
}

pub fn expand_oneof_field_names_method(
    derive_meta: &DeriveMeta,
    ident: &Ident,
    data: &Data,
) -> TokenStream {
    if let Data::Enum(d) = data {
        if derive_meta.prost_type == ProstType::Oneof {
            let variants = d
                .variants
                .iter()
                .map(|v| v.ident.unraw().to_string().to_case(Case::Snake))
                .collect_vec();

            return quote! {
                impl #ident {
                    pub fn field_names() -> &'static [&'static str] {
                        &[#(#variants),*]
                    }
                }
            };
        }
    }

    quote! {}
}
