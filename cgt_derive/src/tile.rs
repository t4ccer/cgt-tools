use proc_macro::TokenStream;
use quote::quote;
use syn::{parenthesized, parse_macro_input, Data, DeriveInput, Ident, LitBool, LitChar, Variant};

const TILE_ATTR: &str = "tile";
const TILE_ATTR_DEFAULT: &str = "default";
const TILE_ATTR_CHAR: &str = "char";
const TILE_ATTR_BOOL: &str = "bool";

struct TileAttr {
    ident: Ident,
    is_default: bool,
    tile_char: Option<char>,
    tile_bool: Option<bool>,
}

fn to_tile_attr(variant: Variant) -> TileAttr {
    let mut tile = TileAttr {
        ident: variant.ident,
        is_default: false,
        tile_char: None,
        tile_bool: None,
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

            if meta.path.is_ident(TILE_ATTR_BOOL) {
                let content;
                parenthesized!(content in meta.input);
                let lit: LitBool = content.parse()?;
                tile.tile_bool = Some(lit.value());
                return Ok(());
            }

            Err(meta.error(format!(
                "Invalid attribute: '{}'",
                meta.path.get_ident().unwrap()
            )))
        })
        .unwrap_or_else(|err| panic!("{}", err));
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
            None
        } else {
            Some(quote! {
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
            })
        }
    };

    let impl_char_bool = {
        let mut with_bool = 0;
        let mut case_true = None;
        let mut case_false = None;

        tiles.iter().for_each(|tile| {
            let constr = &tile.ident;

            if let Some(b) = tile.tile_bool {
                with_bool += 1;

                if b {
                    match case_true {
                        Some(_) => panic!("Only one tile can be 'bool(true)'"),
                        None => case_true = Some(quote! { #tile_enum_name::#constr }),
                    }
                } else {
                    match case_false {
                        Some(_) => panic!("Only one tile can be 'bool(false)'"),
                        None => case_false = Some(quote! { #tile_enum_name::#constr }),
                    }
                }
            }
        });

        if with_bool == 0 {
            None
        } else {
            if with_bool != tiles.len() {
                panic!("Either all or no tiles must have 'bool' attribute");
            }

            let case_true = case_true.unwrap();
            let case_false = case_false.unwrap();

            Some(quote! {
                #[automatically_derived]
                impl #cgt_crate::grid::BitTile for #tile_enum_name {
                    #[inline]
                    fn tile_to_bool(self) -> bool {
                        match self {
                            #case_true => true,
                            #case_false => false,
                        }
                    }


                    #[inline]
                    fn bool_to_tile(input: bool) -> Self {
                        if input {
                            #case_true
                        } else {
                            #case_false
                        }
                    }
                }
            })
        }
    };

    quote! {
        #impl_default
        #impl_char_tile
        #impl_char_bool
    }
    .into()
}
