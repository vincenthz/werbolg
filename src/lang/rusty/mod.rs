//! This is an entire parser and interpreter for a dynamically-typed Rust-like expression-oriented
mod parse;
mod token;

use crate::ast::Statement;
use alloc::{boxed::Box, vec, vec::Vec};

use super::common::{ast, FileUnit, ParseError};

pub fn module(fileunit: &FileUnit) -> Result<ast::Module, ParseError> {
    let m = parse::module(&fileunit.content).map_err(|_| todo!())?;

    let has_main = m.iter().any(|(s, _)| s == "main");

    let mut statements = m
        .into_iter()
        .map(|(n, fun)| {
            let expr = rewrite_stmt(&fun.body);
            Statement::Function(
                ast::Ident::from(n),
                fun.args
                    .into_iter()
                    .map(|s| ast::Ident::from(s))
                    .collect::<Vec<_>>(),
                expr,
            )
        })
        .collect::<Vec<_>>();

    if has_main {
        statements.push(Statement::Expr(ast::Expr::Call(vec![ast::Expr::Ident(
            ast::Ident::from("main"),
        )])));
    }

    Ok(ast::Module { statements })
}

fn rewrite_stmt(span_expr: &(parse::Expr, parse::Span)) -> Vec<ast::Statement> {
    vec![ast::Statement::Expr(rewrite_expr(span_expr))]
}

fn rewrite_expr(span_expr: &(parse::Expr, parse::Span)) -> ast::Expr {
    match &span_expr.0 {
        parse::Expr::Error => todo!(),
        parse::Expr::Literal(lit) => ast::Expr::Literal(lit.clone()),
        parse::Expr::List(_) => todo!(),
        parse::Expr::Local(l) => ast::Expr::Ident(ast::Ident::from(l.as_str())),
        parse::Expr::Let(name, bind, then) => ast::Expr::Let(
            ast::Ident::from(name.as_str()),
            Box::new(rewrite_expr(bind)),
            Box::new(rewrite_expr(then)),
        ),
        parse::Expr::Then(first, second) => ast::Expr::Then(
            Box::new(rewrite_expr(first)),
            Box::new(rewrite_expr(second)),
        ),
        parse::Expr::Binary(left, op, right) => ast::Expr::Call(vec![
            ast::Expr::Ident(ast::Ident::from(op.as_str())),
            rewrite_expr(&left),
            rewrite_expr(&right),
        ]),
        parse::Expr::Call(x, args) => {
            let mut exprs = vec![rewrite_expr(x)];
            for a in args {
                exprs.push(rewrite_expr(a))
            }
            ast::Expr::Call(exprs)
        }
        parse::Expr::If(cond, then_expr, else_expr) => ast::Expr::If {
            cond: Box::new(rewrite_expr(cond)),
            then_expr: Box::new(rewrite_expr(then_expr)),
            else_expr: Box::new(rewrite_expr(else_expr)),
        },
    }
}
