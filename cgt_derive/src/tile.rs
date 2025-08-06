use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Ident, LitBool, LitChar, Variant, parenthesized, parse_macro_input};

const TILE_ATTR: &str = "tile";
const TILE_ATTR_DEFAULT: &str = "default";
const TILE_ATTR_CHAR: &str = "char";
const TILE_ATTR_BOOL: &str = "bool";

struct TileAttr {
    ident: Ident,
    tile_default: Option<Span>,
    tile_char: Option<(char, Span)>,
    tile_bool: Option<(bool, Span)>,
}

struct Error {
    span: Span,
    msg: String,
}

fn to_tile_attr(variant: Variant) -> Result<TileAttr, Error> {
    let mut tile = TileAttr {
        ident: variant.ident,
        tile_default: None,
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

            attr.parse_nested_meta(|meta| {
                if let Some(path) = meta.path.get_ident() {
                    if path == TILE_ATTR_DEFAULT {
                        tile.tile_default = Some(path.span());
                        return Ok(());
                    }

                    if path == TILE_ATTR_CHAR {
                        let content;
                        parenthesized!(content in meta.input);
                        let span = content.span();
                        let lit: LitChar = content.parse()?;
                        tile.tile_char = Some((lit.value(), span));
                        return Ok(());
                    }

                    if path == TILE_ATTR_BOOL {
                        let content;
                        parenthesized!(content in meta.input);
                        let span = content.span();
                        let lit: LitBool = content.parse()?;
                        tile.tile_bool = Some((lit.value(), span));
                        return Ok(());
                    }
                }

                Err(meta.error(format!(
                    "Invalid attribute: '{}'",
                    meta.path
                        .get_ident()
                        .map_or_else(|| String::from("<UNKNOWN>"), ToString::to_string)
                )))
            })
            .map_err(|err| Error {
                span: err.span(),
                msg: format!("{err}"),
            })?;
        }
    }

    Ok(tile)
}

pub fn derive(input: TokenStream) -> TokenStream {
    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();
    let cgt_crate = if crate_name == "cgt" {
        quote! {crate}
    } else {
        quote! {crate_name}
    };

    let ast = parse_macro_input!(input as DeriveInput);

    let tile_enum_name = ast.ident;

    let dummy_impls = quote! {
        #[automatically_derived]
        impl ::std::default::Default for #tile_enum_name {
            #[inline]
            fn default() -> Self {
                unimplemented!()
            }
        }

        #[automatically_derived]
        impl #cgt_crate::grid::BitTile for #tile_enum_name {
            #[inline]
            fn tile_to_bool(self) -> bool {
                unimplemented!()
            }


            #[inline]
            fn bool_to_tile(_input: bool) -> Self {
                unimplemented!()
            }
        }

        #[automatically_derived]
        impl #cgt_crate::grid::CharTile for #tile_enum_name {
            #[inline]
            fn tile_to_char(self) -> char {
                unimplemented!()
            }

            #[inline]
            fn char_to_tile(_input: char) -> Option<Self> {
                unimplemented!()
            }
        }
    };

    let Data::Enum(tile_enum) = ast.data else {
        return quote! {
            #dummy_impls
            compile_error!("'Tile' derive macro can be used only on enum");
        }
        .into();
    };

    match tile_enum
        .variants
        .into_iter()
        .map(to_tile_attr)
        .collect::<Result<Vec<_>, _>>()
    {
        Err(err) => {
            let msg = err.msg;
            quote_spanned! { err.span =>
                #dummy_impls
                compile_error!(#msg);
            }
            .into()
        }
        Ok(tiles) => {
            let impl_default = {
                let default_tiles = tiles
                    .iter()
                    .filter(|tile| tile.tile_default.is_some())
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
                    [_, tile, ..] => {
                        let span = tile.tile_default.unwrap();
                        return quote_spanned! { span =>
                            #dummy_impls
                            compile_error!("Only one tile can be default");
                        }
                        .into();
                    }
                }
            };

            let impl_char_tile = {
                let mut without_char = Vec::new();

                let tile_to_chars = tiles
                    .iter()
                    .filter_map(|tile| {
                        tile.tile_char.map_or_else(
                            || {
                                without_char.push(tile.ident.span());
                                None
                            },
                            |(c, _)| {
                                let constr = &tile.ident;
                                Some(quote! {
                                    #tile_enum_name::#constr => #c
                                })
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                let mut unique_chars = HashSet::new();
                let mut duplicated_span = None;
                let char_to_tiles = tiles
                    .iter()
                    .filter_map(|tile| {
                        tile.tile_char.map(|(c, c_span)| {
                            if !unique_chars.insert(c) {
                                duplicated_span = Some(c_span);
                            }
                            let constr = &tile.ident;
                            Some(quote! {
                                #c => ::core::option::Option::Some(#tile_enum_name::#constr)
                            })
                        })
                    })
                    .collect::<Vec<_>>();

                if !without_char.is_empty() && without_char.len() != tiles.len() {
                    return quote_spanned! { without_char[0] =>
                        #dummy_impls
                        compile_error!("Either all or no tiles must have '#[tile(char(...))]' attribute");
                    }
                    .into();
                }

                if let Some(duplicated_span) = duplicated_span {
                    return quote_spanned! { duplicated_span =>
                        #dummy_impls
                        compile_error!("Every '#[tile(char(...))]' attribute must be unique");
                    }
                    .into();
                }

                // don't generate CharTile implementation
                if without_char.len() == tiles.len() {
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
                let mut without_bool = Vec::new();
                let mut case_true = None;
                let mut case_false = None;

                for tile in &tiles {
                    let constr = &tile.ident;

                    match tile.tile_bool {
                        Some((b, span)) => {
                            if b {
                                match case_true {
                                    Some(_) => {
                                        return quote_spanned! { span =>
                                            #dummy_impls
                                            compile_error!("Only one tile can be 'bool(true)'");
                                        }
                                        .into();
                                    }
                                    None => case_true = Some(quote! { #tile_enum_name::#constr }),
                                }
                            } else {
                                match case_false {
                                    Some(_) => {
                                        return quote_spanned! { span =>
                                            #dummy_impls
                                            compile_error!("Only one tile can be 'bool(false)'");
                                        }
                                        .into();
                                    }
                                    None => case_false = Some(quote! { #tile_enum_name::#constr }),
                                }
                            }
                        }
                        None => without_bool.push(constr.span()),
                    }
                }

                if without_bool.len() == tiles.len() {
                    None
                } else if !without_bool.is_empty() {
                    return quote_spanned! { without_bool[0] =>
                            #dummy_impls
                            compile_error!("Either all or no tiles must have '#[tile(bool(...))]' attribute");
                        }
                        .into();
                } else {
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
    }
}
