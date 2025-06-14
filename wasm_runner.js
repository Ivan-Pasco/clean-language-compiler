#!/usr/bin/env node

const fs = require('fs');
const https = require('https');
const http = require('http');

/**
 * Direct WebAssembly Runner for Clean Language
 * Loads and executes .wasm files compiled from Clean Language
 */

class WasmRunner {
    constructor() {
        this.memory = null;
        this.stringPool = [];
        this.textDecoder = new TextDecoder();
        this.textEncoder = new TextEncoder();
    }

    // HTTP client implementation for WebAssembly imports
    async httpGet(url) {
        return new Promise((resolve, reject) => {
            const client = url.startsWith('https:') ? https : http;
            client.get(url, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => resolve(data));
            }).on('error', reject);
        });
    }

    async httpRequest(method, url, body = '') {
        return new Promise((resolve, reject) => {
            const urlObj = new URL(url);
            const client = urlObj.protocol === 'https:' ? https : http;
            
            const options = {
                hostname: urlObj.hostname,
                port: urlObj.port,
                path: urlObj.pathname + urlObj.search,
                method: method,
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                    'Content-Length': Buffer.byteLength(body),
                    'User-Agent': 'Clean-Language-WASM/1.0'
                }
            };
            
            const req = client.request(options, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => resolve(data));
            });
            
            req.on('error', reject);
            if (body) req.write(body);
            req.end();
        });
    }

    // Memory management helpers
    getStringFromMemory(ptr, len) {
        if (!this.memory) {
            console.log(`[String at ${ptr}, length ${len}]`);
            return `[String at ${ptr}, length ${len}]`;
        }
        
        const bytes = new Uint8Array(this.memory.buffer, ptr, len);
        return this.textDecoder.decode(bytes);
    }

    allocateString(str) {
        // Simplified string allocation - in real implementation would use WASM heap
        const index = this.stringPool.length;
        this.stringPool.push(str);
        return index;
    }

    // WebAssembly import object
    getImports() {
        return {
            env: {
                // Memory (will be set by WASM module)
                memory: new WebAssembly.Memory({ initial: 1 }),
                
                // Print functions - matching compiler expectations
                print: (ptr, len) => {
                    const str = this.getStringFromMemory(ptr, len);
                    process.stdout.write(str);
                },
                
                printl: (ptr, len) => {
                    const str = this.getStringFromMemory(ptr, len);
                    console.log(str);
                },

                // Simplified print functions that take just a value
                print_simple: (value) => {
                    process.stdout.write(value.toString());
                },
                
                printl_simple: (value) => {
                    console.log(value.toString());
                },

                // HTTP client functions
                http_get: async (urlPtr, urlLen) => {
                    const url = this.getStringFromMemory(urlPtr, urlLen);
                    console.log(`🌐 HTTP GET: ${url}`);
                    try {
                        const response = await fetch(url);
                        const responseText = await response.text();
                        console.log(`✅ HTTP GET Response: ${responseText.substring(0, 200)}...`);
                        return this.allocateString(responseText);
                    } catch (error) {
                        console.error(`❌ HTTP GET Error: ${error.message}`);
                        return this.allocateString(`Error: ${error.message}`);
                    }
                },
                
                http_post: async (urlPtr, urlLen, bodyPtr, bodyLen) => {
                    const url = this.getStringFromMemory(urlPtr, urlLen);
                    const body = this.getStringFromMemory(bodyPtr, bodyLen);
                    console.log(`🌐 HTTP POST: ${url} with body: ${body}`);
                    try {
                        const response = await fetch(url, {
                            method: 'POST',
                            headers: {
                                'Content-Type': 'application/json',
                            },
                            body: body
                        });
                        const responseText = await response.text();
                        console.log(`✅ HTTP POST Response: ${responseText.substring(0, 200)}...`);
                        return this.allocateString(responseText);
                    } catch (error) {
                        console.error(`❌ HTTP POST Error: ${error.message}`);
                        return this.allocateString(`Error: ${error.message}`);
                    }
                },
                
                http_put: async (urlPtr, urlLen, bodyPtr, bodyLen) => {
                    const url = this.getStringFromMemory(urlPtr, urlLen);
                    const body = this.getStringFromMemory(bodyPtr, bodyLen);
                    console.log(`🌐 HTTP PUT: ${url} with body: ${body}`);
                    try {
                        const response = await fetch(url, {
                            method: 'PUT',
                            headers: {
                                'Content-Type': 'application/json',
                            },
                            body: body
                        });
                        const responseText = await response.text();
                        console.log(`✅ HTTP PUT Response: ${responseText.substring(0, 200)}...`);
                        return this.allocateString(responseText);
                    } catch (error) {
                        console.error(`❌ HTTP PUT Error: ${error.message}`);
                        return this.allocateString(`Error: ${error.message}`);
                    }
                },
                
                http_delete: async (urlPtr, urlLen) => {
                    const url = this.getStringFromMemory(urlPtr, urlLen);
                    console.log(`🌐 HTTP DELETE: ${url}`);
                    try {
                        const response = await fetch(url, {
                            method: 'DELETE'
                        });
                        const responseText = await response.text();
                        console.log(`✅ HTTP DELETE Response: ${responseText.substring(0, 200)}...`);
                        return this.allocateString(responseText);
                    } catch (error) {
                        console.error(`❌ HTTP DELETE Error: ${error.message}`);
                        return this.allocateString(`Error: ${error.message}`);
                    }
                },
                
                http_patch: async (urlPtr, urlLen, bodyPtr, bodyLen) => {
                    const url = this.getStringFromMemory(urlPtr, urlLen);
                    const body = this.getStringFromMemory(bodyPtr, bodyLen);
                    console.log(`🌐 HTTP PATCH: ${url} with body: ${body}`);
                    try {
                        const response = await fetch(url, {
                            method: 'PATCH',
                            headers: {
                                'Content-Type': 'application/json',
                            },
                            body: body
                        });
                        const responseText = await response.text();
                        console.log(`✅ HTTP PATCH Response: ${responseText.substring(0, 200)}...`);
                        return this.allocateString(responseText);
                    } catch (error) {
                        console.error(`❌ HTTP PATCH Error: ${error.message}`);
                        return this.allocateString(`Error: ${error.message}`);
                    }
                },

                // File system functions
                file_write: (pathPtr, pathLen, contentPtr, contentLen) => {
                    const path = this.getStringFromMemory(pathPtr, pathLen);
                    const content = this.getStringFromMemory(contentPtr, contentLen);
                    console.log(`📝 File write: ${path}`);
                    try {
                        fs.writeFileSync(path, content);
                        return 0; // Success
                    } catch (error) {
                        console.error(`❌ File write error: ${error.message}`);
                        return -1; // Error
                    }
                },
                
                file_read: (pathPtr, pathLen, resultPtr) => {
                    const path = this.getStringFromMemory(pathPtr, pathLen);
                    console.log(`📖 File read: ${path}`);
                    try {
                        const content = fs.readFileSync(path, 'utf8');
                        // In a real implementation, would write to resultPtr
                        // For now, just return the length
                        return content.length;
                    } catch (error) {
                        console.error(`❌ File read error: ${error.message}`);
                        return -1; // Error
                    }
                },
                
                file_exists: (pathPtr, pathLen) => {
                    const path = this.getStringFromMemory(pathPtr, pathLen);
                    console.log(`🔍 File exists check: ${path}`);
                    return fs.existsSync(path) ? 1 : 0;
                },
                
                file_delete: (pathPtr, pathLen) => {
                    const path = this.getStringFromMemory(pathPtr, pathLen);
                    console.log(`🗑️ File delete: ${path}`);
                    try {
                        fs.unlinkSync(path);
                        return 0; // Success
                    } catch (error) {
                        console.error(`❌ File delete error: ${error.message}`);
                        return -1; // Error
                    }
                },
                
                file_append: (pathPtr, pathLen, contentPtr, contentLen) => {
                    const path = this.getStringFromMemory(pathPtr, pathLen);
                    const content = this.getStringFromMemory(contentPtr, contentLen);
                    console.log(`📝 File append: ${path}`);
                    try {
                        fs.appendFileSync(path, content);
                        return 0; // Success
                    } catch (error) {
                        console.error(`❌ File append error: ${error.message}`);
                        return -1; // Error
                    }
                }
            }
        };
    }

    // Load and run a WebAssembly file
    async runWasm(filename) {
        try {
            console.log(`🚀 Loading WebAssembly file: ${filename}`);
            
            if (!fs.existsSync(filename)) {
                throw new Error(`File not found: ${filename}`);
            }

            const wasmBuffer = fs.readFileSync(filename);
            console.log(`📦 File size: ${wasmBuffer.length} bytes`);

            const imports = this.getImports();
            const wasmModule = await WebAssembly.instantiate(wasmBuffer, imports);
            
            // Set memory reference
            this.memory = imports.env.memory;
            
            console.log(`✅ WebAssembly module loaded successfully`);
            console.log(`📋 Exported functions:`, Object.keys(wasmModule.instance.exports));
            
            // Run the start function if it exists
            if (wasmModule.instance.exports.start) {
                console.log(`🎯 Executing start function...`);
                console.log(`--- Output ---`);
                const result = wasmModule.instance.exports.start();
                console.log(`--- End Output ---`);
                console.log(`✅ Execution completed with result: ${result}`);
            } else {
                console.log(`ℹ️  No start function found in WebAssembly module`);
            }

            return wasmModule.instance;

        } catch (error) {
            console.error(`❌ Error running WebAssembly file: ${error.message}`);
            if (error.stack) {
                console.error(error.stack);
            }
            throw error;
        }
    }

    // List available WASM files
    listWasmFiles() {
        const files = fs.readdirSync('.').filter(f => f.endsWith('.wasm'));
        console.log(`📁 Available WebAssembly files:`);
        files.forEach(file => {
            const stats = fs.statSync(file);
            console.log(`  • ${file} (${stats.size} bytes)`);
        });
        return files;
    }
}

// CLI interface
async function main() {
    const runner = new WasmRunner();
    const args = process.argv.slice(2);

    if (args.length === 0) {
        console.log(`🌐 Clean Language WebAssembly Runner`);
        console.log(`Usage: node wasm_runner.js <filename.wasm>`);
        console.log(`       node wasm_runner.js --list`);
        console.log();
        runner.listWasmFiles();
        return;
    }

    if (args[0] === '--list' || args[0] === '-l') {
        runner.listWasmFiles();
        return;
    }

    const filename = args[0];
    await runner.runWasm(filename);
}

// Export for use as module
module.exports = WasmRunner;

// Run if called directly
if (require.main === module) {
    main().catch(console.error);
} 