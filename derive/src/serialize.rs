mod r#enum;
mod field;
mod r#struct;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, Data, DeriveInput, Error, Path};

use self::r#enum::{expand_enum, expand_oneof_field_name_method};
use self::r#struct::expand_struct;
use crate::attr::{DeriveMeta, ProstType};
use crate::context::Context;
use crate::util::wrap_block;

pub fn expand_serialize(input: DeriveInput) -> Result<TokenStream, Vec<Error>> {
    let context = Context::new();

    let Ok(derive_meta) = DeriveMeta::from_ast(&context, &input.attrs) else {
        context.check()?;
        unreachable!()
    };

    let ident = &input.ident;
    let data = &input.data;

    let serde: Path = parse_quote! { _serde };
    let serializer = format_ident!("serializer");

    let Ok(serialization_block) = (match data {
        Data::Struct(d) => {
            if derive_meta.prost_type == ProstType::Message {
                expand_struct(&context, &derive_meta, &serde, &serializer, ident, d)
            } else {
                context.push_error_spanned_by(
                    d.struct_token,
                    "Struct type is only available for `::prost::Message`.",
                );
                Err(())
            }
        }
        Data::Enum(d) => {
            if derive_meta.prost_type == ProstType::Enum
                || derive_meta.prost_type == ProstType::Oneof
            {
                expand_enum(&context, &derive_meta, &serde, &serializer, ident, d)
            } else {
                context.push_error_spanned_by(
                    d.enum_token,
                    "Enum type is only available for `::prost::Enumeration` or `::prost::Oneof`.",
                );
                Err(())
            }
        }
        Data::Union(d) => {
            context.push_error_spanned_by(
                d.union_token,
                "Union type is not available for serialization.",
            );
            Err(())
        }
    }) else {
        context.check()?;
        unreachable!();
    };

    let oneof_field_name_method = expand_oneof_field_name_method(&derive_meta, ident, data);

    let impl_body = quote! {
        extern crate serde as _serde;

        impl #serde::Serialize for #ident {
            fn serialize<S>(&self, #serializer: S) -> Result<S::Ok, S::Error>
            where S: #serde::Serializer,
            {
                #serialization_block
            }

        }

        #oneof_field_name_method
    };

    Ok(wrap_block(impl_body))
}
