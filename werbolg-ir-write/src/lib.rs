extern crate alloc;
extern crate proc_macro;

//#[macro_use]
//mod gen;
mod parse;

use alloc::string::String;
use macro_quote_types::ToTokenTrees;
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

//use gen::ExtendsStream; //, Generator, Path};
use parse::{Parser, ParserTry};

type ParseError = String;

use macro_quote::quote;

enum Statement {
    Use(Span, u32),
    Fn(Span, bool, Ident, Vec<String>, Expr),
}

enum Expr {
    Let(Ident),
    Literal(Literal),
    Path(bool, Vec<Ident>),
    Call(Vec<Expr>),
    If(Box<Expr>),
}

#[proc_macro]
pub fn module(item: TokenStream) -> TokenStream {
    let mut parser: Parser = item.into();
    let mut statements = Vec::new();

    while !parser.is_end() {
        let parser_chain = [parse_use, parse_fn];

        match parser.try_chain(&parser_chain) {
            (Ok(stmt), p) => {
                parser = p;
                let g = generate_statement(stmt);
                statements.push(g);
            }
            (Err(errs), p) => {
                break;
            } //panic!("No parser worked:\n{:?}", errs),
        }
    }

    let inx = vec_macro(statements);
    quote! {
        werbolg_core::ir::Module { statements : #inx }
    }
}

fn vec_macro<X: ToTokenTrees>(inner: Vec<X>) -> TokenStream {
    quote! {
        //::alloc::vec::Vec::from(::alloc::boxed::Box::new(&[ #(#inner),* ]))
        (::alloc::boxed::Box::new([ #(#inner),* ]) as ::alloc::boxed::Box<[_]>).into_vec()
    }
}

fn parse_use(p: &mut ParserTry) -> Result<Statement, ParseError> {
    let i: &Ident = p
        .next_ident()
        .map_err(|e| format!("use: initial keyword {:?}", e))?;
    if i.to_string() != "use" {
        return Err(format!("keyword not matching"));
    }

    //todo!()
    Err(format!("use not implemented"))
}

fn parse_fn(p: &mut ParserTry) -> Result<Statement, ParseError> {
    // parse first keyword and optional pub before
    let i: &Ident = p
        .next_ident()
        .map_err(|e| format!("use: initial keyword {:?}", e))?;
    let span = i.span();
    let is_private = if i.to_string() == "pub" {
        let i2: &Ident = p
            .next_ident()
            .map_err(|e| format!("after pub ident {:?}", e))?;
        if i2.to_string() != "fn" {
            return Err(format!(
                "keyword 'fn' not matching after pub, got {}",
                i2.to_string()
            ));
        }
        false
    } else if i.to_string() == "fn" {
        true
    } else {
        return Err(format!(
            "keyword 'fn' or 'pub fn' not matching, got {}",
            i.to_string()
        ));
    };

    // parse name of function
    let name = p
        .next_ident()
        .map(|x| x.clone())
        .map_err(|e| format!("expecting name after 'fn' got error {:?}", e))?;

    // parse the parameters of function
    let vars_ts = p
        .next_group(|grp| {
            if grp.delimiter() == Delimiter::Parenthesis {
                Some(grp.stream())
            } else {
                None
            }
        })
        .map_err(|e| format!("expecting parens but got {:?}", e))?;

    let body_ts = p
        .next_group(|grp| {
            if grp.delimiter() == Delimiter::Brace {
                Some(grp.stream())
            } else {
                None
            }
        })
        .map_err(|e| format!("expecting brace but got {:?}", e))?;

    let body = parse_expr(Parser::from(body_ts))?;

    Ok(Statement::Fn(span, is_private, name, vec![], body))
}

/*
fn parse_exprs(parser: Parser) -> Result<Vec<Expr>, ParseError> {
    let expr_parsers = parser.sep_by();
    expr_parsers
        .into_iter()
        .map(|p| parse_expr(p))
        .collect::<Result<Vec<_>, _>>()
}
*/

fn parse_expr(parser: Parser) -> Result<Expr, ParseError> {
    fn parse_literal(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        let lit = parser
            .next_literal()
            .map_err(|e| format!("not literal {:?}", e))?;
        Ok(Expr::Literal(lit.clone()))
    }
    fn parse_path(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        let absolute = parser
            .parse_try(|parser| atom_double_colon(parser))
            .map(|()| true)
            .unwrap_or(false);

        let first = parser
            .next_ident()
            .map_err(|e| format!("not an ident {:?}", e))?;
        let mut path = vec![first.clone()];
        loop {
            if atom_double_colon(parser).is_err() {
                break;
            }
            let rem = parser
                .next_ident()
                .map_err(|e| format!("not an ident {:?}", e))?;
            path.push(rem.clone());
        }
        Ok(Expr::Path(absolute, path))
    }
    fn parse_let(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        atom_keyword(parser, "let")?;
        let ident = parser
            .next_ident()
            .map_err(|e| format!("not let ident {:?}", e))?;
        Ok(Expr::Let(ident.clone()))
    }
    fn parse_call(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        let path = parse_path(parser)?;
        let call_params = atom_parens(parser)?;
        Ok(Expr::Call(vec![path]))
    }
    fn parse_if(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        atom_keyword(parser, "if")?;
        let cond = parse_rec_expr(parser)?;
        Ok(Expr::If(Box::new(cond)))
    }
    fn parse_rec_expr(parser: &mut ParserTry) -> Result<Expr, ParseError> {
        let ts = atom_parens(parser)?;
        let e = parse_expr(Parser::from(ts))?;
        Ok(e)
    }
    let (r, mut parser) = parser.try_chain(&[
        parse_let,
        parse_if,
        parse_literal,
        parse_call,
        parse_path,
        parse_rec_expr,
    ]);
    match r {
        Ok(e) => {
            if parser.is_end() {
                Ok(e)
            } else {
                Err(format!("expression tree is not finished"))
            }
        }
        Err(e) => Err(format!("expression parser failed {:?}", e)),
    }
}

fn atom_parens(parser: &mut ParserTry) -> Result<TokenStream, ParseError> {
    parser
        .next_group(|g| {
            if g.delimiter() == Delimiter::Parenthesis {
                Some(g.stream())
            } else {
                None
            }
        })
        .map_err(|e| format!("parens {:?}", e))
}

fn atom_double_colon(parser: &mut ParserTry) -> Result<(), ParseError> {
    parser
        .next_punct(|p| {
            if p.as_char() == ':' && p.spacing() == proc_macro::Spacing::Joint {
                Some(())
            } else {
                None
            }
        })
        .map_err(|e| format!("path sep : not first colon"))?;
    parser
        .next_punct(|p| {
            if p.as_char() == ':' && p.spacing() == proc_macro::Spacing::Alone {
                Some(())
            } else {
                None
            }
        })
        .map_err(|e| format!("path sep : not second colon"))?;
    Ok(())
}
fn atom_keyword(parser: &mut ParserTry, keyword: &str) -> Result<(), ParseError> {
    let first = parser
        .next_ident()
        .map(|v| v.to_string())
        .map_err(|e| format!("no let keyword {:?}", e))?;
    if first == keyword {
        return Err(format!("no {} keyword: found {} instead", keyword, first));
    }
    Ok(())
}

/*
fn span_to_werbolg(_span: &Span) -> TokenStream {
    quote! {
        ::proc_macro::Span::call_site()
    }
}
*/

fn werbolg_span() -> TokenStream {
    quote! {
        core::ops::Range { start: 0, end: 0 }
    }
}

fn werbolg_ident(s: &str) -> TokenStream {
    quote! {
        werbolg_core::Ident::from(#s)
    }
}

fn werbolg_ident_from_ident(s: &Ident) -> TokenStream {
    let x = s.to_string();
    quote! {
        werbolg_core::Ident::from(#x)
    }
}

fn generate_statement(statement: Statement) -> TokenStream {
    match statement {
        Statement::Use(_, _) => todo!(),
        Statement::Fn(span, is_private, name, vars, body) => {
            let span = werbolg_span();
            let v = vec_macro(vars);
            let b = generate_expr(body);
            let name = werbolg_ident_from_ident(&name);
            quote! {
                werbolg_core::ir::Statement::Function(#span, werbolg_core::ir::FunDef {
                    privacy: werbolg_core::ir::Privacy::Public,
                    name: Some(#name),
                    vars: #v,
                    body: #b,
                })
            }
        }
    }
}

fn generate_expr(expr: Expr) -> TokenStream {
    match expr {
        Expr::Let(ident) => {
            let ident = werbolg_ident(&ident.to_string());
            quote! { werbolg_core::Expr::Let(werbolg_core::Binder::Ident(#ident)) }
        }
        Expr::Literal(lit) => quote! { #lit },
        Expr::Path(absolute, fragments) => {
            let fr = vec_macro(
                fragments
                    .into_iter()
                    .map(|i| {
                        let x = i.to_string();
                        quote! { werbolg_core::Ident::from(#x) }
                    })
                    .collect::<Vec<_>>(),
            );
            let span = werbolg_span();
            if absolute {
                quote! { werbolg_core::Expr::Path(#span, werbolg_core::Path::new_raw(werbolg_core::PathType::Absolute, #fr)) }
            } else {
                quote! { werbolg_core::Expr::Path(#span, werbolg_core::Path::new_raw(werbolg_core::PathType::Relative, #fr)) }
            }
        }
        Expr::Call(_) => quote! { werbolg_core::Expr::Call() },
        Expr::If(_) => quote! { werbolg_core::Expr::If { span, cond, then_expr, else_expr } },
    }
}
