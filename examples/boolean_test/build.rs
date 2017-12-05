extern crate wasm_wrapper_gen_build;

fn main() {
    let result = wasm_wrapper_gen_build::Config::new()
        .with_class_name("SimpleSimmulation")
        .translate("src/main.rs", "target/wrapper.js");
    if let Err(e) = result {
        eprintln!("error: {}", e);
        ::std::process::exit(1);
    }
}
