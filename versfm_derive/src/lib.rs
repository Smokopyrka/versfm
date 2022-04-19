extern crate proc_macro;

use proc_macro::{TokenStream};
use quote::{quote, __private::TokenTree, ToTokens};
use regex::Regex;
use syn::{self, Data, DataStruct, parse_macro_input, Fields, DeriveInput, Type, TypePath, Field, punctuated::{Punctuated}, token::Comma};

fn get_field_type(field: &Field) -> String {
    match field.to_owned().ty {
        Type::Path(TypePath {
            path,
            ..
        }) => {
            let mut out = String::new();
            for tree in path.to_token_stream() {
                out.push_str(
                    &match tree {
                        TokenTree::Ident(i) => i.to_string(),
                        TokenTree::Punct(p) => p.to_string(),
                        TokenTree::Literal(l) => l.to_string(),
                        TokenTree::Group(g) => g.to_string(),
                    }
                )
            }
            out
        },
        _ => panic!("Given field has to be of type Path"),
    }
}

fn get_named_fields<'a>(data: Data) -> Punctuated<Field, Comma> {
    match data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => fields.named,
            _ => panic!("To derive this trait, this struct needs to have named fields"),
        }
}

fn get_field_by_name<'a>(fields: &'a Punctuated<Field, Comma>, name: &str) -> &'a Field {
    fields.iter()
        .find(|f| f.to_owned().ident.to_owned().unwrap() == name)
        .expect(&format!("Struct needs a named \"{}\" field 
                to be annotated with StatefulContainer", name))
}

fn check_if_stateful_container_thread_safe(state_field: &Field, items_field: &Field) -> bool {
    let state_field_type = get_field_type(state_field);
    let items_field_type = get_field_type(items_field);
    state_field_type == "Arc<Mutex<ListState>>" 
    && Regex::new("Arc<Mutex<Vec<.*>>>").unwrap()
        .is_match(&items_field_type)
}

#[proc_macro_derive(StatefulContainer)]
pub fn stateful_container_derive(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let struct_name = input.ident;

    let fields_punct = get_named_fields(input.data);
    
    let state_field = get_field_by_name(&fields_punct, "state");
    let items_field = get_field_by_name(&fields_punct, "items");

    if !check_if_stateful_container_thread_safe(state_field, items_field) {
        panic!("Fields items and state have to be of the following types:
                state: Arc<Mutex<ListState>
                items: Arc<Mutex<Vec<*>>>)")
    }

    TokenStream::from(quote! {
        impl StatefulContainer for #struct_name {
            fn get_current(&self) -> ListState {
                self.state.lock().expect("Couldn't lock mutex").clone()
            }

            fn clear_state(&self) {
                self.state.lock().expect("Couldn't lock mutex").select(None);
            }

            fn next(&self) {
                let items = self.items.lock().expect("Couldn't lock mutex");
                let mut state = self.state.lock().expect("Couldn't lock mutex");
                if items.len() > 0 {
                    let i = match state.selected() {
                        Some(i) => {
                            if i >= items.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };

                    state.select(Some(i));
                }
            }

            fn previous(&self) {
                let items = self.items.lock().expect("Couldn't lock mutex");
                let mut state = self.state.lock().expect("Couldn't lock mutex");
                if items.len() > 0 {
                    let i = match state.selected() {
                        Some(i) => {
                            if i == 0 {
                                items.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };

                    state.select(Some(i));
                }
            }
        }
    })
}