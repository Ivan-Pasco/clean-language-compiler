const fs = require('fs');

// Test if strings are working in a simple environment
const wasmBytes = fs.readFileSync('test_actual_strings.wasm');

let memory;
let callCount = 0;

const imports = {
  env: {
    print: (ptr) => {
      callCount++;
      console.log(`\\n=== PRINT CALL #${callCount} ===`);
      console.log('Raw pointer value:', ptr);
      
      if (!memory) {
        console.log('ERROR: Memory not available');
        return;
      }
      
      try {
        if (ptr === 0) {
          console.log('RESULT: (null/empty string)');
          return;
        }
        
        // Directly check if it's a small integer (non-pointer)
        if (ptr < 500) {
          console.log('RESULT: Integer value:', ptr);
          return;
        }
        
        // Try to read as string pointer
        const memView = new DataView(memory.buffer);
        
        // Read length
        const length = memView.getUint32(ptr, true);
        console.log('String length:', length);
        
        if (length === 0) {
          console.log('RESULT: Empty string');
          return;
        }
        
        if (length > 10000) {
          console.log('RESULT: Invalid length, probably not a string pointer');
          return;
        }
        
        // Read content
        const bytes = new Uint8Array(memory.buffer, ptr + 4, length);
        const str = new TextDecoder().decode(bytes);
        console.log('RESULT: String content:', JSON.stringify(str));
        
      } catch (err) {
        console.log('ERROR reading string:', err.message);
      }
    },
    
    printl: (ptr) => {
      imports.env.print(ptr);
      console.log('(with newline)');
    }
  }
};

WebAssembly.instantiate(wasmBytes, imports)
  .then(result => {
    console.log('=== WASM MODULE ANALYSIS ===');
    console.log('Exports:', Object.keys(result.instance.exports));
    
    memory = result.instance.exports.memory;
    if (memory) {
      console.log('Memory size:', memory.buffer.byteLength, 'bytes');
      
      // Search for our strings in memory
      const view = new Uint8Array(memory.buffer);
      const text = new TextDecoder().decode(view);
      
      ['Direct string literal', 'Another string'].forEach(searchStr => {
        const index = text.indexOf(searchStr);
        if (index >= 0) {
          console.log(`Found "${searchStr}" at offset: ${index}`);
          
          // Check if there's a length header 4 bytes before
          if (index >= 4) {
            const lengthView = new DataView(memory.buffer);
            const possibleLength = lengthView.getUint32(index - 4, true);
            console.log(`  Possible length header: ${possibleLength} (expected: ${searchStr.length})`);
          }
        } else {
          console.log(`"${searchStr}" not found in memory`);
        }
      });
      
    } else {
      console.log('No memory export found');
    }
    
    console.log('\\n=== EXECUTING START FUNCTION ===');
    if (result.instance.exports.start) {
      result.instance.exports.start();
    } else {
      console.log('No start function found');
    }
    
    console.log(`\\n=== SUMMARY ===`);
    console.log(`Total print calls: ${callCount}`);
  })
  .catch(err => {
    console.error('Error:', err);
  });