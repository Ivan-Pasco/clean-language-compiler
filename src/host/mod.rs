use wasmtime::{Caller, Linker, Store, Memory};
use crate::stdlib::memory::MemoryManager;
use std::cell::RefCell;

pub fn register_host_functions(linker: &mut Linker<()>, memory: &RefCell<MemoryManager>) -> Result<(), Box<dyn std::error::Error>> {
    // Register print function
    linker.func_wrap("host", "print", move |mut caller: Caller<'_, ()>, str_ptr: i32| {
        let memory = memory.borrow_mut();
        
        // Get memory view
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        
        // Read string length
        let mut len_bytes = [0u8; 4];
        mem.read(&caller, str_ptr as usize, &mut len_bytes).unwrap();
        let str_len = i32::from_le_bytes(len_bytes) as usize;
        
        // Read string content
        let mut str_bytes = vec![0u8; str_len];
        mem.read(&caller, (str_ptr + 4) as usize, &mut str_bytes).unwrap();
        
        // Convert bytes to string and print
        let str_value = std::str::from_utf8(&str_bytes).unwrap();
        print!("{}", str_value);
    })?;

    // Register printl function (print with newline)
    linker.func_wrap("host", "printl", move |mut caller: Caller<'_, ()>, str_ptr: i32| {
        let memory = memory.borrow_mut();
        
        // Get memory view
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        
        // Read string length
        let mut len_bytes = [0u8; 4];
        mem.read(&caller, str_ptr as usize, &mut len_bytes).unwrap();
        let str_len = i32::from_le_bytes(len_bytes) as usize;
        
        // Read string content
        let mut str_bytes = vec![0u8; str_len];
        mem.read(&caller, (str_ptr + 4) as usize, &mut str_bytes).unwrap();
        
        // Convert bytes to string and print with newline
        let str_value = std::str::from_utf8(&str_bytes).unwrap();
        println!("{}", str_value);
    })?;

    // Register string_to_host function
    linker.func_wrap("host", "string_to_host", move |mut caller: Caller<'_, ()>, str_ptr: i32| -> i32 {
        let memory = memory.borrow_mut();
        
        // Get memory view
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        
        // Read string length
        let mut len_bytes = [0u8; 4];
        mem.read(&caller, str_ptr as usize, &mut len_bytes).unwrap();
        let str_len = i32::from_le_bytes(len_bytes) as usize;
        
        // Read string content
        let mut str_bytes = vec![0u8; str_len];
        mem.read(&caller, (str_ptr + 4) as usize, &mut str_bytes).unwrap();
        
        // Convert bytes to string
        let str_value = std::str::from_utf8(&str_bytes).unwrap();
        
        // Allocate new memory for host string
        let new_ptr = memory.allocate(str_value.len() + 4).unwrap();
        
        // Write string length
        let len_bytes = (str_value.len() as i32).to_le_bytes();
        mem.write(&mut caller, new_ptr, &len_bytes).unwrap();
        
        // Write string content
        mem.write(&mut caller, new_ptr + 4, str_value.as_bytes()).unwrap();
        
        new_ptr as i32
    })?;

    // Register float_to_string function
    linker.func_wrap("host", "float_to_string", move |mut caller: Caller<'_, ()>, value: f32| -> i32 {
        let memory = memory.borrow_mut();
        
        // Convert float to string
        let str_value = format!("{}", value);
        
        // Allocate memory for the string
        let str_ptr = memory.allocate(str_value.len() + 4).unwrap();
        
        // Get memory view
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        
        // Write string length
        let len_bytes = (str_value.len() as i32).to_le_bytes();
        mem.write(&mut caller, str_ptr, &len_bytes).unwrap();
        
        // Write string content
        mem.write(&mut caller, str_ptr + 4, str_value.as_bytes()).unwrap();
        
        str_ptr as i32
    })?;

    // Register string_to_float function
    linker.func_wrap("host", "string_to_float", move |mut caller: Caller<'_, ()>, str_ptr: i32| -> f64 {
        let memory = memory.borrow_mut();
        
        // Get memory view
        let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
        
        // Read string length
        let mut len_bytes = [0u8; 4];
        mem.read(&caller, str_ptr as usize, &mut len_bytes).unwrap();
        let str_len = i32::from_le_bytes(len_bytes) as usize;
        
        // Read string content
        let mut str_bytes = vec![0u8; str_len];
        mem.read(&caller, (str_ptr + 4) as usize, &mut str_bytes).unwrap();
        
        // Convert bytes to string
        let str_value = std::str::from_utf8(&str_bytes).unwrap();
        
        // Parse float
        str_value.parse::<f64>().unwrap_or(0.0)
    })?;

    Ok(())
} 