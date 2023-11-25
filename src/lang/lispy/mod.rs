//! an unfinished lang frontend for replacing the scheme lang by a more efficient one
mod parse;
mod token;

use super::common::{FileUnit, ParseError};
use crate::ir;
use crate::ir::{spans_merge, Span};
use alloc::{boxed::Box, vec::Vec};

pub fn module(fileunit: &FileUnit) -> Result<ir::Module, ParseError> {
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

    Ok(ir::Module { statements })
}

/// Turn a vector of lisp parse expression into a ast::Expr of the form `let ID1 = LAMBDA1; let ID2 = LAMBDA2; ...; LAST_EXPR`
///
fn exprs_into_let(exprs: Vec<parse::Expr>) -> ir::Expr {
    let mut exprs = exprs.into_iter().rev();

    let mut accumulator = expr(
        exprs
            .next()
            .expect("exprs_into_let cannot be called on empty vec"),
    );

    while let Some(e) = exprs.next() {
        match e {
            parse::Expr::Define(_span, name, args, body) => {
                let body = exprs_into_let(body);
                let span_args = spans_merge(&mut args.iter().map(|sargs| &sargs.0.span));
                accumulator = ir::Expr::Let(
                    name,
                    Box::new(ir::Expr::Lambda(span_args, args, Box::new(body))),
                    Box::new(accumulator),
                )
            }
            _ => {
                panic!("trying to have a non function in let")
            }
        }
    }

    accumulator
}

fn statement(expr: parse::Expr) -> ir::Statement {
    match expr {
        parse::Expr::Atom(span, ident) => ir::Statement::Expr(ir::Expr::Ident(span, ident)),
        parse::Expr::Literal(span, lit) => {
            ir::Statement::Expr(ir::Expr::Literal(span, literal(lit)))
        }
        parse::Expr::List(span, list) => ir::Statement::Expr(exprs(span, list)),
        parse::Expr::Define(span, name, args, body) => {
            let body = exprs_into_let(body);
            ir::Statement::Function(span, name.unspan(), args, body)
        }
    }
}

fn expr(expr: parse::Expr) -> ir::Expr {
    match expr {
        parse::Expr::Atom(span, ident) => ir::Expr::Ident(span, ident),
        parse::Expr::Literal(span, lit) => ir::Expr::Literal(span, literal(lit)),
        parse::Expr::List(span, e) => exprs(span, e),
        parse::Expr::Define(_, _, _, _) => {
            panic!("cannot have define in expression")
        }
    }
}

fn exprs(span: Span, exprs: Vec<parse::Expr>) -> ir::Expr {
    if let Some((_, _)) = exprs[0].literal() {
        ir::Expr::List(span, exprs.into_iter().map(|e| expr(e)).collect())
    } else {
        ir::Expr::Call(span, exprs.into_iter().map(|e| expr(e)).collect())
    }
}

fn literal(lit: parse::Literal) -> ir::Literal {
    match lit {
        parse::Literal::Bytes(b) => ir::Literal::Bytes(b.into()),
        parse::Literal::Number(n) => {
            ir::Literal::Number(ir::Number::from_str_radix(&n, 10).unwrap())
        }
        parse::Literal::String(s) => ir::Literal::String(s),
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
                panic!("error: {:?}", e)
                //panic!("parsing failed: {:?}\n{}", e, fileunit.resolve_error(&e))
            }
            Ok(res) => {
                for _stmt in res.statements {
                    //println!("{:?}", stmt)
                    ()
                }
            }
        }
    }
}
