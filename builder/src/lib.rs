use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};
use unzip_n::unzip_n;

unzip_n!(4);

fn unwrap_option(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        let segment = tp.path.segments.last()?;
        if segment.ident != "Option" {
            return None;
        }
        if let PathArguments::AngleBracketed(ref generic_args) = segment.arguments {
            if generic_args.args.len() != 1 {
                return None;
            }
            let arg = &generic_args.args[0];
            if let GenericArgument::Type(ty) = arg {
                return Some(ty);
            }
        }
    }
    None
}

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

    let (builder_fields, none_fields, setters, build_fields) = fields
        .iter()
        .map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            let normal_field = quote! { #name: #ty };
            let optional_field = quote! { #name: Option<#ty> };
            let none_field = quote! { #name: None };
            let cloned_field = quote! { #name: self.#name.clone() };

            let setter_field = |setter_ty: &Type| {
                quote! {
                    fn #name(&mut self, #name: #setter_ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                }
            };

            if let Some(unwraped_ty) = unwrap_option(&field.ty) {
                (
                    normal_field,
                    none_field,
                    setter_field(unwraped_ty),
                    cloned_field,
                )
            } else {
                (
                    optional_field,
                    none_field,
                    setter_field(ty),
                    quote! { #cloned_field.ok_or(format!("{} is not set", stringify!(#name)))? },
                )
            }
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

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                Ok(
                    #ident {
                        #(#build_fields),*
                    }
                )
            }
        }
    })
}
