extern crate wasm_wrapper_gen_build;

fn main() {
    wasm_wrapper_gen_build::Config::new()
        .with_class_name("Fibonacci")
        .translate("src/main.rs", "target/wrapper.js")
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            ::std::process::exit(1);
        });
}

