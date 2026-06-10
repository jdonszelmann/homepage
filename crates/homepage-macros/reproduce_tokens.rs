use itertools::Itertools;
use proc_macro2::{Punct, Spacing, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Data, Fields, Ident};

fn generate_converter(
    name: Ident,
    variant_name: Option<Ident>,
    f: Fields,
) -> (TokenStream2, TokenStream2) {
    let variant_name = variant_name.map(|i| quote! {:: #i});
    let pound = Punct::new('#', Spacing::Alone);

    match f {
        Fields::Named(n) => {
            let (names, quoted_names, generators): (Vec<_>, Vec<_>, Vec<_>) = n
                .named
                .into_iter()
                .map(|field| {
                    let ident = field.ident.unwrap();
                    (
                        quote! {#ident , },
                        quote! {#ident: #pound #ident , },
                        quote! {
                            let #ident = homepage_traits::ReproduceTokens::reproduce_tokens(#ident);
                        },
                    )
                })
                .multiunzip();
            (
                quote! {#name #variant_name {#(#names)*}},
                quote! {
                    #(#generators)*

                    homepage_traits::quote! { #name #variant_name { #(#quoted_names)* } }
                },
            )
        }
        Fields::Unnamed(n) => {
            let (names, quoted_names, generators): (Vec<_>, Vec<_>, Vec<_>) = n
                .unnamed
                .into_iter()
                .enumerate()
                .map(|(index, _)| {
                    let ident = format_ident!("field_{index}");
                    (
                        quote! {#ident , },
                        quote! {#pound #ident , },
                        quote! {
                            let #ident = homepage_traits::ReproduceTokens::reproduce_tokens(#ident);
                        },
                    )
                })
                .multiunzip();
            (
                quote! {#name #variant_name (#(#names)*)},
                quote! {
                    #(#generators)*

                    homepage_traits::quote! { #name #variant_name ( #(#quoted_names)* ) }
                },
            )
        }
        Fields::Unit => (
            quote! { #name #variant_name },
            quote! {
                homepage_traits::quote! { #name #variant_name }
            },
        ),
    }
}

pub fn generate_body(name: Ident, data: Data) -> TokenStream2 {
    match data {
        Data::Struct(data_struct) => {
            let (pattern, arm) = generate_converter(name.clone(), None, data_struct.fields);

            quote! {
                let #pattern = self;
                #arm
            }
        }
        Data::Enum(data_enum) => {
            let (patterns, arms): (Vec<_>, Vec<_>) = data_enum
                .variants
                .into_iter()
                .map(|i| generate_converter(name.clone(), Some(i.ident), i.fields))
                .unzip();
            quote! {
                match self {
                    #(#patterns => {#arms})*
                }
            }
        }
        Data::Union(_) => unimplemented!("can't derive ReproduceTokens on unins"),
    }
}
