#!/usr/bin/env node
const fs = require('fs');
const SimpleSummation = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/simple_summation.wasm");

    let module = new WebAssembly.Module(code);

    let instance = new SimpleSummation(module);

    let input = [1, 2, 3, 4, 5];

    let output = instance.sum(input);
    console.log(`sum of ${input}: ${output}`);

    let in_place_input = [1, 2, 3, 4, 5];
    instance.product_in_place(in_place_input);
    console.log(`[1, 2, 3, 4, 5] * 2 in place: ${in_place_input}`);

    let new_output = instance.product_new([1, 2, 3, 4, 5]);
    console.log(`[1, 2, 3, 4, 5] * 2 created new: ${new_output}`);
}

main();
