//! an unfinished lang frontend for replacing the scheme lang by a more efficient one
mod parse;
mod token;

use super::common::{FileUnit, ParseError};
use crate::ast;
use alloc::{boxed::Box, vec, vec::Vec};

pub fn module(fileunit: &FileUnit) -> Result<ast::Module, ParseError> {
    let lex = parse::Lexer::new(&fileunit.content);
    let mut parser = parse::Parser::new(lex);

    let mut statements = Vec::new();
    while let Some(v) = parser.next() {
        match v {
            Err(pe) => {
                //println!("pe {:?}", pe);
                todo!("{:?}", pe)
            }
            Ok(e) => {
                let stmt = statement(e);
                statements.push(stmt)
            }
        }
    }

    Ok(ast::Module { statements })
}

fn exprs_into_let(exprs: &Vec<parse::Expr>) -> ast::Expr {
    enum State {
        None,
        PExpr(parse::Expr),
        AExpr(
            Vec<(
                ast::Ident,
                ast::Span,
                Vec<(ast::Ident, ast::Span)>,
                ast::Expr,
            )>,
        ),
    }
    if exprs.len() == 1 {
        expr(exprs[0].clone())
    } else {
        let mut acc = State::None;
        for exp in &exprs[0..exprs.len() - 1] {
            match acc {
                State::None => acc = State::PExpr(exp.clone()),
                State::PExpr(prev_expr) => {
                    match prev_expr {
                        parse::Expr::Define(span, args, body) => {
                            let (_, name, span_args, args) = define_to_ident_args(args);
                            let body = exprs_into_let(&body);
                            acc = State::AExpr(vec![(name, span_args, args, body)]);
                        }
                        _ => {
                            panic!("trying to have a non function in let")
                        }
                    }
                    //let x = toto; let y = tata; final_expr
                }
                State::AExpr(acc) => {
                    todo!()
                } // ast::Expr::Let(ident, Box::new(bind), Box::new(body)),
            }
        }
        match acc {
            State::AExpr(acc) => {
                let last_expr = exprs[exprs.len() - 1].clone();
                let body = expr(last_expr);

                acc.into_iter()
                    .rev()
                    .fold(body, |acc, (ident, span_args, args, body)| {
                        ast::Expr::Let(
                            ident,
                            Box::new(ast::Expr::Lambda(span_args, args, Box::new(body))),
                            Box::new(acc),
                        )
                    })
            }
            _ => panic!("invalid state for exprs_into_let"),
        }
    }
}

fn define_to_ident_args(
    args: Vec<(ast::Ident, ast::Span)>,
) -> (
    ast::Span,
    ast::Ident,
    ast::Span,
    Vec<(ast::Ident, ast::Span)>,
) {
    let Some(((ident, span), args)) = args.split_first() else {
        panic!("cannot happen")
    };
    let args = args
        .iter()
        .map(|(i, s)| (i.clone(), s.clone()))
        .collect::<Vec<_>>();
    // TODO 2 span need to be the span of all arguments
    (span.clone(), ident.clone(), span.clone(), args)
}

fn statement(expr: parse::Expr) -> ast::Statement {
    match expr {
        parse::Expr::Atom(span, ident) => ast::Statement::Expr(ast::Expr::Ident(span, ident)),
        parse::Expr::Literal(span, lit) => {
            ast::Statement::Expr(ast::Expr::Literal(span, literal(lit)))
        }
        parse::Expr::List(span, list) => ast::Statement::Expr(exprs(span, list)),
        parse::Expr::Define(span, args, body) => {
            let (_, name, span_args, args) = define_to_ident_args(args);
            let body = exprs_into_let(&body);
            ast::Statement::Function(span, name, args, body)
        }
    }
}

fn expr(expr: parse::Expr) -> ast::Expr {
    match expr {
        parse::Expr::Atom(span, ident) => ast::Expr::Ident(span, ident),
        parse::Expr::Literal(span, lit) => ast::Expr::Literal(span, literal(lit)),
        parse::Expr::List(span, e) => exprs(span, e),
        parse::Expr::Define(_, _, _) => {
            panic!("cannot have define in expression")
        }
    }
}

fn exprs(span: parse::Span, exprs: Vec<parse::Expr>) -> ast::Expr {
    if let Some((_, _)) = exprs[0].literal() {
        ast::Expr::List(span, exprs.into_iter().map(|e| expr(e)).collect())
    } else {
        ast::Expr::Call(span, exprs.into_iter().map(|e| expr(e)).collect())
    }
}

fn literal(lit: parse::Literal) -> ast::Literal {
    match lit {
        parse::Literal::Bytes(b) => ast::Literal::Bytes(b.into()),
        parse::Literal::Number(n) => {
            ast::Literal::Number(ast::Number::from_str_radix(&n, 10).unwrap())
        }
        parse::Literal::String(s) => ast::Literal::String(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let snippet = r#"
        (define add3 (a b c)
            (+ (+ a b) c)
        )
        (add3 10 20 30)
        "#;
        let fileunit = FileUnit::from_str("test", snippet);
        let res = module(&fileunit);
        match res {
            Err(e) => {
                //panic!("parsing failed: {:?}\n{}", e, fileunit.resolve_error(&e))
            }
            Ok(res) => {
                for stmt in res.statements {
                    //println!("{:?}", stmt)
                    ()
                }
            }
        }
    }
}
