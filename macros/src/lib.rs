use proc_macro::*;

#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    TokenStream::from_iter([
        stream::code("::picobench::define_benchmark!"),
        stream::group(
            Delimiter::Brace,
            [
                stream::code("#"),
                stream::group(
                    Delimiter::Bracket,
                    [
                        stream::code("picobench::bench"),
                        stream::group(Delimiter::Parenthesis, [attr]),
                    ],
                ),
                item,
            ],
        ),
    ])
}

// all this so we can preserve spans
mod stream {
    use proc_macro::*;

    pub fn group(
        delimiter: Delimiter,
        input: impl IntoIterator<Item = TokenStream>,
    ) -> TokenStream {
        let mut stream = TokenStream::new();
        stream.extend([TokenTree::Group(proc_macro::Group::new(
            delimiter,
            TokenStream::from_iter(input),
        ))]);
        stream
    }

    pub fn code(str: &str) -> TokenStream {
        str.parse::<TokenStream>().unwrap()
    }
}
