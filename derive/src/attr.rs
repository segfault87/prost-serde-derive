use proc_macro2::Span;
use syn::{Attribute, Ident, Lit, Meta, NestedMeta, Path};

use crate::context::Context;

pub enum ProstBytesType {
    Bytes,
    Vec,
}

pub enum ProtobufType {
    Message,
    Enumeration(Path),
    Bool,
    String,
    Bytes(ProstBytesType),
    Int32,
    Fixed32,
    Uint32,
    Int64,
    Fixed64,
    Uint64,
    Float,
    Double,
}

pub enum FieldModifier {
    None,
    Repeated,
    Optional,
}

impl ProtobufType {
    pub fn from_ast(context: &Context, meta: &Meta) -> Result<Self, ()> {
        match meta {
            Meta::NameValue(nv) => {
                let literal = match &nv.lit {
                    Lit::Str(str) => str.value(),
                    _ => {
                        context.error_spanned_by(&nv.lit, "should be a string literal");
                        return Err(());
                    }
                };

                if let Some(ident) = nv.path.get_ident() {
                    match &ident.to_string()[..] {
                        "enumeration" => Ok(Self::Enumeration(syn::parse_str(&literal).unwrap())),
                        "bytes" => match &literal[..] {
                            "bytes" => Ok(Self::Bytes(ProstBytesType::Bytes)),
                            "vec" => Ok(Self::Bytes(ProstBytesType::Vec)),
                            _ => {
                                context.error_spanned_by(&nv.lit, "should be `bytes` or `vec`");
                                Err(())
                            }
                        },
                        _ => {
                            context.error_spanned_by(ident, "unrecognized type");
                            Err(())
                        }
                    }
                } else {
                    Err(())
                }
            }
            Meta::Path(p) => {
                if let Some(ident) = p.get_ident() {
                    match &ident.to_string()[..] {
                        "string" => Ok(ProtobufType::String),
                        "message" => Ok(ProtobufType::Message),
                        "bool" => Ok(ProtobufType::Bool),
                        "int32" => Ok(ProtobufType::Int32),
                        "fixed32" => Ok(ProtobufType::Fixed32),
                        "uint32" => Ok(ProtobufType::Uint32),
                        "int64" => Ok(ProtobufType::Int64),
                        "fixed64" => Ok(ProtobufType::Fixed64),
                        "uint64" => Ok(ProtobufType::Uint64),
                        "float" => Ok(ProtobufType::Float),
                        "double" => Ok(ProtobufType::Double),
                        _ => {
                            context.error_spanned_by(ident, "unrecognized type");
                            Err(())
                        }
                    }
                } else {
                    context.error_spanned_by(p, "invalid directive");
                    Err(())
                }
            }
            _ => {
                context.error_spanned_by(meta, "invalid directive");
                Err(())
            }
        }
    }
}

pub struct ProstAttr {
    pub ty: ProtobufType,
    pub modifier: FieldModifier,
    pub tag: i32,
}

impl ProstAttr {
    pub fn from_ast(context: &Context, attrs: &[Attribute]) -> Result<Self, ()> {
        let prost = Ident::new("prost", Span::call_site());
        let optional_ident = Ident::new("optional", Span::call_site());
        let repeated_ident = Ident::new("repeated", Span::call_site());
        let tag_ident = Ident::new("tag", Span::call_site());

        let mut found = None;
        for attr in attrs.iter() {
            let meta = attr.parse_meta();

            let meta_list = match meta {
                Ok(Meta::List(list)) => list,
                Ok(_) => {
                    continue;
                }
                Err(e) => {
                    context.syn_error(e);
                    return Err(());
                }
            };

            if meta_list.path.is_ident(&prost) {
                let mut modifier = FieldModifier::None;
                let mut tag = None;

                let mut it = meta_list.nested.iter();
                let first = match it.next() {
                    Some(v) => v,
                    None => {
                        context.error_spanned_by(meta_list, "No arguments supplied.");
                        return Err(());
                    }
                };

                let pb_type = if let NestedMeta::Meta(m) = first {
                    ProtobufType::from_ast(context, m)?
                } else {
                    context.error_spanned_by(first, "Invalid directive");
                    return Err(());
                };

                for meta in it.filter_map(|v| {
                    if let NestedMeta::Meta(m) = v {
                        Some(m)
                    } else {
                        None
                    }
                }) {
                    match meta {
                        Meta::Path(p) => {
                            if p.is_ident(&repeated_ident) {
                                if let FieldModifier::None = modifier {
                                    modifier = FieldModifier::Repeated;
                                } else {
                                    context.error_spanned_by(p, "Redundant modifier.");
                                    return Err(());
                                }
                            } else if p.is_ident(&optional_ident) {
                                if let FieldModifier::None = modifier {
                                    modifier = FieldModifier::Optional;
                                } else {
                                    context.error_spanned_by(p, "Redundant modifier.");
                                    return Err(());
                                }
                            }
                        }
                        Meta::NameValue(nv) => {
                            if nv.path.is_ident(&tag_ident) {
                                tag = if let Lit::Str(lit) = &nv.lit {
                                    match lit.value().parse::<i32>() {
                                        Ok(v) => Some(v),
                                        Err(_) => {
                                            context.error_spanned_by(&nv.lit, "Invalid tag value");
                                            return Err(());
                                        }
                                    }
                                } else {
                                    context.error_spanned_by(&nv.lit, "Invalid tag value");
                                    return Err(());
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let tag = if let Some(v) = tag {
                    v
                } else {
                    context.error_spanned_by(&meta_list, "No tag specified");
                    return Err(());
                };

                if found.is_some() {
                    context.error_spanned_by(
                        &meta_list,
                        "There are more than one #[prost] directive.",
                    );
                    return Err(());
                }

                found = Some(ProstAttr {
                    ty: pb_type,
                    modifier,
                    tag,
                });
            }
        }

        match found {
            Some(v) => Ok(v),
            None => {
                static ERROR_MESSAGE: &str =
                    "#[prost()] attribute is required for every field members.";
                if let Some(v) = attrs.first() {
                    context.error_spanned_by(v, ERROR_MESSAGE);
                } else {
                    context.error_spanned_by(prost, ERROR_MESSAGE);
                }
                Err(())
            }
        }
    }
}
