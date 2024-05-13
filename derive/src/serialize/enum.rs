use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::{Data, DataEnum, Ident, Path};

use crate::attr::{DeriveMeta, ProstType};
use crate::context::Context;

pub fn expand_enum(
    _context: &Context,
    derive_meta: &DeriveMeta,
    _serde: &Path,
    serializer: &Ident,
    _ident: &Ident,
    data: &DataEnum,
) -> Result<TokenStream, ()> {
    match derive_meta.prost_type {
        ProstType::Enum => Ok(quote! {
            #serializer.serialize_str(self.as_str_name())
        }),
        ProstType::Oneof => {
            let mut match_arms = Vec::new();
            for variant in data.variants.iter() {
                let ident_variant = &variant.ident;
                let variant = ident_variant.to_string().to_case(Case::Snake);
                match_arms.push(quote! {
                    Self::#ident_variant(ref v) => {
                        #serializer.serialize_newtype_struct(#variant, v)
                    }
                });
            }

            Ok(quote! {
                match self {
                    #(#match_arms)*
                }
            })
        }
        _ => Err(()),
    }
}

pub fn expand_oneof_field_name_method(
    derive_meta: &DeriveMeta,
    ident: &Ident,
    data: &Data,
) -> TokenStream {
    if let Data::Enum(d) = data {
        if derive_meta.prost_type == ProstType::Oneof {
            let mut match_arms = Vec::new();
            for variant in d.variants.iter() {
                let ident_variant = &variant.ident;
                let variant = ident_variant.unraw().to_string().to_case(Case::Snake);
                match_arms.push(quote! {
                    Self::#ident_variant(_) => {
                        #variant
                    }
                });
            }

            return quote! {
                impl #ident {
                    pub fn field_name(&self) -> &'static str {
                        match self {
                            #(#match_arms)*
                        }
                    }
                }
            };
        }
    }

    quote! {}
}
