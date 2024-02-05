//! This is an entire parser and interpreter for a dynamically-typed Rust-like expression-oriented

#![no_std]

extern crate alloc;

mod ast;
mod parse;
mod token;

use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use werbolg_core::{self as ir, Spanned, Statement};

use werbolg_lang_common::{FileUnit, ParseError, ParseErrorKind};

fn simple_to_perr(e: chumsky::error::Simple<String>) -> ParseError {
    match e.reason() {
        chumsky::error::SimpleReason::Unclosed { span: _, delimiter } => ParseError {
            context: None,
            location: e.span(),
            description: format!("Unclosed delimiter {}", delimiter),
            note: Some(format!("delimiter must be closed before")),
            kind: ParseErrorKind::Unknown,
        },
        chumsky::error::SimpleReason::Unexpected => {
            let found = if e.found().is_some() {
                "Unexpected token in input"
            } else {
                "Unexpected end of input"
            };
            let expected = if e.expected().len() == 0 {
                String::from("Something else")
            } else {
                e.expected()
                    .map(|expected| match expected {
                        Some(expected) => String::from(expected),
                        None => String::from("end of input"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let description = format!("{}, expected {}", found, expected);
            ParseError {
                context: None,
                location: e.span(),
                description,
                note: None,
                kind: ParseErrorKind::Unknown,
            }
        }

        chumsky::error::SimpleReason::Custom(msg) => ParseError {
            context: None,
            location: e.span(),
            description: format!("{}", msg),
            note: None,
            kind: ParseErrorKind::Unknown,
        },
    }
}

pub fn module(fileunit: &FileUnit) -> Result<ir::Module, Vec<ParseError>> {
    let m = parse::module(&fileunit.content).map_err(|errs| {
        errs.into_iter()
            .map(|err| simple_to_perr(err))
            .collect::<Vec<_>>()
    })?;

    let statements = m
        .into_iter()
        .map(|(n, span, fun)| {
            let body = rewrite_expr(&fun.body);
            Statement::Function(
                span,
                ir::FunDef {
                    privacy: ir::Privacy::Public,
                    name: ir::Ident::from(n),
                },
                ir::FunImpl {
                    vars: fun.args,
                    body,
                },
            )
        })
        .collect::<Vec<_>>();

    Ok(ir::Module { statements })
}

fn rewrite_expr_spanbox(span_expr: &(parse::Expr, parse::Span)) -> Box<Spanned<ir::Expr>> {
    let span = span_expr.1.clone();
    let expr = rewrite_expr(span_expr);
    Box::new(Spanned::new(span, expr))
}

fn rewrite_expr(span_expr: &(parse::Expr, parse::Span)) -> ir::Expr {
    match &span_expr.0 {
        parse::Expr::Error => todo!(),
        parse::Expr::Literal(lit) => ir::Expr::Literal(span_expr.1.clone(), lit.clone()),
        parse::Expr::List(list) => ir::Expr::Sequence(
            span_expr.1.clone(),
            list.iter().map(|se| rewrite_expr(se)).collect::<Vec<_>>(),
        ),
        parse::Expr::Local(l) => ir::Expr::Path(
            span_expr.1.clone(),
            ir::Path::relative(ir::Ident::from(l.as_str())),
        ),
        parse::Expr::Let(name, bind, then) => ir::Expr::Let(
            ir::Binder::Ident(ir::Ident::from(name.as_str())),
            Box::new(rewrite_expr(bind)),
            Box::new(rewrite_expr(then)),
        ),
        parse::Expr::Then(first, second) => ir::Expr::Let(
            ir::Binder::Ignore,
            Box::new(rewrite_expr(first)),
            Box::new(rewrite_expr(second)),
        ),
        parse::Expr::Binary(left, op, right) => ir::Expr::Call(
            span_expr.1.clone(),
            vec![
                ir::Expr::Path(
                    /* should be op's span */ span_expr.1.clone(),
                    ir::Path::absolute(ir::Ident::from(op.as_str())),
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
            cond: rewrite_expr_spanbox(cond),
            then_expr: rewrite_expr_spanbox(then_expr),
            else_expr: rewrite_expr_spanbox(else_expr),
        },
    }
}
