//! The spec legs of the M2 1:1 gate (ERC-02 / RUL-02): the registry in
//! `codes.rs` must match Platform 09 row for row, and every message template
//! must appear verbatim in Platform 10. Runs against the sibling
//! `clean-language-foundation` checkout; skips (loudly) when it is absent —
//! CI does not clone the private spec repo, so this leg is a local/nightly
//! gate, mirroring the bi-repo acceptance check of M1.

use clean_compiler_types::codes::{self, Severity, Status};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn foundation() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)?
        .parent()?
        .join("clean-language-foundation");
    root.is_dir().then_some(root)
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Parses every `| CODE | \`Name\` | … | Severity | … |` row of the 09 §3
/// index tables into (code → (name, severity-cell)).
fn parse_09_rows(text: &str) -> BTreeMap<&str, (Option<&str>, Option<&str>)> {
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // A table row splits as ["", cell1, ..., ""].
        if cells.len() < 4 {
            continue;
        }
        let code = cells[1];
        let is_code = code.len() >= 6
            && code.chars().take_while(|c| c.is_ascii_uppercase()).count() >= 3
            && code.chars().rev().take(3).all(|c| c.is_ascii_digit())
            && code.chars().all(|c| c.is_ascii_alphanumeric());
        if !is_code {
            continue;
        }
        let name = cells[2].strip_prefix('`').and_then(|n| n.strip_suffix('`'));
        // Severity sits in the second-to-last non-empty cell of the five-cell
        // index rows; shorter rows (the §5 withdrawn record) carry none.
        let severity = (cells.len() >= 6).then(|| cells[cells.len() - 3]);
        // First occurrence wins: IMPORT005 appears again in §5 as a
        // two-cell withdrawal record.
        rows.entry(code).or_insert((name, severity));
    }
    rows
}

#[test]
fn registry_matches_platform_09_and_10() {
    let Some(root) = foundation() else {
        eprintln!("SKIP: ../clean-language-foundation not present; spec leg runs locally only");
        return;
    };
    let spec09 = read(root.join("03 platform/09-error-codes.md"));
    let spec10 = read(root.join("03 platform/10-semantic-rules.md"));
    let rows = parse_09_rows(&spec09);

    // Leg 1 — same code set in both directions.
    for info in codes::REGISTRY {
        assert!(
            rows.contains_key(info.code),
            "{} is registered here but has no row in 09 §3",
            info.code
        );
    }
    for code in rows.keys() {
        assert!(
            codes::lookup(code).is_some(),
            "09 §3 registers {code} but codes.rs does not"
        );
    }

    // Leg 2 — symbolic names and severities agree wherever 09 states them.
    for info in codes::REGISTRY {
        let (name, severity) = &rows[info.code];
        if let Some(name) = name {
            assert_eq!(
                *name, info.name,
                "{}: symbolic name diverges from 09 §3",
                info.code
            );
        }
        if let Some(sev) = severity {
            let expected = match *sev {
                "Error" => Some(Severity::Error),
                "Warning" => Some(Severity::Warning),
                "Info" => Some(Severity::Info),
                "Runtime" => Some(Severity::Runtime),
                "Error / Warning / Info" => Some(Severity::PerDiagnostic),
                "—" => None,
                other => panic!(
                    "{}: unrecognized severity cell {other:?} in 09 §3",
                    info.code
                ),
            };
            assert_eq!(
                expected, info.severity,
                "{}: severity diverges from 09 §3",
                info.code
            );
            if expected.is_none() {
                assert_eq!(
                    info.status,
                    Status::Withdrawn,
                    "{}: 09 marks it withdrawn",
                    info.code
                );
            }
        }
    }

    // Leg 3 — every message template is a verbatim substring of Platform 10
    // (copied, never redacted).
    for info in codes::REGISTRY {
        if let Some(template) = info.template {
            assert!(
                spec10.contains(template),
                "{}: template is not a verbatim substring of Platform 10:\n{template}",
                info.code
            );
        }
    }
}
