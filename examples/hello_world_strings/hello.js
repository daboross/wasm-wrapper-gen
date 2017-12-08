#!/usr/bin/env node
const fs = require('fs');
const Hello = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/hello_world_strings.wasm");
    let module = new WebAssembly.Module(code);
    let hello = new Hello(module);

    console.log(hello.hello_world());
    console.log(hello.hello_x("everyone"));
}

main();
