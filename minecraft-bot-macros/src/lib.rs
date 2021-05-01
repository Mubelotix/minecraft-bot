extern crate proc_macro;
use std::{collections::HashMap, hash::Hash};

use proc_macro::TokenStream;
use proc_macro2::*;
use quote::{format_ident, quote, ToTokens};
use syn::*;

#[derive(Debug, Clone)]
struct MissionState {
    variant_ident: Ident,
    parent_loops: Vec<String>,
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

            quote! {
                GeneratedMissionState::#variant_ident { #(#variant_field_idents,)* } => {
                    #(let #variant_field_mutability #variant_field_idents2 = *#variant_field_idents2;)*
                    #(#stmts)*
                    self.state = GeneratedMissionState::#next_variant_ident { #(#next_variant_fields, )* };
                },
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

    fn switch_to_this_state(&self) -> Expr {
        let variant_ident = &self.variant_ident;
        let fields = self.fields.iter().map(|f| &f.1);
        let tokens = quote! {{
            self.state = GeneratedMissionState::#variant_ident { #(#fields, )* };
            return MissionResult::InProgress;
        }};
        syn::parse2(tokens).unwrap()
    }
}

fn replace_breaks_and_continues_in_expr(expr: &mut Expr, loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>, parent_loops: &[String]) {
    match expr {
        Expr::Array(expr) => {
            for expr in &mut expr.elems {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops);
            }
        }
        Expr::Assign(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops),
        Expr::AssignOp(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops),
        Expr::Async(expr) => replace_breaks_and_continues(&mut expr.block.stmts, loops, parent_loops),
        Expr::Await(expr) => replace_breaks_and_continues_in_expr(&mut expr.base, loops, parent_loops),
        Expr::Binary(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops),
        Expr::Block(expr) => replace_breaks_and_continues(&mut expr.block.stmts, loops, parent_loops),
        Expr::Box(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Break(expr_break) => {
            if let Some(label) = &expr_break.label {
                let label = label.ident.to_string();
                if label.starts_with("mt_") {
                    if !parent_loops.contains(&label) {
                        panic!("Cannot break to {} as there is no parent loop marked with this label.", label);
                    }
                    if expr_break.expr.is_some() {
                        panic!("Break should not contain value");
                    }
                    let next_mission = match loops.get(&label) {
                        Some(next_mission) => next_mission,
                        None => panic!("No loop with the {} label", label),
                    };

                    *expr = next_mission.1.switch_to_this_state();
                    return;
                }
            }
            if let Some(expr) = &mut expr_break.expr {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops);
            }
        }
        Expr::Call(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.func, loops, parent_loops);
            for expr in &mut expr.args {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops);
            }
        }
        Expr::Cast(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Closure(expr) => replace_breaks_and_continues_in_expr(&mut expr.body, loops, parent_loops),
        Expr::Continue(continue_expr) => {
            if let Some(label) = &continue_expr.label {
                let label = label.ident.to_string();
                if label.starts_with("mt_") {
                    if !parent_loops.contains(&label) {
                        panic!("Cannot break to {} as there is no parent loop marked with this label.", label);
                    }
                    let next_mission = match loops.get(&label) {
                        Some(next_mission) => next_mission,
                        None => panic!("No loop with the {} label", label),
                    };

                    *expr = next_mission.0.switch_to_this_state();
                    return;
                }
            }
        }
        Expr::Field(expr) => replace_breaks_and_continues_in_expr(&mut expr.base, loops, parent_loops),
        Expr::ForLoop(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops);
            replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops);
        }
        Expr::Group(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::If(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.cond, loops, parent_loops);
            replace_breaks_and_continues(&mut expr.then_branch.stmts, loops, parent_loops);
            if let Some(expr) = &mut expr.else_branch {
                replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops);
            }
        }
        Expr::Index(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops);
            replace_breaks_and_continues_in_expr(&mut expr.index, loops, parent_loops);
        }
        Expr::Let(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Lit(_) => {}
        Expr::Loop(expr) => replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops),
        Expr::Macro(_) => {}
        Expr::Match(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops);
            for arm in &mut expr.arms {
                if let Some(expr) = &mut arm.guard {
                    replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops);
                }
                replace_breaks_and_continues_in_expr(&mut arm.body, loops, parent_loops);
            }
        }
        Expr::MethodCall(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.receiver, loops, parent_loops);
            for expr in &mut expr.args {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops);
            }
        }
        Expr::Paren(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Path(_) => {}
        Expr::Range(expr) => {
            if let Some(from) = &mut expr.from {
                replace_breaks_and_continues_in_expr(from, loops, parent_loops);
            }
            if let Some(to) = &mut expr.to {
                replace_breaks_and_continues_in_expr(to, loops, parent_loops);
            }
        }
        Expr::Reference(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Repeat(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops);
            replace_breaks_and_continues_in_expr(&mut expr.len, loops, parent_loops);
        }
        Expr::Return(_) => todo!(),
        Expr::Struct(expr) => {
            for field in &mut expr.fields {
                replace_breaks_and_continues_in_expr(&mut field.expr, loops, parent_loops);
            }
            if let Some(rest) = &mut expr.rest {
                replace_breaks_and_continues_in_expr(rest, loops, parent_loops);
            }
        }
        Expr::Try(_) => todo!(),
        Expr::TryBlock(_) => todo!(),
        Expr::Tuple(expr) => for elem in &mut expr.elems {
            replace_breaks_and_continues_in_expr(elem, loops, parent_loops);
        }
        Expr::Type(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Unary(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops),
        Expr::Unsafe(_) => {}
        Expr::Verbatim(_) => {}
        Expr::While(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.cond, loops, parent_loops);
            replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops);
        }
        Expr::Yield(expr) => if let Some(expr) = &mut expr.expr {
            replace_breaks_and_continues_in_expr(expr, loops, parent_loops);
        },
        Expr::__TestExhaustive(_) => {}
    }
}

fn replace_breaks_and_continues(stmts: &mut Vec<Stmt>, loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>, parent_loops: &[String]) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(stmt) => {
                if let Some(expr) = &mut stmt.init {
                    replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops);
                }
            }
            Stmt::Item(_) => {}
            Stmt::Expr(expr) => replace_breaks_and_continues_in_expr(expr, loops, parent_loops),
            Stmt::Semi(expr, _) => replace_breaks_and_continues_in_expr(expr, loops, parent_loops),
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
