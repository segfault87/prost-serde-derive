use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{GenericArgument, Ident, PathArguments, Type};

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

pub fn deraw(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

pub fn ungroup(mut ty: &Type) -> &Type {
    while let Type::Group(group) = ty {
        ty = &group.elem;
    }

    ty
}

pub fn is_option(ty: &Type) -> bool {
    let path = match ungroup(ty) {
        Type::Path(ty) => &ty.path,
        _ => return false,
    };
    let seg = match path.segments.last() {
        Some(seg) => seg,
        None => return false,
    };
    let args = match &seg.arguments {
        PathArguments::AngleBracketed(bracketed) => &bracketed.args,
        _ => return false,
    };
    seg.ident == "Option" && args.len() == 1 && matches!(&args[0], GenericArgument::Type(_))
}
