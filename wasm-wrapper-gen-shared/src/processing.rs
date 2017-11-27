use std::fmt;

use {quote, syn};

use failure::Error;

use MacroError;

use arguments::KnownArgumentType;

pub fn extract_func_info(
    item: &syn::Item,
) -> Result<(&syn::Item, &syn::FnDecl, &syn::Block), MacroError> {
    match item.node {
        syn::ItemKind::Fn(ref decleration, _, _, _, _, ref block) => {
            Ok((item, &**decleration, block))
        }
        ref kind => Err(MacroError::InvalidItemKind { kind: kind.clone() })?,
    }
}

pub fn get_argument_types(decl: &syn::FnDecl) -> Result<Vec<KnownArgumentType>, MacroError> {
    Ok(decl.inputs
        .iter()
        .map(|input| match *input {
            syn::FnArg::SelfRef(_, _) | syn::FnArg::SelfValue(_) => {
                Err(MacroError::InvalidArgument { arg: input.clone() })
            }
            syn::FnArg::Captured(_, ref ty) | syn::FnArg::Ignored(ref ty) => Ok(ty.clone()),
        })
        .map(|ty_result| {
            ty_result.and_then(|ty| KnownArgumentType::new(&ty))
        })
        .collect::<Result<_, _>>()?)
}


// TODO: find and store doc-comments in here for use in generating JS code comments.
pub struct JsFnInfo {
    pub rust_name: String,
    pub args_ty: Vec<KnownArgumentType>,
    pub ret_ty: syn::Ty,
}


impl JsFnInfo {
    pub fn try_from(item: &syn::Item) -> Result<Self, Error> {
        let (item, decl, _) = extract_func_info(item)?;

        let argument_types = get_argument_types(decl)?;
        let ret_ty = match decl.output {
            syn::FunctionRetTy::Default => syn::Ty::Tup(Vec::new()),
            syn::FunctionRetTy::Ty(ref ty) => ty.clone(),
        };

        Ok(JsFnInfo {
            rust_name: item.ident.to_string(),
            args_ty: argument_types,
            ret_ty: ret_ty,
        })
    }
}

static TRANSFORMED_FUNC_PREFX: &'static str = "__js_fn";

#[derive(Debug, Clone)]
pub struct TransformedRustIdent<T> {
    name: T,
}

impl<T> TransformedRustIdent<T> {
    pub fn new(name: T) -> TransformedRustIdent<T> {
        TransformedRustIdent { name }
    }
}

impl<T: fmt::Display> quote::ToTokens for TransformedRustIdent<T> {
    fn to_tokens(&self, tokens: &mut quote::Tokens) {
        tokens.append(format!("{}{}", TRANSFORMED_FUNC_PREFX, self.name));
    }
}

impl<T: fmt::Display> fmt::Display for TransformedRustIdent<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}{}", TRANSFORMED_FUNC_PREFX, self.name)
    }
}
