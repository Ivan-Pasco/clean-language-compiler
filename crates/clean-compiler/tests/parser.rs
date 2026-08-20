//! Milestone 1 step 5b checks: the parser over the M1 surface — the
//! acceptance-guest shapes (host interface, functions, if/else, calls) and
//! the SYN paths for malformed files.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::lexer::lex;
use clean_compiler::parser::ast::{BaseType, Expr, Item, Stmt};
use clean_compiler::parser::parse;
use clean_compiler_types::codes;

fn parse_source(source: &str) -> (Vec<Item>, Vec<clean_compiler_types::Diagnostic>) {
    let mut sink = DiagnosticSink::new();
    let stream = lex("app/main.cln", source, &mut sink);
    let file = parse(&stream, &mut sink);
    (file.items, sink.into_diagnostics())
}

const HOST_BRIDGE: &str = "\
host interface routing version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function register(m: method, path: string, handlerId: integer:u32, opts: options)
\t\tdescription \"Register one route.\"
";

#[test]
fn host_interface_block_parses() {
    let (items, diagnostics) = parse_source(HOST_BRIDGE);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::HostInterface(hi) = &items[0] else {
        panic!("expected host interface, got {items:?}");
    };
    assert_eq!(hi.name, "routing");
    assert_eq!(hi.version, "0.1.0");
    assert_eq!(hi.worlds, ["server"]);
    assert_eq!(hi.functions.len(), 1);
    let f = &hi.functions[0];
    assert_eq!(f.name, "register");
    assert_eq!(f.description, "Register one route.");
    assert_eq!(f.params.len(), 4);
    assert_eq!(f.params[0].name, "m");
    assert!(matches!(f.params[0].ty.base, BaseType::Named(ref n) if n == "method"));
    assert!(matches!(
        f.params[2].ty.base,
        BaseType::Integer(Some(clean_compiler::parser::ast::IntWidth::U32))
    ));
    assert!(f.ret.is_none(), "no returns clause means void");
}

#[test]
fn kebab_interface_names_rejoin() {
    let source = "\
host interface session-envelope version \"0.1.0\":
\trequires host worlds [\"server\"]
";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::HostInterface(hi) = &items[0] else {
        panic!("expected host interface");
    };
    assert_eq!(hi.name, "session-envelope");
}

#[test]
fn functions_block_with_dispatch_parses() {
    let source = "\
functions:
\tvoid handle(integer handlerId)
\t\tif handlerId == 0
\t\t\tsetStatus(200)
\t\telse if handlerId == 4
\t\t\techo()
\t\telse
\t\t\tsetStatus(404)
";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::Functions(functions) = &items[0] else {
        panic!("expected functions block");
    };
    let f = &functions[0];
    assert_eq!(f.name, "handle");
    assert!(matches!(f.ret.base, BaseType::Void));
    assert_eq!(f.params.len(), 1);
    let Stmt::If { else_ifs, els, .. } = &f.body.statements[0] else {
        panic!("expected if, got {:?}", f.body.statements[0]);
    };
    assert_eq!(else_ifs.len(), 1);
    assert!(els.is_some());
}

#[test]
fn precedence_ladder_shapes_arithmetic() {
    let source = "functions:\n\tinteger f()\n\t\treturn 1 + 2 * 3 == 7 and true\n";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::Functions(functions) = &items[0] else {
        panic!()
    };
    let Stmt::Return {
        value: Some(expr), ..
    } = &functions[0].body.statements[0]
    else {
        panic!()
    };
    // and(eq(add(1, mul(2,3)), 7), true)
    use clean_compiler::parser::ast::BinOp;
    let Expr::Binary {
        op: BinOp::And,
        lhs,
        ..
    } = expr
    else {
        panic!("outermost must be 'and', got {expr:?}");
    };
    let Expr::Binary {
        op: BinOp::Eq,
        lhs: add,
        ..
    } = lhs.as_ref()
    else {
        panic!("then '=='");
    };
    let Expr::Binary {
        op: BinOp::Add,
        rhs: mul,
        ..
    } = add.as_ref()
    else {
        panic!("then '+'");
    };
    assert!(matches!(mul.as_ref(), Expr::Binary { op: BinOp::Mul, .. }));
}

#[test]
fn class_with_fields_parses() {
    let source = "class Options\n\tboolean csrf\n";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::Class(class) = &items[0] else {
        panic!()
    };
    assert_eq!(class.name, "Options");
    assert_eq!(class.fields.len(), 1);
    assert!(matches!(class.fields[0].ty.base, BaseType::Boolean));
}

#[test]
fn top_level_statement_is_syn009() {
    let (_, diagnostics) = parse_source("integer x = 5\n");
    assert_eq!(diagnostics[0].code, codes::SYN009);
    assert!(diagnostics[0]
        .message
        .contains("cannot appear at the top level"));
}

#[test]
fn missing_description_is_syn005() {
    let source = "\
host interface log version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emit(message: string)
";
    let (_, diagnostics) = parse_source(source);
    assert!(diagnostics.iter().any(
        |d| d.code == codes::SYN005 && d.message.contains("missing its mandatory description")
    ));
}

#[test]
fn empty_print_block_is_syn008() {
    let source = "functions:\n\tvoid f()\n\t\tprint:\n\t\t\t// nothing\n";
    let (_, diagnostics) = parse_source(source);
    assert!(diagnostics.iter().any(|d| d.code == codes::SYN008));
}

#[test]
fn parser_recovers_and_reports_multiple_errors() {
    let source = "functions:\n\tvoid f()\n\t\treturn )\n\t\treturn (\n";
    let (_, diagnostics) = parse_source(source);
    assert!(
        diagnostics.len() >= 2,
        "both broken lines must report: {diagnostics:#?}"
    );
}

#[test]
fn multiline_parenthesized_expression_crosses_lines() {
    let source = "functions:\n\tinteger f()\n\t\treturn (1 +\n\t\t\t2)\n";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::Functions(functions) = &items[0] else {
        panic!()
    };
    assert!(matches!(
        &functions[0].body.statements[0],
        Stmt::Return { value: Some(_), .. }
    ));
}

// 05 §2 ConstantBody (2026-08-20 erratum for DISCOVERIES-M9 §1c): the
// initializer is grammatical and the body holds at least one declaration.

#[test]
fn constant_without_initializer_is_a_syntax_error() {
    let source = "constant:\n\tinteger maxUsers\n\nfunctions:\n\tvoid init()\n\t\treturn\n";
    let (_, diagnostics) = parse_source(source);
    assert!(
        diagnostics.iter().any(|d| d.code == codes::SYN002),
        "a constant without '=' must not parse: {diagnostics:#?}"
    );
}

#[test]
fn empty_constant_section_is_a_syntax_error() {
    let source = "constant:\n\nfunctions:\n\tvoid init()\n\t\treturn\n";
    let (_, diagnostics) = parse_source(source);
    assert!(
        diagnostics.iter().any(|d| d.code == codes::SYN002),
        "an empty 'constant:' must not parse: {diagnostics:#?}"
    );
}

#[test]
fn constant_with_initializer_still_parses() {
    let source = "constant:\n\tinteger maxUsers = 100\n\nfunctions:\n\tvoid init()\n\t\treturn\n";
    let (items, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let Item::Constants(constants) = &items[0] else {
        panic!("expected constants, got {items:?}");
    };
    assert_eq!(constants.len(), 1);
    assert_eq!(constants[0].name, "maxUsers");
    assert!(constants[0].init.is_some());
}

// 07 §7.8 / 10 §BLD001 (2026-08-20): `max-nesting-depth` — one chained
// depth across expression nodes, INDENT levels, and generic type layers,
// established during parse; {actual} is defined as max + 1.

fn bld001_of(
    diagnostics: &[clean_compiler_types::Diagnostic],
) -> Vec<&clean_compiler_types::Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.code == codes::BLD001)
        .collect()
}

#[test]
fn deep_parentheses_report_bld001_with_the_spec_message() {
    let expr = format!("{}1{}", "(".repeat(300), ")".repeat(300));
    let source = format!("functions:\n\tvoid init()\n\t\tinteger x = {expr}\n\t\treturn\n");
    let (_, diagnostics) = parse_source(&source);
    let bld = bld001_of(&diagnostics);
    assert_eq!(bld.len(), 1, "one BLD001 exactly: {diagnostics:#?}");
    assert_eq!(
        bld[0].message,
        "build limit 'max-nesting-depth' exceeded: 257 > 256"
    );
    assert_eq!(
        bld[0].primary_label.as_deref(),
        Some("nesting here exceeds 'max-nesting-depth'")
    );
}

#[test]
fn nesting_within_the_limit_is_silent() {
    let expr = format!("{}1{}", "(".repeat(200), ")".repeat(200));
    let source = format!("functions:\n\tvoid init()\n\t\tinteger x = {expr}\n\t\treturn\n");
    let (_, diagnostics) = parse_source(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn operator_chains_count_one_level_per_application() {
    // 10 §BLD001's own example: a 300-term sum exceeds the default.
    let chain = vec!["1"; 300].join(" + ");
    let source = format!("functions:\n\tvoid init()\n\t\tinteger x = {chain}\n\t\treturn\n");
    let (_, diagnostics) = parse_source(&source);
    assert_eq!(bld001_of(&diagnostics).len(), 1, "{diagnostics:#?}");
}

#[test]
fn deep_generic_type_layers_report_bld001() {
    let ty = format!("{}integer{}", "list<".repeat(300), ">".repeat(300));
    let source = format!("functions:\n\tvoid init()\n\t\t{ty} xs = []\n\t\treturn\n");
    let (_, diagnostics) = parse_source(&source);
    assert_eq!(bld001_of(&diagnostics).len(), 1, "{diagnostics:#?}");
}

#[test]
fn deep_statement_blocks_report_bld001() {
    let mut body = String::new();
    for depth in 0..300 {
        body.push_str(&"\t".repeat(2 + depth));
        body.push_str("if true\n");
    }
    body.push_str(&"\t".repeat(302));
    body.push_str("return\n");
    let source = format!("functions:\n\tvoid init()\n{body}");
    let (_, diagnostics) = parse_source(&source);
    assert_eq!(bld001_of(&diagnostics).len(), 1, "{diagnostics:#?}");
}
