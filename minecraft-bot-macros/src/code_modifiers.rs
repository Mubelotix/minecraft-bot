use crate::mission_state::*;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use syn::*;

pub(crate) fn replace_code_in_expr(
    expr: &mut Expr,
    loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>,
    parent_loops: &[String],
    state_name: &Ident,
) {
    match expr {
        Expr::Array(expr) => {
            for expr in &mut expr.elems {
                replace_code_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Assign(expr) => replace_code_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::AssignOp(expr) => replace_code_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::Async(expr) => replace_code(&mut expr.block.stmts, loops, parent_loops, state_name),
        Expr::Await(expr) => replace_code_in_expr(&mut expr.base, loops, parent_loops, state_name),
        Expr::Binary(expr) => replace_code_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::Block(expr) => replace_code(&mut expr.block.stmts, loops, parent_loops, state_name),
        Expr::Box(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Break(expr_break) => {
            if let Some(label) = &expr_break.label {
                let label = label.ident.to_string();
                if label.starts_with("mt_") {
                    if !parent_loops.contains(&label) {
                        if label.ends_with("_procmacroderived") {
                            panic!("Automatic sub-mission is nested too deeply nested")
                        }
                        panic!("Cannot break to {} as there is no parent loop marked with this label.", label);
                    }
                    let next_mission = match loops.get(&label) {
                        Some(next_mission) => next_mission,
                        None => panic!("No loop with the {} label", label),
                    };

                    *expr = next_mission.1.switch_to_this_state(expr_break.expr.clone());
                    return;
                }
            }
            if let Some(expr) = &mut expr_break.expr {
                replace_code_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Call(expr) => {
            replace_code_in_expr(&mut expr.func, loops, parent_loops, state_name);
            for expr in &mut expr.args {
                replace_code_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Cast(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Closure(expr) => replace_code_in_expr(&mut expr.body, loops, parent_loops, state_name),
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

                    *expr = next_mission.0.switch_to_this_state(None);
                    return;
                }
            }
        }
        Expr::Field(expr) => replace_code_in_expr(&mut expr.base, loops, parent_loops, state_name),
        Expr::ForLoop(expr) => {
            replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_code(&mut expr.body.stmts, loops, parent_loops, state_name);
        }
        Expr::Group(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::If(expr) => {
            replace_code_in_expr(&mut expr.cond, loops, parent_loops, state_name);
            replace_code(&mut expr.then_branch.stmts, loops, parent_loops, state_name);
            if let Some(expr) = &mut expr.else_branch {
                replace_code_in_expr(&mut expr.1, loops, parent_loops, state_name);
            }
        }
        Expr::Index(expr) => {
            replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_code_in_expr(&mut expr.index, loops, parent_loops, state_name);
        }
        Expr::Let(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Lit(_) => {}
        Expr::Loop(expr) => replace_code(&mut expr.body.stmts, loops, parent_loops, state_name),
        Expr::Macro(_) => {}
        Expr::Match(expr) => {
            replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            for arm in &mut expr.arms {
                if let Some(expr) = &mut arm.guard {
                    replace_code_in_expr(&mut expr.1, loops, parent_loops, state_name);
                }
                replace_code_in_expr(&mut arm.body, loops, parent_loops, state_name);
            }
        }
        Expr::MethodCall(expr) => {
            replace_code_in_expr(&mut expr.receiver, loops, parent_loops, state_name);
            for expr in &mut expr.args {
                replace_code_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Paren(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Path(_) => {}
        Expr::Range(expr) => {
            if let Some(from) = &mut expr.from {
                replace_code_in_expr(from, loops, parent_loops, state_name);
            }
            if let Some(to) = &mut expr.to {
                replace_code_in_expr(to, loops, parent_loops, state_name);
            }
        }
        Expr::Reference(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Repeat(expr) => {
            replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_code_in_expr(&mut expr.len, loops, parent_loops, state_name);
        }
        Expr::Return(expr_return) => {
            let returned_expr = if let Some(returned_expr) = &mut expr_return.expr {
                replace_code_in_expr(returned_expr, loops, parent_loops, state_name);
                returned_expr
            } else {
                todo!()
            };

            let tokens = quote! {{
                *self = #state_name::Done {};
                return MissionResult::Done(#returned_expr);
            }};
            *expr = syn::parse2(tokens).unwrap();
        }
        Expr::Struct(expr) => {
            for field in &mut expr.fields {
                replace_code_in_expr(&mut field.expr, loops, parent_loops, state_name);
            }
            if let Some(rest) = &mut expr.rest {
                replace_code_in_expr(rest, loops, parent_loops, state_name);
            }
        }
        Expr::Try(_) => todo!(),
        Expr::TryBlock(_) => todo!(),
        Expr::Tuple(expr) => {
            for elem in &mut expr.elems {
                replace_code_in_expr(elem, loops, parent_loops, state_name);
            }
        }
        Expr::Type(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Unary(expr) => replace_code_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Unsafe(expr_unsafe) => replace_code(&mut expr_unsafe.block.stmts, loops, parent_loops, state_name),
        Expr::Verbatim(_) => {}
        Expr::While(expr) => {
            replace_code_in_expr(&mut expr.cond, loops, parent_loops, state_name);
            replace_code(&mut expr.body.stmts, loops, parent_loops, state_name);
        }
        Expr::Yield(expr) => {
            if let Some(expr) = &mut expr.expr {
                replace_code_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::__TestExhaustive(_) => {}
    }
}

pub(crate) fn replace_code(
    stmts: &mut Vec<Stmt>,
    loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>,
    parent_loops: &[String],
    state_name: &Ident,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(stmt) => {
                if let Some(expr) = &mut stmt.init {
                    replace_code_in_expr(&mut expr.1, loops, parent_loops, state_name);
                }
            }
            Stmt::Item(_) => {}
            Stmt::Expr(expr) => replace_code_in_expr(expr, loops, parent_loops, state_name),
            Stmt::Semi(expr, _) => replace_code_in_expr(expr, loops, parent_loops, state_name),
        }
    }
}

// TODO: Simplify this as we can't even support nested expressions
pub(crate) fn replace_mt_functions_in_expr(expr: &mut Expr) {
    match expr {
        Expr::Array(expr_array) => {
            for expr in &mut expr_array.elems {
                replace_mt_functions_in_expr(expr);
            }
        }
        Expr::Assign(expr_assign) => replace_mt_functions_in_expr(&mut expr_assign.right),
        Expr::AssignOp(expr_assign_op) => replace_mt_functions_in_expr(&mut expr_assign_op.right),
        Expr::Async(expr_async) => replace_mt_functions(&mut expr_async.block.stmts),
        Expr::Await(expr_await) => replace_mt_functions_in_expr(&mut expr_await.base),
        Expr::Binary(expr_binary) => {
            replace_mt_functions_in_expr(&mut expr_binary.left);
            replace_mt_functions_in_expr(&mut expr_binary.right);
        }
        Expr::Block(expr_block) => replace_mt_functions(&mut expr_block.block.stmts),
        Expr::Box(expr_box) => replace_mt_functions_in_expr(&mut expr_box.expr),
        Expr::Break(expr_break) => {
            if let Some(expr) = &mut expr_break.expr {
                replace_mt_functions_in_expr(expr)
            }
        }
        Expr::Call(expr_call) => {
            for arg in &mut expr_call.args {
                replace_mt_functions_in_expr(arg);
            }

            let func: &mut Expr = &mut expr_call.func;
            let func_path = match func {
                Expr::Path(expr_path) => &mut expr_path.path,
                _ => panic!(),
            };
            let func_name = func_path.segments.last_mut().unwrap();
            if func_name.ident.to_string().starts_with("mt_") {
                // Remove the mt_ prefix
                let new_func_name = &func_name.ident.to_string()[3..];
                func_name.ident = Ident::new(new_func_name, func_name.ident.span());

                // Find the type of the sub-mission
                use convert_case::{Case, Casing};
                let mut mission_ty = func_path.clone();
                let mut mission_ty = mission_ty.segments.last_mut().unwrap();
                mission_ty.ident = Ident::new(&format!("{}Mission", new_func_name.to_case(Case::Pascal)), mission_ty.ident.span());

                // Generate a random loop
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let mut loop_name = String::new();
                loop_name.push_str("'mt_");
                for _ in 0..24 {
                    loop_name.push(rng.gen_range(b'a'..b'z') as char);
                }
                loop_name.push_str("_procmacroderived");
                let loop_name = Lifetime::new(&loop_name, Span::call_site());

                // Build the code
                let tokens = quote! {{
                    let mut submission: #mission_ty = #expr_call;
                    #loop_name: loop {
                        match submission.execute() {
                            MissionResult::InProgress => (),
                            MissionResult::Done(value) => break #loop_name value,
                            MissionResult::Outdated => panic!("Outdated mission"),
                        }
                    }
                }};
                *expr = syn::parse2(tokens).unwrap();
            }
        }
        Expr::Cast(expr_cast) => replace_mt_functions_in_expr(&mut expr_cast.expr),
        Expr::Closure(closure_expr) => replace_mt_functions_in_expr(&mut closure_expr.body),
        Expr::Continue(_) => (),
        Expr::Field(_) => (),
        Expr::ForLoop(expr_for) => {
            replace_mt_functions_in_expr(&mut expr_for.expr);
            replace_mt_functions(&mut expr_for.body.stmts);
        }
        Expr::Group(expr_group) => replace_mt_functions_in_expr(&mut expr_group.expr),
        Expr::If(expr_if) => {
            replace_mt_functions_in_expr(&mut expr_if.cond);
            replace_mt_functions(&mut expr_if.then_branch.stmts);
            if let Some((_, else_expr)) = expr_if.else_branch.as_mut() {
                replace_mt_functions_in_expr(else_expr);
            }
        }
        Expr::Index(expr_index) => {
            replace_mt_functions_in_expr(&mut expr_index.expr);
            replace_mt_functions_in_expr(&mut expr_index.index);
        }
        Expr::Let(expr_let) => replace_mt_functions_in_expr(&mut expr_let.expr),
        Expr::Lit(_) => (),
        Expr::Loop(expr_loop) => replace_mt_functions(&mut expr_loop.body.stmts),
        Expr::Macro(_) => (),
        Expr::Match(expr_match) => {
            replace_mt_functions_in_expr(&mut expr_match.expr);
            for arm in &mut expr_match.arms {
                if let Some((_, guard)) = &mut arm.guard {
                    replace_mt_functions_in_expr(guard);
                }
                replace_mt_functions_in_expr(&mut arm.body);
            }
        }
        Expr::MethodCall(expr_method) => {
            replace_mt_functions_in_expr(&mut expr_method.receiver);
            for arg in &mut expr_method.args {
                replace_mt_functions_in_expr(arg);
            }
        }
        Expr::Paren(expr_paren) => replace_mt_functions_in_expr(&mut expr_paren.expr),
        Expr::Path(_) => (),
        Expr::Range(expr_range) => {
            if let Some(expr) = &mut expr_range.from {
                replace_mt_functions_in_expr(expr);
            }
            if let Some(expr) = &mut expr_range.to {
                replace_mt_functions_in_expr(expr);
            }
        }
        Expr::Reference(expr_ref) => replace_mt_functions_in_expr(&mut expr_ref.expr),
        Expr::Repeat(expr_repeat) => replace_mt_functions_in_expr(&mut expr_repeat.expr),
        Expr::Return(expr_return) => {
            if let Some(expr) = &mut expr_return.expr {
                replace_mt_functions_in_expr(expr)
            }
        }
        Expr::Struct(_) => (),
        Expr::Try(expr_try) => replace_mt_functions_in_expr(&mut expr_try.expr),
        Expr::TryBlock(_) => unimplemented!("try blocks"),
        Expr::Tuple(expr_tuple) => {
            for expr in &mut expr_tuple.elems {
                replace_mt_functions_in_expr(expr)
            }
        }
        Expr::Type(_) => (),
        Expr::Unary(expr_unary) => replace_mt_functions_in_expr(&mut expr_unary.expr),
        Expr::Unsafe(expr_unsafe) => replace_mt_functions(&mut expr_unsafe.block.stmts),
        Expr::Verbatim(_) => (),
        Expr::While(expr_while) => {
            replace_mt_functions_in_expr(&mut expr_while.cond);
            replace_mt_functions(&mut expr_while.body.stmts);
        }
        Expr::Yield(expr_yields) => {
            if let Some(expr) = &mut expr_yields.expr {
                replace_mt_functions_in_expr(expr)
            }
        }
        Expr::__TestExhaustive(_) => todo!(),
    }
}

pub(crate) fn replace_mt_functions(stmts: &mut Vec<Stmt>) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(stmt) => {
                if let Some(expr) = &mut stmt.init {
                    replace_mt_functions_in_expr(&mut expr.1);
                }
            }
            Stmt::Item(_) => {}
            Stmt::Expr(expr) => replace_mt_functions_in_expr(expr),
            Stmt::Semi(expr, _) => replace_mt_functions_in_expr(expr),
        }
    }
}
