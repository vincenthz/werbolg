#[derive(Clone, Debug)]
pub struct Module {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident(pub String);

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
    Function(Ident, Vec<Ident>, Vec<Statement>),
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Literal),
    List(Vec<Expr>),
    Let(Ident, Box<Expr>, Box<Expr>),
    Then(Box<Expr>, Box<Expr>),
    Ident(Ident),
    Call(Vec<Expr>),
    If {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub enum Literal {
    String(String),
    Number(Number),
    Decimal(Decimal),
    Bytes(Box<[u8]>),
}

pub type Number = num_bigint::BigInt;
pub type Decimal = bigdecimal::BigDecimal;
