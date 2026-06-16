use proc_macro::*;

#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    format!(
        "::picobench::define_benchmark!{{#[picobench::bench({})]{}}}",
        attr, item
    )
    .parse()
    .unwrap()
}
