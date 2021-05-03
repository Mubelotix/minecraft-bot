use syn::*;
use crate::mission_state::*;
use std::collections::HashMap;
use quote::quote;

pub(crate) fn replace_breaks_and_continues_in_expr(expr: &mut Expr, loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>, parent_loops: &[String], state_name: &Ident) {
    match expr {
        Expr::Array(expr) => {
            for expr in &mut expr.elems {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Assign(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::AssignOp(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::Async(expr) => replace_breaks_and_continues(&mut expr.block.stmts, loops, parent_loops, state_name),
        Expr::Await(expr) => replace_breaks_and_continues_in_expr(&mut expr.base, loops, parent_loops, state_name),
        Expr::Binary(expr) => replace_breaks_and_continues_in_expr(&mut expr.right, loops, parent_loops, state_name),
        Expr::Block(expr) => replace_breaks_and_continues(&mut expr.block.stmts, loops, parent_loops, state_name),
        Expr::Box(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
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
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Call(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.func, loops, parent_loops, state_name);
            for expr in &mut expr.args {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Cast(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Closure(expr) => replace_breaks_and_continues_in_expr(&mut expr.body, loops, parent_loops, state_name),
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
        Expr::Field(expr) => replace_breaks_and_continues_in_expr(&mut expr.base, loops, parent_loops, state_name),
        Expr::ForLoop(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops, state_name);
        }
        Expr::Group(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::If(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.cond, loops, parent_loops, state_name);
            replace_breaks_and_continues(&mut expr.then_branch.stmts, loops, parent_loops, state_name);
            if let Some(expr) = &mut expr.else_branch {
                replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops, state_name);
            }
        }
        Expr::Index(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_breaks_and_continues_in_expr(&mut expr.index, loops, parent_loops, state_name);
        }
        Expr::Let(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Lit(_) => {}
        Expr::Loop(expr) => replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops, state_name),
        Expr::Macro(_) => {}
        Expr::Match(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            for arm in &mut expr.arms {
                if let Some(expr) = &mut arm.guard {
                    replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops, state_name);
                }
                replace_breaks_and_continues_in_expr(&mut arm.body, loops, parent_loops, state_name);
            }
        }
        Expr::MethodCall(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.receiver, loops, parent_loops, state_name);
            for expr in &mut expr.args {
                replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name);
            }
        }
        Expr::Paren(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Path(_) => {}
        Expr::Range(expr) => {
            if let Some(from) = &mut expr.from {
                replace_breaks_and_continues_in_expr(from, loops, parent_loops, state_name);
            }
            if let Some(to) = &mut expr.to {
                replace_breaks_and_continues_in_expr(to, loops, parent_loops, state_name);
            }
        }
        Expr::Reference(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Repeat(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name);
            replace_breaks_and_continues_in_expr(&mut expr.len, loops, parent_loops, state_name);
        }
        Expr::Return(expr_return) => {
            let returned_expr = if let Some(returned_expr) = &mut expr_return.expr {
                replace_breaks_and_continues_in_expr(returned_expr, loops, parent_loops, state_name);
                returned_expr
            } else {
                todo!()
            };

            let tokens = quote! {{
                self.state = #state_name::Done {};
                return MissionResult::Done(#returned_expr);
            }};
            *expr = syn::parse2(tokens).unwrap();
        },
        Expr::Struct(expr) => {
            for field in &mut expr.fields {
                replace_breaks_and_continues_in_expr(&mut field.expr, loops, parent_loops, state_name);
            }
            if let Some(rest) = &mut expr.rest {
                replace_breaks_and_continues_in_expr(rest, loops, parent_loops, state_name);
            }
        }
        Expr::Try(_) => todo!(),
        Expr::TryBlock(_) => todo!(),
        Expr::Tuple(expr) => for elem in &mut expr.elems {
            replace_breaks_and_continues_in_expr(elem, loops, parent_loops, state_name);
        }
        Expr::Type(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Unary(expr) => replace_breaks_and_continues_in_expr(&mut expr.expr, loops, parent_loops, state_name),
        Expr::Unsafe(_) => {}
        Expr::Verbatim(_) => {}
        Expr::While(expr) => {
            replace_breaks_and_continues_in_expr(&mut expr.cond, loops, parent_loops, state_name);
            replace_breaks_and_continues(&mut expr.body.stmts, loops, parent_loops, state_name);
        }
        Expr::Yield(expr) => if let Some(expr) = &mut expr.expr {
            replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name);
        },
        Expr::__TestExhaustive(_) => {}
    }
}

pub(crate) fn replace_breaks_and_continues(stmts: &mut Vec<Stmt>, loops: &HashMap<String, (Box<MissionState>, Box<MissionState>)>, parent_loops: &[String], state_name: &Ident) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(stmt) => {
                if let Some(expr) = &mut stmt.init {
                    replace_breaks_and_continues_in_expr(&mut expr.1, loops, parent_loops, state_name);
                }
            }
            Stmt::Item(_) => {}
            Stmt::Expr(expr) => replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name),
            Stmt::Semi(expr, _) => replace_breaks_and_continues_in_expr(expr, loops, parent_loops, state_name),
        }
    }
}
