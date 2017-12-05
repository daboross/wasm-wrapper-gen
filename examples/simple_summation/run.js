#!/usr/bin/env node
const fs = require('fs');
const SimpleSummation1 = require('./target/wrapper.js');
const SimpleSummation2 = require('./target/wrapper_dataview.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/simple_summation.wasm");

    let module = new WebAssembly.Module(code);

    console.log("TypedArray style:");

    let instance1 = new SimpleSummation1(module);

    test(instance1);

    console.log("DataView style:");

    let instance2 = new SimpleSummation2(module);

    test(instance2);
}

function test(instance) {
    let input = [1, 2, 3, 4, 5];

    let output = instance.sum(input);
    console.log(`sum of ${input}: ${output}`);

    let in_place_input = [1, 2, 3, 4, 5];
    instance.product_in_place(in_place_input);
    console.log(`[1, 2, 3, 4, 5] * 2 in place: ${in_place_input}`);

    let new_output = instance.product_new([1, 2, 3, 4, 5]);
    console.log(`[1, 2, 3, 4, 5] * 2 created new: ${new_output}`);

    let float_product = instance.float_product([1.0, 1.2, 1.5, 1.6, 1.7]);
    console.log(`[1.0, 1.2, 1.5, 1.6, 1.7] product: ${float_product}`);
}

main();
