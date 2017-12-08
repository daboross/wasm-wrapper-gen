#[macro_use]
extern crate wasm_wrapper_gen;

fn hello_world() -> &'static str {
    "Hello, world!"
}

fn hello_x(user: String) -> String {
    format!("Hello, {}!", user)
}

js_fn! {
    fn hello_world() -> &str => hello_world;
    fn hello_x(_: String) -> String => hello_x;
}
