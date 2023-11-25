use alloc::{borrow::ToOwned, string::String};
use logos::Logos;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnknownToken;

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r";.*")]
#[logos(error = UnknownToken)]
pub enum Token {
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[regex(r"-?(?:0|[1-9]\d*)?", |lex| lex.slice().to_owned())]
    Number(String),
    #[regex(r#"#[a-fA-F0-9]{2}*#"#, |lex| lex.slice().to_owned())]
    Bytes(String),
    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice().to_owned())]
    String(String),
    #[regex(r#"[-_a-zA-Z!@#$%^&*+/][-_a-zA-Z0-9!@#$%^&*+/]*"#, |lex| lex.slice().to_owned())]
    Ident(String),
}
