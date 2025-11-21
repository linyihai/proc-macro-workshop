use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse_macro_input, Error, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);

    sorted_impl(args, item)
}

fn sorted_impl(_: TokenStream, item: Item) -> TokenStream {
    let Item::Enum(enum_item) = item else {
        // Not `item.span`
        return Error::new(Span::call_site(), "expected enum or match expression")
            .to_compile_error()
            .into();
    };
    let mut tokens = TokenStream::from(enum_item.to_token_stream());
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
                let err_token: TokenStream = Error::new(
                    v.ident.span(),
                    format!("{} should sort before {}", v.ident, j.ident),
                )
                .to_compile_error()
                .into();
                // sorted 宏不能直接返回错误，这样子是的没有使用到use导入的包，这里的
                // 解决方法是将错误作为Tokenstream返回
                tokens.extend(err_token);
                return tokens;
            }
        }
    }

    tokens
}
