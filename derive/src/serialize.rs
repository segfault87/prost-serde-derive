mod r#enum;
mod field;
mod r#struct;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Data, DeriveInput, Error, Path};

use self::r#enum::expand_enum;
use self::r#struct::expand_struct;
use crate::attr::DeriveMeta;
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

    let Ok(serialization_block) = (match data {
        Data::Struct(d) => expand_struct(&context, &derive_meta, &serde, ident, d),
        Data::Enum(d) => expand_enum(&context, &serde, d),
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

    let impl_body = quote! {
        extern crate serde as _serde;

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
