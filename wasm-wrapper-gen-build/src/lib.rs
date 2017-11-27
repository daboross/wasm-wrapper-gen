#[macro_use]
extern crate failure;
extern crate quote;
extern crate syn;
extern crate wasm_wrapper_gen_shared;

mod source_searching;
mod js_generation;

use std::path::Path;
use std::io::{Read, Write};
use std::fs;

use failure::Error;

use wasm_wrapper_gen_shared::JsFnInfo;


// TODO: full Options struct for options, not just class name.
pub fn translate_files<P, U>(input_lib: P, output_file: U, js_class_name: &str) -> Result<(), Error>
where
    P: AsRef<Path>,
    U: AsRef<Path>,
{
    let contents = {
        let mut handle = fs::File::open(input_lib)?;

        let mut buffer = String::new();

        handle.read_to_string(&mut buffer)?;

        buffer
    };

    let output = translate_source(&contents, js_class_name)?;

    let mut handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)?;

    write!(handle, "{}", output)?;

    Ok(())
}

pub fn translate_source(source: &str, js_class_name: &str) -> Result<String, Error> {
    let func_definition_items = source_searching::walk_crate_for_js_fns(source)?;

    let js_fn_infos = func_definition_items
        .into_iter()
        .map(|item| JsFnInfo::try_from(&item))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(js_generation::generate_javascript(
        js_class_name,
        &js_fn_infos,
    )?)
}
