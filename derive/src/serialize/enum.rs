use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Path};

use crate::context::Context;

pub fn expand_enum(_context: &Context, _serde: &Path, _data: &DataEnum) -> Result<TokenStream, ()> {
    Ok(quote! {
        serializer.serialize_str(self.as_str_name())
    })
}
