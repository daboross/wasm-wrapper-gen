#[macro_use]
extern crate proc_macro_hack;

#[allow(unused_imports)]
#[macro_use]
extern crate wasm_wrapper_gen_impl;
#[doc(hidden)]
pub use wasm_wrapper_gen_impl::*;

proc_macro_item_decl! {
    js_fn! => __js_fn_impl
}

mod extern_definitions {
    use std::mem;

    #[allow(non_snake_case)]
    unsafe extern "C" fn __js_fn___builtin_alloc(len: usize) -> *mut u8 {
        let memory = Vec::<u8>::with_capacity(len);

        let ptr = memory.as_slice().as_ptr() as *mut u8;

        mem::forget(memory);

        ptr
    }

    #[allow(non_snake_case)]
    unsafe extern "C" fn __js_fn__builtin_dealloc(ptr: *mut u8, len: usize) {
        if len == 0 {
            return;
        }
        assert!(ptr as usize != 0);

        Vec::<u8>::from_raw_parts(ptr, 0, len);
    }
}
