use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, DataStruct, DeriveInput, Expr, Field, Fields, FieldsNamed, Ident, Lit,
    MetaNameValue, PathSegment, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
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
            .map(|f| OptionalField::from(f.clone()))
            .collect(),
        _ => {
            panic!("Builder can only be derived for structs");
        }
    };
    let builder_fields = fields
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

    let builder_methods = generate_builder_methods(&fields);
    // Fill the field value back to the origin struct
    let instance_fields = fields
        .iter()
        .map(|op| {
            let (name, angle_type) = (&op.name, &op.angle_type);
            match angle_type {
                AngleBracketed::Vec | AngleBracketed::Option => quote! {
                    // If the field is `Vec<T>`, it could be ommited setting value.
                    #name: self.#name.take().unwrap_or_default()
                },
                _ => quote! {
                    #name: self.#name.take().expect(&format!("Field {} is not set", stringify!(#name)))
                }
            }
        })
        .collect::<Vec<_>>();

    // Finally assemble the TokeStream.
    let name_builder = format_ident!("{}Builder", name);
    let expanded = quote! {
        pub struct #name_builder  {
            #(#builder_fields),*
        }

        impl #name_builder {
            #(#builder_methods)*

            fn build(&mut self) -> ::core::result::Result<#name, std::boxed::Box<dyn ::std::error::Error>> {
                Ok(#name {
                    #(#instance_fields),*
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
    attr: Option<Ident>,
    inner_ty: Option<Type>,
    angle_type: AngleBracketed,
}

impl From<Field> for OptionalField {
    fn from(field: Field) -> Self {
        let attr = parse_each_attr(field.attrs.first()).unwrap();
        let (ty, inner_ty, angle_type) = parse_ty(field.ty, attr.is_some());

        OptionalField {
            name: field.ident.unwrap(),
            ty,
            attr,
            inner_ty,
            angle_type,
        }
    }
}

enum AngleBracketed {
    Option,
    Vec,
    Other,
}

// For 06-option test, we need to parse the syntax tree of `ty` to determine whether it is optional
fn parse_ty(ty: Type, has_each: bool) -> (Type, Option<Type>, AngleBracketed) {
    match ty {
        Type::Path(TypePath {
            path: syn::Path { ref segments, .. },
            ..
        }) => match segments.iter().next() {
            Some(PathSegment {
                ident,
                arguments: syn::PathArguments::AngleBracketed(ref args),
            }) => match (ident == "Option", ident == "Vec", args.args.iter().next()) {
                (true, _, Some(syn::GenericArgument::Type(ref inner_ty))) => {
                    (ty.clone(), Some(inner_ty.clone()), AngleBracketed::Option)
                }
                (false, true, Some(syn::GenericArgument::Type(ref inner_ty))) if has_each => {
                    (ty.clone(), Some(inner_ty.clone()), AngleBracketed::Vec)
                }
                _ => (ty, None, AngleBracketed::Other),
            },

            _ => (ty, None, AngleBracketed::Other),
        },
        _ => (ty, None, AngleBracketed::Other),
    }
}

/// Generate the methods for `CommandBuilder`
fn generate_builder_methods(fiedls: &[OptionalField]) -> Vec<proc_macro2::TokenStream> {
    fiedls
        .iter()
        .map(|op| {
            match (&op.attr, matches!(op.angle_type, AngleBracketed::Option)) {
                (Some(attr), _) => {
                    let (name, ty) = (&op.name, op.inner_ty.clone().unwrap());
                    quote! {
                        fn #attr (&mut self, #attr: #ty) -> &mut Self {
                            let mut data = self.#name.get_or_insert_default();
                            data.push(#attr);
                            self
                        }
                    }
                },
                (None, true) => {
                    let (name, ty) =  (&op.name, op.inner_ty.clone().unwrap());
                    quote! {
                        fn #name (&mut self, #name: #ty) -> &mut Self {
                            self.#name = ::core::option::Option::Some(::core::option::Option::Some(#name));
                            self
                        }
                    }
                },
                (None, false) => {
                    let (name, ty) = (&op.name, op.ty.clone());
                    quote! {
                        fn #name (&mut self, #name: #ty) -> &mut Self {
                            self.#name = ::core::option::Option::Some(#name);
                            self
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>()
}

// Parse each attribute
// Why returning Ident rather than String, because the String surrounding by the quote, like "a", it's trickly to pass the string in the
// `quote!`. So it's best to pass a ident to `quote!`.
fn parse_each_attr(
    attr: Option<&syn::Attribute>,
) -> Result<Option<Ident>, Box<dyn std::error::Error>> {
    match attr {
        Some(attr) if attr.path().is_ident("builder") => match attr.parse_args()? {
            MetaNameValue {
                path,
                value:
                    Expr::Lit(syn::ExprLit {
                        lit: Lit::Str(lit), ..
                    }),
                ..
            } if path.is_ident("each") => Ok(Some(format_ident!("{}", lit.value()))),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}
