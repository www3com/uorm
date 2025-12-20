mod mapper_assets;
use proc_macro::TokenStream;

#[proc_macro]
pub fn mapper_assets(input: TokenStream) -> TokenStream {
    mapper_assets::mapper_assets_impl(input)
}
