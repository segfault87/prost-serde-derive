use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::{Field, Path};

use crate::attr::{FieldModifier, ProstAttr, ProtobufType};
use crate::context::Context;

pub fn serialize_field(context: &Context, serde: &Path, field: &Field) -> Result<TokenStream, ()> {
    let ident = field.ident.as_ref().ok_or_else(|| {
        context.push_error_spanned_by(field, "Field should have a name");
    })?;
    let ident_str = ident.unraw().to_string();

    let prost_attr = ProstAttr::from_ast(context, &field.attrs)?;

    // it is a special case that the field should be flatten.
    if let ProtobufType::OneOf(_) = prost_attr.ty {
        return Ok(quote! {
            if let Some(v) = &self.#ident {
                state.serialize_field(v.field_name(), &self.#ident)?;
            }
        });
    }

    let serialize_stmt = match prost_attr.ty {
        ProtobufType::Bytes(_) => {
            let base64 = quote! { extern crate base64 as _base64; };
            match prost_attr.modifier {
                FieldModifier::Repeated => {
                    quote! {
                        #base64
                        &self.#ident.iter().map(_base64::encode).collect::<Vec<_>>()
                    }
                }
                FieldModifier::Optional => {
                    quote! {
                        #base64
                        &self.#ident.as_ref().map(_base64::encode)
                    }
                }
                FieldModifier::None => {
                    quote! {
                        #base64
                        &_base64::encode(&self.#ident)
                    }
                }
            }
        }
        ProtobufType::Enumeration(p) => match prost_attr.modifier {
            FieldModifier::Repeated => quote! {
                &self.#ident.iter().map(|v| match #p::from_i32(*v) {
                    Some(v) => Ok(v.as_str_name()),
                    None => Err(#serde::ser::Error::custom(format!("Invalid enum value {}", v))),
                }).collect::<Result<Vec<_>, _>>()?
            },
            FieldModifier::Optional => quote! {
                &match self.#ident {
                    Some(v) => {
                        Some(#p::from_i32(v).ok_or_else(
                            || #serde::ser::Error::custom(format!("Invalid enum value {}", v))
                        )?.as_str_name())
                    },
                    None => None,
                }
            },
            FieldModifier::None => quote! {
                #p::from_i32(self.#ident).ok_or_else(
                    || #serde::ser::Error::custom(format!("Invalid enum value {}", self.#ident))
                )?.as_str_name()
            },
        },
        _ => quote! {
            &self.#ident
        },
    };

    Ok(quote! {
        state.serialize_field(#ident_str, { #serialize_stmt })?;
    })
}
