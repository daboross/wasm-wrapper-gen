Full example of compiled resources
===

This is a copy of the output from the project in `examples/fibonacci`.

Given the input file:

```rust
#[macro_use]
extern crate wasm_wrapper_gen;

fn fib(nth: u32) -> f64 {
    let mut last = 0.0;
    let mut current = 1.0;
    for _ in 0..nth {
        let temp = current + last;
        last = current;
        current = temp;
    }
    current
}

fn fib_all(final_nth: u32) -> Vec<f64> {
    let mut last = 0.0;
    let mut current = 1.0;
    (0..final_nth).map(|_| {
        let temp = current + last;
        last = current;
        current = temp;
        current
    }).collect()
}

js_fn! {
    fn fib(nth: u32) -> f64 => fib;
    fn all(num: u32) -> Vec<f64> => fib_all;
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

fn fib(nth: u32) -> f64 {
    let mut last = 0.0;
    let mut current = 1.0;
    for _ in 0..nth {
        let temp = current + last;
        last = current;
        current = temp;
    }
    current
}

fn fib_all(final_nth: u32) -> Vec<f64> {
    let mut last = 0.0;
    let mut current = 1.0;
    (0..final_nth)
        .map(|_| {
            let temp = current + last;
            last = current;
            current = temp;
            current
        })
        .collect()
}

#[allow(unused)]
enum ProcMacroHack {
    Input = (
        "fn fib ( nth : u32 ) -> f64 => fib ; fn all ( num : u32 ) -> Vec < f64 > =>\nfib_all ;",
        0,
    ).1,
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __js_fn_fib(__arg0: u32) -> f64 {
    let result: f64 = ((fib))(__arg0);
    result
}
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __js_fn_all(__arg0: u32) -> *const usize {
    let result: Vec<f64> = ((fib_all))(__arg0);
    {
        let result_ptr = result.as_slice().as_ptr() as *mut f64;
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
class Fibonacci {
    constructor (wasm_module) {
        this._mod = new WebAssembly.Instance(wasm_module, {});
        this._mem = new DataView(this._mod.exports["memory"].buffer);

        this._alloc = this._mod.exports["__js_fn__builtin_alloc"];
        this._dealloc = this._mod.exports["__js_fn__builtin_dealloc"];

        this._funcs = {
            ['fib']: this._mod.exports["__js_fn_fib"],
            ['all']: this._mod.exports["__js_fn_all"],
        };
    }

    fib(arg0) {
        if (isNaN(arg0)) {
            throw new Error();
        }
        let result = this._funcs['fib'](arg0);
        return result;
    }

    all(arg0) {
        if (isNaN(arg0)) {
            throw new Error();
        }
        let result = this._funcs['all'](arg0);
        let result_temp_ptr = result;
        let return_ptr = this._mem.getUint32(result_temp_ptr, true);
        let return_len = this._mem.getUint32(result_temp_ptr + 4, true);
        let return_cap = this._mem.getUint32(result_temp_ptr + 8, true);
        let return_byte_len = return_len * 8;
        let return_byte_cap = return_cap * 8;
        let return_value_copy = [];
        for (var ret_i = 0; ret_i < return_len; ret_i++) {
            return_value_copy.push(this._mem.getFloat64(return_ptr + 8 * ret_i, true));
        }
        this._dealloc(return_ptr, return_byte_cap);
        this._dealloc(result_temp_ptr, 12);
        return return_value_copy;
    }
}

exports = module.exports = Fibonacci;
```
