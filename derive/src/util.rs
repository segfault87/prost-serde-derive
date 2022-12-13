use proc_macro2::TokenStream;
use quote::format_ident;
use syn::Ident;

pub fn wrap_block(code: TokenStream) -> TokenStream {
    let dummy_const = format_ident!("_");

    let serde = quote! {
        extern crate serde as _serde;
    };

    quote! {
        const #dummy_const: () = {
            #serde
            #code
        };
    }
}
