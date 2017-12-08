wasm-wrapper-gen
================

JavaScript wrapper generation for rust code targeting wasm32-unknown-unknown.

I believe in "release early, release often", so `wasm-wrapper-gen` is available on crates.io as version 0.0.3.
That said, the project is still a work in progress, and breaking changes are to be expected.

---

The main idea:

```rust
fn fib_str(nth: u32) -> String {
    let mut last = 0;
    let mut current = 1u64;
    for _ in 0..(nth as u64) {
        let temp = current + last;
        last = current;
        current = temp;
    }
    format!("fibonacci sequence #{}: {}", nth, current)
}

js_fn! {
    fn fib_str(_: u32) -> String => fib_str;
}
```

```js
let module = new WebAssembly.Module(/*..*/);
let fib = new Fibonacci(module);

console.log(fib.fib_str(20));
```

There are multiple full example projects available in `examples/`, each which tests a different aspect of `wasm-wrapper-gen`. All of these should be directly copyable out of the repository as a starter if needed.

---

### Repository structure:

`wasm-wrapper-gen` is composed of two interlocking parts:
- `wasm-wrapper-gen` provides the `js_fn!()` macro which generates `extern "C"` functions
- `wasm-wrapper-gen-build` is a build-script utility which scrapes the source for usages of `js_fn!()` and generates a JavaScript class using those exported functions.

### Implementation notes:

- The default way to access memory is through a single pre-made DataView. This is efficient for small arrays/strings,
  but TypedArrays are also supported via a configuration option.
- Strings are always converted from utf16->utf8 and vice-versa inside Rust. This means all JavaScript ever does is
  `string.charCodeAt` and `String.fromCharCode`, but that all String arguments and return values require one extra
  allocation for a `Vec<u16>` separately from the `String` or `&'static str`

### Currently supported:

- Argument types:
  - `bool`, `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`, `f32`, `f64`
  - `&[_]`, `&mut [_]`, `Vec<_>` where `_` is any of the above
  - `String` (not `&str` because passing strings in always requires more allocation for utf16->utf8 in rust)
- Return types:
  - `bool`, `u8`, `u16`, `u32`, `usize`, `i8`, `i16`, `i32`, `isize`, `f32`, `f64`
  - `Vec<_>` where `_` is any of the above
  - `String` and `&'static str`
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
