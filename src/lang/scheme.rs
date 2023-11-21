//! parse a scheme looking language
//!
//! MODULE = STATEMENT*
//!
//! STATEMENT = ( "define" DEFINE )
//!     | ( EXPR )
//!
//! DEFINE = IDENT EXPR
//!        | \( IDENT IDENT* \) BODY
//!
//! BODY = ( EXPR )
//!
//! EXPR = LITERAL
//!      | IDENT
//!      | IDENT IDENT*
//!      | [ (EXPR , )* ]
//!
//! LITERAL = STRING | INT | DECIMAL | BYTES

use super::common::{ast, FileUnit, ParseError, ParseErrorKind};
use alloc::format;
use alloc::{rc::Rc, vec::Vec};
use s_expr::{Atom, GroupKind, Position, Span, TokenizerConfig};

pub fn module(fileunit: &FileUnit) -> Result<ast::Module, ParseError> {
    let tokenizer_config = TokenizerConfig::default();
    //let end_pos = fileunit.content.as_bytes().len();
    let parser = s_expr::Parser::new_with_config(&fileunit.content, tokenizer_config);

    let elements = parse_all(parser).map_err(|_| todo!())?;
    let mut parser = Parser::new(&elements);

    let statements = statements(&mut parser)?;
    Ok(ast::Module { statements })
}

fn statements<'a, 'b>(parser: &mut Parser<'a, 'b>) -> Result<Vec<ast::Statement>, ParseError> {
    let mut statements = Vec::new();

    while !parser.finished() {
        let stmt = statement(parser)?;
        statements.push(stmt)
    }

    Ok(statements)
}

fn statement<'a, 'b>(parser: &mut Parser<'a, 'b>) -> Result<ast::Statement, ParseError> {
    if let Ok(mut parser) = parser.group(GroupKind::Paren) {
        if let Ok(()) = parser.ident_matching("define") {
            let def = define(&mut parser).map_err(|e| e.scope("define statement"))?;
            assert!(parser.finished());
            Ok(def)
        } else {
            let inner_exprs = exprs(&mut parser)?;
            Ok(ast::Statement::Expr(ast::Expr::Call(inner_exprs)))
        }
    } else {
        todo!("cannot parse atom as a statement")
    }
}

fn define<'a, 'b>(parser: &mut Parser<'a, 'b>) -> Result<ast::Statement, ParseError> {
    let ident = parser.ident()?;
    let mut params_parser = parser.group(GroupKind::Paren)?;
    let mut params = Vec::new();
    while !params_parser.finished() {
        let ident = params_parser
            .ident()
            .map_err(|e| e.scope("define params"))?;
        params.push(ident)
    }
    let body = statements(parser)?;
    Ok(ast::Statement::Function(ident, params, body))
}

fn exprs<'a, 'b>(parser: &mut Parser<'a, 'b>) -> Result<Vec<ast::Expr>, ParseError> {
    let mut inner_exprs = Vec::new();
    while !parser.finished() {
        let e = expr(parser)?;
        inner_exprs.push(e)
    }
    Ok(inner_exprs)
}

/// parse an expression
///
/// either:
/// * an atom: turns into a expr literal, or expr ident
/// * a group: turns into either a expr call or a expr list
fn expr<'a, 'b>(parser: &mut Parser<'a, 'b>) -> Result<ast::Expr, ParseError> {
    let el = parser.next()?;
    match &el.inner {
        Element::Atom(atom) => match atom {
            Atom::Integral(i) => Ok(ast::Expr::Literal(ast::Literal::Number(transform_num(i)))),
            Atom::Decimal(d) => Ok(ast::Expr::Literal(ast::Literal::Decimal(transform_dec(d)))),
            Atom::Bytes(b) => Ok(ast::Expr::Literal(ast::Literal::Bytes(
                hex::decode(b.0).unwrap().into(),
            ))),
            Atom::String(s) => Ok(ast::Expr::Literal(ast::Literal::String(s.to_string()))),
            Atom::Ident(ident) => Ok(ast::Expr::Ident(ast::Ident::from(*ident))),
        },
        Element::Group(kind, element) => match kind {
            GroupKind::Paren => {
                let mut p = Parser::new(&element);
                let inner_exprs = exprs(&mut p)?;
                if inner_exprs.len() == 1 {
                    Ok(inner_exprs.into_iter().next().unwrap())
                } else {
                    Ok(ast::Expr::Call(inner_exprs))
                }
            }
            GroupKind::Bracket => {
                let mut p = Parser::new(&element);
                let mut inner_exprs = Vec::new();
                while !p.finished() {
                    let e = expr(&mut p)?;
                    // TODO parse some separator like ,
                    inner_exprs.push(e)
                }
                Ok(ast::Expr::List(inner_exprs))
            }
            GroupKind::Brace => todo!("expr brace group"),
        },
    }
}

pub enum Element<'a> {
    Atom(s_expr::Atom<'a>),
    Group(s_expr::GroupKind, Vec<SElement<'a>>),
}

pub struct SElement<'a> {
    span: s_expr::Span,
    inner: Element<'a>,
}

#[derive(Copy, Clone, Debug)]
pub enum AtLevel {
    Root,
    Span(Span),
}

fn parse_all<'a>(mut parser: s_expr::Parser<'a>) -> Result<Vec<SElement<'a>>, s_expr::ParserError> {
    let mut elements = Vec::new();
    loop {
        match parser.next() {
            Err(e) => return Err(e),
            Ok(None) => break,
            Ok(Some(spanned_elem)) => match remap(spanned_elem) {
                None => continue,
                Some(el) => elements.push(el),
            },
        }
    }
    Ok(elements)
}

#[derive(Clone)]
pub struct Parser<'a, 'b> {
    elements: Rc<&'b [SElement<'a>]>,
    pos: usize,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(elements: &'b [SElement<'a>]) -> Parser<'a, 'b> {
        Parser {
            elements: Rc::new(elements),
            pos: 0,
        }
    }

    pub fn finished(&self) -> bool {
        self.elements.len() == self.pos
    }

    fn assert_noneof(&self) -> Result<(), ParseError> {
        if self.pos == self.elements.len() {
            return Err(ParseError {
                location: Span {
                    start: Position { line: 0, col: 0 },
                    end: Position { line: 0, col: 0 },
                },
                kind: ParseErrorKind::Unknown,
            });
        }
        Ok(())
    }

    pub fn peek<'c>(&mut self) -> Result<&SElement<'a>, ParseError> {
        self.assert_noneof()?;
        let current: &SElement<'a> = &self.elements[self.pos];
        Ok(current)
    }

    pub fn next<'c>(&mut self) -> Result<&SElement<'a>, ParseError> {
        self.assert_noneof()?;
        let current: &SElement<'a> = &self.elements[self.pos];
        self.pos += 1;
        Ok(current)
    }

    pub fn group<'c>(&mut self, expected_kind: GroupKind) -> Result<Parser<'a, 'c>, ParseError>
    where
        'b: 'c,
    {
        self.assert_noneof()?;
        let current: &SElement<'a> = &self.elements[self.pos];
        match &current.inner {
            Element::Group(kind, inner) => {
                if *kind != expected_kind {
                    return Err(ParseError {
                        location: current.span,
                        kind: ParseErrorKind::Str(format!("expecting kind paren: {:?}", *kind)),
                    });
                }

                self.pos += 1;
                let r = Parser::new(&inner);
                Ok(r)
            }
            Element::Atom(_atom) => {
                return Err(ParseError {
                    location: current.span,
                    kind: ParseErrorKind::Str(format!("expecting group but got atom {:?}", _atom)),
                });
            }
        }
    }

    pub fn literal(&mut self) -> Result<ast::Literal, ParseError> {
        self.assert_noneof()?;
        let current: &SElement<'a> = &self.elements[self.pos];
        match &current.inner {
            Element::Group(_kind, _inner) => {
                return Err(ParseError {
                    location: current.span,
                    kind: ParseErrorKind::Str(format!("expecting atom but got group {:?}", _kind)),
                });
            }
            Element::Atom(atom) => {
                let lit = match atom {
                    Atom::Integral(i) => ast::Literal::Number(transform_num(i)),
                    Atom::Decimal(d) => ast::Literal::Decimal(transform_dec(d)),
                    Atom::Bytes(b) => ast::Literal::Bytes(hex::decode(b.0).unwrap().into()),
                    Atom::String(s) => ast::Literal::String(s.to_string()),
                    Atom::Ident(_ident) => {
                        return Err(ParseError {
                            location: current.span,
                            kind: ParseErrorKind::Unknown,
                        });
                    }
                };
                self.pos += 1;
                Ok(lit)
            }
        }
    }

    pub fn ident(&mut self) -> Result<ast::Ident, ParseError> {
        let current: &SElement<'a> = &self.elements[self.pos];
        match &current.inner {
            Element::Group(_kind, _inner) => {
                return Err(ParseError {
                    location: current.span,
                    kind: ParseErrorKind::Str(format!("expecting ident but got group")),
                });
            }
            Element::Atom(atom) => match atom {
                Atom::Integral(_) | Atom::Decimal(_) | Atom::Bytes(_) | Atom::String(_) => {
                    return Err(ParseError {
                        location: current.span,
                        kind: ParseErrorKind::Str(format!("expecting ident but got literal atom")),
                    });
                }
                Atom::Ident(ident) => {
                    self.pos += 1;
                    return Ok(ast::Ident::from(*ident));
                }
            },
        }
    }

    pub fn ident_matching(&mut self, expected_ident: &str) -> Result<(), ParseError> {
        let current: &SElement<'a> = &self.elements[self.pos];
        match &current.inner {
            Element::Group(_kind, _inner) => {
                return Err(ParseError {
                    location: current.span,
                    kind: ParseErrorKind::Unknown,
                });
            }
            Element::Atom(atom) => match atom {
                Atom::Integral(_) | Atom::Decimal(_) | Atom::Bytes(_) | Atom::String(_) => {
                    return Err(ParseError {
                        location: current.span,
                        kind: ParseErrorKind::Unknown,
                    });
                }
                Atom::Ident(ident) => {
                    if *ident != expected_ident {
                        return Err(ParseError {
                            location: current.span,
                            kind: ParseErrorKind::Unknown,
                        });
                    }
                    self.pos += 1;
                    return Ok(());
                }
            },
        }
    }
}

fn remap<'a>(el: s_expr::SpannedElement<'a>) -> Option<SElement<'a>> {
    match el.inner {
        s_expr::Element::Group(grp_kind, grp_elements) => Some(SElement {
            span: el.span,
            inner: Element::Group(
                grp_kind,
                grp_elements.into_iter().flat_map(remap).collect::<Vec<_>>(),
            ),
        }),
        s_expr::Element::Atom(atom) => Some(SElement {
            span: el.span,
            inner: Element::Atom(atom),
        }),
        s_expr::Element::Comment(_) => None,
    }
}

fn transform_num(i: &s_expr::ANum) -> ast::Number {
    // cannot error since it's parsed correctly by the s_expr parser
    ast::Number::from_str_radix(&i.digits(), i.radix()).unwrap()
}

fn transform_dec(dec: &s_expr::ADecimal) -> ast::Decimal {
    // cannot error since it's parsed correctly by the s_expr parser
    let s = format!("{}.{}", dec.integral(), dec.fractional());
    let dec = ast::Decimal::from_str(&s).expect("s_expr parsing decimal correctly");
    dec
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn snippet_to_fileunit(snippet: &str) -> FileUnit {
        FileUnit::from_string(String::from("test"), String::from(snippet))
    }

    #[test]
    fn it_works() {
        let snippet = r#"
        (define add3 (a b c)
            (+ (+ a b) c)
        )
        (add3 10 20 30)
        "#;
        let fileunit = &snippet_to_fileunit(snippet);
        let res = module(&fileunit);
        match res {
            Err(e) => {
                panic!("parsing failed: {:?}\n{}", e, fileunit.resolve_error(&e))
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
