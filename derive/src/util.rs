use std::fmt::Display;

use extend::ext;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Meta, Path, Token};

pub fn into_syn_error<A: ToTokens, T: Display>(obj: A, msg: T) -> syn::Error {
    syn::Error::new_spanned(obj.into_token_stream(), msg)
}

pub fn wrap_block(code: TokenStream) -> TokenStream {
    let dummy_const = format_ident!("_");

    quote! {
        const #dummy_const: () = {
            #code
        };
    }
}

pub fn parse_meta_args_from_attrs(
    attrs: &[Attribute],
    ident: &Ident,
    is_strict: bool,
) -> Result<Vec<Meta>, syn::Error> {
    let mut meta_args = Vec::new();
    for attr in attrs.iter() {
        if attr.meta.path().is_ident(ident) {
            if let Meta::List(meta_list) = &attr.meta {
                meta_args.extend(
                    meta_list
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                        .into_iter(),
                );
            } else if is_strict {
                return Err(into_syn_error(&attr.meta, "is not a structured list"));
            }
        }
    }

    Ok(meta_args)
}

pub fn set_option_or_err<T, A: ToTokens>(
    option: &mut Option<T>,
    obj: A,
    value: T,
) -> Result<(), syn::Error> {
    if option.is_some() {
        Err(into_syn_error(obj, "duplicate attribute"))
    } else {
        *option = Some(value);
        Ok(())
    }
}

#[ext]
pub impl Path {
    #[inline]
    fn get_ident_or_err(&self) -> Result<&Ident, syn::Error> {
        self.get_ident()
            .ok_or_else(|| into_syn_error(self, "invalid directive"))
    }
}
