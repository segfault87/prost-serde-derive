mod r#enum;
mod field;
mod r#struct;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error, Ident, Path};

use self::r#enum::expand_enum;
use self::r#struct::expand_struct;
use crate::attr::DeriveMeta;
use crate::context::Context;
use crate::util::wrap_block;

pub fn expand_deserialize(input: DeriveInput) -> Result<TokenStream, Vec<Error>> {
    let context = Context::new();

    let Ok(derive_meta) = DeriveMeta::from_ast(&context, &input.attrs) else {
        context.check()?;
        unreachable!()
    };

    let ident = &input.ident;
    let data = &input.data;

    let serde: Path = parse_quote! { _serde };
    let deserializer: Ident = parse_quote! { deserializer };

    let Ok(deserialization_block) = (match data {
        Data::Struct(d) => expand_struct(&context, &derive_meta, &serde, &deserializer, ident, d),
        Data::Enum(_) => expand_enum(&serde, &deserializer, ident),
        Data::Union(d) => {
            context.push_error_spanned_by(
                d.union_token,
                "Union type is not available for deserialization.",
            );
            Err(())
        }
    }) else {
        context.check()?;
        unreachable!();
    };

    let impl_body = quote! {
        extern crate serde as _serde;

        impl<'de> #serde::Deserialize<'de> for #ident {
            fn deserialize<D>(#deserializer: D) -> Result<#ident, D::Error>
            where D: #serde::Deserializer<'de>,
            {
                #deserialization_block
            }

        }
    };

    Ok(wrap_block(impl_body))
}
