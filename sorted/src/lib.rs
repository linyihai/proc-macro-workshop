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
    let Item::Enum(enum_item) = item else {
        // Not `item.span`
        return Err(Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ));
    };

    for (i, v) in enum_item.variants.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let prev = &enum_item.variants[i - 1];
        if prev.ident <= v.ident {
            continue;
        }
        for j in enum_item.variants.iter().take(i) {
            if j.ident > v.ident {
                return Err(Error::new(
                    v.ident.span(),
                    format!(
                        "{} should sort before {}",
                        v.ident, j.ident
                    ),
                ));
            }
        }
    }

    Ok(TokenStream::from(enum_item.to_token_stream()))
}
