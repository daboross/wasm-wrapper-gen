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
fn fib_u64(nth: u32) -> u64 {
    let mut last = 0u64;
    let mut current = 1u64;
    for _ in 0..(nth as u64) {
        let temp = current.saturating_add(last);
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

fn fib_str(nth: u32) -> String {
    let result = fib_u64(nth);
    format!("{}", result)
}

js_fn! {
    fn fib(nth: u32) -> f64 => fib;
    fn all(num: u32) -> Vec<f64> => fib_all;
    fn fib_str(nth: u32) -> String => fib_str;
}
