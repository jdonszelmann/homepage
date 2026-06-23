use std::borrow::Cow;

pub use homepage_macros::ReproduceTokens;
use proc_macro2::TokenStream;

pub use proc_macro2;
pub use quote::quote;
use time::{Date, Month, OffsetDateTime};

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

impl ReproduceTokens for OffsetDateTime {
    fn reproduce_tokens(&self) -> TokenStream {
        let timestamp = self.unix_timestamp_nanos();
        quote! {
            time::OffsetDateTime::from_unix_timestamp_nanos(#timestamp)
        }
    }
}

impl ReproduceTokens for Date {
    fn reproduce_tokens(&self) -> TokenStream {
        let date = self.to_calendar_date().reproduce_tokens();
        quote! {
            // note: we get the parentheses from the tuple lol
            {let Ok(x) = time::Date::from_calendar_date #date else {unreachable!()}; x}
        }
    }
}

impl ReproduceTokens for Month {
    fn reproduce_tokens(&self) -> TokenStream {
        match self {
            Month::January => quote! {time::Month::January},
            Month::February => quote! {time::Month::February },
            Month::March => quote! {time::Month::March },
            Month::April => quote! {time::Month::April },
            Month::May => quote! {time::Month::May },
            Month::June => quote! {time::Month::June },
            Month::July => quote! {time::Month::July },
            Month::August => quote! {time::Month::August },
            Month::September => quote! {time::Month::September },
            Month::October => quote! {time::Month::October },
            Month::November => quote! {time::Month::November },
            Month::December => quote! {time::Month::December },
        }
    }
}

impl ReproduceTokens for () {
    fn reproduce_tokens(&self) -> TokenStream {
        quote! {()}
    }
}

macro_rules! tuples {
    ($name: ident $($rest: ident)*) => {
        #[allow(non_snake_case)]
        impl<$name: ReproduceTokens, $($rest: ReproduceTokens),*> ReproduceTokens for ($name, $($rest),*) {
            fn reproduce_tokens(&self) -> TokenStream {
                let ($name, $($rest),*) = self;
                let $name = ReproduceTokens::reproduce_tokens($name);
                $(
                    let $rest = ReproduceTokens::reproduce_tokens($rest);
                )*
                quote! {
                    (#$name, $(#$rest),*)
                }
            }
        }
        tuples!($($rest)*);
    };
    () => {};
}

tuples!(A B C D E F G H I J K L M N O P);

macro_rules! to_tokens {
    ($($ty: ty),* $(,)?) => {
        $(
            impl ReproduceTokens for $ty {
                fn reproduce_tokens(&self) -> TokenStream {
                    quote! {
                        #self
                    }
                }
            }
        )*
    };
}

to_tokens!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize,
);
