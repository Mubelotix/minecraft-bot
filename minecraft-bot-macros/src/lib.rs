extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use proc_macro2::*;
use syn::*;

#[derive(Debug, Clone)]
struct MissionState {
    variant_ident: Ident,
    is_loop: bool,
    fields: Vec<(Option<token::Mut>, Ident, PermissiveType)>,
    stmts: Vec<Stmt>,
    next_mission: Option<Box<MissionState>>,
}

impl MissionState {
    fn declaration(&self) -> proc_macro2::TokenStream {
        let variant_ident = &self.variant_ident;
        let variant_field_idents = self.fields.iter().map(|t| &t.1);
        let variant_field_types = self.fields.iter().map(|t| &t.2);
        quote! {
            #variant_ident { #(#variant_field_idents: #variant_field_types,)* }
        }
    }

    fn match_arm(&self) -> proc_macro2::TokenStream {
        let variant_ident = &self.variant_ident;
        let stmts = &self.stmts;
        let variant_field_idents = self.fields.iter().map(|t| &t.1);
        let variant_field_idents2 = self.fields.iter().map(|t| &t.1);
        let variant_field_mutability = self.fields.iter().map(|t| &t.0);

        if let Some(next_mission) = &self.next_mission {
            let next_variant_ident = &next_mission.variant_ident;
            let next_variant_fields = next_mission.fields.iter().map(|f| &f.1);

            if !self.is_loop {
                quote! {
                    GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                        #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                        #(#stmts)*
                        self.state = GeneratedMissionState::#next_variant_ident { #(#next_variant_fields, )* };
                    },
                }
            } else {
                quote! (
                    GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                        #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                        #(#stmts)*
                    },
                )
            }
        } else {
            quote! {
                GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                    #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                    #(#stmts)*
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
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

    let mut mission_states: Vec<MissionState> = Vec::new();
    let mut active_mission_state: Option<MissionState> = None;
    let mut fields = Vec::new();
    for (idx, item) in input.block.stmts.iter().enumerate() {
        // Add the new variant
        // Its created fields are added later so that they are accessible for the next variants only
        let variant_ident = format_ident!("State{}", idx);

        match item {
            Stmt::Local(local) if matches!(local.init.as_ref().map(|(_, b)| *b.clone()), Some(Expr::Loop(_))) => {
                if let Some(active_mission_state) = active_mission_state.take() {
                    mission_states.push(active_mission_state);
                }

                let loop_expr = match local.init.as_ref().map(|(_, b)| *b.clone()) {
                    Some(Expr::Loop(loop_expr)) => loop_expr,
                    _ => unreachable!(),
                };

                let looping_mission_state = MissionState {
                    variant_ident,
                    is_loop: true,
                    fields: fields.clone(),
                    stmts: loop_expr.body.stmts,
                    next_mission: None,
                };
                
                mission_states.push(looping_mission_state);
            }
            expr => {
                let active_mission_state = match &mut active_mission_state {
                    Some(active_mission_state) => active_mission_state,
                    None => {
                        active_mission_state = Some(MissionState {
                            variant_ident,
                            is_loop: false,
                            fields: fields.clone(),
                            stmts: Vec::new(),
                            next_mission: None,
                        });
                        active_mission_state.as_mut().unwrap()
                    }
                };
                active_mission_state.stmts.push(expr.to_owned());
            }
        }

        if let Stmt::Local(local) = item {
            let pat = match &local.pat {
                Pat::Type(pat) => pat,
                other => panic!("unsupported pat {:?} in function", other),
            };

            let mut name_idents: Vec<(Option<token::Mut>, Ident)> = match *pat.pat.clone() {
                Pat::Tuple(tuple) => tuple
                    .elems
                    .into_iter()
                    .map(|e| match e {
                        Pat::Ident(pat) => (pat.mutability, pat.ident),
                        other => panic!("expected an ident for variable name, found {:?}", other),
                    })
                    .collect(),
                Pat::Ident(ident) => vec![(ident.mutability, ident.ident)],
                other => panic!("unsupported pat type {:?} in function", other),
            };

            let mut types: Vec<PermissiveType> = match *pat.ty.clone() {
                Type::Tuple(tuple) => tuple.elems.into_iter().map(PermissiveType::RestrictiveType).collect(),
                Type::Paren(paren) => vec![PermissiveType::RestrictiveType(*paren.elem)],
                Type::Path(path) => vec![PermissiveType::Path(path)],
                other => panic!("unsupported type of variable {:?} in function", other),
            };

            assert_eq!(name_idents.len(), types.len());
            for _ in 0..name_idents.len() {
                let name_ident = name_idents.remove(0);
                fields.push((name_ident.0, name_ident.1, types.remove(0)));
            }
        }
    }

    if let Some(active_mission_state) = active_mission_state.take() {
        mission_states.push(active_mission_state)
    }

    for i in 0..mission_states.len() - 1 {
        mission_states[i].next_mission = Some(Box::new(mission_states[i+1].clone()));
    }

    let declaration = mission_states.iter().map(|m| m.declaration());
    let match_arms = mission_states.iter().map(|m| m.match_arm());

    let expanded = quote! {
        enum GeneratedMissionState {
            #(#declaration,)*
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
                    #(#match_arms)*
                }
                MissionResult::InProgress
            }
        }
    };

    println!("{}", expanded.to_string());

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
