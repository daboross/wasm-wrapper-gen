Short example of using `wasm-wrapper-gen`
===

This is a shortened version of the project in `examples/simple_summation`.

`main.rs` declaring outputs:

```rust
#[macro_use]
extern crate wasm_wrapper_gen;

fn main() {} // required for target

fn sum(input: &[i32]) -> i32 {
    input.iter().cloned().sum()
}

// macro provided by wasm-wrapper-gen
js_fn! {
    fn sum(input: &[i32]) -> i32 => sum;
}
```

`build.rs` generating JS file:

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
