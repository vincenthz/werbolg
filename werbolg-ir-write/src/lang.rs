use super::parse::*;

use proc_macro::{Delimiter, Ident, Literal, Spacing, Span, TokenStream};

type ParseError = ParserError;

pub(crate) enum Statement {
    Use(Span, Path),
    Fn(Span, bool, Ident, Vec<Ident>, Expr),
}

pub(crate) enum Expr {
    Let(Ident, Box<Expr>, Box<Expr>),
    Literal(Literal),
    Path(Path),
    Call(Vec<Expr>),
    If(Box<Expr>),
}

pub(crate) struct Path {
    pub(crate) absolute: bool,
    pub(crate) path: Vec<Ident>,
}

pub(crate) fn parse_use(p: &mut ParserTry) -> Result<Statement, ParserError> {
    let span = atom_keyword(p, "use")?;
    let path = atom_path(p)?;
    atom_semicolon(p)?;

    Ok(Statement::Use(span, path))
}

pub(crate) fn parse_fn(p: &mut ParserTry) -> Result<Statement, ParserError> {
    // parse first keyword and optional pub before
    let is_private = atom_keyword(p, "pub")
        .map(|_| false)
        .unwrap_or_else(|_| true);
    let span = atom_keyword(p, "fn")?;

    // parse name of function
    let name = p.next_ident_clone().map_err(|e| e.context("bind name"))?;

    // parse the parameters of function
    let vars_ts = atom_parens(p)?;
    let mut var_parser = Parser::from(vars_ts);
    let vars = {
        if var_parser.is_end() {
            Vec::new()
        } else {
            let mut vars = Vec::new();
            var_parser.try_parse_to_end(|parser| {
                let ident = parser
                    .next_ident_clone()
                    .map_err(|e| e.context("expecting function variable name"))?;
                vars.push(ident);
                while !parser.is_end() {
                    atom_comma(parser)?;

                    let ident = parser
                        .next_ident()
                        .map(|i| i.clone())
                        .map_err(|e| e.context("expecting function variable name"))?;
                    vars.push(ident);
                }
                Ok(())
            })?;
            vars
        }
    };

    let body_ts = atom_braces(p)?;
    let body = Parser::from(body_ts).try_parse_to_end(|parser| parse_expr(parser))?;
    Ok(Statement::Fn(span, is_private, name, vars, body))
}

fn parse_expr(parser: &mut ParserTry) -> Result<Expr, ParseError> {
    fn parse_let(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        atom_keyword(parser, "let")?;
        let ident = parser
            .next_ident()
            .map(|x| x.clone())
            .map_err(|e| e.context("bind"))?;
        atom_eq(parser)?;
        let bind_expr = parse_expr(parser).map_err(|e| e.context("bind-expr"))?;
        atom_semicolon(parser).map_err(|e| e.context("bind-expr terminator"))?;
        let then_expr = parse_expr(parser).map_err(|e| e.context("then-expr"))?;
        Ok(Expr::Let(ident, Box::new(bind_expr), Box::new(then_expr)))
    }
    fn parse_call(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        let path = parse_path(parser)?;
        let call_params = atom_parens(parser)?;
        let mut call_parser = Parser::from(call_params);
        let mut call_exprs = vec![path];
        if !call_parser.is_end() {
            call_parser.try_parse_to_end(|parser| {
                let e = parse_expr(parser)?;
                call_exprs.push(e);
                while !parser.is_end() {
                    atom_comma(parser)?;
                    let e = parse_expr(parser)?;
                    call_exprs.push(e)
                }
                Ok(())
            })?
        }

        Ok(Expr::Call(call_exprs))
    }
    fn parse_if(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        atom_keyword(parser, "if")?;
        let cond = parse_paren_expr(parser)?;
        Ok(Expr::If(Box::new(cond)))
    }
    parser
        .try_chain(&[
            ("let", parse_let),
            ("if", parse_if),
            ("call", parse_call),
            ("factor", parse_factor),
        ])
        .map_err(|e| e.context("expr"))
}

fn parse_factor(parser: &mut ParserTry) -> Result<Expr, ParseError> {
    parser.try_chain(&[
        ("literal", parse_literal),
        ("path", parse_path),
        ("paren-expr", parse_paren_expr),
    ])
}

fn parse_paren_expr(parser: &mut ParserTry) -> Result<Expr, ParseError> {
    let ts = atom_parens(parser)?;
    Parser::from(ts).try_parse_to_end(parse_expr)
}

fn parse_literal(parser: &mut ParserTry) -> Result<Expr, ParseError> {
    let lit = parser.next_literal()?;
    Ok(Expr::Literal(lit.clone()))
}

fn parse_path(parser: &mut ParserTry) -> Result<Expr, ParseError> {
    atom_path(parser).map(|path| Expr::Path(path))
}

fn atom_path(parser: &mut ParserTry) -> Result<Path, ParseError> {
    let absolute = parser
        .parse_try(|parser| atom_double_colon(parser))
        .map(|()| true)
        .unwrap_or(false);

    let first = parser.next_ident().map_err(|e| e.context("path"))?;
    let mut path = vec![first.clone()];
    loop {
        if atom_double_colon(parser).is_err() {
            break;
        }
        let rem = parser.next_ident().map_err(|e| e.context("path"))?;
        path.push(rem.clone());
    }
    Ok(Path { absolute, path })
}

fn atom_parens(parser: &mut ParserTry) -> Result<TokenStream, ParseError> {
    parser.next_group(|g| {
        if g.delimiter() == Delimiter::Parenthesis {
            Some(g.stream())
        } else {
            None
        }
    })
}

fn atom_braces(parser: &mut ParserTry) -> Result<TokenStream, ParseError> {
    parser.next_group(|g| {
        if g.delimiter() == Delimiter::Brace {
            Some(g.stream())
        } else {
            None
        }
    })
}

fn atom_comma(parser: &mut ParserTry) -> Result<(), ParseError> {
    expecting_punct(parser, ',')
}

fn atom_eq(parser: &mut ParserTry) -> Result<(), ParseError> {
    expecting_punct_spacing(parser, '=', Spacing::Alone)
}

fn atom_semicolon(parser: &mut ParserTry) -> Result<(), ParseError> {
    expecting_punct_spacing(parser, ';', Spacing::Alone)
}

fn atom_double_colon(parser: &mut ParserTry) -> Result<(), ParseError> {
    expecting_punct_spacing(parser, ':', Spacing::Joint)?;
    expecting_punct_spacing(parser, ':', Spacing::Alone)?;
    Ok(())
}

fn expecting_punct(parser: &mut ParserTry, c: char) -> Result<(), ParseError> {
    parser
        .next_punct(|p| if p.as_char() == c { Some(()) } else { None })
        .map_err(|_| ParserError::ExpectingPunct { expecting: c })
}

fn expecting_punct_spacing(
    parser: &mut ParserTry,
    c: char,
    spacing: Spacing,
) -> Result<(), ParseError> {
    parser
        .next_punct(|p| {
            if p.as_char() == c && p.spacing() == spacing {
                Some(())
            } else {
                None
            }
        })
        .map_err(|_| ParserError::ExpectingPunct { expecting: c })
}

fn atom_keyword(parser: &mut ParserTry, keyword: &str) -> Result<Span, ParseError> {
    parser.parse_try(|parser| {
        let first = parser.next_ident_clone().map_err(|e| e.context(keyword))?;
        if first.to_string() != keyword {
            return Err(ParseError::ExpectingIdent {
                expecting: keyword.to_string(),
                got: first.to_string(),
            });
        }
        Ok(first.span())
    })
}
