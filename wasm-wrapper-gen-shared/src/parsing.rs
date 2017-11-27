use syn::TokenTree::*;
use syn::{self, DelimToken};
use quote::{self, ToTokens};

use MacroError;

pub fn transform_mac_to_items(source: syn::Mac) -> Result<Vec<syn::Item>, MacroError> {
    if source.tts.len() > 1 {
        return Err(MacroError::UnexpectedMultiTokenMacro { tokens: source.tts })?;
    }
    match source.tts.into_iter().next() {
        Some(tt) => match tt {
            Delimited(delimited) => transform_macro_input_to_items(delimited.tts),
            Token(t) => Err(MacroError::UnexpectedSingleToken { token: t }),
        },
        None => Err(MacroError::UnexpectedMultiTokenMacro { tokens: Vec::new() }),
    }
}

pub fn transform_macro_input_to_items(
    tts: Vec<syn::TokenTree>,
) -> Result<Vec<syn::Item>, MacroError> {
    let mut found_full = Vec::new();

    let mut so_far = quote::Tokens::new();
    let mut iter = tts.into_iter();
    while let Some(token_tree) = iter.next() {
        match token_tree {
            // This matches a definition like:
            // ```
            // fn a() => modname::funcname;
            // ```
            Token(syn::Token::FatArrow) => {
                // gather all tokens until ';'
                let mut inner_tokens = Vec::new();
                while let Some(inner_token) = iter.next() {
                    match inner_token {
                        Token(syn::Token::Semi) => break,
                        _ => inner_tokens.push(inner_token),
                    }
                }
                Delimited(syn::Delimited {
                    delim: DelimToken::Brace,
                    tts: inner_tokens,
                }).to_tokens(&mut so_far);
                found_full.push(so_far);
                so_far = quote::Tokens::new();
            }
            // This matches a definition like:
            // ```
            // fn a() {
            //     // inline code
            // }
            // ```
            Delimited(syn::Delimited {
                delim: DelimToken::Brace,
                ..
            }) => {
                token_tree.to_tokens(&mut so_far);
                found_full.push(so_far);
                so_far = quote::Tokens::new();
            }
            ref other => other.to_tokens(&mut so_far),
        }
    }

    if !so_far.as_ref().is_empty() {
        return Err(MacroError::UnexpectedEndOfMacroInvocation {
            tokens: so_far,
        });
    }

    found_full
        .into_iter()
        .map(|found| {
            syn::parse_item(found.as_ref()).map_err(|desc| {
                MacroError::UnexpectedReparseFailure { err_msg: desc }
            })
        })
        .collect::<Result<Vec<syn::Item>, MacroError>>()
}
