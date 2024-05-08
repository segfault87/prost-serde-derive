use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path};

use crate::attr::{DeriveMeta, FieldModifier, ProstAttr, ProtobufType};
use crate::context::Context;

pub struct FieldVisitorTokenStream {
    pub value_getter_expr: TokenStream,
    pub narrowing_expr: TokenStream,
}

#[allow(unused)]
pub struct FieldVisitorTokenGenerator<'a> {
    context: &'a Context,
    meta: &'a DeriveMeta,
    serde: &'a Path,
}

impl<'a> FieldVisitorTokenGenerator<'a> {
    pub fn new(context: &'a Context, meta: &'a DeriveMeta, serde: &'a Path) -> Self {
        Self {
            context,
            meta,
            serde,
        }
    }

    pub fn get_value_getter_expr(&self, prost_attr: &ProstAttr) -> TokenStream {
        match prost_attr.modifier {
            FieldModifier::None => self.get_none_value_getter_expr(prost_attr),
            FieldModifier::Repeated => self.get_repeated_value_getter_expr(prost_attr),
            FieldModifier::Optional => self.get_optional_value_getter_expr(prost_attr),
        }
    }

    fn get_none_value_getter_expr(&self, prost_attr: &ProstAttr) -> TokenStream {
        let serde = self.serde;
        let defaut_value = prost_attr.get_default_value();

        match prost_attr.ty {
            ProtobufType::Enumeration(ref path) => self.value_getter(
                Some(quote! { String }),
                quote! {
                    Some(match #path::from_str_name(&value) {
                        Some(v) => v.into(),
                        None => return Err(#serde::de::Error::unknown_variant(&value, &[])),
                    })
                },
                defaut_value,
            ),
            ProtobufType::Bytes(_) => self.value_getter(
                Some(quote! { String }),
                quote! {
                    Some({
                        extern crate base64 as _base64;
                        match _base64::decode(&value) {
                            Ok(v) => v.into(),
                            Err(_) => return Err(#serde::de::Error::invalid_value(#serde::de::Unexpected::Str(&value), &"A base64 string")),
                        }
                    })
                },
                defaut_value,
            ),
            _ => self.value_getter(
                None,
                quote! { Some(value) },
                defaut_value,
            ),
        }
    }

    fn get_repeated_value_getter_expr(&self, prost_attr: &ProstAttr) -> TokenStream {
        let serde = self.serde;
        let default_value = prost_attr.get_default_value();

        match prost_attr.ty {
            ProtobufType::Enumeration(ref path) => self.value_getter(
                Some(quote! { Vec<String> }),
                quote! {
                    Some({
                        let mut result = vec![];
                        for value in value.iter() {
                            match #path::from_str_name(&value) {
                                Some(v) => {
                                    result.push(v.into());
                                }
                                None => {
                                    return Err(#serde::de::Error::unknown_variant(&value, &[]));
                                }
                            }
                        }
                        result
                    })
                },
                default_value,
            ),
            ProtobufType::Bytes(_) => self.value_getter(
                Some(quote! { Vec<String> }),
                quote! {
                    Some({
                        extern crate base64 as _base64;
                        let mut result = vec![];
                        for value in value.iter() {
                            match _base64::decode(value) {
                                Ok(v) => {
                                    result.push(v.into());
                                },
                                Err(_) => {
                                    return Err(
                                        #serde::de::Error::invalid_value(
                                            #serde::de::Unexpected::Str(value),
                                            &"a base64 string",
                                        ),
                                    );
                                }
                            }
                        }
                        result
                    })
                },
                default_value,
            ),
            _ => self.value_getter(
                Some(quote! { Vec<_> }),
                quote! { Some(value) },
                default_value,
            ),
        }
    }

    fn get_optional_value_getter_expr(&self, prost_attr: &ProstAttr) -> TokenStream {
        let serde = self.serde;
        let default_value = prost_attr.get_default_value();

        match prost_attr.ty {
            ProtobufType::Enumeration(ref path) => self.value_getter(
                Some(quote! { Option<String> }),
                quote! {
                    match &value {
                        Some(v) => match #path::from_str_name(v) {
                            Some(v) => Some(v.into()),
                            None => return Err(#serde::de::Error::unknown_variant(v, &[])),
                        },
                        None => None,
                    }
                },
                default_value,
            ),
            ProtobufType::Bytes(_) => self.value_getter(
                Some(quote! { Option<String> }),
                quote! {
                    if let Some(value) = value.as_ref() {
                        Some({
                            extern crate base64 as _base64;
                            match _base64::decode(&value) {
                                Ok(v) => v.into(),
                                Err(_) => return Err(#serde::de::Error::invalid_value(#serde::de::Unexpected::Str(&value), &"a base64 string")),
                            }
                        })
                    } else {
                        None
                    }
                },
                default_value,
            ),
            _ => self.value_getter(None, quote! { value }, default_value),
        }
    }

    fn value_getter(
        &self,
        type_sig: Option<TokenStream>,
        expr: TokenStream,
        default_value: TokenStream,
    ) -> TokenStream {
        let getter = match type_sig {
            Some(v) => quote! { map.next_value::<#v>() },
            None => quote! { map.next_value() },
        };

        if self.meta.omit_type_errors {
            quote! {
                match #getter {
                    Ok(value) => #expr,
                    Err(_) => Some(#default_value)
                }
            }
        } else {
            quote! {
                {
                    let value = #getter?;
                    #expr
                }
            }
        }
    }

    pub fn expand(
        &self,
        prost_attr: &ProstAttr,
        field_name: &String,
        ident_field_var: &Ident,
    ) -> FieldVisitorTokenStream {
        let serde = self.serde;
        let default_value = prost_attr.get_default_value();

        let value_getter_expr = self.get_value_getter_expr(prost_attr);

        let narrowing_expr = match prost_attr.modifier {
            FieldModifier::None => {
                if self.meta.use_default_for_missing_fields {
                    quote! {
                        let #ident_field_var = #ident_field_var.unwrap_or(#default_value);
                    }
                } else {
                    quote! {
                        let #ident_field_var = #ident_field_var.ok_or_else(|| #serde::de::Error::missing_field(#field_name))?;
                    }
                }
            }
            FieldModifier::Repeated => {
                quote! {
                    let #ident_field_var = #ident_field_var.unwrap_or(vec![]);
                }
            }
            _ => quote! {},
        };

        FieldVisitorTokenStream {
            value_getter_expr,
            narrowing_expr,
        }
    }
}
