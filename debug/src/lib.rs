use proc_macro::TokenStream;
use syn::{parse_macro_input, DataStruct, DeriveInput, Error, Fields, FieldsNamed, Ident};

#[proc_macro_derive(CustomDebug)]
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
            .map(|f| Ok(f.ident.clone().expect("Expected named field")))
            .collect::<Result<Vec<Ident>, Error>>(),
        _ => Err(Error::new(
            name.span(),
            "CustomDebug can only be derived for named structs",
        )),
    }?;
    let field_methods = fields.iter().map(|f| {
        let name = f.to_string();
        quote::quote! {
            .field(#name, &self.#f)
        }
    });
    let quote_name = name.to_string();
    Ok(quote::quote! {
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
