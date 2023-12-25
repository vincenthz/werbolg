use alloc::{string::ToString, vec::Vec};
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

use super::helper::to_kind;

/// lightweight wrapper to append easily to a (quasi) TokenStream
pub(crate) struct Output(Vec<TokenTree>);

impl Output {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[inline]
    pub fn push_literal(&mut self, lit: Literal) {
        self.0.push(TokenTree::from(lit))
    }

    #[inline]
    pub fn push_ident(&mut self, p: Ident) {
        self.0.push(TokenTree::from(p))
    }

    #[inline]
    pub fn push_punct(&mut self, p: Punct) {
        self.0.push(TokenTree::from(p))
    }

    #[inline]
    pub fn push_dcolon(&mut self) {
        self.0
            .push(TokenTree::from(Punct::new(':', Spacing::Joint)));
        self.0
            .push(TokenTree::from(Punct::new(':', Spacing::Alone)));
    }

    #[inline]
    pub fn push_dot(&mut self) {
        self.0
            .push(TokenTree::from(Punct::new('.', Spacing::Alone)));
    }

    #[inline]
    pub fn push_semicolon(&mut self) {
        self.0
            .push(TokenTree::from(Punct::new(';', Spacing::Alone)));
    }

    #[inline]
    pub fn push_comma(&mut self) {
        self.0
            .push(TokenTree::from(Punct::new(',', Spacing::Alone)));
    }

    pub fn ts() -> Ident {
        Ident::new("_ts", Span::call_site())
    }

    pub fn ts_inner() -> Ident {
        Ident::new("_ts_inner", Span::call_site())
    }

    pub fn push_ts(&mut self) {
        self.push_ident(Self::ts())
    }

    pub fn push_ts_inner(&mut self) {
        self.push_ident(Self::ts_inner())
    }

    pub fn push_let_ident_eq(&mut self, mutable: bool, ident: &Ident) {
        self.push_ident(Ident::new("let", ident.span()));
        if mutable {
            self.push_ident(Ident::new("mut", ident.span()));
        }
        self.push_ident(ident.clone());
        self.push_punct(Punct::new('=', Spacing::Alone));
    }

    pub fn push_let_some_ident_eq(&mut self, ident: &Ident) {
        self.push_ident(Ident::new("let", ident.span()));
        self.push_ident(Ident::new("Some", ident.span()));
        self.arg1(|inner| inner.push_ident(ident.clone()));
        self.push_punct(Punct::new('=', Spacing::Alone));
    }

    pub fn push_new_ts<F>(&mut self, f: F, it: TokenStream)
    where
        F: FnOnce(&mut Output, TokenStream),
    {
        let mut root = Output::new();
        root.push_let_ident_eq(true, &Self::ts());
        root.push_tokenstream_new();
        root.push_semicolon();

        f(&mut root, it);

        root.push_ts();
        self.push_grp(Group::new(Delimiter::Brace, root.finalize()));
    }

    pub fn push_path(&mut self, span: Span, absolute: bool, fragments: &[&str]) {
        for (i, fragment) in fragments.iter().enumerate() {
            if absolute || i > 0 {
                self.push_dcolon();
            }
            self.push_ident(Ident::new(fragment, span.clone()))
        }
    }

    /*
    pub fn push_call_literal_char(&mut self, s: char) {
        self.push_path(
            Span::call_site(),
            true,
            &["proc_macro", "Literal", "character"],
        );

        self.arg1(|gen1| {
            gen1.push_literal(Literal::character(s));
        })
    }

    pub fn push_call_literal_string(&mut self, s: &str) {
        self.push_path(
            Span::call_site(),
            true,
            &["proc_macro", "Literal", "string"],
        );
        self.arg1(|gen1| {
            gen1.push_literal(Literal::string(s));
        })
    }
    */

    pub fn push_call_span_call_site(&mut self) {
        self.push_path(
            Span::call_site(),
            true,
            &["proc_macro", "Span", "call_site"],
        );
        self.arg0();
    }

    pub fn bracket<F>(&mut self, gen: F)
    where
        F: FnOnce(&mut Output),
    {
        let mut inner: Output = Output::new();
        gen(&mut inner);
        self.push_grp(Group::new(Delimiter::Bracket, inner.finalize()).into());
    }

    pub fn brace<F>(&mut self, gen: F)
    where
        F: FnOnce(&mut Output),
    {
        let mut inner: Output = Output::new();
        gen(&mut inner);
        self.push_grp(Group::new(Delimiter::Brace, inner.finalize()).into());
    }

    pub fn arg0(&mut self) {
        self.0
            .push(Group::new(Delimiter::Parenthesis, TokenStream::new()).into());
    }

    pub fn arg1<F>(&mut self, gen1: F)
    where
        F: FnOnce(&mut Output),
    {
        let mut inner: Output = Output::new();
        gen1(&mut inner);
        self.push_grp(Group::new(Delimiter::Parenthesis, inner.finalize()).into());
    }

    pub fn arg2<F, G>(&mut self, gen1: F, gen2: G)
    where
        F: FnOnce(&mut Output),
        G: FnOnce(&mut Output),
    {
        let mut inner: Output = Output::new();
        gen1(&mut inner);
        inner.push_comma();
        gen2(&mut inner);
        self.push_grp(Group::new(Delimiter::Parenthesis, inner.finalize()).into());
    }

    pub fn ts_extend<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Output),
    {
        self.push_ts();
        self.push_dot();
        self.push_ident(Ident::new("extend", Span::call_site()));
        self.arg1(|gen1| f(gen1));
        self.push_semicolon();
    }

    pub fn ts_extend_one_tokentreeable<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Output),
    {
        self.ts_extend(|inner| inner.wrap_vec(|inner| inner.wrap_tokentree(f)))
    }

    pub fn wrap_vec<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Output),
    {
        self.push_path(Span::call_site(), true, &["alloc", "vec", "Vec", "from"]);
        self.arg1(|gen1| {
            gen1.bracket(f);
        })
    }

    pub fn wrap_tokentree<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Output),
    {
        self.push_path(
            Span::call_site(),
            true,
            &["proc_macro", "TokenTree", "from"],
        );
        self.arg1(f);
    }

    /// Push a call to Literal::new(string, site) into ts
    pub fn push_escaped_literal(&mut self, lit: Literal) {
        let kind = to_kind(&lit);
        let method_name = kind.to_method_name();
        self.ts_extend_one_tokentreeable(|inner| {
            let span = lit.span();
            inner.push_path(span, true, &["proc_macro", "Literal", method_name]);
            inner.arg1(|gen1| gen1.push_literal(lit));
        })
    }

    /// Push a call to Ident::new(string, site)
    pub fn push_escaped_ident(&mut self, p: Ident) {
        self.ts_extend_one_tokentreeable(|inner| {
            inner.push_path(p.span(), true, &["proc_macro", "Ident", "new"]);
            inner.arg2(
                |gen1| {
                    gen1.push_literal(Literal::string(&p.to_string()));
                },
                |gen2| {
                    gen2.push_call_span_call_site();
                },
            );
        })
    }

    /// Push a call to Punct::new(string, site)
    pub fn push_escaped_punct(&mut self, p: Punct) {
        self.ts_extend_one_tokentreeable(|inner| {
            inner.push_path(p.span(), true, &["proc_macro", "Punct", "new"]);
            inner.arg2(
                |gen1| gen1.push_literal(Literal::character(p.as_char())),
                |gen2| match p.spacing() {
                    Spacing::Alone => {
                        gen2.push_path(p.span(), true, &["proc_macro", "Spacing", "Alone"]);
                    }
                    Spacing::Joint => {
                        gen2.push_path(p.span(), true, &["proc_macro", "Spacing", "Joint"]);
                    }
                },
            );
        })
    }

    /// Push a call to Group::new(string, site)
    pub fn push_escaped_grp<F>(&mut self, g: Group, f: F)
    where
        F: FnOnce(&mut Output, TokenStream),
    {
        let delimiter_name = match g.delimiter() {
            Delimiter::Parenthesis => "Parenthesis",
            Delimiter::Brace => "Brace",
            Delimiter::Bracket => "Bracket",
            Delimiter::None => "None",
        };
        self.push_let_ident_eq(false, &Self::ts_inner());
        self.push_new_ts(f, g.stream());
        self.push_semicolon();

        self.ts_extend_one_tokentreeable(|inner| {
            inner.push_path(g.span(), true, &["proc_macro", "Group", "new"]);
            inner.arg2(
                |gen1| gen1.push_path(g.span(), true, &["proc_macro", "Delimiter", delimiter_name]),
                |gen2| gen2.push_ts_inner(),
            );
        })
    }

    // push a call to TokenStream::new()
    pub fn push_tokenstream_new(&mut self) {
        self.push_path(
            Span::call_site(),
            true,
            &["proc_macro", "TokenStream", "new"],
        );
        self.arg0();
    }

    /*
    pub fn push_tokenstream_from_iter(&mut self) {
        /*
        self.push_ident(Ident::new("TokenStream", Span::call_site()));
        self.push_dcolon();
        self.push_ident(Ident::new("new", Span::call_site()));
        self.push_empty_call();
        */
        todo!()
    }
    */

    #[inline]
    pub fn push_grp(&mut self, grp: Group) {
        self.0.push(TokenTree::from(grp))
    }

    /*
    #[inline]
    pub fn append(&mut self, mut trees: Vec<TokenTree>) {
        self.0.append(&mut trees)
    }

    #[inline]
    pub fn push_output(&mut self, delim: Delimiter, p: Output) {
        self.push_grp(Group::new(delim, p.finalize()))
    }
    */

    #[inline]
    pub fn finalize(self) -> TokenStream {
        TokenStream::from_iter(self.0)
    }
}
