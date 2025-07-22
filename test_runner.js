const fs = require('fs');
const path = require('path');

if (process.argv.length < 3) {
    console.log('Usage: node test_runner.js <wasm_file>');
    process.exit(1);
}

const wasmFile = process.argv[2];

// Host environment providing required imports
const hostEnv = {
    env: {
        print: (ptr) => {
            console.log('PRINT:', readStringFromMemory(ptr));
        },
        printl: (ptr) => {
            console.log('PRINTL:', readStringFromMemory(ptr));
        },
        input: () => {
            console.log('INPUT called');
            return 0;
        },
        input_integer: () => {
            console.log('INPUT_INTEGER called');
            return 42;
        },
        input_number: () => {
            console.log('INPUT_NUMBER called');
            return 3.14;
        },
        input_yesno: () => {
            console.log('INPUT_YESNO called');
            return 1;
        },
        file_write: () => { console.log('FILE_WRITE called'); return 0; },
        file_read: () => { console.log('FILE_READ called'); return 0; },
        file_exists: () => { console.log('FILE_EXISTS called'); return 0; },
        file_delete: () => { console.log('FILE_DELETE called'); return 0; },
        file_append: () => { console.log('FILE_APPEND called'); return 0; },
        http_get: () => { console.log('HTTP_GET called'); return 0; },
        http_post: () => { console.log('HTTP_POST called'); return 0; },
        http_put: () => { console.log('HTTP_PUT called'); return 0; },
        http_patch: () => { console.log('HTTP_PATCH called'); return 0; },
        http_delete: () => { console.log('HTTP_DELETE called'); return 0; },
        http_head: () => { console.log('HTTP_HEAD called'); return 0; },
        http_options: () => { console.log('HTTP_OPTIONS called'); return 0; },
        http_get_with_headers: () => { console.log('HTTP_GET_WITH_HEADERS called'); return 0; },
        http_post_with_headers: () => { console.log('HTTP_POST_WITH_HEADERS called'); return 0; },
        http_post_json: () => { console.log('HTTP_POST_JSON called'); return 0; },
        http_put_json: () => { console.log('HTTP_PUT_JSON called'); return 0; },
        http_patch_json: () => { console.log('HTTP_PATCH_JSON called'); return 0; },
        http_post_form: () => { console.log('HTTP_POST_FORM called'); return 0; },
        http_set_user_agent: () => { console.log('HTTP_SET_USER_AGENT called'); return 0; },
        http_set_timeout: () => { console.log('HTTP_SET_TIMEOUT called'); return 0; },
        http_set_max_redirects: () => { console.log('HTTP_SET_MAX_REDIRECTS called'); return 0; },
        http_enable_cookies: () => { console.log('HTTP_ENABLE_COOKIES called'); return 0; },
        http_get_response_code: () => { console.log('HTTP_GET_RESPONSE_CODE called'); return 0; },
        http_get_response_headers: () => { console.log('HTTP_GET_RESPONSE_HEADERS called'); return 0; },
        http_encode_url: () => { console.log('HTTP_ENCODE_URL called'); return 0; },
        http_decode_url: () => { console.log('HTTP_DECODE_URL called'); return 0; },
        http_build_query: () => { console.log('HTTP_BUILD_QUERY called'); return 0; }
    }
};

let wasmInstance;

function readStringFromMemory(ptr) {
    try {
        if (!wasmInstance || !wasmInstance.exports.memory) {
            return `[ptr:${ptr}]`;
        }
        
        const memory = new Uint8Array(wasmInstance.exports.memory.buffer);
        const length = new DataView(wasmInstance.exports.memory.buffer).getUint32(ptr, true);
        
        if (length > 1000) {
            return `[ptr:${ptr},len:${length}]`;
        }
        
        const stringBytes = memory.slice(ptr + 4, ptr + 4 + length);
        const decoder = new TextDecoder('utf-8');
        return decoder.decode(stringBytes);
    } catch (e) {
        return `[ptr:${ptr},error:${e.message}]`;
    }
}

async function runWasm() {
    try {
        console.log(`Running WASM file: ${wasmFile}`);
        
        const wasmBytes = fs.readFileSync(wasmFile);
        const wasmModule = await WebAssembly.compile(wasmBytes);
        wasmInstance = await WebAssembly.instantiate(wasmModule, hostEnv);
        
        console.log('WASM instantiated successfully');
        
        if (wasmInstance.exports.start) {
            console.log('Calling start function...');
            const result = wasmInstance.exports.start();
            console.log('Start function completed, result:', result);
        } else {
            console.log('No start function found');
        }
        
        console.log('WASM execution completed successfully');
        
    } catch (error) {
        console.error('Error running WASM:', error.message);
        if (error.stack) {
            console.error(error.stack);
        }
        process.exit(1);
    }
}

runWasm();