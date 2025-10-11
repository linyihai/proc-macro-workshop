use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, DataStruct, DeriveInput, Fields, FieldsNamed, PathSegment, Type, TypePath,
};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fields = input.data;

    // Get fields and ty from named struct.
    // The difficulty for newbie is the unfamiliar with the syn struct.
    let fields: Vec<OptionalField> = match fields {
        syn::Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => named
            .iter()
            .map(|f| {
                OptionalField::from((
                    f.ident.clone().expect("Only support named struct"),
                    f.ty.clone(),
                ))
            })
            .collect(),
        _ => {
            panic!("Builder can only be derived for structs");
        }
    };
    let build_op_fields = fields
        .iter()
        .map(|op| {
            let (name, ty) = (&op.name, &op.ty);
            // Return another TokenStream from quote! so then it can be used in the quote! macro also.
            quote! {
                #name: ::core::option::Option<#ty>
            }
        })
        .collect::<Vec<_>>();
    let build_fileds = fields
        .iter()
        .map(|op| {
            let name = &op.name;
            quote! {
                #name: None
            }
        })
        .collect::<Vec<_>>();

    let set_fields_methods = set_fields(&fields);
    let set_fields_back = fields
        .iter()
        .map(|op| {
            let (name, optional) = (&op.name, &op.optional);
            if *optional {
                quote!{
                    #name: self.#name.take()
                }
            } else {
                quote!{
                    #name: self.#name.take().expect(&format!("Field {} is not set", stringify!(#name)))
                }
            }
        })
        .collect::<Vec<_>>();

    // Finally assemble the TokeStream.
    let name_builder = format_ident!("{}Builder", name);
    let expanded = quote! {
        pub struct #name_builder  {
            #(#build_op_fields),*
        }

        impl #name_builder {
            #(#set_fields_methods)*

            fn build(&mut self) -> ::core::result::Result<#name, std::boxed::Box<dyn ::std::error::Error>> {
                Ok(#name {
                    #(#set_fields_back),*
                })
            }
        }

        impl #name {
            pub fn builder() -> #name_builder {
                #name_builder {
                    #(#build_fileds),*
                }
            }
        }
    };
    // Although quote! return is TokenStream but it is not the same as proc_macro::TokenStream, So need to covert it.
    proc_macro::TokenStream::from(expanded)
}

struct OptionalField {
    name: proc_macro2::Ident,
    ty: Type,
    optional: bool,
}

impl From<(proc_macro2::Ident, Type)> for OptionalField {
    fn from((name, ty): (proc_macro2::Ident, Type)) -> Self {
        let (ty, optional) = parse_ty(ty);
        OptionalField { name, ty, optional }
    }
}

// For 06-option test, we need to parse the syntax tree of `ty` to determine whether it is optional
fn parse_ty(ty: Type) -> (Type, bool) {
    match ty {
        Type::Path(TypePath {
            path: syn::Path { ref segments, .. },
            ..
        }) => match segments.iter().next() {
            Some(PathSegment {
                ident,
                arguments: syn::PathArguments::AngleBracketed(ref args),
            }) if ident == "Option" => match args.args.iter().next() {
                Some(syn::GenericArgument::Type(ref ty)) => (ty.clone(), true),
                _ => (ty, false),
            },
            _ => (ty, false),
        },
        _ => (ty, false),
    }
}

fn set_fields(fiedls: &[OptionalField]) -> Vec<proc_macro2::TokenStream> {
    fiedls
        .iter()
        .map(|op| {
            let (name, ty) = (&op.name, &op.ty);
            quote! {
                fn #name (&mut self, #name: #ty) -> &mut Self {
                    self.#name = ::core::option::Option::Some(#name);
                    self
                }
            }
        })
        .collect::<Vec<_>>()
}
