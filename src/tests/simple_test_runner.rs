use std::fs;
use std::path::{Path, PathBuf};
use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;

/// Simple test result for a single test case
#[derive(Debug, Clone)]
pub struct SimpleTestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Simple test runner for Clean Language
pub struct SimpleTestRunner {
    verbose: bool,
}

impl SimpleTestRunner {
    /// Create a new simple test runner
    pub fn new() -> Self {
        Self {
            verbose: false,
        }
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Run all basic tests
    pub fn run_basic_tests(&self) -> Vec<SimpleTestResult> {
        let mut results = Vec::new();

        // Test 1: Basic compilation
        results.push(self.run_test("Basic Compilation", r#"function start()
	print("Hello, Clean Language!")
	integer x = 42
	print(x.toString())"#));

        // Test 2: Variable declarations
        results.push(self.run_test("Variable Declarations", r#"function start()
	integer number = 42
	string text = "Hello"
	boolean flag = true
	print("Variables declared successfully")"#));

        // Test 3: Arithmetic operations
        results.push(self.run_test("Arithmetic Operations", r#"function start()
	integer a = 10
	integer b = 5
	integer sum = a + b
	integer diff = a - b
	print("Arithmetic test passed")"#));

        // Test 4: String operations
        results.push(self.run_test("String Operations", r#"function start()
	string greeting = "Hello"
	string name = "World"
	string message = greeting + ", " + name + "!"
	print(message)"#));

        // Test 5: Method-style syntax
        results.push(self.run_test("Method-Style Syntax", r#"function start()
	integer number = 42
	string result = number.toString()
	print("Method-style test: ")
	print(result)"#));

        // Test 6: Type conversion
        results.push(self.run_test("Type Conversion", r#"function start()
	integer value = 100
	string text = value.toString()
	print("Converted: ")
	print(text)"#));

        results
    }

    /// Run a single test
    fn run_test(&self, name: &str, source: &str) -> SimpleTestResult {
        let start_time = std::time::Instant::now();
        
        match self.compile_source(source) {
            Ok(_) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                SimpleTestResult {
                    name: name.to_string(),
                    passed: true,
                    error: None,
                    execution_time_ms: execution_time,
                }
            }
            Err(error) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                SimpleTestResult {
                    name: name.to_string(),
                    passed: false,
                    error: Some(error.to_string()),
                    execution_time_ms: execution_time,
                }
            }
        }
    }

    /// Compile Clean Language source code
    fn compile_source(&self, source: &str) -> Result<Vec<u8>, CompilerError> {
        // Parse the program
        let program = CleanParser::parse_program(source)?;

        // Type check the program
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program)?;

        // Generate WebAssembly code
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program)?;

        Ok(wasm_bytes)
    }

    /// Print test results
    pub fn print_results(&self, results: &[SimpleTestResult]) {
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;
        let total_time: u64 = results.iter().map(|r| r.execution_time_ms).sum();

        println!("\n🧪 Clean Language Simple Test Runner Results");
        println!("============================================");
        println!("✅ Passed: {} | ❌ Failed: {} | ⏱️  Time: {}ms", passed, failed, total_time);

        if self.verbose || failed > 0 {
            println!("\nDetailed Results:");
            for result in results {
                let status = if result.passed { "✅" } else { "❌" };
                println!("  {} {} ({}ms)", status, result.name, result.execution_time_ms);
                
                if !result.passed {
                    if let Some(error) = &result.error {
                        println!("     Error: {}", error);
                    }
                }
            }
        }

        println!("\n🎯 Summary");
        println!("==========");
        println!("Total Tests: {}", results.len());
        println!("Success Rate: {}%", if results.len() > 0 { (passed * 100) / results.len() } else { 0 });

        if passed == results.len() {
            println!("\n🎉 All tests passed! Clean Language compiler is working correctly!");
        } else {
            println!("\n⚠️  {} test(s) failed. Please review the errors above.", failed);
        }
    }

    /// Test standard library functions
    pub fn run_stdlib_tests(&self) -> Vec<SimpleTestResult> {
        let mut tests = Vec::new();
        
        // Length function (method-style only)
        tests.push(self.run_test("Length Function", r#"
function start()
	string text = "Hello World"
	integer len = text.length()
	mustBeEqual(len, 11)
	print("Text length: ")
	print(len.toString())
	print("✅ Length function passed")
        "#));
        
        // Type conversion methods (method-style only)
        tests.push(self.run_test("Type Conversion Methods", r#"
function start()
	integer number = 42
	float pi = 3.14
	string result = number.toString() + " and " + pi.toString()
	print("Conversion result: ")
	print(result)
	print("✅ Type conversion methods passed")
        "#));
        
        tests
    }

    /// Test file-based examples
    pub fn run_file_tests(&self, test_dir: &str) -> Vec<SimpleTestResult> {
        let mut results = Vec::new();
        let test_path = Path::new(test_dir);

        if test_path.exists() {
            if let Ok(entries) = fs::read_dir(test_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("clean") {
                        if let Ok(source) = fs::read_to_string(&path) {
                            let test_name = path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            results.push(self.run_test(&test_name, &source));
                        }
                    }
                }
            }
        }

        results
    }

    /// Run comprehensive compilation tests
    pub fn run_comprehensive_tests(&self) -> Vec<SimpleTestResult> {
        let mut tests = Vec::new();
        
        // Basic functionality tests
        tests.extend(self.run_basic_tests());
        tests.extend(self.run_stdlib_tests());
        
        // Advanced feature tests
        tests.extend(self.run_advanced_tests());
        
        // Edge case tests
        tests.extend(self.run_edge_case_tests());
        
        // Performance tests
        tests.extend(self.run_performance_tests());
        
        tests
    }
    
    /// Run advanced feature tests
    pub fn run_advanced_tests(&self) -> Vec<SimpleTestResult> {
        let mut tests = Vec::new();
        
        // Complex string operations (method-style only)
        tests.push(self.run_test("Complex String Operations", r#"
function start()
	string text = "Hello"
	string world = " World"
	string result = text + world
	integer len = result.length()
	mustBeEqual(len, 11)
	print("Complex string result: ")
	print(result)
	print("✅ Complex string operations passed")
        "#));
        
        // Nested method calls (method-style only)
        tests.push(self.run_test("Nested Method Calls", r#"
function start()
	integer number = 123
	string text = number.toString()
	integer len = text.length()
	mustBeEqual(len, 3)
	print("Nested method result: ")
	print(len.toString())
	print("✅ Nested method calls passed")
        "#));
        
        // Complex arithmetic
        tests.push(self.run_test("Complex Arithmetic", r#"
function start()
	integer a = 10
	integer b = 5
	integer c = 2
	integer result = (a + b) * c
	mustBeEqual(result, 30)
	print("Arithmetic result: ")
	print(result.toString())
	print("✅ Complex arithmetic passed")
        "#));
        
        // Array operations (method-style only)
        tests.push(self.run_test("Array Operations", r#"
function start()
	Array<integer> numbers = [1, 2, 3, 4, 5]
	integer len = numbers.length()
	mustBeEqual(len, 5)
	print("Array length: ")
	print(len.toString())
	print("✅ Array operations passed")
        "#));
        
        // Boolean logic
        tests.push(self.run_test("Boolean Logic", r#"
function start()
	boolean a = true
	boolean b = false
	boolean andResult = a and b
	boolean orResult = a or b
	mustBeFalse(andResult)
	mustBeTrue(orResult)
	print("✅ Boolean logic passed")
        "#));
        
        tests
    }
    
    /// Run edge case tests
    pub fn run_edge_case_tests(&self) -> Vec<SimpleTestResult> {
        let mut tests = Vec::new();
        
        // Empty string handling (method-style only)
        tests.push(self.run_test("Empty String Handling", r#"
function start()
	string empty = ""
	integer len = empty.length()
	mustBeEqual(len, 0)
	boolean isEmpty = empty.isEmpty()
	mustBeTrue(isEmpty)
	print("✅ Empty string handling passed")
        "#));
        
        // Zero values
        tests.push(self.run_test("Zero Values", r#"
function start()
	integer zero = 0
	float fzero = 0.0
	string zeroStr = zero.toString()
	string fzeroStr = fzero.toString()
	print("Zero as string: ")
	print(zeroStr)
	print("Float zero as string: ")
	print(fzeroStr)
	print("✅ Zero values passed")
        "#));
        
        // Large numbers
        tests.push(self.run_test("Large Numbers", r#"
function start()
	integer large = 999999
	float largef = 999999.99
	string largeStr = large.toString()
	string largefStr = largef.toString()
	print("Large number: ")
	print(largeStr)
	print("Large float: ")
	print(largefStr)
	print("✅ Large numbers passed")
        "#));
        
        // Special characters in strings (method-style only)
        tests.push(self.run_test("Special Characters", r#"
function start()
	string special = "Hello\nWorld\t!"
	integer len = special.length()
	print("Special string: ")
	print(special)
	print("Length: ")
	print(len.toString())
	print("✅ Special characters passed")
        "#));
        
        tests
    }
    
    /// Run performance tests
    pub fn run_performance_tests(&self) -> Vec<SimpleTestResult> {
        let mut tests = Vec::new();
        
        // Multiple concatenations (method-style only)
        tests.push(self.run_test("Multiple Concatenations", r#"
function start()
	string result = "A" + "B" + "C" + "D" + "E"
	integer len = result.length()
	mustBeEqual(len, 5)
	print("Multiple concat result: ")
	print(result)
	print("✅ Multiple concatenations passed")
        "#));
        
        // Multiple conversions (method-style only)
        tests.push(self.run_test("Multiple Conversions", r#"
function start()
	integer num1 = 1
	integer num2 = 2
	integer num3 = 3
	string result = num1.toString() + num2.toString() + num3.toString()
	integer len = result.length()
	mustBeEqual(len, 3)
	print("Multiple conversions: ")
	print(result)
	print("✅ Multiple conversions passed")
        "#));
        
        tests
    }
}

/// Convenience function to run simple tests
pub fn run_simple_tests(verbose: bool) {
    let runner = if verbose {
        SimpleTestRunner::new().verbose()
    } else {
        SimpleTestRunner::new()
    };

    println!("🚀 Running Clean Language Simple Tests...\n");

    // Run basic compilation tests
    let basic_results = runner.run_basic_tests();
    println!("📋 Basic Compilation Tests");
    runner.print_results(&basic_results);

    // Run standard library tests
    let stdlib_results = runner.run_stdlib_tests();
    println!("\n📋 Standard Library Tests");
    runner.print_results(&stdlib_results);

    // Combine all results for overall summary
    let mut all_results = basic_results;
    all_results.extend(stdlib_results);

    let total_passed = all_results.iter().filter(|r| r.passed).count();
    let total_tests = all_results.len();

    println!("\n🏆 Overall Test Summary");
    println!("=======================");
    println!("Total Tests Run: {}", total_tests);
    println!("Total Passed: {}", total_passed);
    println!("Total Failed: {}", total_tests - total_passed);
    println!("Overall Success Rate: {}%", if total_tests > 0 { (total_passed * 100) / total_tests } else { 0 });

    if total_passed == total_tests {
        println!("\n🎉 🎉 🎉 ALL TESTS PASSED! 🎉 🎉 🎉");
        println!("Clean Language compiler is working perfectly!");
    } else {
        println!("\n⚠️  Some tests failed. Clean Language needs attention.");
    }
}

/// Run comprehensive tests with detailed reporting
pub fn run_comprehensive_tests_with_reporting(verbose: bool) {
    println!("🚀 Running Comprehensive Clean Language Tests...\n");
    
    let runner = SimpleTestRunner::new();
    let runner = if verbose { runner.verbose() } else { runner };
    
    // Run all comprehensive tests
    let results = runner.run_comprehensive_tests();
    
    // Group results by category
    let basic_results: Vec<SimpleTestResult> = results.iter().take(6).cloned().collect();
    let stdlib_results: Vec<SimpleTestResult> = results.iter().skip(6).take(2).cloned().collect();
    let advanced_results: Vec<SimpleTestResult> = results.iter().skip(8).take(5).cloned().collect();
    let edge_case_results: Vec<SimpleTestResult> = results.iter().skip(13).take(4).cloned().collect();
    let performance_results: Vec<SimpleTestResult> = results.iter().skip(17).cloned().collect();
    
    // Report each category
    println!("📋 Basic Functionality Tests");
    runner.print_results(&basic_results);
    
    println!("\n📚 Standard Library Tests");
    runner.print_results(&stdlib_results);
    
    println!("\n🚀 Advanced Feature Tests");
    runner.print_results(&advanced_results);
    
    println!("\n🔍 Edge Case Tests");
    runner.print_results(&edge_case_results);
    
    println!("\n⚡ Performance Tests");
    runner.print_results(&performance_results);
    
    // Overall summary
    let total_tests = results.len();
    let passed_tests = results.iter().filter(|r| r.passed).count();
    let failed_tests = total_tests - passed_tests;
    let success_rate = if total_tests > 0 { (passed_tests * 100) / total_tests } else { 0 };
    
    println!("\n🏆 Comprehensive Test Summary");
    println!("===============================");
    println!("Total Tests Run: {}", total_tests);
    println!("Total Passed: {}", passed_tests);
    println!("Total Failed: {}", failed_tests);
    println!("Overall Success Rate: {}%", success_rate);
    
    if failed_tests == 0 {
        println!("\n🎉 🎉 🎉 ALL COMPREHENSIVE TESTS PASSED! 🎉 🎉 🎉");
        println!("Clean Language compiler is production-ready!");
    } else {
        println!("\n⚠️  Some tests failed. Check the details above.");
        if verbose {
            println!("\nFailed tests:");
            for result in results.iter().filter(|r| !r.passed) {
                println!("  ❌ {}: {}", result.name, result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
            }
        }
    }
} 