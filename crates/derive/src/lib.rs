//! Errai Derive Macros

#![allow(dead_code)]
#![deny(missing_docs)]

mod frame;

use proc_macro::TokenStream;
use quote::ToTokens;

/// Derive macro for implementing frame content.
#[proc_macro_derive(Frame, attributes(frame))]
pub fn frame(input: TokenStream) -> TokenStream {
  match syn::parse::<frame::Frame>(input) {
    Ok(input) => input.to_token_stream().into(),
    Err(error) => error.into_compile_error().into(),
  }
}
