use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, Ident, Meta, parse_macro_input};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_impl(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_impl(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = input.ident;

    let fields = match &input.data {
        syn::Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => named
            .iter()
            .map(|f| DebugField::try_from(f.clone()))
            .collect::<Result<Vec<DebugField>, Error>>(),
        _ => Err(Error::new(
            name.span(),
            "CustomDebug can only be derived for named structs",
        )),
    }?;
    let field_methods = fields.iter().map(|f| {
        let name = f.ident.to_string();
        let value = f.ident.clone();
        let formatter = f.formatter.clone();
        // `&::std::format_args!(#formatter, &self.#value)` is treated as `Debug`
        // cannot use `format!` here
        quote! {
            .field(#name, &::std::format_args!(#formatter, &self.#value))
        }
    });
    let quote_name = name.to_string();
    Ok(quote! {
        impl ::std::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#quote_name)
                    #(#field_methods)*
                    .finish()
            }
        }
    }
    .into())
}

struct DebugField {
    ident: Ident,
    formatter: proc_macro2::TokenStream,
}

impl TryFrom<Field> for DebugField {
    type Error = Error;

    fn try_from(value: Field) -> Result<Self, Self::Error> {
        let formatter = parse_debug_attr(
            value
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("debug")),
        );
        Ok(DebugField {
            ident: value.ident.unwrap(),
            formatter,
        })
    }
}

fn parse_debug_attr(attr: Option<&syn::Attribute>) -> proc_macro2::TokenStream {
    // Cannot use `Attrbute::parse_args or Attrbute::parse_args_with` to parse `#[debug = "..."]`, since they
    // Only support `#[debug("...")]`
    if let Some(attr) = attr {
        match attr.meta {
            Meta::NameValue(ref meta_formatter) => meta_formatter.value.to_token_stream(),
            _ => quote! {"{:?}"},
        }
    } else {
        quote! {"{:?}"}
    }
}
