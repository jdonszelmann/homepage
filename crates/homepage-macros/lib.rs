use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::reproduce_tokens::generate_body;

mod reproduce_tokens;

#[proc_macro_derive(LiveTemplate, attributes(template_disambiguator))]
pub fn live_template(input: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs,
        vis: _,
        ident,
        generics,
        data: _,
    } = parse_macro_input!(input as DeriveInput);

    let disambiguator = if let Some(attr) = attrs
        .iter()
        .find(|i| i.meta.path().is_ident("template_disambiguator"))
    {
        match attr.meta.require_name_value() {
            Ok(nv) => {
                let expr = &nv.value;
                Some(quote! {#expr}.to_string())
            }
            Err(e) => return e.to_compile_error().into(),
        }
    } else {
        None
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    (quote! {
        impl #impl_generics LiveTemplate for #ident #ty_generics
            #where_clause
        {
            #[cfg(not(feature = "live"))]
            fn render_live(&self) -> Result<String, homepage_live::askama::Error> {
                self.render()
            }

            #[cfg(feature = "live")]
            fn render_live(&self) -> Result<String, homepage_live::askama::Error> {
                use homepage_live::*;
                const PATH: &'static str = concat!(module_path!(), file!(), line!(), #disambiguator);
                {
                    inventory::submit! {
                        TemplateMetadata {
                            path: PATH,
                            do_render: |template: *const ()| -> Result<String, askama::Error> {
                                let template: &#ident = unsafe {
                                    std::mem::transmute(template)
                                };

                                template.render()
                            },
                        }
                    }
                }

                let templates_list = CURRENT_TEMPLATES.lock().unwrap();

                if templates_list.1.is_empty() {
                    return self.render()
                }

                for template in &templates_list.1 {
                    if template.path == PATH {
                        return unsafe {(template.do_render)(std::mem::transmute(self))};
                    }
                }

                unreachable!("no template matched");
            }
        }
    })
    .into()
}

#[proc_macro_derive(ReproduceTokens)]
pub fn reproduce_tokens(input: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs: _,
        vis: _,
        ident,
        generics,
        data,
    } = parse_macro_input!(input as DeriveInput);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = generate_body(ident.clone(), data);

    (quote! {
        impl #impl_generics ReproduceTokens for #ident #ty_generics
            #where_clause
        {
            fn reproduce_tokens(&self) -> homepage_traits::proc_macro2::TokenStream {
                #body
            }
        }
    })
    .into()
}
