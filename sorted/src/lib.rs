use proc_macro::TokenStream;
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn sorted(_: TokenStream, input: TokenStream) -> TokenStream {
    let c = input.clone();
    let _ = parse_macro_input!(c as Item);

    input
}
