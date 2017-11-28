wasm-wrapper-gen
================

JavaScript wrapper generation for rust code targeting wasm32-unknown-unknown.

This repository is currently very WIP, but there's a full working example Cargo project in `examples/simple_summation/`.

General overview:

`wasm-wrapper-gen` is composed of two interlocking parts:
- `wasm-wrapper-gen` provides the `js_fn!()` macro which generates `extern "C"` functions
- `wasm-wrapper-gen-build` is a build-script utility which scrapes the source for usages of `js_fn!()` and generates a JavaScript file which binds to those exported functions.

Note: this assumes little-endian hardware (the majority of modern hardware).

### Currently supported:

- Argument types:
  - `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`
  - `&[_]`, `&mut [_]`, `Vec<_>` wrapping any of the above
- Return types:
  - `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`
  - `Vec<_>` wrapping the above
- Full memory freeing for all allocated types as long as the rust
  function doesn't panic

### Unimplemented:

- Next to do:
  - Add options to build script JS generation
    - Add support for making an async constructor rather than sync one.
    - Add support for using DataView rather than (U)int*Array structures in order
      to allow for big-endian machines
  - Add support for 'bool' as a simple integer type and test boolean parameters, return types, and arrays

- Further future:
  - Make real tests and figure out how to do a build.rs script which only runs for tests
  - Arbitrary argument types implementing some serialization trait
  - Macro to wrap individual structs in separate JavaScript classes
    which all reference the same WebAssembly.Instance

### Example usage:

`main.rs`:

```rust

fn sum(input: &[i32]) -> i32 {
    input.iter().cloned().sum()
}

// macro provided by wasm-wrapper-gen
js_fn! {
    fn sum(input: &[i32]) -> i32 => sum;
}
```

`build.rs`:

```rust
extern crate wasm_wrapper_gen_build;

fn main() {
    let result = wasm_wrapper_gen_build::translate_files(
        "src/main.rs", // in file
        "target/wrapper.js", // out file
        "SimpleSummation",
    );

    if let Err(e) = result {
        panic!("error: {}", e);
    }
}
```

`Cargo.toml`:

```toml
[package]
name = "simple_summation"
version = "0.1.0"
authors = ["David Ross <daboross@daboross.net>"]

[dependencies]
wasm-wrapper-gen = "0.0.2"

[build-dependencies]
wasm-wrapper-gen-build = "0.0.2"
```

And finally, usage from within node.js:

```js
const fs = require('fs');
const SimpleSummation = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/simple_summation.wasm");

    let module = new WebAssembly.Module(code);

    let instance = new SimpleSummation(module);

    let input = [1, 2, 3, 4, 5];

    let output = instance.sum(input);

    console.log(`sum of ${input}: ${output}`);
}

main();
```
