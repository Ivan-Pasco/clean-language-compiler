// Simple test runner for Clean Language WASM files
const fs = require('fs');

async function runWasm(wasmPath) {
    try {
        const wasmBuffer = fs.readFileSync(wasmPath);
        
        let wasmInstance = null;
        
        // Helper function to read string from WASM memory
        function readStringFromMemory(ptr) {
            if (!wasmInstance || !wasmInstance.exports.memory) {
                return `[ptr:${ptr}]`;
            }
            
            try {
                const memory = new Uint8Array(wasmInstance.exports.memory.buffer);
                
                // Read length from header (first 4 bytes, little-endian)
                const length = new DataView(wasmInstance.exports.memory.buffer).getUint32(ptr, true);
                
                if (length > 1000) { // Sanity check
                    return `[ptr:${ptr}, invalid length:${length}]`;
                }
                
                // Read string data starting at ptr + 4 (after length field)
                const stringBytes = memory.slice(ptr + 4, ptr + 4 + length);
                
                // Debug for conversion function pointers
                if (ptr === 232 || ptr === 460 || ptr === 688) {
                    console.log(`Debug ptr ${ptr}: length=${length}, content="${Array.from(stringBytes).map(b => String.fromCharCode(b)).join('')}"`);
                }
                
                // Convert to string
                const decoder = new TextDecoder('utf-8');
                return decoder.decode(stringBytes);
            } catch (error) {
                return `[ptr:${ptr}, error:${error.message}]`;
            }
        }
        
        // Create imports object with required functions
        const imports = {
            env: {
                print: (value) => {
                    const str = readStringFromMemory(value);
                    console.log('PRINT:', str);
                },
                printl: (value) => {
                    const str = readStringFromMemory(value);
                    console.log('PRINTL:', str);
                },
                input: (prompt) => { console.log('INPUT:', prompt); return 0; },
                input_integer: (prompt) => { console.log('INPUT_INTEGER:', prompt); return 42; },
                input_number: (prompt) => { console.log('INPUT_NUMBER:', prompt); return 3.14; },
                input_yesno: (prompt) => { console.log('INPUT_YESNO:', prompt); return 1; },
                file_write: () => 1,
                file_read: () => 0,
                file_exists: () => 1,
                file_delete: () => 1,
                file_append: () => 1,
                http_get: () => 0,
                http_post: () => 0,
                http_put: () => 0,
                http_patch: () => 0,
                http_delete: () => 0,
                http_head: () => 0,
                http_options: () => 0,
                http_get_with_headers: () => 0,
                http_post_with_headers: () => 0,
                http_post_json: () => 0,
                http_put_json: () => 0,
                http_patch_json: () => 0,
                http_post_form: () => 0,
                http_set_user_agent: () => {},
                http_set_timeout: () => {},
                http_set_max_redirects: () => {},
                http_enable_cookies: () => {},
                http_get_response_code: () => 200,
                http_get_response_headers: () => 0,
                http_encode_url: () => 0,
                http_decode_url: () => 0,
                http_build_query: () => 0,
            }
        };
        
        const wasmModule = await WebAssembly.instantiate(wasmBuffer, imports);
        wasmInstance = wasmModule.instance;
        
        console.log('WASM loaded successfully');
        console.log('Exports:', Object.keys(wasmModule.instance.exports));
        
        // Call the start function if it exists
        if (wasmModule.instance.exports.start) {
            console.log('Calling start function...');
            const result = wasmModule.instance.exports.start();
            console.log('Start function result:', result);
        } else {
            console.log('No start function found');
        }
        
    } catch (error) {
        console.error('Error running WASM:', error);
    }
}

// Run the test
const wasmPath = process.argv[2];
if (!wasmPath) {
    console.error('Usage: node test_runner.js <wasm_file>');
    process.exit(1);
}

runWasm(wasmPath);