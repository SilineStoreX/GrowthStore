use crate::form_schema::attrs::FormSchemaAttr;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Item, ReturnType, Signature};

#[allow(dead_code)]
fn metadata(
    _salvo: &Ident,
    _oapi: &Ident,
    _attr: FormSchemaAttr,
    _name: &Ident,
    mut _modifiers: Vec<TokenStream>,
) -> syn::Result<TokenStream> {
    let stream = quote! {};
    Ok(stream)
}

#[allow(dead_code)]
pub(crate) fn generate(mut _attr: FormSchemaAttr, input: Item) -> syn::Result<TokenStream> {
    match input {
        Item::Fn(mut item_fn) => {
            let attrs = &item_fn.attrs;
            let vis = &item_fn.vis;
            let sig = &mut item_fn.sig;
            let body = &item_fn.block;
            let name = &sig.ident;
            let docs = item_fn
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .cloned()
                .collect::<Vec<_>>();

            let _sdef = quote! {
                #(#docs)*
                #[allow(non_camel_case_types)]
                #[derive(Debug)]
                #vis struct #name;
                impl #name {
                    #(#attrs)*
                    #sig {
                        #body
                    }
                }
            };

            Ok(quote! {})
        }
        Item::Impl(item_impl) => {
            let _attrs = &item_impl.attrs;
            Ok(quote! {})
        }
        _ => Err(syn::Error::new_spanned(
            input,
            "#[handler] must added to `impl` or `fn`",
        )),
    }
}

#[allow(dead_code)]
fn handle_fn(
    _salvo: &Ident,
    oapi: &Ident,
    sig: &Signature,
) -> syn::Result<(TokenStream, Vec<TokenStream>)> {
    let mut modifiers = Vec::new();

    let hfn = match &sig.output {
        ReturnType::Default => {
            if sig.asyncness.is_none() {
                quote! {
                    log::info!("nothing");
                }
            } else {
                quote! {
                    log::info!("blocking");
                }
            }
        }
        ReturnType::Type(_, ty) => {
            modifiers.push(quote! {
                <#ty as #oapi::oapi::EndpointOutRegister>::register(components, operation);
            });
            if sig.asyncness.is_none() {
                quote! {
                    log::info!("nothing");
                }
            } else {
                quote! {
                    log::info!("blocking");
                }
            }
        }
    };
    Ok((hfn, modifiers))
}
