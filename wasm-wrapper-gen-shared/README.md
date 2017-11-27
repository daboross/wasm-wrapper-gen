wasm-wrapper-gen-shared
======================

wasm-wrapper-gen: JavaScript-wrapper generation for rust code targetting wasm32-unknown-unknown.

`wasm-wrapper-gen` is composed of two parts: a procedural macro to generate `extern "C"` functions with appropriate
parameters, and a build script which scrapes the source for instances of that macro and generates JavaScript bindings
calling those methods.

`wasm-wrapper-gen-shared` is a set of utility functions which are shared between the procedural macro crate and the
build script crate.

This crate is not meant for direct use.

See `wasm-wrapper-gen` for more information:
- [`wasm-wrapper-gen` on crates.io](https://crates.io/crate/wasm-wrapper-gen/)
- [`wasm-wrapper-gen` on github](https://github.com/daboross/wasm-wrapper-gen)
