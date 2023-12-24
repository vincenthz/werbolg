use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};

pub(crate) struct Generator {
    stream: TokenStream,
}

impl Generator {
    pub fn tt_comma() -> TokenTree {
        Punct::new(',', proc_macro::Spacing::Alone).into()
    }

    pub fn tt_semicolon() -> TokenTree {
        Punct::new(';', proc_macro::Spacing::Alone).into()
    }

    pub fn new() -> Self {
        Self {
            stream: TokenStream::new(),
        }
    }

    pub fn finalize(self) -> TokenStream {
        self.stream
    }

    pub fn push_tokentree(&mut self, tt: TokenTree) {
        self.stream.extend(vec![tt]);
    }

    pub fn push_tokentrees(&mut self, tts: Vec<TokenTree>) {
        self.stream.extend(tts)
    }

    pub fn push_generator(&mut self, gen: Generator) {
        self.stream.extend(gen.stream)
    }

    pub fn call<I: Iterator<Item = Generator>>(&mut self, span: Span, ident: &str, elements: I) {
        let mut inner = Generator::new();
        let comma = Self::tt_comma();
        for (i, element) in elements.enumerate() {
            if i > 0 {
                inner.push_tokentree(comma.clone())
            }
            inner.push_generator(element)
        }
        let call_name = Ident::new(ident, span);
        let group = Group::new(Delimiter::Parenthesis, inner.stream);

        self.push_tokentrees(vec![call_name.into(), group.into()])
    }
}
