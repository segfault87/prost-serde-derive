use std::iter;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{parse_quote, Data, DataStruct, Error, Fields, FieldsNamed, Path};

use crate::{
    attr::EnumerationTypeAttr,
    context::Context,
    util::{deraw, is_option, wrap_block},
};

struct NamedStructDeserializer<'a> {
    context: &'a Context,
    serde: &'a Path,
    ident: &'a Ident,
    fields: &'a FieldsNamed,
}

impl<'a> NamedStructDeserializer<'a> {
    pub fn new(
        context: &'a Context,
        serde: &'a Path,
        ident: &'a Ident,
        fields: &'a FieldsNamed,
    ) -> Self {
        Self {
            context,
            serde,
            ident,
            fields,
        }
    }

    #[inline]
    fn get_field_idents(&self) -> impl Iterator<Item = &Ident> {
        self.fields.named.iter().map(|v| v.ident.as_ref().unwrap())
    }

    fn expand_field_deserializer_impl(&self) -> (Ident, TokenStream, Vec<Ident>) {
        let serde = self.serde;

        let variants = self
            .get_field_idents()
            .map(|v| Ident::new(&deraw(v).to_case(Case::Pascal), Span::call_site()))
            .collect::<Vec<_>>();
        let field_names =
            itertools::join(self.get_field_idents().map(|v| format!("`{}`", v)), " or ");

        let ident = Ident::new("Field", Span::call_site());
        let ident_visitor = Ident::new(&(ident.to_string() + "Visitor"), Span::call_site());

        let pat_fields = iter::zip(self.get_field_idents().map(deraw), variants.iter()).map(
            |(name, variant)| {
                quote! {
                    #name => Ok(#ident::#variant)
                }
            },
        );

        let expr = quote! {
            enum #ident {
                #(#variants),*
            }

            impl<'de> #serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<#ident, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    struct #ident_visitor;

                    impl<'de> #serde::de::Visitor<'de> for #ident_visitor {
                        type Value = #ident;

                        fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            formatter.write_str(#field_names)
                        }

                        fn visit_str<E>(self, value: &str) -> Result<#ident, E>
                        where
                            E: #serde::de::Error,
                        {
                            match value {
                                #(#pat_fields,)*
                                _ => Err(#serde::de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(#ident_visitor)
                }
            }
        };

        (ident, expr, variants)
    }

    fn expand_visitor_impl(&self) -> Result<(Ident, TokenStream), ()> {
        let serde = self.serde;

        let ident = self.ident;
        let expecting = format!("struct {}", ident);
        let visitor_ident = Ident::new("Visitor", Span::call_site());

        let (field_enum_ident, field_deserializer, field_variants) =
            self.expand_field_deserializer_impl();

        let mut var_decls = vec![];
        let mut var_pat_fields = vec![];
        let mut var_narrowings = vec![];
        let mut var_fields = vec![];

        for (field, field_variant) in iter::zip(self.fields.named.iter(), field_variants.iter()) {
            let field_ident = field.ident.as_ref().unwrap();
            let field_name = field_ident.to_string();
            let is_optional = is_option(&field.ty);
            var_decls.push(quote! {
                let mut #field_ident = None;
            });

            let enum_attr = EnumerationTypeAttr::from_ast(self.context, &field.attrs)?;

            let value_getter_expr = match enum_attr {
                Some(enum_attr) => {
                    let path = &enum_attr.typ;
                    quote! {
                        {
                            let string_value: String = map.next_value()?;
                            match #path::from_str_name(&string_value) {
                                Some(v) => v as i32,
                                None => return Err(#serde::de::Error::unknown_variant(&string_value, &[])),
                            }
                        }
                    }
                }
                None => quote! {
                    map.next_value()?
                },
            };

            var_pat_fields.push(quote! {
                #field_enum_ident::#field_variant => {
                    if #field_ident.is_some() {
                        return Err(#serde::de::Error::duplicate_field(#field_name));
                    }
                    #field_ident = Some(#value_getter_expr);
                }
            });
            if !is_optional {
                var_narrowings.push(quote! {
                    let #field_ident = #field_ident.ok_or_else(|| #serde::de::Error::missing_field(#field_name))?;
                });
            }
            var_fields.push(field_ident);
        }

        let expr = quote! {
            #field_deserializer

            struct #visitor_ident;

            impl<'de> #serde::de::Visitor<'de> for #visitor_ident {
                type Value = #ident;

                fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    formatter.write_str(#expecting)
                }

                fn visit_map<V>(self, mut map: V) -> Result<#ident, V::Error>
                where
                    V: #serde::de::MapAccess<'de>,
                {
                    #(#var_decls)*
                    while let Some(key) = map.next_key()? {
                        match key {
                            #(#var_pat_fields),*
                        };
                    }
                    #(#var_narrowings)*

                    Ok(#ident {
                        #(#var_fields),*
                    })
                }
            }
        };

        Ok((visitor_ident, expr))
    }

    pub fn expand(&self) -> Result<TokenStream, ()> {
        let ident_name = self.ident.to_string();
        let fields = self
            .get_field_idents()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let (visitor_ident, visitor_impl) = self.expand_visitor_impl()?;

        Ok(quote! {
            #visitor_impl

            const FIELDS: &'static [&'static str] = &[ #(#fields), * ];
            deserializer.deserialize_struct(#ident_name, &FIELDS, #visitor_ident)
        })
    }
}

fn expand_struct(
    context: &Context,
    serde: &Path,
    ident: &Ident,
    data: &DataStruct,
) -> Result<TokenStream, ()> {
    match &data.fields {
        Fields::Named(f) => NamedStructDeserializer::new(context, serde, ident, f).expand(),
        Fields::Unnamed(_) => {
            context.error_spanned_by(&data.fields, "Not implemented");
            Err(())
        }
        Fields::Unit => {
            context.error_spanned_by(
                &data.fields,
                "Unit struct is not available for deserialization.",
            );
            Err(())
        }
    }
}

pub fn expand_deserialize(ident: &Ident, data: &Data) -> Result<TokenStream, Vec<Error>> {
    let context = Context::new();

    let serde: Path = parse_quote! { _serde };

    let deserialization_block = match data {
        Data::Struct(d) => expand_struct(&context, &serde, ident, d),
        Data::Enum(d) => {
            context.error_spanned_by(d.enum_token, "Not implemented");
            Err(())
        }
        Data::Union(d) => {
            context.error_spanned_by(
                d.union_token,
                "Union type is not available for deserialization.",
            );
            Err(())
        }
    }
    .unwrap_or_else(|_| quote! {});

    context.check()?;

    let impl_body = quote! {
        impl<'de> #serde::Deserialize<'de> for #ident {

            fn deserialize<D>(deserializer: D) -> Result<#ident, D::Error>
            where D: #serde::Deserializer<'de>,
            {
                #deserialization_block
            }

        }
    };

    Ok(wrap_block(impl_body))
}
