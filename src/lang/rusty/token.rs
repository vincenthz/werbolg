use logos::Logos;

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//.*")]
pub enum Token {
    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Bool(bool),
    #[token("{")]
    BraceOpen,
    #[token("}")]
    BraceClose,
    #[token("[")]
    BracketOpen,
    #[token("]")]
    BracketClose,
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("fn")]
    Fn,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("=")]
    Eq,
    #[regex(r"-?(?:0|[1-9]\d*)?", |lex| lex.slice().parse::<u64>().unwrap())]
    Number(u64),
    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice().to_owned())]
    String(String),
    #[regex(r#"[_a-zA-Z][a-zA-Z0-9]*"#, |lex| lex.slice().to_owned())]
    Ident(String),
    #[regex(r#"[-!@#$%^&*+=|<>?.]+"#, |lex| lex.slice().to_owned())]
    Operator(String),
}
