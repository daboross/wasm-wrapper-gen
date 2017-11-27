wasm-wrapper-gen
================

JavaScript wrapper generation for rust code targeting wasm32-unknown-unknown.

This repository is currently very WIP, but there's a full working example Cargo project in `examples/simple_summation/`.

General overview:

`wasm-wrapper-gen` is composed of two interlocking parts:
- `wasm-wrapper-gen` provides the `js_fn!()` macro which generates `extern "C"` functions
- `wasm-wrapper-gen-build` is a build-script utility which scrapes the source for usages of `js_fn!()` and generates a JavaScript file which binds to those exported functions.


### Example usage:

`main.rs`:

```rust
// return values not yet supported
fn sum(input: &[u8], output: &mut [u8]) {
    let sum = input.iter().cloned().sum();
    output[0] = sum;
}

// macro provided by wasm-wrapper-gen
js_fn! {
    fn sum(_: &[u8], _: &mut [u8]) => sum;
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
wasm-wrapper-gen = "0.0.1"

[build-dependencies]
wasm-wrapper-gen-build = "0.0.1"
```

And finally, usage from within node.js:

```js
const fs = require('fs');
const SimpleSummation = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/simple_summation.wasm");

    let module = new WebAssembly.Module(code);

    let instance = new SimpleSummation(module);

    console.log(instance._mod.exports);

    let input = [1, 2, 3, 4, 5];
    // hack since we don't support return arguments natively yet.
    let output = [0];

    instance.sum(input, output);
    console.log(`sum of ${input}: ${output[0]}`);
}

main();
```
