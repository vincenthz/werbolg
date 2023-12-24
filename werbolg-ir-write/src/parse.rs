use alloc::vec::Vec;
use proc_macro::{
    token_stream::IntoIter, Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree,
};

/// A Parser for TokenTree, with an arbitrary sized lookahead
pub(crate) struct Parser {
    /// Lookahead buffer of dynamic size
    la: Vec<TokenTree>,
    /// Rest of the stream
    ts: IntoIter,
}

impl From<TokenStream> for Parser {
    fn from(ts: TokenStream) -> Parser {
        Parser {
            la: Vec::new(),
            ts: ts.into_iter(),
        }
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
            self.ts.next()
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

    pub fn parse_try(self) -> ParserTry {
        ParserTry {
            parser: self.parser,
            current: self.current,
        }
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
}
