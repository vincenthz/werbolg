use alloc::vec::Vec;
use proc_macro::{
    token_stream::{self, IntoIter},
    Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree,
};

/// A Parser for TokenTree, with an arbitrary sized lookahead
pub(crate) struct Parser {
    /// First seen span of this parser
    first_span: Option<Span>,
    /// Last seen span of this parser
    last_span: Option<Span>,
    /// Lookahead buffer of dynamic size
    la: Vec<TokenTree>,
    /// Rest of the stream
    ts: IntoIter,
}

impl From<TokenStream> for Parser {
    fn from(ts: TokenStream) -> Parser {
        Parser {
            first_span: None,
            last_span: None,
            la: Vec::new(),
            ts: ts.into_iter(),
        }
    }
}

impl From<Vec<TokenTree>> for Parser {
    fn from(ts: Vec<TokenTree>) -> Parser {
        Parser::from(TokenStream::from_iter(ts))
    }
}

#[derive(Debug, Clone)]
pub enum TokenKind {
    Literal,
    Ident,
    Punct,
    Group,
}

impl From<&TokenTree> for TokenKind {
    fn from(tt: &TokenTree) -> Self {
        match tt {
            TokenTree::Group(_) => TokenKind::Group,
            TokenTree::Ident(_) => TokenKind::Ident,
            TokenTree::Punct(_) => TokenKind::Punct,
            TokenTree::Literal(_) => TokenKind::Literal,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParserError {
    EndOfStream {
        expecting: Option<TokenKind>,
    },
    Expecting {
        expecting: TokenKind,
        got: TokenKind,
    },
    NotMatches,
}

impl Parser {
    /// Check if the parser has reach end
    ///
    /// note is some case, it pull the next element from the stream
    /// and put it in the parser lookahead
    pub fn is_end(&mut self) -> bool {
        if self.la.is_empty() {
            self.peek_at(0).is_none()
        } else {
            false
        }
    }

    /// Return the index'th element in the stream without consuming anything
    pub fn peek_at(&mut self, index: usize) -> Option<&TokenTree> {
        while self.la.len() <= index {
            if let Some(token) = self.ts.next() {
                self.la.push(token);
            } else {
                return None;
            }
        }
        Some(&self.la[index])
    }

    /// Consume N elements from the token stream
    pub fn consume(&mut self, nb_elements: usize) {
        let la_elements = self.la.len();
        if la_elements > nb_elements {
            let _ = self.la.drain(0..nb_elements);
            return;
        } else {
            self.la.clear();
        }
        let mut rem = nb_elements - la_elements;
        while rem > 0 {
            if let Some(_) = self.next() {
                rem -= 1
            } else {
                panic!(
                    "cannot consume {} elements: still {} to consume but got end of stream",
                    nb_elements, rem
                )
            }
        }
    }

    /// Return the next element and consume it from the stream
    pub fn next(&mut self) -> Option<TokenTree> {
        if self.la.is_empty() {
            let tt = self.ts.next();
            if let Some(tt) = &tt {
                if self.first_span.is_none() {
                    self.first_span = Some(tt.span());
                }
                self.last_span = Some(tt.span());
            };
            tt
        } else {
            let token = self.la.remove(0);
            Some(token)
        }
    }

    pub fn try_chain<E, T>(
        self,
        parsers: &[fn(&mut ParserTry) -> Result<T, E>],
    ) -> (Result<T, Vec<E>>, Self) {
        let mut errors: Vec<E> = Vec::new();
        let mut current = self;
        for p in parsers.iter() {
            let mut try_parser = current.try_parse();
            match p(&mut try_parser) {
                Ok(t) => return (Ok(t), try_parser.commit()),
                Err(e) => {
                    current = try_parser.fail();
                    errors.push(e);
                }
            }
        }
        (Err(errors), current)
    }

    pub fn try_parse(self) -> ParserTry {
        ParserTry::new(self)
    }

    pub fn sep_by(mut self) -> Vec<Parser> {
        let mut parsers = Vec::new();
        let mut v = Vec::new();
        while let Some(x) = self.next() {
            match x {
                TokenTree::Punct(punct) if punct.as_char() == ';' => {
                    let mut out = Vec::new();
                    core::mem::swap(&mut v, &mut out);
                    parsers.push(Parser::from(out))
                }
                TokenTree::Punct(_)
                | TokenTree::Group(_)
                | TokenTree::Ident(_)
                | TokenTree::Literal(_) => {
                    v.push(x);
                }
            }
        }
        if v.len() > 0 {
            parsers.push(Parser::from(v));
        }
        parsers
    }
}

/// Tentative parser
pub(crate) struct ParserTry {
    parser: Parser,
    current: usize,
}

impl ParserTry {
    /// Create a new tentative parser that own the underlying parser
    ///
    /// This tentative parser only peek in the stream
    ///
    /// This parser must be either terminated with `commit` or `fail`,
    pub fn new(parser: Parser) -> Self {
        ParserTry { parser, current: 0 }
    }

    /// Return the kind of the next token
    pub fn peek_kind(&mut self) -> Option<TokenKind> {
        self.parser.peek_at(self.current).map(|tt| tt.into())
    }

    pub fn next(&mut self) -> Option<&TokenTree> {
        let tt = self.parser.peek_at(self.current);
        if tt.is_some() {
            self.current += 1
        }
        tt
    }

    /// Commit this parser tentative advance into the stream and return the modified parser
    pub fn commit(mut self) -> Parser {
        self.parser.consume(self.current);
        self.parser
    }

    /// Try to parse using f. On failure leave the parser position where it was
    pub fn parse_try<F, O, E>(&mut self, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut Self) -> Result<O, E>,
    {
        let save_current = self.current;
        match f(self) {
            Ok(t) => Ok(t),
            Err(e) => {
                self.current = save_current;
                Err(e)
            }
        }
    }

    /// Try to parse using f or on failure g. If both closures fails, the parser
    /// position is not changed.
    pub fn alternative<F, G, O, E>(&mut self, f: F, g: G) -> Result<O, E>
    where
        F: FnOnce(&mut Self) -> Result<O, E>,
        G: FnOnce(&mut Self) -> Result<O, E>,
    {
        self.parse_try(f).or_else(|_| self.parse_try(g))
    }

    /// Return the un-modified `Parser`
    ///
    /// Do note that the parser itself is modified as some items might have moved from the
    /// stream to the lookahead state, but the iterator ordering itself doesn't change.
    pub fn fail(self) -> Parser {
        self.parser
    }

    pub fn next_ident(&mut self) -> Result<&Ident, ParserError> {
        match self.next() {
            Some(tt) => match tt {
                TokenTree::Ident(ident) => Ok(ident),
                _ => Err(ParserError::Expecting {
                    expecting: TokenKind::Ident,
                    got: TokenKind::from(tt),
                }),
            },
            None => Err(ParserError::EndOfStream {
                expecting: Some(TokenKind::Ident),
            }),
        }
    }

    pub fn next_literal(&mut self) -> Result<&Literal, ParserError> {
        match self.next() {
            Some(tt) => match tt {
                TokenTree::Literal(literal) => Ok(literal),
                _ => Err(ParserError::Expecting {
                    expecting: TokenKind::Literal,
                    got: TokenKind::from(tt),
                }),
            },
            None => Err(ParserError::EndOfStream {
                expecting: Some(TokenKind::Literal),
            }),
        }
    }

    /// Try to get the next punct
    pub fn next_punct<M, T>(&mut self, f: M) -> Result<T, ParserError>
    where
        M: FnOnce(&Punct) -> Option<T>,
    {
        match self.next() {
            Some(tt) => match tt {
                TokenTree::Punct(punct) => match f(punct) {
                    None => Err(ParserError::NotMatches),
                    Some(t) => Ok(t),
                },
                _ => Err(ParserError::Expecting {
                    expecting: TokenKind::Punct,
                    got: TokenKind::from(tt),
                }),
            },
            None => Err(ParserError::EndOfStream {
                expecting: Some(TokenKind::Punct),
            }),
        }
    }

    /// Try to get the next group
    pub fn next_group<M, T>(&mut self, f: M) -> Result<T, ParserError>
    where
        M: FnOnce(&Group) -> Option<T>,
    {
        match self.next() {
            Some(tt) => match tt {
                TokenTree::Group(group) => match f(group) {
                    None => Err(ParserError::NotMatches),
                    Some(t) => Ok(t),
                },
                _ => Err(ParserError::Expecting {
                    expecting: TokenKind::Group,
                    got: TokenKind::from(tt),
                }),
            },
            None => Err(ParserError::EndOfStream {
                expecting: Some(TokenKind::Group),
            }),
        }
    }

    pub fn try_chain<E, T>(
        self,
        parsers: &[fn(&mut ParserTry) -> Result<T, E>],
    ) -> Result<T, Vec<E>> {
        let mut errors: Vec<E> = Vec::new();
        let mut current = self;
        for p in parsers.iter() {
            match current.parse_try(p) {
                Ok(t) => return Ok(t),
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        Err(errors)
    }
}
