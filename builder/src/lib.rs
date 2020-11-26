use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};
use unzip_n::unzip_n;

unzip_n!(3);

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

    let (builder_fields, none_fields, setters) = fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            let optional_field = quote! { #name: Option<#ty> };
            let none_field = quote! { #name: None };

            let setter_field = quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            };

            (optional_field, none_field, setter_field)
        })
        .unzip_n_vec();

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

        impl #builder {
            #(#setters)*
        }
    })
}
