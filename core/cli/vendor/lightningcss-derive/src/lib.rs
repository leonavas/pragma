use proc_macro::TokenStream;

mod parse;
mod to_css;

#[proc_macro_derive(Parse, attributes(css))]
pub fn derive_parse(input: TokenStream) -> TokenStream {
  parse::derive_parse(input)
}

#[proc_macro_derive(ToCss, attributes(css))]
pub fn derive_to_css(input: TokenStream) -> TokenStream {
  to_css::derive_to_css(input)
}
