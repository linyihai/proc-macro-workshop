use proc_macro::TokenStream;

use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, ExprRange, Ident, Result, Token,
};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    proc_macro::TokenStream::new();

    let _ = parse_macro_input!(input as SeqParese);

    TokenStream::new()
}

struct SeqParese {
    _name: Ident,
    _range: ExprRange,
    _body: proc_macro2::TokenStream,
}

impl Parse for SeqParese {
    fn parse(input: ParseStream) -> Result<Self> {
        let _name = input.parse::<Ident>()?;
        input.parse::<Token![in]>()?;
        let _range = input.parse::<ExprRange>()?;
        let _body;
        braced! {_body in input};

        Ok(SeqParese {
            _name,
            _range,
            _body: _body.parse()?,
        })
    }
}
