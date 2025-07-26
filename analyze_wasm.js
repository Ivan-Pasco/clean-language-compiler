const fs = require('fs');

async function analyzeWasm() {
    try {
        const wasmBuffer = fs.readFileSync('test_output.wasm');
        console.log('WASM file size:', wasmBuffer.length, 'bytes');
        
        // Try to instantiate the module
        const wasmModule = await WebAssembly.compile(wasmBuffer);
        console.log('Module compiled successfully');
        
        // Get imports and exports
        const imports = WebAssembly.Module.imports(wasmModule);
        const exports = WebAssembly.Module.exports(wasmModule);
        
        console.log('\n=== IMPORTS ===');
        imports.forEach((imp, i) => {
            console.log(`${i}: ${imp.module}.${imp.name} (${imp.kind})`);
        });
        
        console.log('\n=== EXPORTS ===');
        exports.forEach((exp, i) => {
            console.log(`${i}: ${exp.name} (${exp.kind})`);
        });
        
        // Try to instantiate with minimal imports
        console.log('\n=== TRYING TO INSTANTIATE ===');
        const instance = await WebAssembly.instantiate(wasmModule, {
            env: {
                memory: new WebAssembly.Memory({ initial: 256 }),
                // Add minimal required functions
                print: (arg) => console.log('print:', arg),
                debug: (arg) => console.log('debug:', arg),
            }
        });
        
        console.log('Instance created successfully');
        console.log('Available functions:', Object.keys(instance.exports));
        
    } catch (error) {
        console.error('Error:', error.message);
        if (error.stack) {
            console.error('Stack:', error.stack);
        }
    }
}

analyzeWasm();