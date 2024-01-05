#![no_std]

extern crate alloc;

mod ast;
mod parse;
mod token;

use werbolg_lang_common::{FileUnit, ParseError, ParseErrorKind};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use ast::Ast;
use werbolg_core::{self as ir, spans_merge, Span, Spanned};

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
        parse::ParseError::LexingError(span, ch) => ParseError {
            location: span,
            kind: ParseErrorKind::Str(format!("unknown character {}", ch)),
        },
        parse::ParseError::DefineEmptyName {
            define_span,
            args_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define with empty name")),
        },
        parse::ParseError::IfArityFailed { if_span, nb_args } => ParseError {
            location: if_span,
            kind: ParseErrorKind::Str(format!("if required 3 parameters, but {} given", nb_args)),
        },
        parse::ParseError::DefineArgumentNotIdent {
            define_span,
            arg_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define argument not ident")),
        },
        parse::ParseError::DefineArgumentNotList {
            define_span,
            args_span: _,
        } => ParseError {
            location: define_span,
            kind: ParseErrorKind::Str(String::from("define argument not a list")),
        },
        parse::ParseError::AtomListNotList { arg_span } => ParseError {
            location: arg_span,
            kind: ParseErrorKind::Str(String::from("atom list is not a list")),
        },
        parse::ParseError::ArgumentNotAtom {
            args_span,
            arg_invalid_span: _,
        } => ParseError {
            location: args_span,
            kind: ParseErrorKind::Str(String::from("argument not an atom in list")),
        },
        parse::ParseError::StructArgumentNotIdent {
            struct_span,
            arg_span: _,
        } => ParseError {
            location: struct_span,
            kind: ParseErrorKind::Str(String::from("struct argument not ident")),
        },
    }
}

/// Turn a vector of lisp parse expression into a ast::Expr of the form `let ID1 = LAMBDA1; let ID2 = LAMBDA2; ...; LAST_EXPR`
///
fn exprs_into_let(exprs: Vec<Spanned<Ast>>) -> Result<ir::Expr, ParseError> {
    // parse expressions in reverse order
    let mut exprs = exprs.into_iter().rev();

    let Some(last) = exprs.next() else {
        return Err(ParseError {
            location: Span { start: 0, end: 0 },
            kind: ParseErrorKind::Str(format!("no expression found")),
        });
    };
    let mut accumulator = expr(last)?;

    while let Some(e) = exprs.next() {
        match e.inner {
            Ast::Define(name, args, body) => {
                let body = exprs_into_let(body)?;
                let span_args = spans_merge(&mut args.iter().map(|sargs| &sargs.0.span));
                accumulator = ir::Expr::Let(
                    ir::Binder::Ident(name.clone().unspan()),
                    Box::new(ir::Expr::Lambda(
                        span_args,
                        Box::new(ir::FunImpl {
                            vars: args,
                            body: body,
                        }),
                    )),
                    Box::new(accumulator),
                )
            }
            x => {
                return Err(ParseError {
                    location: e.span,
                    kind: ParseErrorKind::Str(format!(
                        "trying to have a non function in let: {:?}",
                        x
                    )),
                });
            }
        }
    }

    Ok(accumulator)
}

fn statement(ast: Spanned<Ast>) -> Result<ir::Statement, ParseError> {
    match ast.inner {
        Ast::Atom(ident) => Ok(ir::Statement::Expr(ir::Expr::Path(
            ast.span,
            ir::Path::relative(ident),
        ))),
        Ast::Literal(lit) => Ok(ir::Statement::Expr(ir::Expr::Literal(
            ast.span,
            literal(lit),
        ))),
        Ast::List(list) => Ok(ir::Statement::Expr(exprs(ast.span, list)?)),
        Ast::If(cond_expr, then_expr, else_expr) => Ok(ir::Statement::Expr(ir::Expr::If {
            span: ast.span,
            cond: Box::new(spanned_expr(cond_expr.as_ref().clone())?),
            then_expr: Box::new(spanned_expr(then_expr.as_ref().clone())?),
            else_expr: Box::new(spanned_expr(else_expr.as_ref().clone())?),
        })),
        Ast::Lambda(args, body) => {
            let body = exprs_into_let(body)?;
            Ok(ir::Statement::Expr(ir::Expr::Lambda(
                ast.span,
                Box::new(ir::FunImpl {
                    vars: args,
                    body: body,
                }),
            )))
        }
        Ast::Define(name, args, body) => {
            let body = exprs_into_let(body)?;
            Ok(ir::Statement::Function(
                ast.span,
                ir::FunDef {
                    privacy: ir::Privacy::Public,
                    name: name.unspan(),
                },
                ir::FunImpl {
                    vars: args,
                    body: body,
                },
            ))
        }
        Ast::Struct(name, fields) => Ok(ir::Statement::Struct(
            ast.span,
            ir::StructDef { name, fields },
        )),
    }
}

fn spanned_expr(ast: Spanned<Ast>) -> Result<Spanned<ir::Expr>, ParseError> {
    let span = ast.span.clone();
    expr(ast).map(|a| Spanned::new(span, a))
}

fn expr(ast: Spanned<Ast>) -> Result<ir::Expr, ParseError> {
    match ast.inner {
        Ast::Atom(ident) => Ok(ir::Expr::Path(ast.span, ir::Path::relative(ident))),
        Ast::Literal(lit) => Ok(ir::Expr::Literal(ast.span, literal(lit))),
        Ast::Lambda(vars, body) => {
            let body = exprs_into_let(body)?;
            Ok(ir::Expr::Lambda(
                ast.span,
                Box::new(ir::FunImpl { vars: vars, body }),
            ))
        }
        Ast::List(e) => exprs(ast.span, e),
        Ast::If(cond_expr, then_expr, else_expr) => Ok(ir::Expr::If {
            span: ast.span,
            cond: Box::new(spanned_expr(cond_expr.as_ref().clone())?),
            then_expr: Box::new(spanned_expr(then_expr.as_ref().clone())?),
            else_expr: Box::new(spanned_expr(else_expr.as_ref().clone())?),
        }),
        Ast::Define(_, _, _) => Err(ParseError {
            location: Span { start: 0, end: 0 },
            kind: ParseErrorKind::Str(format!("cannot have define in expression")),
        }),
        Ast::Struct(_, _) => Err(ParseError {
            location: Span { start: 0, end: 0 },
            kind: ParseErrorKind::Str(format!("cannot have struct in expression")),
        }),
    }
}

fn exprs(span: Span, exprs: Vec<Spanned<Ast>>) -> Result<ir::Expr, ParseError> {
    let build_list = exprs.is_empty() || exprs[0].literal().is_some();
    let params = exprs
        .into_iter()
        .map(|e| expr(e))
        .collect::<Result<Vec<_>, _>>()?;

    if build_list {
        Ok(ir::Expr::List(span, params))
    } else {
        let params = params
            .into_iter()
            .filter(|e| match e {
                ir::Expr::List(_, e) if e.is_empty() => false,
                _ => true,
            })
            .collect();
        Ok(ir::Expr::Call(span, params))
    }
}

fn literal(lit: ast::Literal) -> ir::Literal {
    match lit {
        ast::Literal::Bytes(b) => ir::Literal::Bytes(b.into_boxed_slice()),
        ast::Literal::Number(n) => ir::Literal::Number(n.into_boxed_str()),
        ast::Literal::String(s) => ir::Literal::String(s.into_boxed_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let snippet = r#"
        (define (add3 a b c)
            (+ (+ a b) c)
        )
        (define main (add3 10 20 30))
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
