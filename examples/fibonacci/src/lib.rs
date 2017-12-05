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

fn fib_all(final_nth: u32) -> Vec<f64> {
    let mut last = 0.0;
    let mut current = 1.0;
    (0..final_nth)
        .map(|_| {
            let temp = current + last;
            last = current;
            current = temp;
            current
        })
        .collect()
}

js_fn! {
    fn fib(nth: u32) -> f64 => fib;
    fn all(num: u32) -> Vec<f64> => fib_all;
}
