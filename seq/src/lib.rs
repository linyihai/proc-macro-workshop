use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenTree};
use syn::parse_quote_spanned;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::Result;

use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, ExprRange, Ident, Token,
};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    proc_macro::TokenStream::new();

    let input = parse_macro_input!(input as SeqParese);
    match exec_seq(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn exec_seq(seq: SeqParese) -> Result<TokenStream> {
    let Some(start) = seq.range.start else {
        return Err(syn::Error::new_spanned(
            seq.range.start,
            "start of range must be specified",
        ));
    };
    let Some(end) = seq.range.end else {
        return Err(syn::Error::new_spanned(
            seq.range.end,
            "end musts be specified",
        ));
    };
    let start = parse_range(&start)?;
    let end = parse_range(&end)?;

    let inclusive = matches!(seq.range.limits, syn::RangeLimits::Closed(_));
    let mut tokens = Vec::with_capacity(end + start);

    for i in start..end {
        // `usize_unsuffixed` split the `usize` of `1usize`, and left it with `1`.
        let num = proc_macro2::Literal::usize_unsuffixed(i);
        let num = syn::parse_quote!(#num);
        let mut placeholder = Placeholder {
            placeholder: seq.name.clone(),
            replacement: num,
        };
        let mut body = seq.body.clone();
        placeholder.visit_token_stream_mut(&mut body);
        tokens.push(body);
    }

    if inclusive {
        let a = syn::parse_quote!(#end);
        let mut placeholder = Placeholder {
            placeholder: seq.name.clone(),
            replacement: a,
        };
        let mut body = seq.body.clone();
        placeholder.visit_token_stream_mut(&mut body);
        tokens.push(body);
    }

    let tokens = quote::quote!(#(#tokens)*);
    Ok(TokenStream::from(tokens))
}

struct SeqParese {
    name: Ident,
    range: ExprRange,
    body: proc_macro2::TokenStream,
}

// Find and Replace all the Placeholders T
struct Placeholder {
    placeholder: Ident,
    // TokenTree type required
    replacement: proc_macro2::TokenTree,
}

impl VisitMut for Placeholder {
    fn visit_token_stream_mut(&mut self, node: &mut proc_macro2::TokenStream) {
        let cl_node = node.clone();
        let mut iter = cl_node.into_iter().peekable();
        let mut tokens = vec![];

        while iter.peek().is_some() {
            let mut window = iter.clone().take(2);
            let mut t = iter.next().unwrap();
            // This's the critical code. By traversing the token tree, if there is `(N` found, then we supersede N with the
            // replacement token tree.
            let (Some(TokenTree::Punct(_)), Some(TokenTree::Ident(_))) =
                (window.next(), window.next())
            else {
                self.expanded(&mut t);
                tokens.push(t);
                continue;
            };

            self.expanded(&mut t);
            tokens.push(t);
        }

        *node = parse_quote_spanned! {node.span()=> #(#tokens)*};
    }
}

impl Placeholder {
    fn expanded(&mut self, node: &mut proc_macro2::TokenTree) {
        match node {
            TokenTree::Group(g) => {
                let mut stream = g.stream();
                let delimiter = g.delimiter();
                // Yeah, Once there is still TokenStream left, we recursively parse it by `visit_token_stream_mut`. 
                // Beware that stream had spilted off the delimiter (like `[]` or `()` or `{}`)
                self.visit_token_stream_mut(&mut stream);
                // We need to recover the delimiter, so encapsulate the stream with the delimiter.
                *node = match delimiter {
                    Delimiter::Brace => {
                        parse_quote_spanned! {node.span()=> {#stream}}
                    }
                    Delimiter::Bracket => {
                        parse_quote_spanned! {node.span()=> [#stream]}
                    }
                    Delimiter::Parenthesis => {
                        parse_quote_spanned! {node.span()=> (#stream)}
                    }
                    Delimiter::None => {
                        parse_quote_spanned! {node.span()=> #stream}
                    }
                };
            }
            TokenTree::Ident(t) => {
                if *t == self.placeholder {
                    let replacement = self.replacement.clone();
                    *node = parse_quote_spanned!(node.span()=> #replacement);
                }
            }
            _ => (),
        }
    }
}

impl Parse for SeqParese {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse::<Ident>()?;
        input.parse::<Token![in]>()?;
        let range = input.parse::<ExprRange>()?;
        let body;
        braced! {body in input};

        Ok(SeqParese {
            name,
            range,
            body: body.parse()?,
        })
    }
}

fn parse_range(input: &syn::Expr) -> Result<usize> {
    match input {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i),
            ..
        }) => Ok(i.base10_parse().unwrap()),
        _ => Err(syn::Error::new(input.span(), "Not support expr")),
    }
}
