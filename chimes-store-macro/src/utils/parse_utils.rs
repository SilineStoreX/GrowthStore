use std::fmt::Display;

use proc_macro::Span;
use proc_macro2::{Group, Ident, TokenStream};
use proc_macro_error::abort;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, Expr, LitBool, LitStr, Token,
};

#[allow(dead_code)]
trait OptionExt<T> {
    fn expect_or_abort(self, message: &str) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    fn expect_or_abort(self, message: &str) -> T {
        self.unwrap_or_else(|| abort!(Span::call_site(), message))
    }
}

#[allow(dead_code)]
trait ResultExt<T> {
    fn unwrap_or_abort(self) -> T;
    fn expect_or_abort(self, message: &str) -> T;
}

impl<T> ResultExt<T> for Result<T, syn::Error> {
    fn unwrap_or_abort(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => abort!(error.span(), format!("{error}")),
        }
    }

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => abort!(error.span(), format!("{error}: {message}")),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum Value {
    LitStr(LitStr),
    Expr(Expr),
}

impl Value {
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::LitStr(s) if s.value().is_empty())
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::LitStr(LitStr::new("", proc_macro2::Span::call_site()))
    }
}

impl Parse for Value {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            Ok::<Value, Error>(Value::LitStr(input.parse::<LitStr>()?))
        } else {
            Ok(Value::Expr(input.parse::<Expr>()?))
        }
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::LitStr(str) => str.to_tokens(tokens),
            Self::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LitStr(str) => write!(f, "{str}", str = str.value()),
            Self::Expr(expr) => write!(f, "{expr}", expr = expr.into_token_stream()),
        }
    }
}

#[allow(dead_code)]
pub(crate) fn parse_next<T: Sized>(input: ParseStream, next: impl FnOnce() -> T) -> T {
    input
        .parse::<Token![=]>()
        .expect_or_abort("expected equals token before value assignment");
    next()
}

#[allow(dead_code)]
pub(crate) fn parse_next_literal_str(input: ParseStream) -> syn::Result<String> {
    Ok(parse_next(input, || input.parse::<LitStr>())?.value())
}

#[allow(dead_code)]
pub(crate) fn parse_next_literal_str_or_expr(input: ParseStream) -> syn::Result<Value> {
    parse_next(input, || Value::parse(input)).map_err(|error| {
        syn::Error::new(
            error.span(),
            format!("expected literal string or expression argument: {error}"),
        )
    })
}

#[allow(dead_code)]
pub(crate) fn parse_groups<T, R>(input: ParseStream) -> syn::Result<R>
where
    T: Sized,
    T: Parse,
    R: FromIterator<T>,
{
    Punctuated::<Group, Token![,]>::parse_terminated(input).and_then(|groups| {
        groups
            .into_iter()
            .map(|group| syn::parse2::<T>(group.stream()))
            .collect::<syn::Result<R>>()
    })
}

#[allow(dead_code)]
pub(crate) fn parse_punctuated_within_parenthesis<T>(
    input: ParseStream,
) -> syn::Result<Punctuated<T, Token![,]>>
where
    T: Parse,
{
    let content;
    parenthesized!(content in input);
    Punctuated::<T, Token![,]>::parse_terminated(&content)
}

#[allow(dead_code)]
pub(crate) fn parse_bool_or_true(input: ParseStream) -> syn::Result<bool> {
    if input.peek(Token![=]) && input.peek2(LitBool) {
        input.parse::<Token![=]>()?;

        Ok(input.parse::<LitBool>()?.value())
    } else {
        Ok(true)
    }
}

/// Parse `json!(...)` as a [`TokenStream`].
#[allow(dead_code)]
pub(crate) fn parse_json_token_stream(input: ParseStream) -> syn::Result<TokenStream> {
    if input.peek(syn::Ident) && input.peek2(Token![!]) {
        input.parse::<Ident>().and_then(|ident| {
            if ident != "json" {
                return Err(Error::new(
                    ident.span(),
                    format!("unexpected token {ident}, expected: json!(...)"),
                ));
            }

            Ok(ident)
        })?;
        input.parse::<Token![!]>()?;

        Ok(input.parse::<Group>()?.stream())
    } else {
        Err(Error::new(
            input.span(),
            "unexpected token, expected json!(...)",
        ))
    }
}
