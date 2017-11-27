use syn;

use MacroError;

pub enum KnownArgumentType {
    // &[u8]
    U8SliceRef,
    // &mut [u8]
    U8SliceMutRef,
    // TODO: we're just starting with one type, to get the basic infrastructure down.
    // // Vec<u8>
    // U8Vec,
    // // TODO: more
}

fn resolve_parens(mut ty: &syn::Ty) -> &syn::Ty {
    while let syn::Ty::Paren(ref temp) = *ty {
        ty = temp;
    }
    ty
}

impl KnownArgumentType {
    pub fn new(ty: &syn::Ty) -> Result<Self, MacroError> {
        let ty = resolve_parens(ty);
        if let syn::Ty::Rptr(_, ref slice_ty_mut) = *ty {
            let slice_ty = resolve_parens(&slice_ty_mut.ty);
            if let syn::Ty::Slice(ref byte_ty) = *slice_ty {
                let byte_ty = resolve_parens(byte_ty);
                if let syn::Ty::Path(None, ref path) = *byte_ty {
                    if path.segments
                        == &[
                            syn::PathSegment {
                                ident: syn::Ident::new("u8"),
                                parameters: syn::PathParameters::none(),
                            },
                        ] {
                        return Ok(match slice_ty_mut.mutability {
                            syn::Mutability::Immutable => KnownArgumentType::U8SliceRef,
                            syn::Mutability::Mutable => KnownArgumentType::U8SliceMutRef,
                        });
                    }
                }
            }
        }
        Err(MacroError::UnhandledArgumentType { ty: ty.clone() })?
    }
}
