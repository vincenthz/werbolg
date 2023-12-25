extern crate alloc;
extern crate proc_macro;

//#[macro_use]
//mod gen;
mod parse;

use alloc::string::String;
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

//use gen::ExtendsStream; //, Generator, Path};
use parse::{Parser, ParserTry};

type ParseError = String;

use macro_quote::quote;

enum Statement {
    Use(Span, u32),
    Fn(Span, String, Vec<String>),
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
            (Err(errs), _) => panic!("No parser worked:\n{:?}", errs),
        }
    }

    let inx = vec_macro(statements);
    quote! {
        werbolg_core::ir::Module { statements : #inx }
    }
}

fn vec_macro(inner: Vec<TokenStream>) -> TokenStream {
    quote! {
        ::alloc::slice::into_vec(::alloc::boxed::Box::new(&[ #(#inner),* ]))
    }
}

fn parse_use(p: &mut ParserTry) -> Result<Statement, ParseError> {
    let i: &Ident = p
        .next_ident()
        .map_err(|e| format!("use: initial keyword {:?}", e))?;
    if i.to_string() != "use" {
        return Err(format!("keyword not matching"));
    }

    todo!()
}

fn parse_fn(p: &mut ParserTry) -> Result<Statement, ParseError> {
    todo!()
}

fn span_to_werbolg(span: &Span) -> TokenStream {
    todo!()
    /*
    quote! {
        core::ops::Range { start: 0, end: 0 }
    }
    */
}

fn generate_statement(statement: Statement) -> TokenStream {
    match statement {
        Statement::Use(_, _) => todo!(),
        Statement::Fn(span, name, vars) => {
            let span = span_to_werbolg(&span);
            let v = quote! {
                ::alloc::slice::into_vec(::alloc::boxed::Box::new(&[ #(#vars),* ]))
            };
            quote! {
                werbolg_core::ir::Statement::Function(#span, werbolg_core::ir::FunDef {
                    private: werbolg_core::ir::Privacy::Public,
                    name: Some(#name),
                    var: #v,
                    body: ,
                })
            }
        }
    }
}
