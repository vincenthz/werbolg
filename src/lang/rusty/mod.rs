//! This is an entire parser and interpreter for a dynamically-typed Rust-like expression-oriented
mod parse;
mod token;

use crate::ir::{Spanned, Statement};
use alloc::{boxed::Box, vec, vec::Vec};

use super::common::{ir, FileUnit, ParseError};

pub fn module(fileunit: &FileUnit) -> Result<ir::Module, ParseError> {
    let m = parse::module(&fileunit.content).map_err(|_| todo!())?;

    let has_main = m.iter().any(|(s, _, _)| s == "main");

    let mut statements = m
        .into_iter()
        .map(|(n, span, fun)| {
            //let expr = rewrite_stmt(&fun.body);
            let body = rewrite_expr(&fun.body);
            Statement::Function(span, ir::Ident::from(n), fun.args, body)
        })
        .collect::<Vec<_>>();

    if has_main {
        let fake_span = core::ops::Range { start: 0, end: 0 };
        statements.push(Statement::Expr(ir::Expr::Call(
            fake_span.clone(),
            vec![ir::Expr::Ident(fake_span, ir::Ident::from("main"))],
        )));
    }

    Ok(ir::Module { statements })
}

fn rewrite_expr(span_expr: &(parse::Expr, parse::Span)) -> ir::Expr {
    match &span_expr.0 {
        parse::Expr::Error => todo!(),
        parse::Expr::Literal(lit) => ir::Expr::Literal(span_expr.1.clone(), lit.clone()),
        parse::Expr::List(_) => todo!(),
        parse::Expr::Local(l) => ir::Expr::Ident(span_expr.1.clone(), ir::Ident::from(l.as_str())),
        parse::Expr::Let(name, bind, then) => ir::Expr::Let(
            Spanned::new(span_expr.1.clone(), ir::Ident::from(name.as_str())),
            Box::new(rewrite_expr(bind)),
            Box::new(rewrite_expr(then)),
        ),
        parse::Expr::Then(first, second) => ir::Expr::Then(
            Box::new(rewrite_expr(first)),
            Box::new(rewrite_expr(second)),
        ),
        parse::Expr::Binary(left, op, right) => ir::Expr::Call(
            span_expr.1.clone(),
            vec![
                ir::Expr::Ident(
                    /* should be op's span */ span_expr.1.clone(),
                    ir::Ident::from(op.as_str()),
                ),
                rewrite_expr(&left),
                rewrite_expr(&right),
            ],
        ),
        parse::Expr::Call(x, args) => {
            let mut exprs = vec![rewrite_expr(x)];
            for a in args {
                exprs.push(rewrite_expr(a))
            }
            ir::Expr::Call(span_expr.1.clone(), exprs)
        }
        parse::Expr::If(cond, then_expr, else_expr) => ir::Expr::If {
            span: span_expr.1.clone(),
            cond: Box::new(rewrite_expr(cond)),
            then_expr: Box::new(rewrite_expr(then_expr)),
            else_expr: Box::new(rewrite_expr(else_expr)),
        },
    }
}
