#[macro_use]
extern crate wasm_wrapper_gen;

/// main method is necessary, and can be empty.
fn main() {}

js_fn! {
    fn sum(input: &[u8], output: &mut [u8]) {
        let sum: u8 = input.iter().cloned().sum();
        output[0] = sum;
    }
}
