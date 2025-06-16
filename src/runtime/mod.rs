// Clean Language WebAssembly Runtime with Async Support
// Provides enhanced runtime capabilities for async programming features

use crate::error::CompilerError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use wasmtime::{Config, Engine, Module, Store, Linker, Caller, Val, Instance};

pub mod async_runtime;
pub mod task_scheduler;
pub mod future_resolver;

/// Enhanced WebAssembly runtime with async support
pub struct CleanRuntime {
    engine: Engine,
    task_scheduler: Arc<Mutex<TaskScheduler>>,
    future_resolver: Arc<Mutex<FutureResolver>>,
    background_tasks: Arc<Mutex<Vec<BackgroundTask>>>,
}

/// Represents a background task running in the runtime
#[derive(Debug)]
pub struct BackgroundTask {
    pub id: u32,
    pub name: String,
    pub started_at: Instant,
    pub status: TaskStatus,
}

/// Status of a background task
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed(String),
}

/// Task scheduler for managing async operations
pub struct TaskScheduler {
    next_task_id: u32,
    running_tasks: HashMap<u32, BackgroundTask>,
}

/// Future resolver for handling later assignments
pub struct FutureResolver {
    futures: HashMap<String, FutureValue>,
}

/// Represents a future value that will be resolved later
#[derive(Debug, Clone)]
pub struct FutureValue {
    pub id: String,
    pub value: Option<i32>, // For now, using i32 as the basic value type
    pub resolved: bool,
    pub created_at: Instant,
}

impl CleanRuntime {
    /// Create a new Clean Language runtime with async support
    pub fn new() -> Result<Self, CompilerError> {
        // Enable async support in Wasmtime configuration
        let mut config = Config::new();
        config.async_support(true);
        config.wasm_threads(true);
        
        let engine = Engine::new(&config)
            .map_err(|e| CompilerError::runtime_error(
                format!("Failed to create async WebAssembly engine: {}", e),
                None, None
            ))?;
        
        Ok(CleanRuntime {
            engine,
            task_scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            future_resolver: Arc::new(Mutex::new(FutureResolver::new())),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    /// Execute a WebAssembly module with async support
    pub async fn execute_async(&self, wasm_bytes: &[u8]) -> Result<(), CompilerError> {
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| CompilerError::runtime_error(
                format!("Failed to create WebAssembly module: {}", e),
                None, None
            ))?;
        
        let mut store = Store::new(&self.engine, ());
        let mut linker = Linker::new(&self.engine);
        
        // Add async-aware runtime functions
        self.add_async_runtime_functions(&mut linker)?;
        
        // Instantiate the module
        let instance = linker.instantiate_async(&mut store, &module).await
            .map_err(|e| CompilerError::runtime_error(
                format!("Failed to instantiate WebAssembly module: {}", e),
                None, None
            ))?;
        
        // Execute the start function
        if let Some(start_func) = instance.get_func(&mut store, "start") {
            println!("🚀 Executing Clean Language program with async support...");
            println!("--- Output ---");
            
            start_func.call_async(&mut store, &[], &mut []).await
                .map_err(|e| CompilerError::runtime_error(
                    format!("Runtime error during execution: {}", e),
                    None, None
                ))?;
            
            println!("--- End Output ---");
            
            // Wait for background tasks to complete
            self.wait_for_background_tasks().await;
            
            println!("✅ Execution completed successfully!");
        } else {
            return Err(CompilerError::runtime_error(
                "No start function found in WebAssembly module".to_string(),
                None, None
            ));
        }
        
        Ok(())
    }
    
    /// Add async-aware runtime functions to the linker
    fn add_async_runtime_functions(&self, linker: &mut Linker<()>) -> Result<(), CompilerError> {
        let task_scheduler = Arc::clone(&self.task_scheduler);
        let future_resolver = Arc::clone(&self.future_resolver);
        let background_tasks = Arc::clone(&self.background_tasks);
        
        // Enhanced print functions with async support
        linker.func_wrap("env", "print", |mut caller: Caller<'_, ()>, str_ptr: i32, str_len: i32| {
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let data = memory.data(&caller);
                    if str_ptr >= 0 && str_len >= 0 {
                        let start = str_ptr as usize;
                        let len = str_len as usize;
                        if start + len <= data.len() {
                            if let Ok(string) = std::str::from_utf8(&data[start..start + len]) {
                                print!("{}", string);
                            } else {
                                print!("[invalid UTF-8]");
                            }
                        } else {
                            print!("[out of bounds]");
                        }
                    } else {
                        print!("[invalid pointer/length]");
                    }
                }
            }
            Ok(())
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create print function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "println", |mut caller: Caller<'_, ()>, str_ptr: i32, str_len: i32| {
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let data = memory.data(&caller);
                    if str_ptr >= 0 && str_len >= 0 {
                        let start = str_ptr as usize;
                        let len = str_len as usize;
                        if start + len <= data.len() {
                            if let Ok(string) = std::str::from_utf8(&data[start..start + len]) {
                                println!("{}", string);
                            } else {
                                println!("[invalid UTF-8]");
                            }
                        } else {
                            println!("[out of bounds]");
                        }
                    } else {
                        println!("[invalid pointer/length]");
                    }
                }
            }
            Ok(())
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create println function: {}", e),
            None, None
        ))?;
        
        // Async task management functions
        let task_scheduler_clone = Arc::clone(&task_scheduler);
        linker.func_wrap("env", "start_background_task", move |_caller: Caller<'_, ()>, task_name_ptr: i32, task_name_len: i32| -> i32 {
            let mut scheduler = task_scheduler_clone.lock().unwrap();
            let task_id = scheduler.create_task("background_task".to_string());
            println!("🔄 Started background task #{}", task_id);
            task_id as i32
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create start_background_task function: {}", e),
            None, None
        ))?;
        
        // Future resolution functions
        let future_resolver_clone = Arc::clone(&future_resolver);
        linker.func_wrap("env", "create_future", move |_caller: Caller<'_, ()>, future_name_ptr: i32, future_name_len: i32| -> i32 {
            let mut resolver = future_resolver_clone.lock().unwrap();
            let future_id = format!("future_{}", resolver.futures.len());
            resolver.create_future(future_id.clone());
            println!("🔮 Created future: {}", future_id);
            1 // Return success
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create create_future function: {}", e),
            None, None
        ))?;
        
        let future_resolver_clone2 = Arc::clone(&future_resolver);
        linker.func_wrap("env", "resolve_future", move |_caller: Caller<'_, ()>, future_id: i32, value: i32| -> i32 {
            let mut resolver = future_resolver_clone2.lock().unwrap();
            let future_name = format!("future_{}", future_id);
            resolver.resolve_future(future_name, value);
            println!("✅ Resolved future #{} with value: {}", future_id, value);
            1 // Return success
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create resolve_future function: {}", e),
            None, None
        ))?;
        
        // Background processing function
        let background_tasks_clone = Arc::clone(&background_tasks);
        linker.func_wrap("env", "execute_background", move |_caller: Caller<'_, ()>, operation_ptr: i32, operation_len: i32| -> i32 {
            let mut tasks = background_tasks_clone.lock().unwrap();
            let task = BackgroundTask {
                id: tasks.len() as u32,
                name: "background_operation".to_string(),
                started_at: Instant::now(),
                status: TaskStatus::Running,
            };
            println!("🔄 Executing background operation #{}", task.id);
            tasks.push(task);
            
            // Simulate background work
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                println!("✅ Background operation completed");
            });
            
            1 // Return success
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create execute_background function: {}", e),
            None, None
        ))?;
        
        // Add standard library functions
        self.add_stdlib_functions(linker)?;
        
        Ok(())
    }
    
    /// Add standard library functions (HTTP, File I/O, etc.)
    fn add_stdlib_functions(&self, linker: &mut Linker<()>) -> Result<(), CompilerError> {
        // Type conversion functions - CRITICAL for runtime functionality
        linker.func_wrap("env", "int_to_string", |mut caller: Caller<'_, ()>, value: i32| -> i32 {
            let string_value = value.to_string();
            
            // Get memory to store the string
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let mut data = memory.data_mut(&mut caller);
                    
                    // Simple string storage: length (4 bytes) + string data
                    let string_bytes = string_value.as_bytes();
                    let total_size = 4 + string_bytes.len();
                    
                    // Find a place to store the string (simple allocation at end of used memory)
                    let mut offset = 1024; // Start after initial memory
                    while offset + total_size < data.len() {
                        // Check if this area is free (all zeros)
                        let is_free = data[offset..offset + total_size].iter().all(|&b| b == 0);
                        if is_free {
                            break;
                        }
                        offset += 32; // Move in 32-byte chunks
                    }
                    
                    if offset + total_size < data.len() {
                        // Store length
                        data[offset..offset + 4].copy_from_slice(&(string_bytes.len() as u32).to_le_bytes());
                        // Store string data
                        data[offset + 4..offset + 4 + string_bytes.len()].copy_from_slice(string_bytes);
                        return offset as i32;
                    }
                }
            }
            
            0 // Return null pointer on failure
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create int_to_string function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "float_to_string", |mut caller: Caller<'_, ()>, value: f64| -> i32 {
            let string_value = value.to_string();
            
            // Get memory to store the string
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let mut data = memory.data_mut(&mut caller);
                    
                    // Simple string storage: length (4 bytes) + string data
                    let string_bytes = string_value.as_bytes();
                    let total_size = 4 + string_bytes.len();
                    
                    // Find a place to store the string
                    let mut offset = 1024;
                    while offset + total_size < data.len() {
                        let is_free = data[offset..offset + total_size].iter().all(|&b| b == 0);
                        if is_free {
                            break;
                        }
                        offset += 32;
                    }
                    
                    if offset + total_size < data.len() {
                        // Store length
                        data[offset..offset + 4].copy_from_slice(&(string_bytes.len() as u32).to_le_bytes());
                        // Store string data
                        data[offset + 4..offset + 4 + string_bytes.len()].copy_from_slice(string_bytes);
                        return offset as i32;
                    }
                }
            }
            
            0 // Return null pointer on failure
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create float_to_string function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "bool_to_string", |mut caller: Caller<'_, ()>, value: i32| -> i32 {
            let string_value = if value != 0 { "true" } else { "false" };
            
            // Get memory to store the string
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let mut data = memory.data_mut(&mut caller);
                    
                    let string_bytes = string_value.as_bytes();
                    let total_size = 4 + string_bytes.len();
                    
                    let mut offset = 1024;
                    while offset + total_size < data.len() {
                        let is_free = data[offset..offset + total_size].iter().all(|&b| b == 0);
                        if is_free {
                            break;
                        }
                        offset += 32;
                    }
                    
                    if offset + total_size < data.len() {
                        data[offset..offset + 4].copy_from_slice(&(string_bytes.len() as u32).to_le_bytes());
                        data[offset + 4..offset + 4 + string_bytes.len()].copy_from_slice(string_bytes);
                        return offset as i32;
                    }
                }
            }
            
            0
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create bool_to_string function: {}", e),
            None, None
        ))?;
        
        // String parsing functions
        linker.func_wrap("env", "string_to_int", |mut caller: Caller<'_, ()>, str_ptr: i32| -> i32 {
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let data = memory.data(&caller);
                    
                    if str_ptr >= 0 && (str_ptr as usize) + 4 < data.len() {
                        // Read string length
                        let len_bytes = &data[str_ptr as usize..str_ptr as usize + 4];
                        let str_len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
                        
                        if str_ptr as usize + 4 + str_len < data.len() {
                            // Read string data
                            let str_data = &data[str_ptr as usize + 4..str_ptr as usize + 4 + str_len];
                            if let Ok(string_value) = std::str::from_utf8(str_data) {
                                return string_value.parse::<i32>().unwrap_or(0);
                            }
                        }
                    }
                }
            }
            0
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create string_to_int function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "string_to_float", |mut caller: Caller<'_, ()>, str_ptr: i32| -> f64 {
            if let Some(memory) = caller.get_export("memory") {
                if let Some(memory) = memory.into_memory() {
                    let data = memory.data(&caller);
                    
                    if str_ptr >= 0 && (str_ptr as usize) + 4 < data.len() {
                        let len_bytes = &data[str_ptr as usize..str_ptr as usize + 4];
                        let str_len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
                        
                        if str_ptr as usize + 4 + str_len < data.len() {
                            let str_data = &data[str_ptr as usize + 4..str_ptr as usize + 4 + str_len];
                            if let Ok(string_value) = std::str::from_utf8(str_data) {
                                return string_value.parse::<f64>().unwrap_or(0.0);
                            }
                        }
                    }
                }
            }
            0.0
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create string_to_float function: {}", e),
            None, None
        ))?;
        
        // HTTP functions with async simulation
        linker.func_wrap("env", "http_get", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32| -> i32 {
            println!("🌐 [HTTP GET] Simulating async request...");
            thread::sleep(Duration::from_millis(50)); // Simulate network delay
            println!("✅ [HTTP GET] Request completed");
            0 // Return mock string pointer
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create http_get function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "http_post", |_caller: Caller<'_, ()>, _url_ptr: i32, _url_len: i32, _body_ptr: i32, _body_len: i32| -> i32 {
            println!("🌐 [HTTP POST] Simulating async request...");
            thread::sleep(Duration::from_millis(75)); // Simulate network delay
            println!("✅ [HTTP POST] Request completed");
            0 // Return mock string pointer
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create http_post function: {}", e),
            None, None
        ))?;
        
        // File I/O functions with async simulation
        linker.func_wrap("env", "file_read", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32| -> i32 {
            println!("📁 [FILE READ] Simulating async file read...");
            thread::sleep(Duration::from_millis(25)); // Simulate disk I/O
            println!("✅ [FILE READ] File read completed");
            0 // Return mock content pointer
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create file_read function: {}", e),
            None, None
        ))?;
        
        linker.func_wrap("env", "file_write", |_caller: Caller<'_, ()>, _path_ptr: i32, _path_len: i32, _content_ptr: i32, _content_len: i32| -> i32 {
            println!("📁 [FILE WRITE] Simulating async file write...");
            thread::sleep(Duration::from_millis(30)); // Simulate disk I/O
            println!("✅ [FILE WRITE] File write completed");
            0 // Return success
        })
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create file_write function: {}", e),
            None, None
        ))?;
        
        Ok(())
    }
    
    /// Wait for all background tasks to complete
    async fn wait_for_background_tasks(&self) {
        let mut completed = false;
        let mut iterations = 0;
        const MAX_WAIT_ITERATIONS: u32 = 100; // Prevent infinite waiting
        
        while !completed && iterations < MAX_WAIT_ITERATIONS {
            {
                let tasks = self.background_tasks.lock().unwrap();
                completed = tasks.iter().all(|task| matches!(task.status, TaskStatus::Completed | TaskStatus::Failed(_)));
                
                if !completed {
                    let running_count = tasks.iter().filter(|task| matches!(task.status, TaskStatus::Running)).count();
                    if running_count > 0 {
                        println!("⏳ Waiting for {} background task(s) to complete...", running_count);
                    }
                }
            }
            
            if !completed {
                tokio::time::sleep(Duration::from_millis(50)).await;
                iterations += 1;
            }
        }
        
        if iterations >= MAX_WAIT_ITERATIONS {
            println!("⚠️  Timeout waiting for background tasks to complete");
        } else {
            println!("✅ All background tasks completed");
        }
    }
}

impl TaskScheduler {
    pub fn new() -> Self {
        TaskScheduler {
            next_task_id: 1,
            running_tasks: HashMap::new(),
        }
    }
    
    pub fn create_task(&mut self, name: String) -> u32 {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        
        let task = BackgroundTask {
            id: task_id,
            name,
            started_at: Instant::now(),
            status: TaskStatus::Running,
        };
        
        self.running_tasks.insert(task_id, task);
        task_id
    }
    
    pub fn complete_task(&mut self, task_id: u32) {
        if let Some(task) = self.running_tasks.get_mut(&task_id) {
            task.status = TaskStatus::Completed;
        }
    }
    
    pub fn fail_task(&mut self, task_id: u32, error: String) {
        if let Some(task) = self.running_tasks.get_mut(&task_id) {
            task.status = TaskStatus::Failed(error);
        }
    }
}

impl FutureResolver {
    pub fn new() -> Self {
        FutureResolver {
            futures: HashMap::new(),
        }
    }
    
    pub fn create_future(&mut self, id: String) {
        let future = FutureValue {
            id: id.clone(),
            value: None,
            resolved: false,
            created_at: Instant::now(),
        };
        self.futures.insert(id, future);
    }
    
    pub fn resolve_future(&mut self, id: String, value: i32) {
        if let Some(future) = self.futures.get_mut(&id) {
            future.value = Some(value);
            future.resolved = true;
        }
    }
    
    pub fn get_future_value(&self, id: &str) -> Option<i32> {
        self.futures.get(id).and_then(|f| if f.resolved { f.value } else { None })
    }
    
    pub fn is_future_resolved(&self, id: &str) -> bool {
        self.futures.get(id).map(|f| f.resolved).unwrap_or(false)
    }
}

/// Convenience function to create and run a Clean Language program with async support
pub async fn run_clean_program_async(wasm_bytes: &[u8]) -> Result<(), CompilerError> {
    let runtime = CleanRuntime::new()?;
    runtime.execute_async(wasm_bytes).await
}

/// Synchronous wrapper for async execution (for backward compatibility)
pub fn run_clean_program_sync(wasm_bytes: &[u8]) -> Result<(), CompilerError> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| CompilerError::runtime_error(
            format!("Failed to create async runtime: {}", e),
            None, None
        ))?;
    
    rt.block_on(run_clean_program_async(wasm_bytes))
} 