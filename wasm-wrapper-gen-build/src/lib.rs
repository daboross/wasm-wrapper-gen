#[macro_use]
extern crate failure;
extern crate syn;
extern crate wasm_wrapper_gen_shared;

mod source_searching;
mod generation;
mod style;

use std::path::Path;
use std::io::{Read, Write};
use std::fs;

use failure::Error;

use wasm_wrapper_gen_shared::JsFnInfo;

pub use style::{AccessStyle, Config};

impl<'a> Config<'a> {
    pub fn translate<P, U>(&self, input_file: P, output_file: U) -> Result<(), Error>
    where
        P: AsRef<Path>,
        U: AsRef<Path>,
    {
        translate_files(input_file, output_file, self)
    }
}


// TODO: full Options struct for options, not just class name.
fn translate_files<P, U>(input_lib: P, output_file: U, config: &Config) -> Result<(), Error>
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

    let output = translate_source(&contents, config)?;

    let mut handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)?;

    write!(handle, "{}", output)?;

    Ok(())
}

fn translate_source(source: &str, config: &Config) -> Result<String, Error> {
    let func_definition_items = source_searching::walk_crate_for_js_fns(source)?;

    let js_fn_infos = func_definition_items
        .into_iter()
        .map(|item| JsFnInfo::try_from(&item))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(generation::generate_javascript(config, &js_fn_infos)?)
}
