use crate::utils::parse_utils;
use proc_macro2::Ident;
use syn::punctuated::Punctuated;
use syn::Token;
use syn::{parenthesized, parse::Parse, Expr, LitStr};

#[allow(dead_code)]
#[derive(Default, Debug)]
pub(crate) struct FormSchemaAttr {
    pub(crate) status_codes: Vec<Expr>,
    pub(crate) operation_id: Option<Expr>,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) doc_comments: Option<Vec<String>>,
    pub(crate) deprecated: Option<bool>,
}

impl Parse for FormSchemaAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        const EXPECTED_ATTRIBUTE_MESSAGE: &str =
            "unexpected identifier, expected any of: operation_id, request_body, responses, parameters, tags, security";
        let mut attr = FormSchemaAttr::default();

        while !input.is_empty() {
            let ident = input.parse::<Ident>().map_err(|error| {
                syn::Error::new(
                    error.span(),
                    format!("{EXPECTED_ATTRIBUTE_MESSAGE}, {error}"),
                )
            })?;
            match &*ident.to_string() {
                "operation_id" => {
                    attr.operation_id =
                        Some(parse_utils::parse_next(input, || Expr::parse(input))?);
                }
                "status_codes" => {
                    let status_codes;
                    parenthesized!(status_codes in input);
                    attr.status_codes =
                        Punctuated::<Expr, Token![,]>::parse_terminated(&status_codes)
                            .map(|punctuated| punctuated.into_iter().collect::<Vec<Expr>>())?;
                }
                "tags" => {
                    let tags;
                    parenthesized!(tags in input);
                    attr.tags = Some(
                        Punctuated::<LitStr, Token![,]>::parse_terminated(&tags).map(
                            |punctuated| {
                                punctuated
                                    .into_iter()
                                    .map(|t| t.value())
                                    .collect::<Vec<_>>()
                            },
                        )?,
                    );
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), EXPECTED_ATTRIBUTE_MESSAGE));
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(attr)
    }
}
