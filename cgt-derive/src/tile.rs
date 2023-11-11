use proc_macro::TokenStream;
use quote::quote;
use syn::{parenthesized, parse_macro_input, Data, DeriveInput, Ident, LitChar, Variant};

const TILE_ATTR: &'static str = "tile";
const TILE_ATTR_DEFAULT: &'static str = "default";
const TILE_ATTR_CHAR: &'static str = "char";

struct TileAttr {
    ident: Ident,
    is_default: bool,
    tile_char: Option<char>,
}

fn to_tile_attr(variant: Variant) -> TileAttr {
    let mut tile = TileAttr {
        ident: variant.ident,
        is_default: false,
        tile_char: None,
    };

    for attr in &variant.attrs {
        if !attr.path().is_ident(TILE_ATTR) {
            continue;
        }

        if let syn::Meta::List(meta) = &attr.meta {
            if meta.tokens.is_empty() {
                continue;
            }
        } else {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(TILE_ATTR_DEFAULT) {
                tile.is_default = true;
                return Ok(());
            }

            if meta.path.is_ident(TILE_ATTR_CHAR) {
                let content;
                parenthesized!(content in meta.input);
                let lit: LitChar = content.parse()?;
                tile.tile_char = Some(lit.value());
                return Ok(());
            }

            Err(meta.error(format!("Invalid attribute: {:?}", meta.path)))
        })
        .expect("foo");
    }

    tile
}

pub(crate) fn derive(input: TokenStream) -> TokenStream {
    // TODO: Detect external usage and use '::cgt' instead
    // maybe via bootstrap internal feature
    let cgt_crate = quote! {crate};

    let ast = parse_macro_input!(input as DeriveInput);

    let Data::Enum(tile_enum) = ast.data else {
        panic!("'Tile' derive macro can be used only on enum.")
    };
    let tile_enum_name = ast.ident;

    let tiles = tile_enum
        .variants
        .into_iter()
        .map(to_tile_attr)
        .collect::<Vec<_>>();

    let impl_default = {
        let default_tiles = tiles
            .iter()
            .filter(|tile| tile.is_default)
            .collect::<Vec<_>>();
        match &default_tiles[..] {
            [tile] => {
                let constr = &tile.ident;
                Some(quote! {
                    #[automatically_derived]
                    impl ::std::default::Default for #tile_enum_name {
                        #[inline]
                        fn default() -> Self {
                            #tile_enum_name::#constr
                        }
                    }
                })
            }
            [] => None,
            _ => panic!("Only one tile can be default"),
        }
    };

    let impl_char_tile = {
        let mut without_char = 0;

        let tile_to_chars = tiles
            .iter()
            .flat_map(|tile| match tile.tile_char {
                Some(c) => {
                    let constr = &tile.ident;
                    Some(quote! {
                        #tile_enum_name::#constr => #c
                    })
                }
                None => {
                    without_char += 1;
                    None
                }
            })
            .collect::<Vec<_>>();

        let char_to_tiles = tiles
            .iter()
            .flat_map(|tile| match tile.tile_char {
                Some(c) => {
                    let constr = &tile.ident;
                    Some(quote! {
                        #c => ::core::option::Option::Some(#tile_enum_name::#constr)
                    })
                }
                None => None,
            })
            .collect::<Vec<_>>();

        if without_char > 0 && without_char != tiles.len() {
            panic!("Either all or no tiles must have 'char' attribute");
        }

        // don't generate CharTile implementation
        if without_char == tiles.len() {
            return quote!().into();
        }

        quote! {
        #[automatically_derived]
        impl #cgt_crate::grid::CharTile for #tile_enum_name {
            #[inline]
            fn tile_to_char(self) -> char {
                match self {
                    #(#tile_to_chars),*
                }
            }

            #[inline]
            fn char_to_tile(input: char) -> Option<Self> {
                match input {
                    #(#char_to_tiles),*
                    ,_ => ::core::option::Option::None,
                }
            }
        }
        }
    };

    quote! {
        #impl_default
        #impl_char_tile
    }
    .into()
}
