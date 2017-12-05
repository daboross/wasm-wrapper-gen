#[macro_use]
extern crate wasm_wrapper_gen;

js_fn! {
    fn count_booleans(input: &[bool]) -> i32 {
        input.iter().fold(0, |acc, &b| if b { acc + 1 } else { acc })
    }
    fn is_sum_even(input: &[i32]) -> bool {
        input.iter().fold(true, |acc, &v| {
            if v % 2 == 1 {
                !acc
            } else {
                acc
            }
        })
    }
    fn bits_within(input: u8) -> Vec<bool> {
        (0..8).rev().map(|bit| input & (1 << bit) != 0).collect()
    }
}
