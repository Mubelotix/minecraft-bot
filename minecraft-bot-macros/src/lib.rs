extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use proc_macro2::*;
use syn::*;

enum PermissiveType {
    RestrictiveType(Type),
    Path(TypePath),
}

impl ToTokens for PermissiveType {
    fn to_tokens(&self, tokens: &mut __private::TokenStream2) {
        match self {
            PermissiveType::RestrictiveType(e) => e.to_tokens(tokens),
            PermissiveType::Path(e) => e.to_tokens(tokens),
        }
    }
}

#[proc_macro_attribute]
pub fn fsm(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as ItemFn);

    println!("{:#?}", input.block);

    let mut variants = Vec::new();
    let mut variant_field_idents = Vec::new();
    let mut variant_field_types: Vec<PermissiveType> = Vec::new();
    let mut variant_match_arms = Vec::new();
    for (idx, item) in input.block.stmts.iter().enumerate() {
        // Add the new variant
        // Its created fields are added later so that they are accessible for the next variants only
        let variant_ident = format_ident!("State{}", idx);
        if variant_field_idents.len() != variant_field_types.len() {
            panic!("The number of names is different than the number of types. Remember that enums must be destructured.")
        }

        let variant = quote! {
            #variant_ident { #(#variant_field_idents: #variant_field_types,)* }
        };
        variants.push(variant);

        match item {
            Stmt::Local(item) => {
                let pat = match &item.pat {
                    Pat::Type(pat) => pat,
                    other => panic!("unsupported pat {:?} in function", other),
                };

                let code = item.init.as_ref().unwrap();
                let code = match *code.1.clone() {
                    Expr::Block(block) => block.block,
                    other => Block {
                        brace_token: token::Brace {span: Span::call_site()},
                        stmts: vec![Stmt::Expr(other)],
                    }
                };
                variant_match_arms.push(quote! {
                    GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => #code,
                });
        
                let name_idents: Vec<Ident> = match *pat.pat.clone() {
                    Pat::Tuple(tuple) => tuple
                        .elems
                        .into_iter()
                        .map(|e| match e {
                            Pat::Ident(pat) => pat.ident,
                            other => panic!("expected an ident for variable name, found {:?}", other),
                        })
                        .collect(),
                    Pat::Ident(ident) => vec![ident.ident],
                    other => panic!("unsupported pat type {:?} in function", other),
                };
        
                for name_ident in name_idents {
                    variant_field_idents.push(name_ident);
                }
        
                let types: Vec<PermissiveType> = match *pat.ty.clone() {
                    Type::Tuple(tuple) => tuple.elems.into_iter().map(PermissiveType::RestrictiveType).collect(),
                    Type::Paren(paren) => vec![PermissiveType::RestrictiveType(*paren.elem)],
                    Type::Path(path) => vec![PermissiveType::Path(path)],
                    other => panic!("unsupported type of variable {:?} in function", other),
                };
        
                for r#type in types {
                    variant_field_types.push(r#type);
                }
            },
            Stmt::Expr(expr) => {
                let code = match expr {
                    Expr::Block(block) => {
                        block.block.clone()
                    }
                    other => Block {
                        brace_token: token::Brace {span: Span::call_site()},
                        stmts: vec![Stmt::Expr(other.to_owned())],
                    }
                };

                variant_match_arms.push(quote! {
                    GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => #code,
                });
            }
            other => panic!("unsupported item {:?} in function", other),
        };
    }

    let expanded = quote! {
        enum GeneratedMissionState {
            #(#variants,)*
        }

        pub struct GeneratedMission {
            state: GeneratedMissionState,
        }

        impl GeneratedMission {
            pub fn new() -> GeneratedMission {
                GeneratedMission {
                    state: todo!()
                }
            }
        }

        impl Mission for GeneratedMission {
            #[allow(unused_variables)]
            fn execute(&mut self, bot: &mut Bot /* todo add packets */) -> MissionResult {
                match &mut self.state {
                    #(#variant_match_arms)*
                }
                MissionResult::InProgress
            }
        }
    };

    println!("{}", expanded.to_string());

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
