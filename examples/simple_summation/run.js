const fs = require('fs');
const SimpleSummation = require('./target/wrapper.js');

function main() {
    let code = fs.readFileSync("target/wasm32-unknown-unknown/release/simple_summation.wasm");

    let module = new WebAssembly.Module(code);

    let instance = new SimpleSummation(module);

    console.log(instance._mod.exports);

    let input = [1, 2, 3, 4, 5];
    // hack since we don't support return arguments natively yet.
    let output = [0];

    instance.sum(input, output);
    console.log(`sum of ${input}: ${output[0]}`);
}

main();
