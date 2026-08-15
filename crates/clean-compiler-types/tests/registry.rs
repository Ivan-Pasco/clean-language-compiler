//! Self-consistency of the code registry (Platform 09 §1, ERC-01/ERC-02
//! mechanics that need no spec checkout).

use clean_compiler_types::codes::{self, Status};
use std::collections::BTreeSet;

/// ERC-01: every code matches `PREFIX###` — 3–6 uppercase letters then
/// exactly three digits.
#[test]
fn every_code_matches_the_registered_format() {
    for info in codes::REGISTRY {
        let letters: String = info
            .code
            .chars()
            .take_while(|c| c.is_ascii_uppercase())
            .collect();
        let digits = &info.code[letters.len()..];
        assert!(
            (3..=6).contains(&letters.len())
                && digits.len() == 3
                && digits.chars().all(|c| c.is_ascii_digit()),
            "{} violates ERC-01 (PREFIX###)",
            info.code
        );
    }
}

#[test]
fn codes_are_unique() {
    let mut seen = BTreeSet::new();
    for info in codes::REGISTRY {
        assert!(seen.insert(info.code), "{} registered twice", info.code);
    }
}

/// The counts of Platform 09 §1.1 as of 2026-08-10: 162 registered rows.
/// Six identifiers are retired (SCOPE005, IMPORT005, LIB005, LIB007,
/// LIB008, LIB009); the module docs record the reconciliation with 09's
/// "161 active / 1 withdrawn" phrasing. Changing either number is a spec
/// change (ERC-03), never an implementation choice.
#[test]
fn registry_counts_match_platform_09() {
    assert_eq!(codes::REGISTRY.len(), 162, "registered rows");
    let withdrawn = codes::REGISTRY
        .iter()
        .filter(|i| i.status == Status::Withdrawn)
        .count();
    assert_eq!(withdrawn, 6, "withdrawn identifiers");
    let emittable = codes::REGISTRY
        .iter()
        .filter(|i| i.is_compiler_emittable())
        .count();
    assert_eq!(emittable, 121, "codes the compiler pipeline may emit");
}

/// Withdrawn rows carry no severity, no emitter, no template — nothing may
/// ever emit them (DOC-13); active rows carry severity and emitter.
#[test]
fn withdrawn_rows_are_inert_and_active_rows_are_complete() {
    for info in codes::REGISTRY {
        match info.status {
            Status::Withdrawn => {
                assert!(
                    info.severity.is_none() && info.emitter.is_none() && info.template.is_none(),
                    "{} is withdrawn and must be inert",
                    info.code
                );
            }
            Status::Active => {
                assert!(
                    info.severity.is_some() && info.emitter.is_some(),
                    "{} is active and must carry severity and emitter",
                    info.code
                );
                assert!(!info.name.is_empty(), "{} has no symbolic name", info.code);
            }
        }
    }
}

/// Message templates obey the DIA-02 style floor where they are headlines:
/// single line (RUN011 is the registered multi-line exception) and no
/// trailing period.
#[test]
fn templates_have_no_trailing_punctuation() {
    for info in codes::REGISTRY {
        if let Some(template) = info.template {
            assert!(
                !template.ends_with('.'),
                "{} template ends in punctuation",
                info.code
            );
            if info.code != "RUN011" {
                assert!(
                    !template.contains('\n'),
                    "{} template is multi-line",
                    info.code
                );
            }
        }
    }
}

#[test]
fn lookup_finds_rows_by_code() {
    assert_eq!(
        codes::lookup("SEM001").map(|i| i.name),
        Some("AssignTypeMismatch")
    );
    assert_eq!(
        codes::lookup("SCOPE005").map(|i| i.status),
        Some(Status::Withdrawn)
    );
    assert!(codes::lookup("SEM999").is_none());
}
