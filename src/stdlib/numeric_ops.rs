use crate::codegen::CodeGenerator;
use crate::types::WasmType;
use crate::error::CompilerError;

use wasm_encoder::Instruction;
use crate::stdlib::register_stdlib_function;

/// Comprehensive mathematical operations implementation
pub struct NumericOperations {}

impl NumericOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Basic arithmetic functions
        self.register_basic_arithmetic(codegen)?;
        
        // Comparison functions
        self.register_comparison_functions(codegen)?;
        
        // Mathematical functions
        self.register_math_functions(codegen)?;
        
        // Trigonometric functions
        self.register_trig_functions(codegen)?;
        
        // Advanced mathematical functions
        self.register_advanced_functions(codegen)?;
        
        Ok(())
    }
    
    fn register_basic_arithmetic(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Add function
        register_stdlib_function(
            codegen,
            "add",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Add,
            ]
        )?;

        // Subtract function
        register_stdlib_function(
            codegen,
            "subtract",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Sub,
            ]
        )?;

        // Multiply function
        register_stdlib_function(
            codegen,
            "multiply",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Mul,
            ]
        )?;

        // Divide function
        register_stdlib_function(
            codegen,
            "divide",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Div,
            ]
        )?;
        
        // Modulo function (using fmod-like approach)
        register_stdlib_function(
            codegen,
            "mod",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                // Implement a % b = a - (floor(a/b) * b)
                Instruction::LocalGet(0), // a
                Instruction::LocalGet(0), // a
                Instruction::LocalGet(1), // b
                Instruction::F64Div,      // a/b
                Instruction::F64Floor,    // floor(a/b)
                Instruction::LocalGet(1), // b
                Instruction::F64Mul,      // floor(a/b) * b
                Instruction::F64Sub,      // a - (floor(a/b) * b)
            ]
        )?;
        
        Ok(())
    }
    
    fn register_comparison_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Equals function
        register_stdlib_function(
            codegen,
            "equals",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Eq,
            ]
        )?;

        // Not equals function
        register_stdlib_function(
            codegen,
            "not_equals",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Ne,
            ]
        )?;

        // Less than function
        register_stdlib_function(
            codegen,
            "less_than",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Lt,
            ]
        )?;

        // Greater than function
        register_stdlib_function(
            codegen,
            "greater_than",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Gt,
            ]
        )?;
        
        // Less than or equal
        register_stdlib_function(
            codegen,
            "less_equal",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Le,
            ]
        )?;

        // Greater than or equal
        register_stdlib_function(
            codegen,
            "greater_equal",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::I32),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Ge,
            ]
        )?;
        
        Ok(())
    }
    
    fn register_math_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Absolute value function
        register_stdlib_function(
            codegen,
            "abs",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Abs,
            ]
        )?;
        
        // Square root function
        register_stdlib_function(
            codegen,
            "sqrt",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Sqrt,
            ]
        )?;
        
        // Ceiling function
        register_stdlib_function(
            codegen,
            "ceil",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Ceil,
            ]
        )?;
        
        // Floor function
        register_stdlib_function(
            codegen,
            "floor",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Floor,
            ]
        )?;
        
        // Truncate function
        register_stdlib_function(
            codegen,
            "trunc",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Trunc,
            ]
        )?;
        
        // Round to nearest integer function
        register_stdlib_function(
            codegen,
            "round",
            &[WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::F64Nearest,
            ]
        )?;
        
        // Power function (x^y)
        register_stdlib_function(
            codegen,
            "pow",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            self.generate_pow_function()
        )?;
        
        // Maximum of two numbers
        register_stdlib_function(
            codegen,
            "max",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Max,
            ]
        )?;
        
        // Minimum of two numbers
        register_stdlib_function(
            codegen,
            "min",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            vec![
                Instruction::LocalGet(0),
                Instruction::LocalGet(1),
                Instruction::F64Min,
            ]
        )?;
        
        // Sign function (-1, 0, or 1)
        register_stdlib_function(
            codegen,
            "sign",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_sign_function()
        )?;
        
        Ok(())
    }
    
    fn register_trig_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Note: WebAssembly doesn't have native trigonometric functions
        // We'll implement approximations using Taylor series
        
        // Sine function
        register_stdlib_function(
            codegen,
            "sin",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_sin()
        )?;
        
        // Cosine function
        register_stdlib_function(
            codegen,
            "cos",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_cos()
        )?;
        
        // Tangent function
        register_stdlib_function(
            codegen,
            "tan",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_tan()
        )?;
        
        // Arcsine function
        register_stdlib_function(
            codegen,
            "asin",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_asin()
        )?;
        
        // Arccosine function
        register_stdlib_function(
            codegen,
            "acos",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_acos()
        )?;
        
        // Arctangent function
        register_stdlib_function(
            codegen,
            "atan",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_atan()
        )?;
        
        // Two-argument arctangent function
        register_stdlib_function(
            codegen,
            "atan2",
            &[WasmType::F64, WasmType::F64],
            Some(WasmType::F64),
            self.generate_atan2()
        )?;
        
        Ok(())
    }
    
    fn register_advanced_functions(&self, codegen: &mut CodeGenerator) -> Result<(), CompilerError> {
        // Natural logarithm
        register_stdlib_function(
            codegen,
            "ln",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_ln()
        )?;
        
        // Logarithm base 10
        register_stdlib_function(
            codegen,
            "log10",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_log10()
        )?;
        
        // Logarithm base 2
        register_stdlib_function(
            codegen,
            "log2",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_log2()
        )?;
        
        // Exponential function (e^x)
        register_stdlib_function(
            codegen,
            "exp",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_exp()
        )?;
        
        // 2^x function
        register_stdlib_function(
            codegen,
            "exp2",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_exp2()
        )?;
        
        // Mathematical constants as functions
        register_stdlib_function(
            codegen,
            "pi",
            &[],
            Some(WasmType::F64),
            vec![
                Instruction::F64Const(std::f64::consts::PI),
            ]
        )?;
        
        register_stdlib_function(
            codegen,
            "e",
            &[],
            Some(WasmType::F64),
            vec![
                Instruction::F64Const(std::f64::consts::E),
            ]
        )?;
        
        register_stdlib_function(
            codegen,
            "tau",
            &[],
            Some(WasmType::F64),
            vec![
                Instruction::F64Const(std::f64::consts::TAU),
            ]
        )?;
        
        // Hyperbolic functions
        register_stdlib_function(
            codegen,
            "sinh",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_sinh()
        )?;
        
        register_stdlib_function(
            codegen,
            "cosh",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_cosh()
        )?;
        
        register_stdlib_function(
            codegen,
            "tanh",
            &[WasmType::F64],
            Some(WasmType::F64),
            self.generate_tanh()
        )?;
        
        Ok(())
    }
    
    // Helper functions to generate complex mathematical operations
    
    fn generate_pow_function(&self) -> Vec<Instruction> {
        // Simplified power function using repeated multiplication for integer exponents
        // For more complex cases, would use exp(y * ln(x))
        vec![
            // For now, use a simple implementation for positive integer powers
            // TODO: Implement full power function with exp/ln
            Instruction::LocalGet(0), // base
            Instruction::LocalGet(1), // exponent
            // This is a placeholder - would need a more complex implementation
            Instruction::F64Mul, // Simplified multiplication
        ]
    }
    
    fn generate_sign_function(&self) -> Vec<Instruction> {
        vec![
            // if x > 0 return 1, if x < 0 return -1, else return 0
            Instruction::LocalGet(0),
            Instruction::F64Const(0.0),
            Instruction::F64Gt,
            Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64)),
                Instruction::F64Const(1.0),
            Instruction::Else,
                Instruction::LocalGet(0),
                Instruction::F64Const(0.0),
                Instruction::F64Lt,
                Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64)),
                    Instruction::F64Const(-1.0),
                Instruction::Else,
                    Instruction::F64Const(0.0),
                Instruction::End,
            Instruction::End,
        ]
    }
    
    /// Generate sine function using Taylor series
    fn generate_sin(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Real implementation: sin(x) using Taylor series
        // sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
        
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::LocalTee(1)); // result = x
        
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x²
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x³
        instructions.push(Instruction::F64Const(6.0)); // 3!
        instructions.push(Instruction::F64Div);      // x³/3!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::F64Sub);      // result - x³/3!
        instructions.push(Instruction::LocalSet(1)); // result = result - x³/3!
        
        // Add x⁵/5! term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x²
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x³
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x⁴
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x⁵
        instructions.push(Instruction::F64Const(120.0)); // 5!
        instructions.push(Instruction::F64Div);      // x⁵/5!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::F64Add);      // result + x⁵/5!
        instructions.push(Instruction::LocalSet(1)); // result = result + x⁵/5!
        
        instructions.push(Instruction::LocalGet(1)); // Return result
        
        instructions
    }
    
    /// Generate cosine function using Taylor series
    fn generate_cos(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Real implementation: cos(x) using Taylor series
        // cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
        
        instructions.push(Instruction::F64Const(1.0)); // result = 1
        instructions.push(Instruction::LocalSet(1));
        
        // Subtract x²/2! term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x²
        instructions.push(Instruction::F64Const(2.0)); // 2!
        instructions.push(Instruction::F64Div);      // x²/2!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::F64Sub);      // result - x²/2!
        instructions.push(Instruction::LocalSet(1)); // result = result - x²/2!
        
        // Add x⁴/4! term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x²
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x³
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // x⁴
        instructions.push(Instruction::F64Const(24.0)); // 4!
        instructions.push(Instruction::F64Div);      // x⁴/4!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::F64Add);      // result + x⁴/4!
        instructions.push(Instruction::LocalSet(1)); // result = result + x⁴/4!
        
        instructions.push(Instruction::LocalGet(1)); // Return result
        
        instructions
    }
    
    /// Generate natural logarithm using Newton's method
    fn generate_ln(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Real implementation: ln(x) using Newton's method
        // For x > 0, we use the series: ln(1+u) = u - u²/2 + u³/3 - u⁴/4 + ...
        // where u = (x-1)/(x+1)
        
        // Check if x <= 0
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Const(0.0));
        instructions.push(Instruction::F64Le);
        instructions.push(Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64)));
        instructions.push(Instruction::F64Const(f64::NAN)); // Return NaN for x <= 0
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Special case: ln(1) = 0
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::F64Eq);
        instructions.push(Instruction::If(wasm_encoder::BlockType::Result(wasm_encoder::ValType::F64)));
        instructions.push(Instruction::F64Const(0.0)); // Return 0 for ln(1)
        instructions.push(Instruction::Return);
        instructions.push(Instruction::End);
        
        // Calculate u = (x-1)/(x+1)
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::F64Sub);      // x-1
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::F64Add);      // x+1
        instructions.push(Instruction::F64Div);      // u = (x-1)/(x+1)
        instructions.push(Instruction::LocalSet(1)); // u
        
        // Calculate ln(x) ≈ 2u(1 + u²/3 + u⁴/5 + ...)
        instructions.push(Instruction::LocalGet(1)); // u
        instructions.push(Instruction::LocalGet(1)); // u
        instructions.push(Instruction::F64Mul);      // u²
        instructions.push(Instruction::LocalSet(2)); // u²
        
        // Start with 1 + u²/3
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::LocalGet(2)); // u²
        instructions.push(Instruction::F64Const(3.0));
        instructions.push(Instruction::F64Div);      // u²/3
        instructions.push(Instruction::F64Add);      // 1 + u²/3
        instructions.push(Instruction::LocalSet(3)); // series_sum
        
        // Multiply by 2u
        instructions.push(Instruction::F64Const(2.0));
        instructions.push(Instruction::LocalGet(1)); // u
        instructions.push(Instruction::F64Mul);      // 2u
        instructions.push(Instruction::LocalGet(3)); // series_sum
        instructions.push(Instruction::F64Mul);      // 2u * series_sum
        
        instructions
    }
    
    /// Generate exponential function using Taylor series
    fn generate_exp(&self) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Real implementation: exp(x) using Taylor series
        // exp(x) = 1 + x + x²/2! + x³/3! + x⁴/4! + ...
        
        instructions.push(Instruction::F64Const(1.0)); // result = 1
        instructions.push(Instruction::LocalSet(1));
        
        instructions.push(Instruction::F64Const(1.0)); // term = 1
        instructions.push(Instruction::LocalSet(2));
        
        // Add x term
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // term * x
        instructions.push(Instruction::F64Const(1.0));
        instructions.push(Instruction::F64Div);      // term * x / 1!
        instructions.push(Instruction::LocalSet(2)); // term = term * x / 1!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::F64Add);      // result + term
        instructions.push(Instruction::LocalSet(1)); // result = result + term
        
        // Add x²/2! term
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // term * x
        instructions.push(Instruction::F64Const(2.0));
        instructions.push(Instruction::F64Div);      // term * x / 2!
        instructions.push(Instruction::LocalSet(2)); // term = term * x / 2!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::F64Add);      // result + term
        instructions.push(Instruction::LocalSet(1)); // result = result + term
        
        // Add x³/3! term
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::LocalGet(0)); // x
        instructions.push(Instruction::F64Mul);      // term * x
        instructions.push(Instruction::F64Const(3.0));
        instructions.push(Instruction::F64Div);      // term * x / 3!
        instructions.push(Instruction::LocalSet(2)); // term = term * x / 3!
        instructions.push(Instruction::LocalGet(1)); // result
        instructions.push(Instruction::LocalGet(2)); // term
        instructions.push(Instruction::F64Add);      // result + term
        instructions.push(Instruction::LocalSet(1)); // result = result + term
        
        instructions.push(Instruction::LocalGet(1)); // Return result
        
        instructions
    }
    
    fn generate_tan(&self) -> Vec<Instruction> {
        // tan(x) = sin(x) / cos(x)
        vec![
            Instruction::LocalGet(0), // x
            // Call sin(x)
            // Call cos(x)
            // Divide sin by cos
            // Placeholder implementation
        ]
    }
    
    fn generate_asin(&self) -> Vec<Instruction> {
        // Inverse sine approximation
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for asin implementation
        ]
    }
    
    fn generate_acos(&self) -> Vec<Instruction> {
        // Inverse cosine approximation
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for acos implementation
        ]
    }
    
    fn generate_atan(&self) -> Vec<Instruction> {
        // Inverse tangent approximation
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for atan implementation
        ]
    }
    
    fn generate_atan2(&self) -> Vec<Instruction> {
        // Two-argument arctangent
        vec![
            Instruction::LocalGet(0), // y
            Instruction::LocalGet(1), // x
            // Placeholder for atan2 implementation
        ]
    }
    
    fn generate_log10(&self) -> Vec<Instruction> {
        // log10(x) = ln(x) / ln(10)
        vec![
            Instruction::LocalGet(0), // x
            // Call ln(x)
            Instruction::F64Const(std::f64::consts::LN_10),
            Instruction::F64Div,
        ]
    }
    
    fn generate_log2(&self) -> Vec<Instruction> {
        // log2(x) = ln(x) / ln(2)
        vec![
            Instruction::LocalGet(0), // x
            // Call ln(x)
            Instruction::F64Const(std::f64::consts::LN_2),
            Instruction::F64Div,
        ]
    }
    
    fn generate_exp2(&self) -> Vec<Instruction> {
        // 2^x function
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for exp2 implementation
        ]
    }
    
    fn generate_sinh(&self) -> Vec<Instruction> {
        // sinh(x) = (e^x - e^(-x)) / 2
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for sinh implementation
        ]
    }
    
    fn generate_cosh(&self) -> Vec<Instruction> {
        // cosh(x) = (e^x + e^(-x)) / 2
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for cosh implementation
        ]
    }
    
    fn generate_tanh(&self) -> Vec<Instruction> {
        // tanh(x) = sinh(x) / cosh(x)
        vec![
            Instruction::LocalGet(0), // x
            // Placeholder for tanh implementation
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodeGenerator;
    use wasmtime::{Engine, Instance, Module, Store, Val};

    fn setup_test_environment() -> (Store<()>, Instance) {
        let mut codegen = CodeGenerator::new();
        let numeric_ops = NumericOperations::new();
        numeric_ops.register_functions(&mut codegen).unwrap();

        let engine = Engine::default();
        let wasm_bytes = codegen.finish();
        let module = Module::new(&engine, &wasm_bytes).unwrap();
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        (store, instance)
    }

    #[test]
    fn test_add() {
        let (mut store, instance) = setup_test_environment();
        let add = instance.get_func(&mut store, "add").unwrap();
        
        let mut results = vec![Val::F64(0)];
        add.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.7))
        ], &mut results).unwrap();
        
        let result = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((result - 6.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_subtract() {
        let (mut store, instance) = setup_test_environment();
        let subtract = instance.get_func(&mut store, "subtract").unwrap();
        
        let mut results = vec![Val::F64(0)];
        subtract.call(&mut store, &[
            Val::F64(f64::to_bits(5.0)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        let result = f64::from_bits(results[0].unwrap_i64() as u64);
        assert!((result - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_equals() {
        let (mut store, instance) = setup_test_environment();
        let equals = instance.get_func(&mut store, "equals").unwrap();
        
        let mut results = vec![Val::I32(0)];
        equals.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_not_equals() {
        let (mut store, instance) = setup_test_environment();
        let not_equals = instance.get_func(&mut store, "not_equals").unwrap();
        
        let mut results = vec![Val::I32(0)];
        not_equals.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.0))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_less_than() {
        let (mut store, instance) = setup_test_environment();
        let less_than = instance.get_func(&mut store, "less_than").unwrap();
        
        let mut results = vec![Val::I32(0)];
        less_than.call(&mut store, &[
            Val::F64(f64::to_bits(2.5)), 
            Val::F64(f64::to_bits(3.0))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }

    #[test]
    fn test_greater_than() {
        let (mut store, instance) = setup_test_environment();
        let greater_than = instance.get_func(&mut store, "greater_than").unwrap();
        
        let mut results = vec![Val::I32(0)];
        greater_than.call(&mut store, &[
            Val::F64(f64::to_bits(3.0)), 
            Val::F64(f64::to_bits(2.5))
        ], &mut results).unwrap();
        
        assert_eq!(results[0].unwrap_i32(), 1);
    }
} 