use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DataStruct, Fields, Path};

use super::field::serialize_field;
use crate::attr::DeriveMeta;
use crate::context::Context;

pub fn expand_struct(
    context: &Context,
    _meta: &DeriveMeta,
    serde: &Path,
    serializer: &Ident,
    ident: &Ident,
    data: &DataStruct,
) -> Result<TokenStream, ()> {
    match &data.fields {
        Fields::Named(f) => {
            let name = ident.to_string();
            let fields = f
                .named
                .iter()
                .map(|v| serialize_field(context, serde, v))
                .collect::<Result<Vec<TokenStream>, ()>>()?;
            let count = f.named.len();
            Ok(quote! {
                use #serde::ser::SerializeStruct;

                let mut state = #serializer.serialize_struct(#name, #count)?;
                #(#fields)*
                state.end()
            })
        }
        Fields::Unnamed(_) => {
            context.push_error_spanned_by(
                &data.fields,
                "Unnamed struct is not available for serialization.",
            );
            Err(())
        }
        Fields::Unit => {
            context.push_error_spanned_by(
                &data.fields,
                "Unit struct is not available for serialization.",
            );
            Err(())
        }
    }
}
