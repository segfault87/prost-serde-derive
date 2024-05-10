use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_quote, Attribute, Expr, ExprLit, Lit, Meta, Path};

use crate::context::Context;
use crate::util::{into_syn_error, parse_meta_args_from_attrs, set_option_or_err, PathExt};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProstType {
    Message,
    Enum,
    Oneof,
}

pub struct DeriveMeta {
    pub prost_type: ProstType,
    pub omit_type_errors: bool,
    pub use_default_for_missing_fields: bool,
    pub ignore_unknown_fields: bool,
}

impl DeriveMeta {
    fn from_attributes(attributes: &[Attribute]) -> Result<Self, syn::Error> {
        #[derive(Default)]
        pub struct DeriveMetaDefault {
            pub prost_type: Option<ProstType>,
            pub omit_type_errors: bool,
            pub use_default_for_missing_fields: bool,
            pub ignore_unknown_fields: bool,
        }

        let mut derive_meta = DeriveMetaDefault::default();

        let ident_default_derive = format_ident!("derive");
        {
            let meta_args = parse_meta_args_from_attrs(attributes, &ident_default_derive, true)?;

            let prost_message: Path = parse_quote!(::prost::Message);
            let prost_enumeration: Path = parse_quote!(::prost::Enumeration);
            let prost_oneof: Path = parse_quote!(::prost::Oneof);

            for meta in meta_args {
                if let Meta::Path(p) = &meta {
                    if *p == prost_message {
                        set_option_or_err(&mut derive_meta.prost_type, p, ProstType::Message)?;
                    } else if *p == prost_enumeration {
                        set_option_or_err(&mut derive_meta.prost_type, p, ProstType::Enum)?;
                    } else if *p == prost_oneof {
                        set_option_or_err(&mut derive_meta.prost_type, p, ProstType::Oneof)?;
                    }
                }
            }
        }
        {
            let ident_derive = format_ident!("prost_serde_derive");
            let ident_omit_type_errors = format_ident!("omit_type_errors");
            let ident_use_default_for_missing_fields =
                format_ident!("use_default_for_missing_fields");
            let ident_ignore_unknown_fields = format_ident!("ignore_unknown_fields");

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
        }

        Ok(DeriveMeta {
            prost_type: derive_meta.prost_type.ok_or_else(|| {
                into_syn_error(
                    &ident_default_derive,
                    "missing prost type(::prost:Message, ::prost::Enumeration, or ::prost::Oneof)",
                )
            })?,
            omit_type_errors: derive_meta.omit_type_errors,
            use_default_for_missing_fields: derive_meta.use_default_for_missing_fields,
            ignore_unknown_fields: derive_meta.ignore_unknown_fields,
        })
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

#[derive(Clone, Copy)]
pub enum ProstBytesType {
    Bytes,
    Vec,
}

#[derive(Clone)]
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
    OneOf(Path),
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
                    "oneof" => Ok(Self::OneOf(syn::parse_str(&value_literal)?)),
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

#[derive(Clone)]
pub enum Tag {
    Tag(i32),
    OneofTag(Vec<i32>),
}

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

                Ok(Self::Tag(tag))
            } else if nv.path.is_ident("tags") {
                let value_literal = match &nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) => lit.value(),
                    _ => {
                        return Err(into_syn_error(&nv.value, "should be a string literal"));
                    }
                };

                let Ok(tags) = value_literal.split(", ").map(str::parse::<i32>).collect() else {
                    return Err(into_syn_error(&nv.value, "invalid tag values"));
                };

                Ok(Self::OneofTag(tags))
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
