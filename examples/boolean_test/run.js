#!/usr/bin/env node
const fs = require('fs');
const SimpleSummation = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/boolean_test.wasm");
    let module = new WebAssembly.Module(code);
    let instance = new SimpleSummation(module);

    let input1 = [true, false, true, true, false, false, true];
    console.log(`count_booleans(${input1}): ${instance.count_booleans(input1)}`);

    let input2 = [123312312, 14910241, 1231241290];
    console.log(`is_sum_even(${input2}): ${instance.is_sum_even(input2)}`);

    let input3 = 250;
    console.log(`bits_within(${input3}): ${instance.bits_within(input3)}`);
}

main();
