mod attr;
mod context;
mod deserialize;
mod serialize;
mod util;

use quote::quote;

use crate::{deserialize::expand_deserialize, serialize::expand_serialize};

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(Error::to_compile_error);
    quote!(#(#compile_errors)*)
}

#[proc_macro_derive(Deserialize, attributes(prost, prost_serde_derive))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_deserialize(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

#[proc_macro_derive(Serialize, attributes(prost, prost_serde_derive))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_serialize(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}
