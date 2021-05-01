extern crate proc_macro;
use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::*;
use quote::{format_ident, quote};
use syn::*;

mod mission_state;
mod code_modifier;
use code_modifier::*;
use mission_state::*;

fn analyse_block(
    items: Vec<Stmt>,
    mut fields: Vec<(Option<token::Mut>, Ident, PermissiveType)>,
    mission_states: &mut Vec<MissionState>,
    is_loop: bool,
    loops: &mut HashMap<String, (usize, usize)>,
    parent_loops: Vec<String>,
) {
    let mut active_mission_state: Option<MissionState> = None;
    let first_idx = mission_states.len();

    for item in items.iter() {
        // Add the new variant
        // Its created fields are added later so that they are accessible for the next variants only
        let variant_ident = format_ident!("State{}", mission_states.len());

        match item {
            Stmt::Local(local)
                if matches!(local.init.as_ref().map(|(_, b)| *b.clone()), Some(Expr::Loop(ExprLoop {
                label: Some(label),
                ..
            })) if label.name.ident.to_string().starts_with("mt_")) =>
            {
                if let Some(active_mission_state) = active_mission_state.take() {
                    mission_states.push(active_mission_state);
                }

                let loop_expr = match local.init.as_ref().map(|(_, b)| *b.clone()) {
                    Some(Expr::Loop(loop_expr)) => loop_expr,
                    _ => unreachable!(),
                };

                let label = loop_expr.label.as_ref().unwrap().name.ident.to_string();
                let mut parent_loops = parent_loops.clone();
                parent_loops.push(label.clone());

                let continue_index = mission_states.len();
                analyse_block(loop_expr.body.stmts, fields.clone(), mission_states, true, loops, parent_loops);
                let break_index = mission_states.len();
                loops.insert(label, (continue_index, break_index));
            }
            Stmt::Expr(Expr::Loop(loop_expr)) if loop_expr.label.is_some() && loop_expr.label.as_ref().unwrap().name.ident.to_string().starts_with("mt_") => {
                if let Some(active_mission_state) = active_mission_state.take() {
                    mission_states.push(active_mission_state);
                };

                let label = loop_expr.label.as_ref().unwrap().name.ident.to_string();
                let mut parent_loops = parent_loops.clone();
                parent_loops.push(label.clone());

                let continue_index = mission_states.len();
                analyse_block(loop_expr.body.stmts.clone(), fields.clone(), mission_states, true, loops, parent_loops);
                let break_index = mission_states.len();
                loops.insert(label, (continue_index, break_index));
            }
            expr => {
                let active_mission_state = match &mut active_mission_state {
                    Some(active_mission_state) => active_mission_state,
                    None => {
                        active_mission_state = Some(MissionState {
                            variant_ident,
                            parent_loops: parent_loops.clone(),
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
        mission_states.push(active_mission_state);
    }

    if is_loop {
        if let Some(first) = mission_states.get(first_idx).cloned() {
            let last = mission_states.last_mut().unwrap();
            last.next_mission = Some(Box::new(first));
        }
    }
}

#[proc_macro_attribute]
pub fn tick_distributed(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as ItemFn);

    let mut mission_states: Vec<MissionState> = Vec::new();
    let mut loop_indexes: HashMap<String, (usize, usize)> = HashMap::new();
    analyse_block(input.block.stmts, Vec::new(), &mut mission_states, false, &mut loop_indexes, Vec::new());
    for i in 0..mission_states.len() - 1 {
        if mission_states[i].next_mission.is_none() {
            mission_states[i].next_mission = Some(Box::new(mission_states[i + 1].clone()))
        }
    }

    let mut loops: HashMap<String, (Box<MissionState>, Box<MissionState>)> = HashMap::new();
    for (label, (continue_index, break_index)) in loop_indexes {
        let continue_state = mission_states.get(continue_index).unwrap().clone(); // todo can these panic?
        let break_state = mission_states.get(break_index).unwrap().clone();
        loops.insert(label, (Box::new(continue_state), Box::new(break_state)));
    }
    for mission_state in &mut mission_states {
        replace_breaks_and_continues(&mut mission_state.stmts, &loops, &mission_state.parent_loops);
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
            #[allow(unused_mut)]
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
