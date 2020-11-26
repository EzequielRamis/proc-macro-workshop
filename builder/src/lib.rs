use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Error, Field, Fields, GenericArgument, Lit, Meta,
    PathArguments, Type,
};
use unzip_n::unzip_n;

unzip_n!(4);

fn unwrap_type(wrapper: String, ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        let segment = tp.path.segments.last()?;
        if segment.ident != wrapper {
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

fn builder_each_setter(field: &Field) -> Option<proc_macro2::TokenStream> {
    let ty = &field.ty;
    let unwrapped_ty = unwrap_type("Vec".to_owned(), ty)?;
    let field_ident = field.ident.as_ref()?;
    for attr in field.attrs.iter() {
        if let Some(ident) = attr.path.get_ident() {
            if ident == "builder" {
                let args = attr.parse_args().ok();
                if let Some(Meta::NameValue(name_value)) = args {
                    if let (Some(name), Lit::Str(lit)) =
                        (name_value.path.get_ident(), name_value.lit)
                    {
                        if name == "each" {
                            let value = lit.value();
                            let value_ident = format_ident!("{}", &value);
                            let setter = quote! {
                                fn #value_ident(&mut self, #value_ident: #unwrapped_ty) -> &mut Self {
                                    self.#field_ident.push(#value_ident);
                                    self
                                }
                            };

                            if *field_ident != value {
                                return Some(quote! {
                                    #setter
                                    fn #field_ident(&mut self, #field_ident: #ty) -> &mut Self {
                                        self.#field_ident = #field_ident;
                                        self
                                    }
                                });
                            }
                            return Some(setter);
                        }

                        let span = attr.parse_meta().unwrap();
                        let error = Error::new_spanned(span, "expected `builder(each = \"...\")`");
                        return Some(error.to_compile_error());
                    }
                }
            }
        }
    }
    None
}

#[proc_macro_derive(Builder, attributes(builder))]
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

            if let Some(unwraped_ty) = unwrap_type("Option".to_owned(), &field.ty) {
                (
                    normal_field,
                    none_field,
                    setter_field(unwraped_ty),
                    cloned_field,
                )
            } else if let Some(each_setter) = builder_each_setter(field) {
                (
                    normal_field,
                    quote! { #name: std::vec::Vec::new() },
                    each_setter,
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
