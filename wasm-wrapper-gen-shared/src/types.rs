use std::fmt;

use quote::{ToTokens, Tokens};

use syn;

use MacroError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SimpleIntegerTy {
    U8,
    U16,
    U32,
    // U64, // 64-bit types aren't well supported
    I8,
    I16,
    I32,
    // I64, // 64-bit types aren't well supported
    USize,
    ISize,
    Bool,
}

impl SimpleIntegerTy {
    pub fn new<T: AsRef<str>>(ident: &T) -> Option<Self> {
        use self::SimpleIntegerTy::*;
        match ident.as_ref() {
            "u8" => Some(U8),
            "u16" => Some(U16),
            "u32" => Some(U32),
            // "u64" => Some(U64),
            "i8" => Some(I8),
            "i16" => Some(I16),
            "i32" => Some(I32),
            // "i64" => Some(I64),
            "usize" => Some(USize),
            "isize" => Some(ISize),
            "bool" => Some(Bool),
            _ => None,
        }
    }

    pub fn size_in_bytes(&self) -> usize {
        use self::SimpleIntegerTy::*;
        // not using `std::mem::size_of` since that's for the current platform, not wasm.
        match *self {
            U8 => 1,
            U16 => 2,
            U32 => 4,
            // U64 => 8,
            I8 => 1,
            I16 => 2,
            I32 => 4,
            // I64 => 8,
            USize => 4,
            ISize => 4,
            Bool => 1,
        }
    }
}

impl AsRef<str> for SimpleIntegerTy {
    fn as_ref(&self) -> &str {
        use self::SimpleIntegerTy::*;
        match *self {
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            // U64 => "u64",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            // I64 => "i64",
            USize => "usize",
            ISize => "isize",
            Bool => "bool",
        }
    }
}

impl fmt::Display for SimpleIntegerTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl ToTokens for SimpleIntegerTy {
    fn to_tokens(&self, tokens: &mut Tokens) {
        tokens.append(self.as_ref());
    }
}

pub enum SupportedArgumentType {
    // &[u8]
    IntegerSliceRef(SimpleIntegerTy),
    // &mut [u8]
    IntegerSliceMutRef(SimpleIntegerTy),
    // Vec<u8>
    IntegerVec(SimpleIntegerTy),
    // u8, u16, u32, u64, i8, i16, i32, i64, usize, isize,
    Integer(SimpleIntegerTy),
    // TODO: more types
}

fn resolve_parens(mut ty: &syn::Ty) -> &syn::Ty {
    while let syn::Ty::Paren(ref temp) = *ty {
        ty = temp;
    }
    ty
}

fn is_u8(ty: &syn::Ty) -> bool {
    let ty = resolve_parens(ty);
    if let syn::Ty::Path(None, ref path) = *ty {
        if path.segments.len() <= 1 {
            if let Some(segment) = path.segments.first() {
                if segment.ident == "u8" && segment.parameters == syn::PathParameters::none() {
                    return true;
                }
            }
        }
    }
    false
}

fn as_simple_integer(ty: &syn::Ty) -> Option<SimpleIntegerTy> {
    let ty = resolve_parens(ty);
    if let syn::Ty::Path(None, ref path) = *ty {
        if path.segments.len() <= 1 {
            if let Some(segment) = path.segments.first() {
                if segment.parameters == syn::PathParameters::none() {
                    return SimpleIntegerTy::new(&segment.ident);
                }
            }
        }
    }
    None
}

fn as_vec_simple_integer_type(ty: &syn::Ty) -> Option<SimpleIntegerTy> {
    let ty = resolve_parens(ty);
    if let syn::Ty::Path(None, ref path) = *ty {
        if path.segments.len() <= 1 {
            if let Some(segment) = path.segments.first() {
                if segment.ident == "Vec" {
                    if let syn::PathParameters::AngleBracketed(ref params) = segment.parameters {
                        if params.lifetimes.is_empty() && params.bindings.is_empty()
                            && params.types.len() == 1
                        {
                            if let Some(single_param_type) = params.types.first() {
                                return as_simple_integer(single_param_type);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

impl SupportedArgumentType {
    pub fn new(ty: &syn::Ty) -> Result<Self, MacroError> {
        let ty = resolve_parens(ty);
        if let syn::Ty::Rptr(_, ref slice_ty_mut) = *ty {
            let slice_ty = resolve_parens(&slice_ty_mut.ty);
            if let syn::Ty::Slice(ref byte_ty) = *slice_ty {
                if let Some(inner_ty) = as_simple_integer(byte_ty) {
                    return Ok(match slice_ty_mut.mutability {
                        syn::Mutability::Immutable => {
                            SupportedArgumentType::IntegerSliceRef(inner_ty)
                        }
                        syn::Mutability::Mutable => {
                            SupportedArgumentType::IntegerSliceMutRef(inner_ty)
                        }
                    });
                }
                if is_u8(byte_ty) {}
            }
        }
        if let Some(int_ty) = as_simple_integer(ty) {
            return Ok(SupportedArgumentType::Integer(int_ty));
        }
        if let Some(item_ty) = as_vec_simple_integer_type(ty) {
            return Ok(SupportedArgumentType::IntegerVec(item_ty));
        }
        Err(MacroError::UnhandledArgumentType { ty: ty.clone() })?
    }
}

pub enum SupportedRetType {
    // Vec<u8>
    IntegerVec(SimpleIntegerTy),
    // u8, u16, u32, u64, i8, i16, i32, i64, usize, isize,
    Integer(SimpleIntegerTy),
    // ()
    Unit,
}


impl SupportedRetType {
    pub fn new(ty: &syn::Ty) -> Result<Self, MacroError> {
        let ty = resolve_parens(ty);
        if let Some(int_ty) = as_simple_integer(ty) {
            return Ok(SupportedRetType::Integer(int_ty));
        }
        if let Some(item_ty) = as_vec_simple_integer_type(ty) {
            return Ok(SupportedRetType::IntegerVec(item_ty));
        }
        if let syn::Ty::Tup(ref items) = *ty {
            if items.is_empty() {
                return Ok(SupportedRetType::Unit);
            }
        }
        Err(MacroError::UnhandledRetType { ty: ty.clone() })?
    }

    pub fn unit() -> Self {
        SupportedRetType::Unit
    }
}

impl ToTokens for SupportedRetType {
    fn to_tokens(&self, tokens: &mut Tokens) {
        use SupportedRetType::*;
        match *self {
            IntegerVec(int_ty) => tokens.append(quote! { Vec<#int_ty> }),
            Integer(int_ty) => int_ty.to_tokens(tokens),
            Unit => tokens.append("()"),
        }
    }
}
