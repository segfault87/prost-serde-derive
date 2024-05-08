use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Expr, ExprLit, Lit, Meta, Path};

use crate::context::Context;
use crate::util::{into_syn_error, parse_meta_args_from_attrs, set_option_or_err, PathExt};

#[derive(Default)]
pub struct DeriveMeta {
    pub omit_type_errors: bool,
    pub use_default_for_missing_fields: bool,
    pub ignore_unknown_fields: bool,
}

impl DeriveMeta {
    fn from_attributes(attributes: &[Attribute]) -> Result<Self, syn::Error> {
        let ident_derive = format_ident!("prost_serde_derive");
        let ident_omit_type_errors = format_ident!("omit_type_errors");
        let ident_use_default_for_missing_fields = format_ident!("use_default_for_missing_fields");
        let ident_ignore_unknown_fields = format_ident!("ignore_unknown_fields");

        let mut derive_meta = DeriveMeta::default();

        let meta_args = parse_meta_args_from_attrs(attributes, &ident_derive, true)?;

        for meta in meta_args {
            if let Meta::Path(p) = &meta {
                if p.is_ident(&ident_omit_type_errors) {
                    derive_meta.omit_type_errors = true;
                } else if p.is_ident(&ident_use_default_for_missing_fields) {
                    derive_meta.use_default_for_missing_fields = true;
                } else if p.is_ident(&ident_ignore_unknown_fields) {
                    derive_meta.ignore_unknown_fields = true;
                } else {
                    return Err(into_syn_error(meta, "unrecognized option"));
                }
            } else {
                return Err(into_syn_error(meta, "unrecognized option"));
            }
        }

        Ok(derive_meta)
    }

    pub fn from_ast(context: &Context, attributes: &[Attribute]) -> Result<DeriveMeta, ()> {
        match Self::from_attributes(attributes) {
            Ok(v) => Ok(v),
            Err(e) => {
                context.push_syn_error(e);
                Err(())
            }
        }
    }
}

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

impl TryFrom<&Meta> for ProtobufType {
    type Error = syn::Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        match value {
            Meta::NameValue(nv) => {
                let value_literal = match &nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) => lit.value(),
                    _ => {
                        return Err(into_syn_error(&nv.value, "should be a string literal"));
                    }
                };

                let ident = nv.path.get_ident_or_err()?;

                match &ident.to_string()[..] {
                    "enumeration" => Ok(Self::Enumeration(syn::parse_str(&value_literal)?)),
                    "bytes" => match &value_literal[..] {
                        "bytes" => Ok(Self::Bytes(ProstBytesType::Bytes)),
                        "vec" => Ok(Self::Bytes(ProstBytesType::Vec)),
                        _ => Err(into_syn_error(&nv.value, "should be `bytes` or `vec`")),
                    },
                    _ => Err(into_syn_error(ident, "unrecognized type")),
                }
            }
            Meta::Path(p) => {
                let ident = p.get_ident_or_err()?;

                match &ident.to_string()[..] {
                    "string" => Ok(ProtobufType::String),
                    "message" => Ok(ProtobufType::Message),
                    "bool" => Ok(ProtobufType::Bool),
                    "int32" => Ok(ProtobufType::Int32),
                    "fixed32" | "sfixed32" => Ok(ProtobufType::Fixed32),
                    "uint32" => Ok(ProtobufType::Uint32),
                    "int64" => Ok(ProtobufType::Int64),
                    "fixed64" | "sfixed64" => Ok(ProtobufType::Fixed64),
                    "uint64" => Ok(ProtobufType::Uint64),
                    "float" => Ok(ProtobufType::Float),
                    "double" => Ok(ProtobufType::Double),
                    _ => Err(into_syn_error(ident, "unrecognized type")),
                }
            }
            _ => Err(into_syn_error(value, "invalid directive")),
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum FieldModifier {
    #[default]
    None,
    Repeated,
    Optional,
}

impl TryFrom<&Meta> for FieldModifier {
    type Error = syn::Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        if let Meta::Path(p) = value {
            let ident = p.get_ident_or_err()?;

            match &ident.to_string()[..] {
                "repeated" => Ok(Self::Repeated),
                "optional" => Ok(Self::Optional),
                _ => Err(into_syn_error(ident, "unrecognized modifier")),
            }
        } else {
            Err(into_syn_error(value, "invalid directive"))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Tag(pub i32);

impl TryFrom<&Meta> for Tag {
    type Error = syn::Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        if let Meta::NameValue(nv) = value {
            if nv.path.is_ident("tag") {
                let value_literal = match &nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) => lit.value(),
                    _ => {
                        return Err(into_syn_error(&nv.value, "should be a string literal"));
                    }
                };

                let Ok(tag) = value_literal.parse::<i32>() else {
                    return Err(into_syn_error(&nv.value, "invalid tag value"));
                };

                Ok(Self(tag))
            } else {
                Err(into_syn_error(&nv.path, "invalid directive"))
            }
        } else {
            Err(into_syn_error(value, "invalid directive"))
        }
    }
}

pub struct ProstAttr {
    pub ty: ProtobufType,
    pub modifier: FieldModifier,
    pub tag: Tag,
}

impl ProstAttr {
    fn from_attributes(attrs: &[Attribute]) -> Result<Self, syn::Error> {
        let ident_prost = format_ident!("prost");
        let ident_tag = format_ident!("tag");

        let meta_args = parse_meta_args_from_attrs(attrs, &ident_prost, false)?;

        let mut ty = None;
        let mut modifier = None;
        let mut tag = None;

        for meta in meta_args {
            if let Ok(t) = ProtobufType::try_from(&meta) {
                set_option_or_err(&mut ty, meta, t)?;
            } else if let Ok(m) = FieldModifier::try_from(&meta) {
                set_option_or_err(&mut modifier, meta, m)?;
            } else if let Ok(t) = Tag::try_from(&meta) {
                set_option_or_err(&mut tag, meta, t)?;
            }
        }

        Ok(Self {
            ty: ty.ok_or_else(|| into_syn_error(&ident_prost, "missing type"))?,
            modifier: modifier.unwrap_or_default(),
            tag: tag.ok_or_else(|| into_syn_error(&ident_tag, "missing tag"))?,
        })
    }

    pub fn from_ast(context: &Context, attrs: &[Attribute]) -> Result<Self, ()> {
        match Self::from_attributes(attrs) {
            Ok(v) => Ok(v),
            Err(e) => {
                context.push_syn_error(e);
                Err(())
            }
        }
    }

    pub fn get_default_value(&self) -> TokenStream {
        match self.modifier {
            FieldModifier::None => match &self.ty {
                ProtobufType::Enumeration(p) => quote! { #p::default() as i32 },
                _ => quote! { Default::default() },
            },
            _ => quote! { Default::default() },
        }
    }
}
