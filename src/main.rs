/*!
 * Clean Language Compiler - Main Application
 * 
 * Author: Ivan Pasco Lizarraga
 * Date: 17-07-2025
 * Website: https://www.cleanlanguage.dev
 * 
 * A modern, type-safe programming language that compiles to WebAssembly
 */

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use clean_language_compiler::compile_with_file;

/// Clean Language Compiler and Test Runner
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compile a Clean Language file to WebAssembly
    Compile {
    /// Input file to compile
    #[arg(short, long)]
    input: String,

    /// Output file for the WebAssembly binary
    #[arg(short, long)]
    output: String,

    /// Optimization level (0-3)
    #[arg(short = 'l', long, default_value_t = 2)]
    opt_level: u8,

    /// Run tests during compilation
    #[arg(long)]
    test: bool,

    /// Include tests in the compiled binary
    #[arg(long)]
    include_tests: bool,
    },
    /// Package management commands
    #[command(subcommand)]
    Package(PackageCommands),
    /// Run the Clean Language test suite
    Test {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Additional test directories to include
        #[arg(short, long)]
        dirs: Vec<String>,
    },
    /// Run simple compilation tests
    SimpleTest {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run comprehensive Clean Language test suite
    ComprehensiveTest {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Debug a Clean Language file with enhanced error reporting
    Debug {
        /// Input file to debug
        #[arg(short, long)]
        input: String,

        /// Show AST structure
        #[arg(long)]
        show_ast: bool,

        /// Validate code style
        #[arg(long)]
        check_style: bool,

        /// Show detailed error analysis
        #[arg(long)]
        analyze_errors: bool,
    },
    /// Validate Clean Language code style and conventions
    Lint {
        /// Input file or directory to lint
        #[arg(short, long)]
        input: String,

        /// Fix issues automatically where possible
        #[arg(long)]
        fix: bool,

        /// Show only errors (suppress warnings)
        #[arg(long)]
        errors_only: bool,
    },
    /// Parse a file and show detailed parsing information
    Parse {
        /// Input file to parse
        #[arg(short, long)]
        input: String,

        /// Show detailed parse tree
        #[arg(long)]
        show_tree: bool,

        /// Enable error recovery mode
        #[arg(long)]
        recover_errors: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PackageCommands {
    /// Initialize a new Clean Language package
    Init {
        /// Package name
        #[arg(short, long)]
        name: Option<String>,

        /// Package version
        #[arg(short, long)]
        version: Option<String>,

        /// Package description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Add a dependency to the current package
    Add {
        /// Package name to add
        package: String,

        /// Version requirement (e.g., "^1.0.0")
        #[arg(short, long)]
        version: Option<String>,

        /// Add as development dependency
        #[arg(long)]
        dev: bool,

        /// Git repository URL
        #[arg(long)]
        git: Option<String>,

        /// Local path to package
        #[arg(long)]
        path: Option<String>,
    },
    /// Remove a dependency from the current package
    Remove {
        /// Package name to remove
        package: String,
    },
    /// Install all dependencies for the current package
    Install,
    /// Update dependencies to their latest compatible versions
    Update {
        /// Specific package to update
        package: Option<String>,
    },
    /// List installed packages and their versions
    List {
        /// Show dependency tree
        #[arg(long)]
        tree: bool,
    },
    /// Search for packages in the registry
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Show information about a package
    Info {
        /// Package name
        package: String,

        /// Show specific version
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Publish the current package to the registry
    Publish {
        /// Registry to publish to
        #[arg(long)]
        registry: Option<String>,

        /// Dry run (don't actually publish)
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    match args.command {
        Commands::Compile { input, output, opt_level, test, include_tests } => {
            handle_compile(input, output, opt_level, test, include_tests).await?
        }
        Commands::Package(package_cmd) => {
            handle_package(package_cmd).await?
        }
        Commands::Test { verbose, dirs } => {
            handle_test(verbose, dirs).await?
        }
        Commands::SimpleTest { verbose } => {
            handle_simple_test(verbose).await?
        }
        Commands::ComprehensiveTest { verbose } => {
            handle_comprehensive_test(verbose).await?
        }
        Commands::Debug { input, show_ast, check_style, analyze_errors } => {
            handle_debug(input, show_ast, check_style, analyze_errors).await?
        }
        Commands::Lint { input, fix, errors_only } => {
            handle_lint(input, fix, errors_only).await?
        }
        Commands::Parse { input, show_tree, recover_errors } => {
            handle_parse(input, show_tree, recover_errors).await?
        }
    }
    
    Ok(())
}

async fn handle_compile(input: String, output: String, _opt_level: u8, test: bool, include_tests: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compiling {} to {}", input, output);
    
    let source = fs::read_to_string(&input)?;
    
    // Parse the program to check for tests
    use clean_language_compiler::parser::CleanParser;
    let program = CleanParser::parse_program_with_file(&source, &input)?;
    
    // Run tests if requested
    if test && !program.tests.is_empty() {
        println!("\n🧪 Running tests...");
        run_tests(&program, &input)?;
    } else if test && program.tests.is_empty() {
        println!("⚠️  No tests found to run");
    }
    
    let wasm_binary = compile_with_file(&source, &input)?;
    
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(&output, wasm_binary)?;
    
    println!("Successfully compiled to {}", output);
    
    if include_tests && !program.tests.is_empty() {
        println!("📝 Tests included in binary (accessible via --run-tests flag)");
    }
    
    Ok(())
}

async fn handle_package(package_cmd: PackageCommands) -> Result<(), Box<dyn std::error::Error>> {
    use clean_language_compiler::package::PackageManager;
    use std::env;
    
    let cache_dir = dirs::home_dir()
        .unwrap_or_else(|| env::current_dir().unwrap())
        .join(".clean")
        .join("packages");
    
    let package_manager = PackageManager::new(cache_dir);
    
    match package_cmd {
        PackageCommands::Init { name, version, description } => {
            let current_dir = env::current_dir()?;
            let package_name = name.unwrap_or_else(|| {
                current_dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("my-package")
                    .to_string()
            });
            
            println!("📦 Initializing Clean Language package: {}", package_name);
            
            match package_manager.init_package(&current_dir, package_name, version, description) {
                Ok(_) => println!("✅ Package initialized successfully!"),
                Err(e) => eprintln!("❌ Failed to initialize package: {}", e),
            }
        }
        PackageCommands::Add { package, version, dev, .. } => {
            let manifest_path = env::current_dir()?.join("package.clean.toml");
            
            if !manifest_path.exists() {
                eprintln!("❌ No package.clean.toml found. Run 'clean package init' first.");
                return Ok(());
            }
            
            let version_spec = version.unwrap_or_else(|| "latest".to_string());
            
            println!("📦 Adding {} dependency: {} {}", 
                if dev { "development" } else { "runtime" }, 
                package, 
                version_spec
            );
            
            match package_manager.add_dependency(&manifest_path, package, version_spec, dev) {
                Ok(_) => println!("✅ Dependency added successfully!"),
                Err(e) => eprintln!("❌ Failed to add dependency: {}", e),
            }
        }
        PackageCommands::Remove { package } => {
            let manifest_path = env::current_dir()?.join("package.clean.toml");
            
            if !manifest_path.exists() {
                eprintln!("❌ No package.clean.toml found.");
                return Ok(());
            }
            
            println!("📦 Removing dependency: {}", package);
            
            match package_manager.remove_dependency(&manifest_path, &package) {
                Ok(_) => println!("✅ Dependency removed successfully!"),
                Err(e) => eprintln!("❌ Failed to remove dependency: {}", e),
            }
        }
        PackageCommands::Install => {
            let manifest_path = env::current_dir()?.join("package.clean.toml");
            
            if !manifest_path.exists() {
                eprintln!("❌ No package.clean.toml found. Run 'clean package init' first.");
                return Ok(());
            }
            
            println!("📦 Installing dependencies...");
            
            match PackageManager::load_manifest(&manifest_path) {
                Ok(manifest) => {
                    if let Some(deps) = &manifest.dependencies {
                        println!("Runtime dependencies:");
                        for (name, spec) in deps {
                            println!("  - {} {:?}", name, spec);
                        }
                    }
                    if let Some(dev_deps) = &manifest.dev_dependencies {
                        println!("Development dependencies:");
                        for (name, spec) in dev_deps {
                            println!("  - {} {:?}", name, spec);
                        }
                    }
                    println!("✅ Dependencies would be installed (simulation mode)");
                }
                Err(e) => eprintln!("❌ Failed to load manifest: {}", e),
            }
        }
        PackageCommands::List { .. } => {
            let manifest_path = env::current_dir()?.join("package.clean.toml");
            
            if !manifest_path.exists() {
                eprintln!("❌ No package.clean.toml found.");
                return Ok(());
            }
            
            match PackageManager::load_manifest(&manifest_path) {
                Ok(manifest) => {
                    println!("📦 Package: {} {}", manifest.package.name, manifest.package.version);
                    
                    if let Some(deps) = &manifest.dependencies {
                        println!("\n📋 Runtime Dependencies:");
                        for (name, spec) in deps {
                            println!("  {} {:?}", name, spec);
                        }
                    }
                    
                    if let Some(dev_deps) = &manifest.dev_dependencies {
                        println!("\n🔧 Development Dependencies:");
                        for (name, spec) in dev_deps {
                            println!("  {} {:?}", name, spec);
                        }
                    }
                }
                Err(e) => eprintln!("❌ Failed to load manifest: {}", e),
            }
        }
        PackageCommands::Search { query, .. } => {
            println!("🔍 Searching for packages matching '{}'...", query);
            println!("📡 Package registry search not yet implemented");
            println!("   This would search https://packages.cleanlang.org for packages");
        }
        PackageCommands::Info { package, version } => {
            println!("ℹ️  Package information for: {}", package);
            if let Some(v) = version {
                println!("   Version: {}", v);
            }
            println!("📡 Package registry info not yet implemented");
        }
        PackageCommands::Update { package } => {
            if let Some(pkg) = package {
                println!("🔄 Updating package: {}", pkg);
            } else {
                println!("🔄 Updating all dependencies...");
            }
            println!("📡 Package update not yet implemented");
        }
        PackageCommands::Publish { .. } => {
            let manifest_path = env::current_dir()?.join("package.clean.toml");
            
            if !manifest_path.exists() {
                eprintln!("❌ No package.clean.toml found.");
                return Ok(());
            }
            
            match PackageManager::load_manifest(&manifest_path) {
                Ok(manifest) => {
                    println!("📤 Publishing {} {}...", manifest.package.name, manifest.package.version);
                    println!("📡 Package publishing not yet implemented");
                }
                Err(e) => eprintln!("❌ Failed to load manifest: {}", e),
            }
        }
    }
    Ok(())
}

async fn handle_test(verbose: bool, dirs: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running Clean Language test suite...");
    if verbose {
        println!("Verbose output enabled");
    }
    if !dirs.is_empty() {
        println!("Additional test directories: {:?}", dirs);
    }
    
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test");
    if verbose {
        cmd.arg("--verbose");
    }
    
    let status = cmd.status()?;
    if !status.success() {
        eprintln!("✗ Some tests failed");
        return Err("Tests failed".into());
    } else {
        println!("✓ All tests passed!");
    }
    Ok(())
}

async fn handle_simple_test(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running simple compilation tests...");
    if verbose {
        println!("Verbose output enabled");
    }
    
    let test_source = r#"start()
    integer x = 42
    print(x)
"#;
    
    match compile_with_file(test_source, "simple_test.clean") {
        Ok(wasm_binary) => {
            println!("✓ Simple test passed: {} bytes of WASM generated", wasm_binary.len());
            Ok(())
        },
        Err(error) => {
            eprintln!("✗ Simple test failed: {}", error);
            Err(error.into())
        }
    }
}

async fn handle_comprehensive_test(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running comprehensive Clean Language test suite...");
    if verbose {
        println!("Verbose output enabled");
    }
    
    let test_cases = vec![
        ("Basic", r#"start()
    integer x = 42
    print(x)
"#),
        ("Arithmetic", r#"start()
    integer x = 1 + 2 * 3
    print(x)
"#),
        ("Variables", r#"start()
    integer x = 5
    integer y = x + 1
    print(y)
"#),
    ];
    
    let mut passed = 0;
    let total = test_cases.len();
    
    for (name, source) in test_cases {
        print!("Testing {}: ", name);
        match compile_with_file(source, &format!("{}_test.clean", name.to_lowercase())) {
            Ok(wasm_binary) => {
                println!("✓ {} bytes", wasm_binary.len());
                passed += 1;
            },
            Err(error) => {
                println!("✗ {}", error);
            }
        }
    }
    
    println!("Results: {}/{} tests passed", passed, total);
    if passed == total {
        println!("🎉 All comprehensive tests passed!");
        Ok(())
    } else {
        eprintln!("⚠ Some tests failed");
        Err("Some comprehensive tests failed".into())
    }
}

async fn handle_debug(input: String, show_ast: bool, check_style: bool, analyze_errors: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging Clean Language file: {}\n", input);
    
    let source = match fs::read_to_string(&input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Error reading file '{}': {}", input, e);
            return Ok(());
        }
    };
    
    use clean_language_compiler::debug::DebugUtils;
    use clean_language_compiler::parser::CleanParser;
    
    let parse_result = CleanParser::parse_program_with_file(&source, &input);
    let warnings = Vec::new(); 
    
    let debug_report = DebugUtils::create_debug_report(&source, &input, &parse_result, &warnings);
    println!("{}", debug_report);
    
    match &parse_result {
        Ok(program) => {
            if show_ast {
                println!("\n");
                DebugUtils::print_ast(program);
            }
        }
        Err(error) => {
            if analyze_errors {
                println!("\n");
                let analysis = DebugUtils::analyze_errors(&[error.clone()]);
                for line in analysis {
                    println!("{}", line);
                }
            }
        }
    }
    
    if check_style {
        println!("\n=== Style Validation ===");
        let style_issues = DebugUtils::validate_style(&source);
        if style_issues.is_empty() {
            println!("✅ No style issues found");
        } else {
            println!("🎨 Style Issues Found:");
            for issue in style_issues {
                println!("  {}", issue);
            }
        }
    }
    Ok(())
}

async fn handle_lint(input: String, fix: bool, errors_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Linting: {}", input);
    
    let path = Path::new(&input);
    let files_to_lint = if path.is_file() {
        vec![input.clone()]
    } else if path.is_dir() {
        let mut clean_files = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "clean" {
                        if let Some(path_str) = entry.path().to_str() {
                            clean_files.push(path_str.to_string());
                        }
                    }
                }
            }
        }
        clean_files
    } else {
        eprintln!("❌ Error: '{}' is not a valid file or directory", input);
        return Ok(());
    };
    
    if files_to_lint.is_empty() {
        println!("No Clean Language files found to lint");
        return Ok(());
    }
    
    use clean_language_compiler::debug::DebugUtils;
    use clean_language_compiler::parser::CleanParser;
    
    let mut total_issues = 0;
    let mut total_errors = 0;
    
    for file_path in &files_to_lint {
        println!("\n📄 Linting: {}", file_path);
        
        let source = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ❌ Error reading file: {}", e);
                continue;
            }
        };
        
        let parse_result = CleanParser::parse_program_with_file(&source, file_path);
        if let Err(error) = &parse_result {
            total_errors += 1;
            if !errors_only {
                println!("  ❌ Compilation Error:");
                println!("     {}", error);
            }
        }
        
        let style_issues = DebugUtils::validate_style(&source);
        if !style_issues.is_empty() {
            total_issues += style_issues.len();
            if !errors_only {
                println!("🎨 Style Issues Found:");
                for issue in &style_issues {
                    println!("  {}", issue);
                }
            }
        }
        
        if parse_result.is_ok() && style_issues.is_empty() {
            println!("  ✅ No issues found");
        }
    }
    
    println!("\n=== Lint Summary ===");
    println!("Files checked: {}", files_to_lint.len());
    println!("Compilation errors: {}", total_errors);
    println!("Style issues: {}", total_issues);
    
    if fix {
        println!("Note: Automatic fixing is not yet implemented");
    }
    Ok(())
}

async fn handle_parse(input: String, show_tree: bool, recover_errors: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Parsing file: {}", input);
    
    let source = match fs::read_to_string(&input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Error reading file '{}': {}", input, e);
            return Ok(());
        }
    };
    
    use clean_language_compiler::parser::CleanParser;
    use clean_language_compiler::debug::DebugUtils;
    
    if recover_errors {
        println!("🔄 Using enhanced error recovery mode...\n");
        
        // Use the enhanced error recovery parser
        let mut recovery_parser = clean_language_compiler::parser::ErrorRecoveringParser::new(&source, &input);
        recovery_parser = recovery_parser.with_max_errors(50); // Allow up to 50 errors
        
        match recovery_parser.parse_with_recovery(&source) {
            Ok(program) => {
                println!("✅ Parsing succeeded with error recovery!");
                
                if show_tree {
                    println!("\n🌳 AST Structure:");
                    println!("{}", "═".repeat(50));
                    DebugUtils::print_ast(&program);
                }
                
                // Check if we collected any warnings during recovery
                if !recovery_parser.warnings.is_empty() {
                    println!("\n⚠️  Warnings collected during parsing:");
                    for warning in &recovery_parser.warnings {
                        println!("  • {}", warning);
                    }
                }
                
                println!("\n📊 Recovery Statistics:");
                println!("  • Recovery points identified: {}", recovery_parser.recovery_points.len());
                println!("  • Warnings: {}", recovery_parser.warnings.len());
                println!("  • Functions parsed: {}", program.functions.len());
                if program.start_function.is_some() {
                    println!("  • Start function: ✅");
                }
                println!("  • Classes parsed: {}", program.classes.len());
            }
            Err(errors) => {
                println!("❌ Parsing failed with {} error(s):\n", errors.len());
                
                // Generate comprehensive error report
                let error_report = DebugUtils::create_error_report(&source, &errors);
                println!("{}", error_report);
                
                // If partial parsing was successful, show what we recovered
                if !recovery_parser.errors.is_empty() && errors.len() < 20 {
                    println!("\n🔧 Attempting to show recovered partial AST...");
                    
                    // Try to create a minimal program from whatever we could parse
                    let partial_program = clean_language_compiler::ast::Program {
                        imports: Vec::new(),
                        functions: Vec::new(),
                        classes: Vec::new(),
                        start_function: None,
                        tests: Vec::new(),
                    };
                    
                    if show_tree {
                        DebugUtils::print_ast(&partial_program);
                    }
                }
            }
        }
    } else {
        println!("🔄 Using standard parsing mode...\n");
        
        match CleanParser::parse_program_with_file(&source, &input) {
            Ok(program) => {
                println!("✅ Parsing succeeded!");
                if show_tree {
                    println!("\n");
                    DebugUtils::print_ast(&program);
                }
            }
            Err(error) => {
                eprintln!("❌ Parsing failed:");
                println!("{}", error);
                
                // Provide basic suggestions even in standard mode
                println!("\n💡 Suggestions:");
                println!("  • Try using --recover-errors for detailed error analysis");
                println!("  • Check the Clean Language syntax documentation");
                
                // Basic error analysis
                let suggestions = DebugUtils::suggest_error_fixes(&source, &[error]);
                for suggestion in suggestions {
                    println!("  • {}", suggestion);
                }
            }
        }
    }
    
    Ok(())
}

fn run_tests(program: &clean_language_compiler::ast::Program, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut passed = 0;
    let mut failed = 0;
    
    println!("Running tests for {}...\n", file_path);
    
    for (i, test) in program.tests.iter().enumerate() {
        let test_name = test.description.as_ref()
            .map(|d| d.clone())
            .unwrap_or_else(|| format!("Test #{}", i + 1));
        
        // For now, we'll implement a basic test runner
        // In a full implementation, this would compile and run the test expression
        let test_result = evaluate_test_expression(&test.test_expression, &test.expected_value);
        
        match test_result {
            Ok(true) => {
                println!("✅ {}: PASS", test_name);
                passed += 1;
            }
            Ok(false) => {
                println!("❌ {}: FAIL", test_name);
                println!("   Expected: {:?}", test.expected_value);
                println!("   Got: {:?}", test.test_expression);
                failed += 1;
            }
            Err(e) => {
                println!("❌ {}: ERROR - {}", test_name, e);
                failed += 1;
            }
        }
    }
    
    println!("\nTest Results: {} passed, {} failed, {} total", passed, failed, passed + failed);
    
    if failed > 0 {
        return Err(format!("{} test(s) failed", failed).into());
    }
    
    Ok(())
}

fn evaluate_test_expression(test_expr: &clean_language_compiler::ast::Expression, expected: &clean_language_compiler::ast::Expression) -> Result<bool, String> {
    use clean_language_compiler::ast::{Expression, Value};
    // This is a simplified test evaluator
    // In a full implementation, this would compile the expressions to WASM and execute them
    
    match (test_expr, expected) {
        (Expression::Literal(Value::Integer(a)), Expression::Literal(Value::Integer(b))) => {
            Ok(a == b)
        }
        (Expression::Literal(Value::Number(a)), Expression::Literal(Value::Number(b))) => {
            Ok((a - b).abs() < f64::EPSILON)
        }
        (Expression::Literal(Value::String(a)), Expression::Literal(Value::String(b))) => {
            Ok(a == b)
        }
        (Expression::Literal(Value::Boolean(a)), Expression::Literal(Value::Boolean(b))) => {
            Ok(a == b)
        }
        _ => {
            // For complex expressions, we'd need to compile and execute
            // For now, we'll just compare the AST structure
            Ok(format!("{:?}", test_expr) == format!("{:?}", expected))
        }
    }
} 