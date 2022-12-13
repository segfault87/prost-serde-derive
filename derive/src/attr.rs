use proc_macro2::Span;
use syn::{Attribute, Ident, Lit, Meta, NestedMeta, Path};

use crate::context::Context;

pub struct EnumerationTypeAttr {
    pub typ: Path,
}

impl EnumerationTypeAttr {
    pub fn from_ast(context: &Context, attrs: &[Attribute]) -> Result<Option<Self>, ()> {
        let prost = Ident::new("prost", Span::call_site());
        let enumeration = Ident::new("enumeration", Span::call_site());

        let mut found = None;
        for attr in attrs.iter() {
            let meta = attr.parse_meta();

            let meta_list = match meta {
                Ok(Meta::List(list)) => list,
                Ok(_) => {
                    context.error_spanned_by(attr, "");
                    return Err(());
                }
                Err(e) => {
                    context.syn_error(e);
                    return Err(());
                }
            };

            if meta_list.path.is_ident(&prost) {
                let nested = &meta_list.nested;
                for nested_meta in nested.iter() {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
                        if nv.path.is_ident(&enumeration) {
                            match &nv.lit {
                                Lit::Str(s) => {
                                    if found.is_some() {
                                        context.error_spanned_by(
                                            nv,
                                            "Multiple `enumeration` attributes are not allowed.",
                                        );
                                        return Err(());
                                    }
                                    let path = s.value();
                                    found = Some(EnumerationTypeAttr {
                                        typ: syn::parse_str(&path).unwrap(),
                                    });
                                    break;
                                }
                                _ => {
                                    context.error_spanned_by(
                                        nv,
                                        "`enumeration` attribute should be in string literal.",
                                    );
                                    return Err(());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(found)
    }
}
