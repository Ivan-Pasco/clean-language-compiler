//! Parsing and selection of the target world (Milestone 1 step 3).
//!
//! The WIT text arrives by value in `target_world.wit` (ADR-0033); it is
//! parsed from memory — the compiler never reads WIT from disk or network
//! (CMP-01). A `world` selector that does not name a world present in the
//! WIT is refused with `RQD002`, as is WIT that does not parse: both are
//! malformed-request conditions, not compile errors.

use clean_compiler_types::request::TargetWorld;
use clean_compiler_types::{codes, Diagnostic, Level, Span};
use wit_parser::{Resolve, WorldId, WorldItem, WorldKey};

use crate::diag::{render_cli, DiagnosticSink};

/// The resolved target world pass [9] validates against and pass [10]
/// embeds. Owns the full `Resolve` so later passes can look up interface
/// and function signatures.
pub struct ParsedWorld {
    pub resolve: Resolve,
    pub world: WorldId,
}

impl ParsedWorld {
    /// Names of the interfaces the selected world exports, in declaration
    /// order.
    pub fn exported_interfaces(&self) -> Vec<String> {
        self.resolve.worlds[self.world]
            .exports
            .iter()
            .filter_map(|(key, item)| match item {
                WorldItem::Interface { .. } => Some(self.key_name(key)),
                _ => None,
            })
            .collect()
    }

    /// Names of the freestanding functions the selected world imports (for
    /// the `server` world: `init` and `handle` — the guest's entry points).
    pub fn imported_functions(&self) -> Vec<String> {
        self.resolve.worlds[self.world]
            .imports
            .iter()
            .filter_map(|(key, item)| match item {
                WorldItem::Function(_) => Some(self.key_name(key)),
                _ => None,
            })
            .collect()
    }

    /// The version of the WIT package the world lives in (`clean:host@0.1.0`
    /// → `0.1.0`) — the version half of every interface-qualified import
    /// name. Distinct from `target_world.version`, which is the host's
    /// release version.
    pub fn package_version(&self) -> String {
        let package = self.resolve.worlds[self.world]
            .package
            .expect("a selected world always has a package");
        self.resolve.packages[package]
            .name
            .version
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0.0.0".to_string())
    }

    fn key_name(&self, key: &WorldKey) -> String {
        match key {
            WorldKey::Name(name) => name.clone(),
            WorldKey::Interface(id) => self.resolve.interfaces[*id]
                .name
                .clone()
                .unwrap_or_default(),
        }
    }
}

/// Parses `target_world.wit` and selects `target_world.world`. Failures are
/// `RQD002` diagnostics naming the offending request field.
pub fn parse(target_world: &TargetWorld, sink: &mut DiagnosticSink) -> Option<ParsedWorld> {
    let mut resolve = Resolve::new();
    let package = match resolve.push_str("target_world.wit", &target_world.wit) {
        Ok(package) => package,
        Err(err) => {
            sink.push(request_error(format!(
                "invalid compilation request: target_world.wit does not parse ({}) at '$.target_world.wit'",
                first_line(&err.to_string())
            )));
            return None;
        }
    };

    let world = match resolve.select_world(&[package], Some(&target_world.world)) {
        Ok(world) => world,
        Err(_) => {
            sink.push(request_error(format!(
                "invalid compilation request: world '{}' is not declared in target_world.wit at '$.target_world.world'",
                target_world.world
            )));
            return None;
        }
    };

    Some(ParsedWorld { resolve, world })
}

fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).to_string()
}

fn request_error(message: String) -> Diagnostic {
    let mut diagnostic = Diagnostic {
        level: Level::Error,
        code: codes::RQD002.to_string(),
        message,
        primary_span: Span::request_document(),
        primary_label: None,
        secondary: Vec::new(),
        notes: Vec::new(),
        helps: Vec::new(),
        suggestions: Vec::new(),
        doc_url: Diagnostic::doc_url_for(codes::RQD002),
        rendered: String::new(),
    };
    diagnostic.rendered = render_cli(&diagnostic);
    diagnostic
}
