use std::fs;
use std::path::{Path, PathBuf};
use crate::parser::CleanParser;
use crate::semantic::SemanticAnalyzer;
use crate::codegen::CodeGenerator;
use crate::error::CompilerError;
use crate::runtime::CleanRuntime;

/// Test result for a single test case
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub output: String,
}

/// Test suite containing multiple test cases
#[derive(Debug)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestResult>,
    pub total_time_ms: u64,
}

/// Main test runner for Clean Language
pub struct CleanTestRunner {
    runtime: CleanRuntime,
    test_directories: Vec<PathBuf>,
    verbose: bool,
}

impl CleanTestRunner {
    /// Create a new test runner
    pub fn new() -> Result<Self, CompilerError> {
        Ok(Self {
            runtime: CleanRuntime::new()?,
            test_directories: vec![
                PathBuf::from("tests/stdlib"),
                PathBuf::from("tests/runtime"),
                PathBuf::from("tests/integration"),
                PathBuf::from("examples"),
            ],
            verbose: false,
        })
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Add a test directory
    pub fn add_test_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.test_directories.push(path.as_ref().to_path_buf());
        self
    }

    /// Run all tests and return results
    pub async fn run_all_tests(&self) -> Result<Vec<TestSuite>, CompilerError> {
        let mut test_suites = Vec::new();

        // Run built-in test suites
        test_suites.push(self.run_stdlib_tests().await?);
        test_suites.push(self.run_runtime_tests().await?);
        test_suites.push(self.run_method_style_tests().await?);
        test_suites.push(self.run_type_conversion_tests().await?);

        // Discover and run file-based tests
        for test_dir in &self.test_directories {
            if test_dir.exists() {
                let suite = self.run_directory_tests(test_dir).await?;
                test_suites.push(suite);
            }
        }

        Ok(test_suites)
    }

    /// Run standard library tests
    async fn run_stdlib_tests(&self) -> Result<TestSuite, CompilerError> {
        let start_time = std::time::Instant::now();
        let mut tests = Vec::new();

        // Array tests
        tests.push(self.run_test("Array.length", r#"
function start()
	integer[] numbers = [1, 2, 3, 4, 5]
	integer len = length(numbers)
	print("Array length: ")
	print(len.toString())
	mustBeEqual(len, 5)
        "#).await);

        tests.push(self.run_test("Array.push", r#"
function start()
	integer[] numbers = [1, 2, 3]
	push(numbers, 4)
	integer len = length(numbers)
	mustBeEqual(len, 4)
	print("✅ Array.push test passed")
        "#).await);

        tests.push(self.run_test("Array.contains", r#"
function start()
	string[] fruits = ["apple", "banana", "orange"]
	boolean hasApple = contains(fruits, "apple")
	boolean hasGrape = contains(fruits, "grape")
	mustBeTrue(hasApple)
	mustBeFalse(hasGrape)
	print("✅ Array.contains test passed")
        "#).await);

        // String tests
        tests.push(self.run_test("String.length", r#"
function start()
	string text = "Hello, World!"
	integer len = length(text)
	mustBeEqual(len, 13)
	print("✅ String.length test passed")
        "#).await);

        tests.push(self.run_test("String.concat", r#"
function start()
	string result = concat("Hello", " World")
	mustBeEqual(result, "Hello World")
	print("✅ String.concat test passed")
        "#).await);

        tests.push(self.run_test("String.substring", r#"
function start()
	string text = "Hello, World!"
	string sub = substring(text, 0, 5)
	mustBeEqual(sub, "Hello")
	print("✅ String.substring test passed")
        "#).await);

        // Math tests
        tests.push(self.run_test("Math.abs", r#"
function start()
	integer result1 = abs(-42)
	integer result2 = abs(42)
	mustBeEqual(result1, 42)
	mustBeEqual(result2, 42)
	print("✅ Math.abs test passed")
        "#).await);

        tests.push(self.run_test("Math.max", r#"
function start()
	integer result = max(10, 20)
	mustBeEqual(result, 20)
	print("✅ Math.max test passed")
        "#).await);

        let total_time = start_time.elapsed().as_millis() as u64;
        Ok(TestSuite {
            name: "Standard Library Tests".to_string(),
            tests,
            total_time_ms: total_time,
        })
    }

    /// Run runtime functionality tests
    async fn run_runtime_tests(&self) -> Result<TestSuite, CompilerError> {
        let start_time = std::time::Instant::now();
        let mut tests = Vec::new();

        tests.push(self.run_test("Basic Print", r#"
            function start()
                print("Hello, Clean Language!")
                print("Runtime test successful")
        "#).await);

        tests.push(self.run_test("Variable Declaration", r#"
            function start()
                integer number = 42
                string text = "Hello"
                boolean flag = true
                print("Variables declared successfully")
        "#).await);

        tests.push(self.run_test("Arithmetic Operations", r#"
            function start()
                integer a = 10
                integer b = 5
                integer sum = a + b
                integer diff = a - b
                integer product = a * b
                integer quotient = a / b
                mustBeEqual(sum, 15)
                mustBeEqual(diff, 5)
                mustBeEqual(product, 50)
                mustBeEqual(quotient, 2)
                print("✅ Arithmetic operations test passed")
        "#).await);

        tests.push(self.run_test("String Operations", r#"
            function start()
                string greeting = "Hello"
                string name = "World"
                string message = greeting + ", " + name + "!"
                mustBeEqual(message, "Hello, World!")
                print("✅ String operations test passed")
        "#).await);

        let total_time = start_time.elapsed().as_millis() as u64;
        Ok(TestSuite {
            name: "Runtime Tests".to_string(),
            tests,
            total_time_ms: total_time,
        })
    }

    /// Run method-style syntax tests
    async fn run_method_style_tests(&self) -> Result<TestSuite, CompilerError> {
        let start_time = std::time::Instant::now();
        let mut tests = Vec::new();

        tests.push(self.run_test("Method-style length", r#"
            function start()
                string text = "Hello"
                integer len = text.length()
                mustBeEqual(len, 5)
                print("✅ Method-style length test passed")
        "#).await);

        tests.push(self.run_test("Method-style isEmpty", r#"
            function start()
                string empty = ""
                string notEmpty = "Hello"
                boolean isEmpty1 = empty.isEmpty()
                boolean isEmpty2 = notEmpty.isEmpty()
                mustBeTrue(isEmpty1)
                mustBeFalse(isEmpty2)
                print("✅ Method-style isEmpty test passed")
        "#).await);

        tests.push(self.run_test("Method-style keepBetween", r#"
            function start()
                integer value = 150
                integer clamped = value.keepBetween(0, 100)
                mustBeEqual(clamped, 100)
                print("✅ Method-style keepBetween test passed")
        "#).await);

        tests.push(self.run_test("Method chaining", r#"
            function start()
                integer number = 42
                string result = number.toFloat().toString()
                print("Chained result: ")
                print(result)
                print("✅ Method chaining test passed")
        "#).await);

        let total_time = start_time.elapsed().as_millis() as u64;
        Ok(TestSuite {
            name: "Method-Style Syntax Tests".to_string(),
            tests,
            total_time_ms: total_time,
        })
    }

    /// Run type conversion tests
    async fn run_type_conversion_tests(&self) -> Result<TestSuite, CompilerError> {
        let start_time = std::time::Instant::now();
        let mut tests = Vec::new();

        tests.push(self.run_test("Integer to String", r#"
            function start()
                integer number = 42
                string text = number.toString()
                print("Number as string: ")
                print(text)
                print("✅ Integer to string test passed")
        "#).await);

        tests.push(self.run_test("Float to String", r#"
            function start()
                float pi = 3.14159
                string text = pi.toString()
                print("Pi as string: ")
                print(text)
                print("✅ Float to string test passed")
        "#).await);

        tests.push(self.run_test("Boolean to String", r#"
            function start()
                boolean flag = true
                string text = flag.toString()
                print("Boolean as string: ")
                print(text)
                print("✅ Boolean to string test passed")
        "#).await);

        tests.push(self.run_test("Type conversion chaining", r#"
            function start()
                integer value = 100
                string result = value.toFloat().toString()
                print("Conversion chain result: ")
                print(result)
                print("✅ Type conversion chaining test passed")
        "#).await);

        let total_time = start_time.elapsed().as_millis() as u64;
        Ok(TestSuite {
            name: "Type Conversion Tests".to_string(),
            tests,
            total_time_ms: total_time,
        })
    }

    /// Run tests from a directory
    async fn run_directory_tests(&self, dir: &Path) -> Result<TestSuite, CompilerError> {
        let start_time = std::time::Instant::now();
        let mut tests = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("clean") {
                    if let Ok(source) = fs::read_to_string(&path) {
                        let test_name = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        tests.push(self.run_test(&test_name, &source).await);
                    }
                }
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;
        Ok(TestSuite {
            name: format!("Directory Tests: {}", dir.display()),
            tests,
            total_time_ms: total_time,
        })
    }

    /// Run a single test
    async fn run_test(&self, name: &str, source: &str) -> TestResult {
        let start_time = std::time::Instant::now();
        
        match self.compile_and_run(source).await {
            Ok(output) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                TestResult {
                    name: name.to_string(),
                    passed: true,
                    error: None,
                    execution_time_ms: execution_time,
                    output,
                }
            }
            Err(error) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                TestResult {
                    name: name.to_string(),
                    passed: false,
                    error: Some(error.to_string()),
                    execution_time_ms: execution_time,
                    output: String::new(),
                }
            }
        }
    }

    /// Compile and run Clean Language source code
    async fn compile_and_run(&self, source: &str) -> Result<String, CompilerError> {
        // Parse the program
        let program = CleanParser::parse_program(source)?;

        // Type check the program
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.check(&program)?;

        // Generate WebAssembly code
        let mut codegen = CodeGenerator::new();
        let wasm_bytes = codegen.generate(&program)?;

        // Execute using the enhanced runtime
        self.runtime.execute_async(&wasm_bytes).await?;

        Ok("Program executed successfully".to_string())
    }

    /// Print test results
    pub fn print_results(&self, test_suites: &[TestSuite]) {
        let mut total_tests = 0;
        let mut total_passed = 0;
        let mut total_time = 0;

        println!("\n🧪 Clean Language Test Runner Results");
        println!("=====================================");

        for suite in test_suites {
            let passed = suite.tests.iter().filter(|t| t.passed).count();
            let failed = suite.tests.len() - passed;
            
            total_tests += suite.tests.len();
            total_passed += passed;
            total_time += suite.total_time_ms;

            println!("\n📋 {}", suite.name);
            println!("   ✅ Passed: {} | ❌ Failed: {} | ⏱️  Time: {}ms", 
                     passed, failed, suite.total_time_ms);

            if self.verbose {
                for test in &suite.tests {
                    let status = if test.passed { "✅" } else { "❌" };
                    println!("     {} {} ({}ms)", status, test.name, test.execution_time_ms);
                    
                    if !test.passed {
                        if let Some(error) = &test.error {
                            println!("        Error: {}", error);
                        }
                    }
                }
            } else {
                // Show failed tests even in non-verbose mode
                for test in &suite.tests {
                    if !test.passed {
                        println!("     ❌ {} - {}", test.name, 
                                test.error.as_deref().unwrap_or("Unknown error"));
                    }
                }
            }
        }

        println!("\n🎯 Overall Results");
        println!("==================");
        println!("Total Tests: {}", total_tests);
        println!("Passed: {} ({}%)", total_passed, 
                 if total_tests > 0 { (total_passed * 100) / total_tests } else { 0 });
        println!("Failed: {}", total_tests - total_passed);
        println!("Total Time: {}ms", total_time);

        if total_passed == total_tests {
            println!("\n🎉 All tests passed! Clean Language is working perfectly!");
        } else {
            println!("\n⚠️  Some tests failed. Please review the errors above.");
        }
    }
}

/// Convenience function to run all tests
pub async fn run_all_tests() -> Result<(), CompilerError> {
    let runner = CleanTestRunner::new()?.verbose();
    let results = runner.run_all_tests().await?;
    runner.print_results(&results);
    Ok(())
}

/// Convenience function to run tests with custom configuration
pub async fn run_tests_with_config(verbose: bool, test_dirs: Vec<&str>) -> Result<(), CompilerError> {
    let mut runner = CleanTestRunner::new()?;
    
    if verbose {
        runner = runner.verbose();
    }
    
    for dir in test_dirs {
        runner = runner.add_test_directory(dir);
    }
    
    let results = runner.run_all_tests().await?;
    runner.print_results(&results);
    Ok(())
} 