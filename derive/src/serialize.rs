use proc_macro2::{Ident, TokenStream};
use syn::{parse_quote, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Path};

use crate::{
    attr::{DeriveMeta, FieldModifier, ProstAttr, ProtobufType},
    context::Context,
    util::{deraw, wrap_block},
};

fn serialize_field(context: &Context, serde: &Path, field: &Field) -> Result<TokenStream, ()> {
    let ident = field.ident.as_ref().ok_or_else(|| {
        context.error_spanned_by(field, "Field should have a name");
    })?;
    let ident_str = deraw(ident);

    let prost_attr = ProstAttr::from_ast(context, &field.attrs)?;

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
                        &self.#ident.map(_base64::encode)
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

fn expand_struct(
    context: &Context,
    _meta: &DeriveMeta,
    serde: &Path,
    ident: &Ident,
    data: &DataStruct,
) -> Result<TokenStream, ()> {
    match &data.fields {
        Fields::Named(f) => {
            let ident_name = ident.to_string();
            let fields = f
                .named
                .iter()
                .map(|v| serialize_field(context, serde, v))
                .collect::<Result<Vec<TokenStream>, ()>>()?;
            let count = f.named.len();
            Ok(quote! {
                use #serde::ser::SerializeStruct;

                let mut state = serializer.serialize_struct(#ident_name, #count)?;
                #(#fields)*
                state.end()
            })
        }
        Fields::Unnamed(_) => {
            context.error_spanned_by(
                &data.fields,
                "Unnamed struct is not available for serialization.",
            );
            Err(())
        }
        Fields::Unit => {
            context.error_spanned_by(
                &data.fields,
                "Unit struct is not available for serialization.",
            );
            Err(())
        }
    }
}

fn expand_enum(_context: &Context, _serde: &Path, _data: &DataEnum) -> Result<TokenStream, ()> {
    Ok(quote! {
        serializer.serialize_str(self.as_str_name())
    })
}

pub fn expand_serialize(input: DeriveInput) -> Result<TokenStream, Vec<Error>> {
    let context = Context::new();

    let derive_meta = match DeriveMeta::from_ast(&context, &input.attrs) {
        Ok(v) => v,
        Err(_) => {
            context.check()?;
            return Err(vec![]); // This never happens becuase `context.check()` always returns errors
        }
    };

    let ident = &input.ident;
    let data = &input.data;

    let serde: Path = parse_quote! { _serde };

    let serialization_block = match data {
        Data::Struct(d) => expand_struct(&context, &derive_meta, &serde, ident, d),
        Data::Enum(d) => expand_enum(&context, &serde, d),
        Data::Union(d) => {
            context.error_spanned_by(
                d.union_token,
                "Union type is not available for serialization.",
            );
            Err(())
        }
    }
    .unwrap_or_else(|_| quote! {});

    context.check()?;

    let impl_body = quote! {
        impl #serde::Serialize for #ident {

            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: #serde::Serializer,
            {
                #serialization_block
            }

        }
    };

    Ok(wrap_block(impl_body))
}
