use proc_macro::TokenStream;

#[proc_macro]
pub fn seq(_: TokenStream) -> TokenStream {
    proc_macro::TokenStream::new()
}
