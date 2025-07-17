use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;
use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Http class implementation for Clean Language
/// Provides HTTP client operations as static methods
pub struct HttpClass;

impl HttpClass {
    pub fn new() -> Self {
        Self
    }

    /// Register all Http class methods as static functions
    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Basic HTTP operations
        self.register_basic_operations(codegen)?;
        
        // Advanced HTTP operations
        self.register_advanced_operations(codegen)?;
        
        // Utility operations
        self.register_utility_operations(codegen)?;
        
        Ok(())
    }
    
    fn register_basic_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // http.get(string url) -> string
        register_stdlib_function(
            codegen,
            "http.get",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_get")?
        )?;
        
        // http.post(string url, string data) -> string
        register_stdlib_function(
            codegen,
            "http.post",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_post")?
        )?;
        
        // http.put(string url, string data) -> string
        register_stdlib_function(
            codegen,
            "http.put",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_put")?
        )?;
        
        // http.patch(string url, string data) -> string
        register_stdlib_function(
            codegen,
            "http.patch",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_patch")?
        )?;
        
        // http.delete(string url) -> string
        register_stdlib_function(
            codegen,
            "http.delete",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_delete")?
        )?;
        
        // http.head(string url) -> string
        register_stdlib_function(
            codegen,
            "http.head",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_head")?
        )?;
        
        // http.options(string url) -> string
        register_stdlib_function(
            codegen,
            "http.options",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_options")?
        )?;
        
        Ok(())
    }
    
    fn register_advanced_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // http.getWithHeaders(string url, array<string> headers) -> string
        register_stdlib_function(
            codegen,
            "http.getWithHeaders",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_get_with_headers")?
        )?;
        
        // http.postWithHeaders(string url, string data, array<string> headers) -> string
        register_stdlib_function(
            codegen,
            "http.postWithHeaders",
            &[WasmType::I32, WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_headers_host_call(codegen, "http_post_with_headers")?
        )?;
        
        // http.postJson(string url, string jsonData) -> string
        register_stdlib_function(
            codegen,
            "http.postJson",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_post_json")?
        )?;
        
        // http.putJson(string url, string jsonData) -> string
        register_stdlib_function(
            codegen,
            "http.putJson",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_put_json")?
        )?;
        
        // http.patchJson(string url, string jsonData) -> string
        register_stdlib_function(
            codegen,
            "http.patchJson",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_patch_json")?
        )?;
        
        // http.postForm(string url, array<string> formData) -> string
        register_stdlib_function(
            codegen,
            "http.postForm",
            &[WasmType::I32, WasmType::I32],
            Some(WasmType::I32),
            self.generate_post_with_host_call(codegen, "http_post_form")?
        )?;
        
        Ok(())
    }
    
    fn register_utility_operations(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // http.setUserAgent(string userAgent) -> void
        register_stdlib_function(
            codegen,
            "http.setUserAgent",
            &[WasmType::I32],
            None,
            self.generate_set_user_agent_host_call(codegen, "http_set_user_agent")?
        )?;
        
        // http.setTimeout(integer timeoutMs) -> void
        register_stdlib_function(
            codegen,
            "http.setTimeout",
            &[WasmType::I32],
            None,
            self.generate_simple_host_call(codegen, "http_set_timeout")?
        )?;
        
        // http.setMaxRedirects(integer maxRedirects) -> void
        register_stdlib_function(
            codegen,
            "http.setMaxRedirects",
            &[WasmType::I32],
            None,
            self.generate_simple_host_call(codegen, "http_set_max_redirects")?
        )?;
        
        // http.enableCookies(boolean enable) -> void
        register_stdlib_function(
            codegen,
            "http.enableCookies",
            &[WasmType::I32],
            None,
            self.generate_simple_host_call(codegen, "http_enable_cookies")?
        )?;
        
        // http.getResponseCode() -> integer
        register_stdlib_function(
            codegen,
            "http.getResponseCode",
            &[],
            Some(WasmType::I32),
            self.generate_no_params_host_call(codegen, "http_get_response_code")?
        )?;
        
        // http.getResponseHeaders() -> array<string>
        register_stdlib_function(
            codegen,
            "http.getResponseHeaders",
            &[],
            Some(WasmType::I32),
            self.generate_no_params_host_call(codegen, "http_get_response_headers")?
        )?;
        
        // http.encodeUrl(string url) -> string
        register_stdlib_function(
            codegen,
            "http.encodeUrl",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_encode_url")?
        )?;
        
        // http.decodeUrl(string encodedUrl) -> string
        register_stdlib_function(
            codegen,
            "http.decodeUrl",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_decode_url")?
        )?;
        
        // http.buildQuery(array<string> params) -> string
        register_stdlib_function(
            codegen,
            "http.buildQuery",
            &[WasmType::I32],
            Some(WasmType::I32),
            self.generate_get_with_host_call(codegen, "http_build_query")?
        )?;
        
        Ok(())
    }

    // Implementation methods for HTTP operations

    /// Generate instructions for HTTP GET-style calls (single URL parameter)
    fn generate_get_with_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Load URL string data pointer (skip 4-byte length prefix)
            Instruction::LocalGet(0), // url string pointer
            Instruction::I32Const(4), // offset to string data (past length field)
            Instruction::I32Add, // ptr + 4 = actual string data pointer
            
            // Load URL string length
            Instruction::LocalGet(0), // url string pointer again
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // load string length from [ptr]
            
            // Call the HTTP host function
            Instruction::Call(import_index),
        ])
    }

    /// Generate instructions for HTTP POST-style calls (URL + body parameters)
    fn generate_post_with_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Load URL string data pointer and length
            Instruction::LocalGet(0), // url string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // url data ptr
            Instruction::LocalGet(0),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // url length
            
            // Load body string data pointer and length
            Instruction::LocalGet(1), // body string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // body data ptr
            Instruction::LocalGet(1),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // body length
            
            // Call the HTTP host function
            Instruction::Call(import_index),
        ])
    }

    /// Generate instructions for HTTP calls with headers (URL + body + headers parameters)
    fn generate_post_with_headers_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Load URL string data pointer and length
            Instruction::LocalGet(0), // url string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // url data ptr
            Instruction::LocalGet(0),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // url length
            
            // Load body string data pointer and length
            Instruction::LocalGet(1), // body string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // body data ptr
            Instruction::LocalGet(1),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // body length
            
            // Load headers string data pointer and length
            Instruction::LocalGet(2), // headers string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // headers data ptr
            Instruction::LocalGet(2),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // headers length
            
            // Call the HTTP host function
            Instruction::Call(import_index),
        ])
    }

    /// Generate instructions for simple HTTP calls with one integer parameter
    fn generate_simple_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Pass the integer parameter directly
            Instruction::LocalGet(0),
            
            // Call the HTTP host function
            Instruction::Call(import_index),
        ])
    }

    /// Generate instructions for HTTP calls with no parameters
    fn generate_no_params_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Call the HTTP host function with no parameters
            Instruction::Call(import_index),
        ])
    }

    /// Generate instructions for setUserAgent (string parameter)
    fn generate_set_user_agent_host_call(&self, codegen: &CodeGenerator, host_func_name: &str) -> Result<Vec<Instruction>, CompilerError> {
        let import_index = codegen.get_http_import_index(host_func_name)
            .ok_or_else(|| CompilerError::codegen_error(
                &format!("HTTP import function '{}' not found", host_func_name),
                Some("Make sure HTTP imports are properly registered".to_string()),
                None
            ))?;

        Ok(vec![
            // Load user agent string data pointer and length
            Instruction::LocalGet(0), // string pointer
            Instruction::I32Const(4),
            Instruction::I32Add, // string data ptr
            Instruction::LocalGet(0),
            Instruction::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }), // string length
            
            // Call the HTTP host function
            Instruction::Call(import_index),
        ])
    }

}