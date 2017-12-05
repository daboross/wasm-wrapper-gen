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
  - `bool`, `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`, `f32`, `f64`
  - `&[_]`, `&mut [_]`, `Vec<_>` where `_` is any of the above
- Return types:
  - `bool`, `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`, `f32`, `f64`
  - `Vec<_>` where `_` is any of the above
- Full automatic memory management and freeing unless rust function panics
- Configuration to use either a single DataView or a TypedArray instance per argument
  to access arrays
- Configurable output JS indentation

### Unimplemented:

- Next to do:
  - Add support for making an async constructor rather than sync one.
  - Add support for `impl` blocks with `self` arguments and creating wrapper JS types
    which manage allocation sanely.

- Further future:
  - Make real tests and figure out how to do a build.rs script which only runs for tests
  - Arbitrary argument types implementing some serialization trait
  - Macro to wrap individual structs in separate JavaScript classes
    which all reference the same WebAssembly.Instance

### Links:

- [Full example of usage](docs/usage-example.md)
- [Full example of generated code](docs/compiled-example.md)
