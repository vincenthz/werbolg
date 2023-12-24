extern crate alloc;
extern crate proc_macro;

mod gen;
mod parse;

use alloc::string::String;
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

use gen::Generator;
use parse::{Parser, ParserTry};

struct Transformer {
    parser: Parser,
    generator: Generator,
}

impl Transformer {
    pub fn new(parser: Parser, generator: Generator) -> Self {
        Self { parser, generator }
    }
}

type ParseError = String;

enum Statement {
    Use(u32),
}

#[proc_macro]
pub fn module(item: TokenStream) -> TokenStream {
    let mut transformer = Transformer::new(item.into(), Generator::new());

    while !transformer.parser.is_end() {
        let parser_chain = [parse_use, parse_fn];

        match transformer.parser.try_chain(&parser_chain) {
            (Ok(stmt), p) => {
                transformer.parser = p;
                generate_statement(&mut transformer.generator, stmt);
            }
            (Err(errs), _) => panic!("No parser worked:\n{:?}", errs),
        }
    }

    /*
    let mut ts = TokenStream::new();
    let group = {
        let mut ts = TokenStream::new();
        ts.extend(vec![TokenTree::from(Ident::new("a", Span::call_site()))]);
        let group = Group::new(Delimiter::Parenthesis, ts);
        group
    };
    ts.extend(vec![TokenTree::from(group)]);
    ts
    */
    transformer.generator.finalize()
}

fn parse_use(p: &mut ParserTry) -> Result<Statement, ParseError> {
    let i: &Ident = p
        .next_ident()
        .map_err(|e| format!("use: initial keyword {:?}", e))?;
    if i.to_string() != "use" {
        return Err(format!("keyword not matching"));
    }

    //parse_namespace(p);
    //let namespace = p.next_ident()?

    //Ok()
    todo!()
}

fn parse_fn(p: &mut ParserTry) -> Result<Statement, ParseError> {
    todo!()
}

fn generate_statement(generator: &mut Generator, statement: Statement) {
    //
}
