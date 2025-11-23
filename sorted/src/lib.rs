use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Arm, Error, Item, ItemFn, Pat, PatTupleStruct};

#[proc_macro_attribute]
pub fn sorted(_: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);

    sorted_impl(item)
}

#[proc_macro_attribute]
pub fn check(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemFn);
    let mut checker = Checker { errors: vec![] };
    checker.visit_item_fn_mut(&mut item);
    let errors = checker.errors;
    let mut result: TokenStream = item.to_token_stream().into();
    result.extend(errors);
    result
}

fn sorted_impl(item: Item) -> TokenStream {
    let Item::Enum(enum_item) = item else {
        // Not `item.span`
        return Error::new(Span::call_site(), "expected enum or match expression")
            .to_compile_error()
            .into();
    };
    let mut tokens = TokenStream::from(enum_item.to_token_stream());
    let items = enum_item
        .variants
        .iter()
        .map(|v| v.ident.clone())
        .collect::<Vec<_>>();
    if let Some(hint) = disorder_hint(&items) {
        let first = &enum_item.variants[hint.0].ident;
        let second = &enum_item.variants[hint.1].ident;
        let err_token: TokenStream = Error::new(
            first.span(),
            format!("{} should sort before {}", first, second),
        )
        .to_compile_error()
        .into();
        tokens.extend(err_token);
    }
    tokens
}

fn disorder_hint<T: PartialOrd>(enum_item: &[T]) -> Option<(usize, usize)> {
    for (i, v) in enum_item.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let prev = &enum_item[i - 1];
        if prev <= v {
            continue;
        }
        for (j, cur) in enum_item.iter().take(i).enumerate() {
            if cur > v {
                return Some((i, j));
            }
        }
    }
    None
}

struct Checker {
    errors: Vec<TokenStream>,
}

impl VisitMut for Checker {
    fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
        // Check if the match expression has the #[sorted] attribute
        let idx = node
            .attrs
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_, v)| v.path().is_ident("sorted"))
            .collect::<Vec<_>>();
        if idx.is_empty() {
            return;
        }
        for (i, _) in idx {
            node.attrs.remove(i);
        }
        let mut paths = vec![];
        for Arm { pat, .. } in &node.arms {
            if let Pat::TupleStruct(PatTupleStruct { path, .. }) = pat {
                paths.push(path.get_ident().unwrap().clone());
            }
        }
        if let Some(hint) = disorder_hint(&paths) {
            let first = &paths[hint.0];
            let second = &paths[hint.1];
            let errors = Error::new(
                first.span(),
                format!(
                    "{} should sort before {}",
                    first.to_token_stream(),
                    second.to_token_stream()
                ),
            )
            .to_compile_error();
            self.errors.push(errors.into());
        }

        // Continue visiting nested expressions
        syn::visit_mut::visit_expr_match_mut(self, node);
    }
}
