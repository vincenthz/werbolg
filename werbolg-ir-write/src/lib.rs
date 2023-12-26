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
    Fn(Span, bool, String, Vec<String>, Expr),
}

enum Expr {
    Let,
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
        ::alloc::vec::Vec::from(::alloc::boxed::Box::new(&[ #(#inner),* ]))
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
        .map(|x| x.to_string())
        .map_err(|e| format!("expecting name after 'fn' got error {:?}", e))?;

    let vars = p
        .next_group(|grp| {
            if grp.delimiter() == Delimiter::Parenthesis {
                Some(grp.stream())
            } else {
                None
            }
        })
        .map_err(|e| format!("expecting parens but got {:?}", e))?;
    Ok(Statement::Fn(span, is_private, name, vec![], Expr::Let))
}

fn span_to_werbolg(span: &Span) -> TokenStream {
    TokenStream::new()
    //todo!()
    /*
    quote! {
        core::ops::Range { start: 0, end: 0 }
    }
    */
}

fn generate_statement(statement: Statement) -> TokenStream {
    match statement {
        Statement::Use(_, _) => todo!(),
        Statement::Fn(span, is_private, name, vars, body) => {
            let span = span_to_werbolg(&span);
            let v = quote! {
                ::alloc::vec::Vec::from(::alloc::boxed::Box::new(&[ #(#vars),* ]))
            };
            let b = quote! {
                123
            };
            quote! {
                werbolg_core::ir::Statement::Function(#span, werbolg_core::ir::FunDef {
                    privacy: werbolg_core::ir::Privacy::Public,
                    name: Some(#name),
                    var: #v,
                    body: [],
                })
            }
        }
    }
}
