use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse_macro_input, Error, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);

    match sorted_impl(args, item) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn sorted_impl(_: TokenStream, item: Item) -> Result<TokenStream, Error> {
    if !matches!(item, Item::Enum(_)) {
        // Not `item.span`
        return Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ));
    };

    Ok(TokenStream::from(item.to_token_stream()))
}
