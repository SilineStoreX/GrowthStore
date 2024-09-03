//! The macros lib of chimes-store  framework.
//!
//! Read more: <https://chimes-store.rs>
use form_schema::attrs::FormSchemaAttr;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, Item};

mod form_schema;
mod utils;

/// `form_schema` is a macro to help create `FormSchema` from function or impl block easily.
///
/// `form_schema` is a trait, if `#[form_schema]` applied to `fn`,  `fn` will converted to a struct, and then implement `Handler`,
/// after use `form_schema`, you don't need to care arguments' order, omit unused arguments.
///
/// View `chimes-store-macro::form_schema` for more details.
#[doc(hidden)]
#[proc_macro_error]
#[proc_macro_attribute]
pub fn form_schema(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as FormSchemaAttr);
    let item = parse_macro_input!(input as Item);
    match crate::form_schema::generator::generate(attr, item) {
        Ok(stream) => stream.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
