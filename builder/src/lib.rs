use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let data = &input.data;
    let ident = &input.ident;
    let builder = format_ident!("{}Builder", ident);

    let fields = match data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    };

    let (builder_fields, none_fields): (Vec<_>, Vec<_>) = fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            let optional_field = quote! { #name: Option<#ty> };
            let none_field = quote! { #name: None };

            (optional_field, none_field)
        })
        .unzip();

    TokenStream::from(quote! {
        pub struct #builder {
            #(#builder_fields),*
        }

        impl #ident {
            pub fn builder() -> #builder {
                #builder {
                    #(#none_fields),*
                }
            }
        }
    })
}
