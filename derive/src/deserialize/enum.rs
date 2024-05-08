use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Path;

pub fn expand_enum(serde: &Path, deserializer: &Ident, ident: &Ident) -> Result<TokenStream, ()> {
    EnumDeserializer::new(serde, deserializer, ident).expand()
}

struct EnumDeserializer<'a> {
    serde: &'a Path,
    deserializer: &'a Ident,
    ident: &'a Ident,
}

impl<'a> EnumDeserializer<'a> {
    pub fn new(serde: &'a Path, deserializer: &'a Ident, ident: &'a Ident) -> Self {
        Self {
            serde,
            deserializer,
            ident,
        }
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let serde = self.serde;
        let deserializer = self.deserializer;
        let ident = self.ident;

        Ok(quote! {
            let s = String::deserialize(#deserializer)?;
            #ident::from_str_name(&s).ok_or(#serde::de::Error::unknown_variant(&s, &[]))
        })
    }
}
