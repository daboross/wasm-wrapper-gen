#[macro_use]
extern crate wasm_wrapper_gen;

fn sum(input: &[u32]) -> i32 {
    let sum: i32 = input.iter().map(|&x| x as i32).sum();
    sum
}

fn product_in_place(input: &mut [u8]) {
    for item in input {
        *item *= 2;
    }
}

fn product_new(input: Vec<u8>) -> Vec<u8> {
    input.into_iter().map(|x| x * 2).collect()
}

js_fn! {
    fn sum(input: &[u32]) -> i32 => sum;
    fn product_in_place(input: &mut [u8]) => product_in_place;
    fn product_new(input: Vec<u8>) -> Vec<u8> => product_new;
    fn float_product(input: Vec<f64>) -> f64 {
        input.into_iter().fold(1.0, |a, b| a * b)
    }
}
