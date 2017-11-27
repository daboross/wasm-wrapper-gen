extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate quote;
extern crate syn;

mod arguments;
mod processing;
mod parsing;

pub use arguments::KnownArgumentType;
pub use processing::{extract_func_info, get_argument_types, JsFnInfo, TransformedRustIdent};
pub use parsing::{transform_mac_to_items, transform_macro_input_to_items};

#[derive(Debug, Fail)]
pub enum MacroError {
    #[fail(display = "expected function, found invalid item '{:?}'", kind)]
    InvalidItemKind { kind: syn::ItemKind },
    #[fail(display = "expected regular non-self function parameter, found '{:?}'", arg)]
    InvalidArgument { arg: syn::FnArg },
    #[fail(display = "expected one of the known argument types (&[u8], &mut [u8]), found '{:?}",
           ty)]
    UnhandledArgumentType { ty: syn::Ty },
    #[fail(display = "expected macro to contain a single delimited token tree, found \
                      multiple: {:?}",
           tokens)]
    UnexpectedMultiTokenMacro { tokens: Vec<syn::TokenTree> },
    #[fail(display = "expected multiple tokens in js_fn! macro invocation, found single \
                      token: '{:?}'",
           token)]
    UnexpectedSingleToken { token: syn::Token },
    #[fail(display = "expected all complete `fn a(..) => ..;` or `fn a(..) {{ .. }}` \
                      inside js_fn! macro, found incomplete tokens left: {:?}",
           tokens)]
    UnexpectedEndOfMacroInvocation { tokens: quote::Tokens },
    #[fail(display = "failed to parse processed macro invocation: {:?}", err_msg)]
    UnexpectedReparseFailure { err_msg: String },
}
