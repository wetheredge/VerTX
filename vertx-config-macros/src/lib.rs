use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput};

fn config_crate() -> proc_macro2::TokenStream {
    match crate_name("vertx-config").unwrap() {
        FoundCrate::Itself => quote!(crate),
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro::Span::call_site().into());
            quote!(#ident)
        }
    }
}

fn parse_fields(input: &syn::DeriveInput) -> Vec<syn::Ident> {
    let syn::Data::Struct(data) = &input.data else {
        abort!(input, "Can only be used on structs")
    };
    let syn::Fields::Named(fields) = &data.fields else {
        abort!(input, "Can only be used on structs with named fields")
    };

    fields
        .named
        .iter()
        .map(|f| f.ident.clone().unwrap())
        .collect()
}

fn update_common(
    input: TokenStream,
    trait_name: syn::Ident,
    method: syn::Ident,
    is_mut: bool,
) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let fields = parse_fields(&input);

    let field_strs = fields.iter().map(ToString::to_string);

    let config = config_crate();
    let maybe_mut = is_mut.then_some(quote!(mut));

    TokenStream::from(quote! {
        impl #config::#trait_name for #name {
            async fn #method<'a>(
                & #maybe_mut self,
                key: &'a str,
                update: #config::update::Update<'a>,
            ) -> #config::update::Result {
                let (head, tail) = #config::split_key(key);

                match head {
                    #(#field_strs => self.#fields.#method(tail, update).await,)*
                    _ => Err(#config::update::Error::KeyNotFound),
                }
            }
        }
    })
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(UpdateMut)]
pub fn update_mut(input: TokenStream) -> TokenStream {
    update_common(
        input,
        parse_quote!(UpdateMut),
        parse_quote!(update_mut),
        true,
    )
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(UpdateRef)]
pub fn update_ref(input: TokenStream) -> TokenStream {
    update_common(
        input,
        parse_quote!(UpdateRef),
        parse_quote!(update_ref),
        false,
    )
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Storage)]
pub fn storage(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let fields = parse_fields(&input);

    let field_count = fields.len();
    let field_strs = fields.iter().map(ToString::to_string).collect::<Vec<_>>();

    let config = config_crate();

    TokenStream::from(quote! {
        impl #config::Storage for #name {
            async fn save<'a, S: #config::storage::Serializer>(&'a self, serializer: S) {
                use #config::storage::StructSerializer;
                let mut serializer = serializer.structure(#field_count);
                #(serializer.field(#field_strs, &self.#fields).await;)*
                serializer.finish();
            }

            fn load<'a>(from: #config::storage::Stored<'a>) -> Self {
                let mut config: Self = ::core::default::Default::default();

                if let #config::storage::Stored::Struct(fields) = from {
                    for (field, value) in fields {
                        match field {
                            #(#field_strs => config.#fields = #config::Storage::load(value),)*
                            _ => {}
                        }
                    }
                }

                config
            }
        }

    })
}
