use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Arm, Error, Item, ItemFn, Pat, PatTupleStruct, Path};

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

struct DisorderEnum {
    enum_name: String,
    span: Span,
}

impl DisorderEnum {
    fn new(enum_name: String, span: Span) -> Self {
        Self { enum_name, span }
    }
}

impl PartialEq<Self> for DisorderEnum {
    fn eq(&self, other: &Self) -> bool {
        self.enum_name.eq(&other.enum_name)
    }
}

impl PartialOrd for DisorderEnum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.enum_name.cmp(&other.enum_name))
    }
}

fn get_path(path: &Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
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
        let mut paths: Vec<DisorderEnum> = vec![];
        for Arm { pat, .. } in &node.arms {
            match pat {
                Pat::TupleStruct(PatTupleStruct { path, .. }) => {
                    // path.span() 的结果跟使用的rust版本紧密相关，rust稳定版和nightly版结果不一样
                    let t = DisorderEnum::new(get_path(path), path.span());
                    paths.push(t);
                }
                Pat::Ident(ident) => {
                    let t = DisorderEnum::new(ident.ident.to_string(), ident.span());
                    paths.push(t);
                }
                Pat::Wild(_) => {
                    if paths.len() + 1 != node.arms.len() {
                        let errors = Error::new(pat.span(), "_ should be last arm in #[sorted]")
                            .to_compile_error();
                        self.errors.push(errors.into());
                        break;
                    }
                }
                _ => {
                    let errors =
                        Error::new(pat.span(), "unsupported by #[sorted]").to_compile_error();
                    self.errors.push(errors.into());
                    break;
                }
            }
        }
        if let Some(hint) = disorder_hint(&paths) {
            let first = &paths[hint.0];
            let second = &paths[hint.1];
            let errors = Error::new(
                first.span,
                format!(
                    "{} should sort before {}",
                    first.enum_name, second.enum_name
                ),
            )
            .to_compile_error();
            self.errors.push(errors.into());
        }

        // Continue visiting nested expressions
        syn::visit_mut::visit_expr_match_mut(self, node);
    }
}
