use std::fmt;

use quote::{ToTokens, Tokens};

use syn;

use MacroError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SupportedCopyTy {
    U8,
    U16,
    U32,
    USize,
    I8,
    I16,
    I32,
    ISize,
    F32,
    F64,
    Bool,
}

impl SupportedCopyTy {
    pub fn new<T: AsRef<str>>(ident: &T) -> Option<Self> {
        use self::SupportedCopyTy::*;
        match ident.as_ref() {
            "u8" => Some(U8),
            "u16" => Some(U16),
            "u32" => Some(U32),
            "usize" => Some(USize),
            "i8" => Some(I8),
            "i16" => Some(I16),
            "i32" => Some(I32),
            "isize" => Some(ISize),
            "f32" => Some(F32),
            "f64" => Some(F64),
            "bool" => Some(Bool),
            _ => None,
        }
    }

    pub fn size_in_bytes(&self) -> usize {
        use self::SupportedCopyTy::*;
        // not using `std::mem::size_of` since that's for the current platform, not wasm.
        match *self {
            Bool | U8 => 1,
            U16 => 2,
            USize | U32 => 4,
            I8 => 1,
            I16 => 2,
            ISize | I32 => 4,
            F32 => 4,
            F64 => 8,
        }
    }
}

impl AsRef<str> for SupportedCopyTy {
    fn as_ref(&self) -> &str {
        use self::SupportedCopyTy::*;
        match *self {
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            USize => "usize",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            ISize => "isize",
            F32 => "f32",
            F64 => "f64",
            Bool => "bool",
        }
    }
}

impl fmt::Display for SupportedCopyTy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl ToTokens for SupportedCopyTy {
    fn to_tokens(&self, tokens: &mut Tokens) {
        tokens.append(self.as_ref());
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SupportedArgumentType {
    // &[u8]
    IntegerSliceRef(SupportedCopyTy),
    // &mut [u8]
    IntegerSliceMutRef(SupportedCopyTy),
    // Vec<u8>
    IntegerVec(SupportedCopyTy),
    // u8, u16, u32, u64, i8, i16, i32, i64, usize, isize,
    Integer(SupportedCopyTy),
    // String
    OwnedString,
    // String slice is not supported due to string always needing allocation
    // to convert from JavaScript to rust.
    // TODO: wtf-8 or utf16 type.
    // TODO: more types, and more nesting.
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

fn as_simple_integer(ty: &syn::Ty) -> Option<SupportedCopyTy> {
    let ty = resolve_parens(ty);
    if let syn::Ty::Path(None, ref path) = *ty {
        if path.segments.len() <= 1 {
            if let Some(segment) = path.segments.first() {
                if segment.parameters == syn::PathParameters::none() {
                    return SupportedCopyTy::new(&segment.ident);
                }
            }
        }
    }
    None
}

fn path_as_single_segment(ty: &syn::Ty) -> Option<&syn::PathSegment> {
    let ty = resolve_parens(ty);
    match *ty {
        syn::Ty::Path(None, ref path) => match (path.segments.len(), path.segments.first()) {
            (1, Some(segment)) => Some(segment),
            _ => None,
        },
        _ => None,
    }
}

fn as_vec_simple_integer_type(ty: &syn::Ty) -> Option<SupportedCopyTy> {
    let ty = resolve_parens(ty);
    if let Some(segment) = path_as_single_segment(ty) {
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
    None
}

fn is_string_slice(ty: &syn::Ty) -> bool {
    let ty = resolve_parens(ty);
    if let syn::Ty::Rptr(_, ref str_ty_with_mut) = *ty {
        if str_ty_with_mut.mutability == syn::Mutability::Immutable {
            let should_be_str = resolve_parens(&str_ty_with_mut.ty);
            if let Some(segment) = path_as_single_segment(should_be_str) {
                if segment.ident == "str" && segment.parameters.is_empty() {
                    return true;
                }
            }
        }
    }

    false
}

fn is_owned_string(ty: &syn::Ty) -> bool {
    if let Some(segment) = path_as_single_segment(ty) {
        if segment.ident == "String" && segment.parameters.is_empty() {
            return true;
        }
    }

    false
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
        // if is_string_slice(ty) {
        //     return Ok(SupportedArgumentType::StringSlice);
        // }
        if is_owned_string(ty) {
            return Ok(SupportedArgumentType::OwnedString);
        }
        Err(MacroError::UnhandledArgumentType { ty: ty.clone() })?
    }
}

pub enum SupportedRetType {
    // Vec<u8>
    IntegerVec(SupportedCopyTy),
    // u8, u16, u32, u64, i8, i16, i32, i64, usize, isize,
    Integer(SupportedCopyTy),
    // ()
    Unit,
    // &str
    StringSlice,
    // String
    OwnedString,
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
        if is_string_slice(ty) {
            return Ok(SupportedRetType::StringSlice);
        }
        if is_owned_string(ty) {
            return Ok(SupportedRetType::OwnedString);
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
            // TODO: handle reference lifetime
            StringSlice => tokens.append("&str"),
            OwnedString => tokens.append("String"),
        }
    }
}
