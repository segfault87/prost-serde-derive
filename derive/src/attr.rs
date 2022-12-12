use syn::{Attribute, Meta, NestedMeta, Path};

use crate::context::Context;

pub struct EnumerationTypeAttr {
    pub typ: Path,
}

impl EnumerationTypeAttr {
    pub fn from_ast(context: &Context, attrs: &[Attribute]) -> Result<Option<Self>, ()> {
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

            if meta_list.path.get_ident().map(ToString::to_string)
                == Some("enumeration".to_string())
            {
                let nested = &meta_list.nested;
                match nested.len() {
                    1 => {}
                    _ => {
                        context.error_spanned_by(nested, "should be #[enumeration(some::Type)]");
                        return Err(());
                    }
                }

                if found.is_some() {
                    context.error_spanned_by(
                        nested,
                        "multiple #[enumeration] attributes are not allowed",
                    );
                    return Err(());
                }

                let path = match nested.first().unwrap() {
                    NestedMeta::Meta(Meta::Path(p)) => p,
                    _ => {
                        context.error_spanned_by(nested, "should be #[enumeration(some::Type)]");
                        return Err(());
                    }
                };

                found = Some(EnumerationTypeAttr { typ: path.clone() });
            }
        }

        Ok(found)
    }
}
