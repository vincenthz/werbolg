//! an unfinished lang frontend for replacing the scheme lang by a more efficient one

use super::ast::{Ast, Literal};
use super::token::{Token, UnknownToken};
use alloc::{boxed::Box, vec, vec::Vec};
use logos::Logos;
use werbolg_core::{Ident, Span, Spanned, Variable, span_merge, spans_merge};
use werbolg_lang_common::hex_decode;

pub struct Lexer<'a>(logos::Lexer<'a, Token>);

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
        let lex = Token::lexer(content);
        Lexer(lex)
    }

    pub fn slice(&self) -> &'a str {
        self.0.slice()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Spanned<Result<Token, UnknownToken>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(token) => {
                let span = self.0.span();
                Some(Spanned::new(span, token))
            }
        }
    }
}

pub struct ListCreate {
    start: Span,
    exprs: Vec<Spanned<Ast>>,
}

pub struct Parser<'a> {
    errored: bool,
    context: Vec<ListCreate>,
    lex: Lexer<'a>,
}

#[derive(Clone, Debug)]
pub enum ParseError {
    NotStartedList(Span),
    UnterminatedList(Span),
    LexingError(Span, char), //
    IfArityFailed {
        if_span: Span,
        nb_args: usize,
    },
    DefineEmptyName {
        define_span: Span,
        args_span: Span,
    },
    DefineArgumentNotIdent {
        define_span: Span,
        arg_span: Span,
    },
    DefineArityFailed {
        define_span: Span,
        nb_args: usize,
    },
    StructArgumentNotIdent {
        struct_span: Span,
        arg_span: Span,
    },
    DefineArgumentNotList {
        define_span: Span,
        args_span: Span,
    },
    AtomListNotList {
        arg_span: Span,
    },
    ArgumentNotAtom {
        args_span: Span,
        arg_invalid_span: Span,
    },
}

pub enum ParserRet {
    Continue,
    Yield(Spanned<Ast>),
}

/// drop nb elements from the start of the vector in place
fn vec_drop_start<T>(v: &mut Vec<T>, nb: usize) {
    if nb == 0 {
        return;
    } else if nb >= v.len() {
        v.truncate(0)
    } else {
        v.reverse();
        v.truncate(v.len() - nb);
        v.reverse()
    }
}

impl<'a> Parser<'a> {
    pub fn new(lex: Lexer<'a>) -> Self {
        Self {
            lex,
            errored: false,
            context: Vec::new(),
        }
    }
    fn process_list(
        &mut self,
        list_span: Span,
        exprs: Vec<Spanned<Ast>>,
    ) -> Result<Spanned<Ast>, ParseError> {
        match exprs.first() {
            None => Ok(Spanned::new(list_span, Ast::List(exprs))),
            Some(first_elem) => {
                if first_elem.atom_eq("define") {
                    parse_define(list_span.clone(), exprs).map(|a| Spanned::new(list_span, a))
                } else if first_elem.atom_eq("lambda") {
                    parse_lambda(list_span.clone(), exprs).map(|a| Spanned::new(list_span, a))
                } else if first_elem.atom_eq("struct") {
                    parse_struct(list_span.clone(), exprs).map(|a| Spanned::new(list_span, a))
                } else if first_elem.atom_eq("if") {
                    parse_if(list_span.clone(), exprs).map(|a| Spanned::new(list_span, a))
                } else {
                    Ok(Spanned::new(list_span.clone(), Ast::List(exprs)))
                }
            }
        }
    }

    fn push_list(&mut self, span: Span) -> Result<ParserRet, ParseError> {
        self.context.push(ListCreate {
            start: span,
            exprs: Vec::with_capacity(0),
        });
        Ok(ParserRet::Continue)
    }

    fn pop_list(&mut self, end_span: Span) -> Result<ParserRet, ParseError> {
        match self.context.pop() {
            None => Err(ParseError::NotStartedList(end_span)),
            Some(ListCreate { start, exprs }) => {
                let list_span = span_merge(&start, &end_span);
                let e = self.process_list(list_span, exprs)?;
                match self.context.last_mut() {
                    None => Ok(ParserRet::Yield(e)),
                    Some(ctx) => {
                        ctx.exprs.push(e);
                        Ok(ParserRet::Continue)
                    }
                }
            }
        }
    }

    fn push_literal(&mut self, span: Span, literal: Literal) -> Result<ParserRet, ParseError> {
        match self.context.last_mut() {
            None => {
                let ret = Spanned::new(span, Ast::Literal(literal));
                Ok(ParserRet::Yield(ret))
            }
            Some(ctx) => {
                ctx.exprs.push(Spanned::new(span, Ast::Literal(literal)));
                Ok(ParserRet::Continue)
            }
        }
    }

    fn push_ident(&mut self, span: Span, ident: Ident) -> Result<ParserRet, ParseError> {
        match self.context.last_mut() {
            None => {
                let ret = Spanned::new(span, Ast::Atom(ident));
                Ok(ParserRet::Yield(ret))
            }
            Some(ctx) => {
                ctx.exprs.push(Spanned::new(span, Ast::Atom(ident)));
                Ok(ParserRet::Continue)
            }
        }
    }

    fn push_token(&mut self, stok: Spanned<Token>) -> Result<ParserRet, ParseError> {
        match stok.inner {
            Token::ParenOpen => self.push_list(stok.span),
            Token::ParenClose => self.pop_list(stok.span),
            Token::Number(n) => self.push_literal(stok.span, Literal::Number(n)),
            Token::Bytes(b) => self.push_literal(stok.span, Literal::Bytes(hex_decode(&b))),
            Token::String(s) => self.push_literal(stok.span, Literal::String(s)),
            Token::Ident(a) => self.push_ident(stok.span, Ident::from(a)),
        }
    }

    fn ret_error<A>(&mut self, error: ParseError) -> Result<A, ParseError> {
        self.errored = true;
        Err(error)
    }
}

fn parse_if(list_span: Span, mut exprs: Vec<Spanned<Ast>>) -> Result<Ast, ParseError> {
    vec_drop_start(&mut exprs, 1);
    if exprs.len() != 3 {
        return Err(ParseError::IfArityFailed {
            if_span: list_span,
            nb_args: exprs.len(),
        });
    }
    let mut e = exprs.into_iter();
    let cond_expr = e.next().unwrap();
    let then_expr = e.next().unwrap();
    let else_expr = e.next().unwrap();
    Ok(Ast::If(
        Box::new(cond_expr),
        Box::new(then_expr),
        Box::new(else_expr),
    ))
}

fn parse_lambda(_list_span: Span, mut exprs: Vec<Spanned<Ast>>) -> Result<Ast, ParseError> {
    let vars = parse_atom_list(&exprs[1])?
        .into_iter()
        .map(|si| werbolg_core::Variable(si))
        .collect();

    vec_drop_start(&mut exprs, 2);
    Ok(Ast::Lambda(vars, exprs))
}

fn parse_define(list_span: Span, mut exprs: Vec<Spanned<Ast>>) -> Result<Ast, ParseError> {
    if exprs.len() != 3 {
        return Err(ParseError::DefineArityFailed {
            define_span: list_span,
            nb_args: exprs.len(),
        });
    }
    // (define (name args*) body)
    // (define name body)
    let span_name = exprs[1].span.clone();
    let (ident, args) = match &exprs[1].inner {
        Ast::List(id_args) => {
            // on empty list, raise an error
            let Some((first_expr, args_exprs)) = id_args.split_first() else {
                return Err(ParseError::DefineEmptyName {
                    define_span: list_span,
                    args_span: exprs[1].span.clone(),
                });
            };

            let span_args = spans_merge(&mut id_args.iter().map(|e| &e.span));

            let Some(ident) = first_expr.atom() else {
                return Err(ParseError::DefineArgumentNotIdent {
                    define_span: list_span,
                    arg_span: span_args.clone(),
                });
            };
            let ident = Ident(ident.0.clone());

            let args = args_exprs
                .into_iter()
                .map(|arg_expr| match arg_expr.atom() {
                    None => Err(ParseError::ArgumentNotAtom {
                        args_span: span_args.clone(),
                        arg_invalid_span: arg_expr.span.clone(),
                    }),
                    Some(sident) => Ok(Variable(Spanned::new(
                        arg_expr.span.clone(),
                        sident.clone(),
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()?;

            (ident, args)
        }
        Ast::Atom(id) => (id.clone(), vec![]),
        _ => {
            return Err(ParseError::DefineArgumentNotList {
                define_span: list_span,
                args_span: exprs[1].span.clone(),
            });
        }
    };

    // drop 'define' atom and first name or list of name+args
    vec_drop_start(&mut exprs, 2);
    Ok(Ast::Define(Spanned::new(span_name, ident), args, exprs))
}

fn parse_struct(list_span: Span, exprs: Vec<Spanned<Ast>>) -> Result<Ast, ParseError> {
    // (struct name (field+)
    let span_name = exprs[1].span.clone();
    let ident = exprs[1]
        .inner
        .atom()
        .ok_or(ParseError::StructArgumentNotIdent {
            struct_span: list_span,
            arg_span: exprs[1].span.clone(),
        })?;
    let fields = parse_atom_list(&exprs[2])?;

    Ok(Ast::Struct(Spanned::new(span_name, ident.clone()), fields))
}

fn parse_atom_list(ast: &Spanned<Ast>) -> Result<Vec<Spanned<Ident>>, ParseError> {
    let Ast::List(l) = &ast.inner else {
        return Err(ParseError::AtomListNotList {
            arg_span: ast.span.clone(),
        });
    };

    l.into_iter()
        .map(|arg_expr| match arg_expr.atom() {
            None => Err(ParseError::ArgumentNotAtom {
                args_span: ast.span.clone(),
                arg_invalid_span: arg_expr.span.clone(),
            }),
            Some(sident) => Ok(Spanned::new(arg_expr.span.clone(), sident.clone())),
        })
        .collect::<Result<Vec<_>, _>>()
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Spanned<Ast>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored {
            return None;
        }

        loop {
            let Some(next) = self.lex.next() else {
                match self.context.pop() {
                    None => return None,
                    Some(ListCreate { start, exprs: _ }) => {
                        // if still have context and there's no more token, some list are not terminated
                        return Some(self.ret_error(ParseError::UnterminatedList(start.clone())));
                    }
                }
            };

            let Spanned { span, inner } = next;
            let stok = match inner {
                Err(_) => {
                    return Some(self.ret_error(ParseError::LexingError(
                        span,
                        self.lex.slice().chars().next().unwrap(),
                    )));
                }
                Ok(n) => Spanned::new(span, n),
            };

            match self.push_token(stok) {
                Err(e) => {
                    return Some(self.ret_error(e));
                }
                Ok(ParserRet::Yield(e)) => return Some(Ok(e)),
                Ok(ParserRet::Continue) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use werbolg_core::Spanned;

    fn match_expr(e1: &Ast, e2: &Ast) -> bool {
        match (e1, e2) {
            (Ast::Atom(i1), Ast::Atom(i2)) => i1 == i2,
            (Ast::Literal(l1), Ast::Literal(l2)) => l1 == l2,
            (Ast::List(l1), Ast::List(l2)) => match_exprs(l1, l2),
            (Ast::Define(i1, a1, b1), Ast::Define(i2, a2, b2)) => {
                i1 == i2
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(a1, a2)| a1.0 == a2.0)
                    && match_exprs(b1, b2)
            }
            _ => false,
        }
    }

    fn match_exprs(e1: &Vec<Spanned<Ast>>, e2: &Vec<Spanned<Ast>>) -> bool {
        e1.len() == e2.len() && e1.iter().zip(e2.iter()).all(|(e1, e2)| match_expr(e1, e2))
    }

    #[test]
    fn it_works() {
        let snippet = r#"
        (define (add3 a b c)
            (+ (+ a b) c)
        )
        (add3 10 20 30)
        "#;
        let lex = Lexer::new(snippet);
        let mut parser = Parser::new(lex);

        // fake span factory
        let fs = || Span { start: 0, end: 0 };
        let mk_atom = |s: &str| Spanned::new(fs(), Ast::Atom(Ident::from(s)));
        let mk_num = |s: &str| Spanned::new(fs(), Ast::Literal(Literal::Number(String::from(s))));
        let mk_list = |v: Vec<Spanned<Ast>>| Spanned::new(fs(), Ast::List(v));
        let mk_sident = |s: &str| Spanned::new(fs(), Ident::from(s));
        let mk_var = |s: &str| Variable(mk_sident(s));

        match parser.next() {
            None => panic!("parser terminated early"),
            Some(e) => match e {
                Err(e) => panic!("parser error on first statement: {:?}", e),
                Ok(d) => {
                    if !match_expr(
                        &d,
                        &Ast::Define(
                            mk_sident("add3"),
                            vec![mk_var("a"), mk_var("b"), mk_var("c")],
                            vec![mk_list(vec![
                                mk_atom("+"),
                                mk_list(vec![mk_atom("+"), mk_atom("a"), mk_atom("b")]),
                                mk_atom("c"),
                            ])],
                        ),
                    ) {
                        panic!("not parsed a define")
                    }
                }
            },
        }

        match parser.next() {
            None => panic!("parser terminated early"),
            Some(e) => match e {
                Err(e) => panic!("parser error on first statement: {:?}", e),
                Ok(d) => {
                    if !match_expr(
                        &d,
                        &mk_list(vec![
                            mk_atom("add3"),
                            mk_num("10"),
                            mk_num("20"),
                            mk_num("30"),
                        ]),
                    ) {
                        panic!("not parsed a define")
                    }
                }
            },
        }

        assert!(
            parser.next().is_none(),
            "parser is unfinished when it should be finished"
        );
    }
}
