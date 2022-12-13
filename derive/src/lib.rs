#[macro_use]
extern crate quote;

mod attr;
mod context;
mod deserialize;
mod util;

use crate::deserialize::expand_deserialize;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(Error::to_compile_error);
    quote!(#(#compile_errors)*)
}

#[proc_macro_derive(Deserialize, attributes(enumeration))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand_deserialize(&input.ident, &input.data)
        .unwrap_or_else(to_compile_errors)
        .into()
}
