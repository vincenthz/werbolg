//! an unfinished lang frontend for replacing the scheme lang by a more efficient one
mod ast;
mod parse;
mod token;

use super::common::{FileUnit, ParseError, ParseErrorKind};
use crate::ir;
use crate::ir::{spans_merge, Span};
use alloc::{boxed::Box, format, string::String, vec::Vec};
use ast::Ast;

pub fn module(fileunit: &FileUnit) -> Result<ir::Module, ParseError> {
    let lex = parse::Lexer::new(&fileunit.content);
    let parser = parse::Parser::new(lex);

    let statements = parser
        .into_iter()
        .map(|re| re.map_err(remap_err).and_then(|e| statement(e)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ir::Module { statements })
}

fn remap_err(e: parse::ParseError) -> ParseError {
    match e {
        parse::ParseError::NotStartedList(span) => ParseError {
            location: span,
            kind: ParseErrorKind::Str(String::from("list not start")),
        },
        parse::ParseError::UnterminatedList(span) => ParseError {
            location: span,
            kind: ParseErrorKind::Str(String::from("unterminated list")),
        },
        parse::ParseError::LexingError(span) => ParseError {
            location: span,
            kind: ParseErrorKind::Str(String::from("unknown character")),
        },
        parse::ParseError::DefineEmptyName {
            define_span,
            args_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define with empty name")),
        },
        parse::ParseError::DefineArgumentNotList {
            define_span,
            args_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define argument not a list")),
        },
        parse::ParseError::DefineArgumentNotAtom {
            define_span,
            args_span: _,
            arg_invalid_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define argument not an atom")),
        },
    }
}

/// Turn a vector of lisp parse expression into a ast::Expr of the form `let ID1 = LAMBDA1; let ID2 = LAMBDA2; ...; LAST_EXPR`
///
fn exprs_into_let(exprs: Vec<Ast>) -> Result<ir::Expr, ParseError> {
    let mut exprs = exprs.into_iter().rev();

    let Some(last) = exprs.next() else {
        return Err(ParseError {
            location: Span { start: 0, end: 0 },
            kind: ParseErrorKind::Str(format!("no expression found")),
        });
    };
    let mut accumulator = expr(last)?;

    while let Some(e) = exprs.next() {
        match e {
            Ast::Define(_span, name, args, body) => {
                let body = exprs_into_let(body)?;
                let span_args = spans_merge(&mut args.iter().map(|sargs| &sargs.0.span));
                accumulator = ir::Expr::Let(
                    name,
                    Box::new(ir::Expr::Lambda(span_args, args, Box::new(body))),
                    Box::new(accumulator),
                )
            }
            _ => {
                return Err(ParseError {
                    location: Span { start: 0, end: 0 },
                    kind: ParseErrorKind::Str(format!("trying to have a non function in let")),
                });
            }
        }
    }

    Ok(accumulator)
}

fn statement(expr: Ast) -> Result<ir::Statement, ParseError> {
    match expr {
        Ast::Atom(span, ident) => Ok(ir::Statement::Expr(ir::Expr::Ident(span, ident))),
        Ast::Literal(span, lit) => Ok(ir::Statement::Expr(ir::Expr::Literal(span, literal(lit)))),
        Ast::List(span, list) => Ok(ir::Statement::Expr(exprs(span, list)?)),
        Ast::Define(span, name, args, body) => {
            let body = exprs_into_let(body)?;
            Ok(ir::Statement::Function(
                span,
                ir::FunDef {
                    name: name.unspan(),
                    vars: args,
                    body: body,
                },
            ))
        }
    }
}

fn expr(expr: Ast) -> Result<ir::Expr, ParseError> {
    match expr {
        Ast::Atom(span, ident) => Ok(ir::Expr::Ident(span, ident)),
        Ast::Literal(span, lit) => Ok(ir::Expr::Literal(span, literal(lit))),
        Ast::List(span, e) => exprs(span, e),
        Ast::Define(_, _, _, _) => Err(ParseError {
            location: Span { start: 0, end: 0 },
            kind: ParseErrorKind::Str(format!("cannot have define in expression")),
        }),
    }
}

fn exprs(span: Span, exprs: Vec<Ast>) -> Result<ir::Expr, ParseError> {
    let build_list = exprs[0].literal().is_some();
    let params = exprs
        .into_iter()
        .map(|e| expr(e))
        .collect::<Result<Vec<_>, _>>()?;

    if build_list {
        Ok(ir::Expr::List(span, params))
    } else {
        Ok(ir::Expr::Call(span, params))
    }
}

fn literal(lit: ast::Literal) -> ir::Literal {
    match lit {
        ast::Literal::Bytes(b) => ir::Literal::Bytes(b.into()),
        ast::Literal::Number(n) => ir::Literal::Number(ir::Number::from_str_radix(&n, 10).unwrap()),
        ast::Literal::String(s) => ir::Literal::String(s),
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
