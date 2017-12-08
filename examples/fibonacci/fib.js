#!/usr/bin/env node
const fs = require('fs');
const Fibonacci = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/fibonacci.wasm");
    let module = new WebAssembly.Module(code);
    let fib = new Fibonacci(module);

    console.log(`fib(0): ${fib.fib(0)}`);
    console.log(`fib(1): ${fib.fib(1)}`);
    console.log(`fib(2): ${fib.fib(2)}`);
    console.log(`fib(20): ${fib.fib(20)}`);
    console.log(`fib(200): ${fib.fib(200)}`);
    for (var i = 20; i < 200; i += 20) {
        console.log(`fib_u64_saturating(${i}) = ${fib.fib_str(i)}`);
    }

    console.log(`all numbers: ${fib.all(64)}`);
}

main();
