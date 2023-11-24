//! an unfinished lang frontend for replacing the scheme lang by a more efficient one
mod parse;
mod token;

use super::common::{FileUnit, ParseError};
use crate::ast;
use alloc::vec::Vec;

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

fn statement(expr: parse::Expr) -> ast::Statement {
    match expr {
        parse::Expr::Atom(span, ident) => ast::Statement::Expr(ast::Expr::Ident(span, ident)),
        parse::Expr::Literal(span, lit) => {
            ast::Statement::Expr(ast::Expr::Literal(span, literal(lit)))
        }
        parse::Expr::List(span, list) => ast::Statement::Expr(exprs(span, list)),
        parse::Expr::Define(span, args, body) => {
            let Some((ident, args)) = args.split_first() else {
                panic!("cannot happen")
            };
            let name = ident.0.clone();
            let args = args.iter().map(|(i, _)| i.clone()).collect::<Vec<_>>();
            let body = body.into_iter().map(statement).collect::<Vec<_>>();
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
