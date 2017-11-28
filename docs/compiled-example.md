Full example of compiled resources
===

This is a copy of the output from the project in `examples/simple_summation`.

Given the input file:

```rust
#[macro_use]
extern crate wasm_wrapper_gen;

/// main method is necessary, and can be empty.
fn main() {}

fn sum(input: &[u32]) -> i32 {
    let sum: i32 = input.iter().map(|&x| x as i32).sum();
    sum
}

fn product_in_place(input: &mut [u8]) {
    for item in input {
        *item *= 2;
    }
}

fn product_new(input: Vec<u8>) -> Vec<u8> {
    input.into_iter().map(|x| x * 2).collect()
}

js_fn! {
    fn sum(input: &[u32]) -> i32 => sum;
    fn product_in_place(input: &mut [u8]) => product_in_place;
    fn product_new(input: Vec<u8>) -> Vec<u8> => product_new;
}
```

The following output is generated (retrieved with `cargo-expand`):

```rust
#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std as std;
#[macro_use]
extern crate wasm_wrapper_gen;

/// main method is necessary, and can be empty.
fn main() {}

fn sum(input: &[u32]) -> i32 {
    let sum: i32 = input.iter().map(|&x| x as i32).sum();
    sum
}

fn product_in_place(input: &mut [u8]) {
    for item in input {
        *item *= 2;
    }
}

fn product_new(input: Vec<u8>) -> Vec<u8> {
    input.into_iter().map(|x| x * 2).collect()
}

#[allow(unused)]
enum ProcMacroHack {
    Input =
;",
         0).1,
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __js_fn_sum(__arg0_ptr: *const u32, __arg0_len: usize) -> i32 {
    let __arg0: &[u32] = unsafe { ::std::slice::from_raw_parts(__arg0_ptr, __arg0_len) };
    let result: i32 = ((sum))(__arg0);
    result
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __js_fn_product_in_place(__arg0_ptr: *mut u8, __arg0_len: usize) {
    let __arg0: &mut [u8] = unsafe { ::std::slice::from_raw_parts_mut(__arg0_ptr, __arg0_len) };
    let result: () = ((product_in_place))(__arg0);
    result
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __js_fn_product_new(__arg0_ptr: *mut u8, __arg0_len: usize) -> *const usize {
    let __arg0: Vec<u8> =
        unsafe { ::std::vec::Vec::from_raw_parts(__arg0_ptr, __arg0_len, __arg0_len) };
    let result: Vec<u8> = ((product_new))(__arg0);
    {
        let result_ptr = result.as_slice().as_ptr() as *mut u8;
        let result_len = result.len();
        let result_cap = result.capacity();
        let to_return = Box::new([result_ptr as usize, result_len, result_cap]);
        ::std::mem::forget(result);
        ::std::boxed::Box::into_raw(to_return) as *const usize
    }
}
```

And the following JS is generated:

```js
class SimpleSummation {
    constructor(wasm_module) {
        this._mod = new WebAssembly.Instance(wasm_module, {
            // TODO: imports
        });
        this._mem = new Uint8Array(this._mod.exports["memory"].buffer);

        this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
        this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];

        this._funcs = {
            ['sum']: this._mod.exports["__js_fn_sum"],
            ['product_in_place']: this._mod.exports["__js_fn_product_in_place"],
            ['product_new']: this._mod.exports["__js_fn_product_new"],
        };
    }

    sum(arg0) {
        if (arg0 == null || isNaN(arg0.length)) {
            throw new Error();
        }
        let arg0_len = arg0.length;
        let arg0_byte_len = arg0_len * 4;
        let arg0_ptr = this._alloc(arg0_byte_len);
        let arg0_view = new Uint32Array(this._mem.buffer, arg0_ptr, arg0_byte_len);
        arg0_view.set(arg0);
        let result = this._funcs['sum'](arg0_ptr, arg0_len);
        this._dealloc(arg0_ptr, arg0_byte_len);
        return result;
    }

    product_in_place(arg0) {
        if (arg0 == null || isNaN(arg0.length)) {
            throw new Error();
        }
        let arg0_len = arg0.length;
        let arg0_ptr = this._alloc(arg0_len);
        let arg0_view = this._mem.subarray(arg0_ptr, arg0_ptr + arg0_len);
        arg0_view.set(arg0);
        let result = this._funcs['product_in_place'](arg0_ptr, arg0_len);
        if (typeof arg0.set == 'function') {
            arg0.set(arg0_view);
        } else {
            for (var i = 0; i < arg0_len; i++) {
                arg0[i] = arg0_view[i];
            }
        }
        this._dealloc(arg0_ptr, arg0_len);
        return;
    }

    product_new(arg0) {
        if (arg0 == null || isNaN(arg0.length)) {
            throw new Error();
        }
        let arg0_len = arg0.length;
        let arg0_ptr = this._alloc(arg0_len);
        let arg0_view = this._mem.subarray(arg0_ptr, arg0_ptr + arg0_len);
        arg0_view.set(arg0);
        let result = this._funcs['product_new'](arg0_ptr, arg0_len);
        let result_temp_ptr = result;
        let result_temp_len = 3;
        let result_temp_byte_len = result_temp_len * 4;
        let result_temp_view = new Uint32Array(this._mem.buffer, result, result_temp_byte_len);
        let return_ptr = result_temp_view[0];
        let return_len = result_temp_view[1];
        let return_cap = result_temp_view[2];
        let return_byte_len = return_len * 1;
        let return_byte_cap = return_cap * 1;
        let return_value_copy = Uint8Array.from(new Uint8Array(this._mem.buffer, return_ptr, return_byte_len));
        this._dealloc(return_ptr, return_byte_cap);
        this._dealloc(result_temp_ptr, result_temp_byte_len);
        return return_value_copy;
    }
}

exports = module.exports = SimpleSummation
```
