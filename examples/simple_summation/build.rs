extern crate failure;
extern crate wasm_wrapper_gen_build;

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), failure::Error> {
    wasm_wrapper_gen_build::Config::new()
        .with_class_name("SimpleSimmulation")
        .with_array_access_style(wasm_wrapper_gen_build::AccessStyle::TypedArrays)
        .translate("src/main.rs", "target/wrapper.js")?;

    wasm_wrapper_gen_build::Config::new()
        .with_class_name("SimpleSimmulation")
        .with_array_access_style(wasm_wrapper_gen_build::AccessStyle::DataView)
        .translate("src/main.rs", "target/wrapper_dataview.js")?;

    Ok(())
}
