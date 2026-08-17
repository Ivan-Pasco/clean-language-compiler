//! M4 class/capability/contract checks (chapters 10/14/16): nominal
//! inheritance, capability claims and dynamic dispatch, contract purity,
//! and the chapter-16 dispatch surface, over the full `compile()` path
//! (`Unsupported` is not a rejection — class lowering is M6).

use clean_compiler::{compile, CompileError};

mod common;

fn request_for(sources: &[(&str, &str)]) -> clean_compiler_types::CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources = sources
        .iter()
        .map(
            |(path, content)| clean_compiler_types::request::SourceFile {
                path: path.to_string(),
                sha256: common::sha256_hex(content.as_bytes()),
                content: content.to_string(),
            },
        )
        .collect();
    request
}

fn diagnostics(sources: &[(&str, &str)]) -> Vec<clean_compiler_types::Diagnostic> {
    match compile(request_for(sources)) {
        Err(CompileError::Rejected(diagnostics)) => diagnostics,
        other => panic!("expected rejection, got {other:?}"),
    }
}

fn typechecks(sources: &[(&str, &str)]) {
    match compile(request_for(sources)) {
        Ok(_) | Err(CompileError::Incomplete { .. }) | Err(CompileError::Unsupported(_)) => {}
        Err(CompileError::Rejected(diagnostics)) => {
            panic!("program was rejected: {diagnostics:#?}")
        }
    }
}

#[test]
fn child_instance_fits_a_parent_slot() {
    typechecks(&[(
        "app/main.cln",
        "class Animal\n\tinteger legs\n\n\tfunctions:\n\t\tinteger count()\n\t\t\treturn legs\n\nclass Dog is Animal\n\tinteger tail\n\nfunctions:\n\tinteger legsOf(Animal a)\n\t\treturn a.count()\n\nstart:\n\tDog d = Dog(4)\n\tinteger n = legsOf(d)\n",
    )]);
}

#[test]
fn capability_typed_value_dispatches_dynamically() {
    typechecks(&[(
        "app/main.cln",
        "can Speak:\n\tsay() -> string\n\nclass Person can Speak\n\tstring name\n\n\tfunctions:\n\t\tstring say()\n\t\t\treturn name\n\nfunctions:\n\tstring hear(Speak s)\n\t\treturn s.say()\n\nstart:\n\tPerson p = Person(\"ada\")\n\tstring words = hear(p)\n",
    )]);
}

#[test]
fn class_without_the_claim_does_not_fit_a_capability_slot() {
    let diagnostics = diagnostics(&[(
        "app/main.cln",
        "can Speak:\n\tsay() -> string\n\nclass Rock\n\tinteger mass\n\n\tfunctions:\n\t\tstring say()\n\t\t\treturn \"...\"\n\nfunctions:\n\tstring hear(Speak s)\n\t\treturn s.say()\n\nstart:\n\tRock r = Rock(9)\n\tstring words = hear(r)\n",
    )]);
    // CLS-03 is nominal: having the method is not having the capability.
    assert!(
        diagnostics.iter().any(|d| d.code == "SEM016"),
        "nominal capability check must reject: {diagnostics:#?}"
    );
}

#[test]
fn parent_implementation_satisfies_child_claim() {
    typechecks(&[(
        "app/main.cln",
        "can Speak:\n\tsay() -> string\n\nclass Base\n\tstring name\n\n\tfunctions:\n\t\tstring say()\n\t\t\treturn name\n\nclass Kid is Base can Speak\n\tinteger age\n",
    )]);
}

#[test]
fn contract_calling_contracted_function_is_class009() {
    let diagnostics = diagnostics(&[(
        "app/main.cln",
        "functions:\n\tinteger g(integer n)\n\t\tbefore:\n\t\t\tn > 0\n\t\treturn n\n\tinteger f(integer n)\n\t\tbefore:\n\t\t\tg(n) > 0\n\t\treturn n\n",
    )]);
    assert_eq!(diagnostics[0].code, "CLASS009");
    assert_eq!(
        diagnostics[0].message,
        "Contract expression must be pure: 'g' is not allowed here"
    );
}

#[test]
fn conversions_are_typed_with_zero_arity() {
    typechecks(&[(
        "app/main.cln",
        "start:\n\tinteger n = 3\n\tstring s = n.toString()\n\tnumber x = n.toNumber()\n\tboolean b = n.toBoolean()\n\tinteger back = s.toInteger()\n",
    )]);
}

#[test]
fn namespace_call_reaches_public_functions_of_the_module() {
    typechecks(&[
        (
            "utils.cln",
            "functions:\n\tpublic:\n\t\tinteger add(integer a, integer b)\n\t\t\treturn a + b\n",
        ),
        (
            "main.cln",
            "import:\n\tutils as u\n\nstart:\n\tinteger s = u.add(2, 3)\n",
        ),
    ]);
}
