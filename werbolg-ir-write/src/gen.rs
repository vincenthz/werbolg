use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

/*
pub(crate) struct Generator {
    stream: TokenStream,
}

pub struct Path(Span, bool, Vec<String>);

impl Path {
    pub(crate) fn from_str(span: Span, s: &str) -> Self {
        let mut el = s.split("::");
        let mut v = el.map(|s| s.to_string()).collect::<Vec<_>>();
        if v.len() > 0 {
            let global = v[0] == "";
            if global {
                v.remove(0);
            }
            Self(span, global, v)
        } else {
            panic!("cannot have empty path")
        }
    }

    fn to_tokentree(&self) -> Vec<TokenTree> {
        let mut v = vec![];

        for (i, p) in self.2.iter().enumerate() {
            if i > 0 || self.1 {
                v.push(Punct::new(':', proc_macro::Spacing::Joint).into());
                v.push(Punct::new(':', proc_macro::Spacing::Alone).into());
            }
            v.push(Ident::new(&p, self.0).into());
        }
        v
    }
}
*/

pub(crate) trait ExtendsStream {
    fn extends_stream(self, ts: &mut TokenStream);
}

/*
impl ExtendsStream for Path {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(self.to_tokentree())
    }
}

impl ExtendsStream for &Path {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(self.to_tokentree())
    }
}
*/

impl ExtendsStream for &TokenStream {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(self.clone())
    }
}

impl ExtendsStream for TokenStream {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(self)
    }
}

impl ExtendsStream for u32 {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(vec![TokenTree::from(Literal::u32_unsuffixed(self))])
    }
}

impl ExtendsStream for String {
    fn extends_stream(self, ts: &mut TokenStream) {
        ts.extend(vec![TokenTree::from(Ident::new(&self, Span::call_site()))])
    }
}

macro_rules! quote {
    () => {
        TokenStream::new()
    };
    ($($tt:tt)*) => {{
            let mut _ts = TokenStream::new();
            quote_inner!{_ts $($tt)*}
            _ts
    }};
}

macro_rules! quote_inner {
    ($ts:ident) => {};
    ($ts:ident # $n:ident $($tail:tt)*) => {{
        $n.extends_stream(&mut $ts);
        quote_inner!($ts $($tail)*);
    }};
    ($ts:ident # $n:ident $($tail:tt)*) => {
        $n.extends_stream(&mut $ts);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident #[#$inner:ident],* $($tail:tt)*) => {
        let mut _inner = TokenStream::new();
        for (i, e) in $inner.iter().enumerate() {
            if i > 0 {
                quote_inner!(_inner ,)
            }
            quote_inner!(_inner #e)
        }
        let token = Group::new(Delimiter::Bracket, _inner);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident [ $($any:tt),* ] $($tail:tt)*) => {
        let mut _inner = TokenStream::new();
        quote_inner!(_inner $($any),*);
        let token = Group::new(Delimiter::Bracket, _inner);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident { $($any:tt)* } $($tail:tt)*) => {
        let mut _inner = TokenStream::new();
        quote_inner!(_inner $($any)*);
        let token = Group::new(Delimiter::Brace, _inner);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident ( $($any:tt)* ) $($tail:tt)*) => {
        let mut _inner = quote!($($any)*);
        let token = Group::new(Delimiter::Parenthesis, _inner);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident $i:ident $($tail:tt)*) => {
        let token = Ident::new(stringify!($i), Span::call_site());
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident $i:literal $($tail:tt)*) => {
        $i.extends_stream(&mut $ts);
        quote_inner!($ts $($tail)*);
    };
    ($ts:ident : $($tail:tt)*) => {{
        let token1 = Punct::new(':', proc_macro::Spacing::Alone);
        $ts.extend(vec![TokenTree::from(token1)]);
        quote_inner!($ts $($tail)*);
    }};
    ($ts:ident :: $($tail:tt)*) => {{
        let token1 = Punct::new(':', proc_macro::Spacing::Joint);
        let token2 = Punct::new(':', proc_macro::Spacing::Alone);
        $ts.extend(vec![TokenTree::from(token1), TokenTree::from(token2)]);
        quote_inner!($ts $($tail)*);
    }};
    ($ts:ident , $($tail:tt)*) => {{
        let token = Punct::new(',', proc_macro::Spacing::Alone);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    }};
    ($ts:ident ; $($tail:tt)*) => {
        let token = Punct::new(';', proc_macro::Spacing::Alone);
        $ts.extend(vec![TokenTree::from(token)]);
        quote_inner!($ts $($tail)*);
    };
}
