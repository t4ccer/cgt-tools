use proc_macro::TokenStream;

mod tile;
#[proc_macro_derive(Tile, attributes(tile))]
pub fn derive_tile(input: TokenStream) -> TokenStream {
    crate::tile::derive(input)
}
