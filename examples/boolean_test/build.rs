extern crate wasm_wrapper_gen_build;

fn main() {
    let result = wasm_wrapper_gen_build::translate_files(
        "src/main.rs",
        "target/wrapper.js",
        "SimpleSummation",
    );

    if let Err(e) = result {
        panic!("error: {}", e);
    }
}
