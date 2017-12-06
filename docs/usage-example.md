Short example of using `wasm-wrapper-gen`
===

This is a shortened version of the project in `examples/fibonacci`.

`lib.rs` declaring outputs:

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

js_fn! {
    fn fib(nth: u32) -> f64 => fib;
}
```

`build.rs` generating JS file:

```rust
extern crate wasm_wrapper_gen_build;

fn main() {
    wasm_wrapper_gen_build::Config::new()
        .with_class_name("Fibonacci")
        .translate("src/lib.rs", "target/wrapper.js")
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            ::std::process::exit(1);
        });
}
```

`Cargo.toml`:

```toml
[package]
name = "fibonacci"
version = "0.1.0"
authors = ["David Ross <daboross@daboross.net>"]

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-wrapper-gen = { version = "0.0.3", path = "../../" }

[build-dependencies]
wasm-wrapper-gen-build = { version = "0.0.3", path = "../../wasm-wrapper-gen-build/" }
```

And finally, usage from within node.js:

```js
#!/usr/bin/env node
const fs = require('fs');
const Fibonacci = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/fibonacci.wasm");
    let module = new WebAssembly.Module(code);
    let fib = new Fibonacci(module);

    console.log(`fib(0): ${fib.fib(0)}`);
    console.log(`fib(1): ${fib.fib(1)}`);
    console.log(`fib(2): ${fib.fib(2)}`);
    console.log(`fib(20): ${fib.fib(20)}`);
    console.log(`fib(200): ${fib.fib(200)}`);
}

main();
```
