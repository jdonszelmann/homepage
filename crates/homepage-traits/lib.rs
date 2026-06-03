use std::borrow::Cow;

pub use homepage_macros::ReproduceTokens;
use proc_macro2::TokenStream;

pub use proc_macro2;
pub use quote::quote;

pub trait ReproduceTokens {
    fn reproduce_tokens(&self) -> TokenStream;
}

impl<T: ReproduceTokens> ReproduceTokens for Option<T> {
    fn reproduce_tokens(&self) -> TokenStream {
        match self {
            Some(s) => {
                let s = ReproduceTokens::reproduce_tokens(s);
                quote! {
                    Some(#s)
                }
            }
            None => quote! { None },
        }
    }
}

impl<T: ReproduceTokens + Clone> ReproduceTokens for Cow<'static, [T]> {
    fn reproduce_tokens(&self) -> TokenStream {
        let res: Vec<_> = self
            .as_ref()
            .iter()
            .map(|i| T::reproduce_tokens(i))
            .collect();
        quote! {
            std::borrow::Cow::Borrowed(&[#(#res ,)*])
        }
    }
}

impl ReproduceTokens for Cow<'static, str> {
    fn reproduce_tokens(&self) -> TokenStream {
        let s = self.as_ref();
        quote! {
            std::borrow::Cow::Borrowed(#s)
        }
    }
}

impl ReproduceTokens for bool {
    fn reproduce_tokens(&self) -> TokenStream {
        let b = *self;
        quote! {
            #b
        }
    }
}
