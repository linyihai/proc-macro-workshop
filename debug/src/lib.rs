use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, Generics, Ident,
    Meta, Type,
};

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
        let value = &f.ident;
        let formatter = f.formatter.clone();
        // `&::std::format_args!(#formatter, &self.#value)` is treated as `Debug`
        // cannot use `format!` here
        //
        // stringify! can add quote to the name.
        quote! {
            .field(::std::stringify!(#value), &::std::format_args!(#formatter, &self.#value))
        }
    });

    let generics = add_debug_trait_bound(input.generics, &fields);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(::std::stringify!(#name))
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
    ty: Type,
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
            ty: value.ty,
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

fn add_debug_trait_bound(mut generics: Generics, fields: &[DebugField]) -> Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = param {
            let t = &type_param.ident;
            let expected_ty: Type = syn::parse_quote!(PhantomData<#t>);

            // I misunderstood the intention here, we need skip add debug trait bound to T if the struct contains PhantomData<T>
            // rather than add debug trait bound to PhantomData<T>.
            if fields.iter().any(|field| expected_ty == field.ty) {
                continue;
            }
            // The is for 04-type-parameter only Generic Param T as the Struct field then add debug trait bound
            if fields.iter().any(|field| {
                field.ty.clone().into_token_stream().into_iter().any(
                |token| matches!(token, proc_macro2::TokenTree::Ident( ref ident) if * t == * ident)
            )
            }) {
                type_param.bounds.push(syn::parse_quote!(::std::fmt::Debug));
            }
        }
    }

    generics
}
