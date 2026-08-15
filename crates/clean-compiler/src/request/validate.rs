//! JSON intake and semantic validation for the request document.
//!
//! Message templates are copied verbatim from Platform 10 §16:
//! - RQD001: `"request integrity failure: '{path}' declares sha256 {declared}, content hashes to {actual}"`
//! - RQD002: `"invalid compilation request: {reason} at '{json_path}'"`

use clean_compiler_types::{codes, CompileRequest, Diagnostic, Level, Span};
use sha2::{Digest, Sha256};

use crate::codegen::world::ParsedWorld;
use crate::diag::{render_cli, DiagnosticSink};

/// Output of pass [1]: the schema- and integrity-checked request plus the
/// parsed target world later passes validate against and embed.
pub struct ValidatedRequest {
    pub request: CompileRequest,
    pub world: ParsedWorld,
}

/// Deserializes the request JSON. Schema violations — unknown keys, missing
/// required fields, malformed values — are `RQD002` (Platform 14 §14.1.1:
/// there is no ignore-and-continue for schema drift).
pub fn from_json(json: &str, sink: &mut DiagnosticSink) -> Option<CompileRequest> {
    let de = &mut serde_json::Deserializer::from_str(json);
    match serde_path_to_error::deserialize::<_, CompileRequest>(de) {
        Ok(request) => Some(request),
        Err(err) => {
            let json_path = match err.path().to_string() {
                p if p == "." => "$".to_string(),
                p => format!("$.{p}"),
            };
            let reason = normalize_serde_reason(&err.inner().to_string());
            sink.push(request_error(
                codes::RQD002,
                format!("invalid compilation request: {reason} at '{json_path}'"),
            ));
            None
        }
    }
}

/// Semantic validation over an already-deserialized request: `spec_version`
/// support, `target_world` hash agreement, and every `sources[].sha256`
/// (CMP-01). All findings are collected before the pipeline stops — a single
/// bad hash does not hide the others (Platform 14 §14.4).
pub fn validate(request: CompileRequest, sink: &mut DiagnosticSink) -> Option<ValidatedRequest> {
    if request.spec_version != "1" {
        sink.push(request_error(
            codes::RQD002,
            format!(
                "invalid compilation request: unsupported spec_version '{}', highest supported is '1' at '$.spec_version'",
                request.spec_version
            ),
        ));
    }

    // ADR-0033: `target_world.sha256` is the forensic link to the lockfile —
    // a disagreement with the inlined text must be detectable, not silent.
    let world_hash = sha256_hex(request.target_world.wit.as_bytes());
    if world_hash != request.target_world.sha256 {
        sink.push(request_error(
            codes::RQD002,
            format!(
                "invalid compilation request: target_world.sha256 '{}' does not match its wit text (hashes to '{}') at '$.target_world.sha256'",
                request.target_world.sha256, world_hash
            ),
        ));
    }

    for source in &request.sources {
        let actual = sha256_hex(source.content.as_bytes());
        if actual != source.sha256 {
            sink.push(request_error(
                codes::RQD001,
                format!(
                    "request integrity failure: '{}' declares sha256 {}, content hashes to {}",
                    source.path, source.sha256, actual
                ),
            ));
        }
    }

    // ADR-0033: `target_world.world` must name a world present in the WIT;
    // unparseable WIT and a missing world are both RQD002. Parsed here so
    // every later pass receives the world already resolved.
    let world = crate::codegen::world::parse(&request.target_world, sink);

    match world {
        Some(world) if !sink.has_errors() => Some(ValidatedRequest { request, world }),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Maps serde's error wording onto the registry's vocabulary. The serde text
/// is an implementation detail; the emitted reason is the compiler's
/// observable output, locked by snapshot tests.
fn normalize_serde_reason(serde_message: &str) -> String {
    let message = serde_message
        .split(" at line ")
        .next()
        .unwrap_or(serde_message);
    if let Some(rest) = message.strip_prefix("unknown field ") {
        let name = rest.split(',').next().unwrap_or(rest);
        format!("unknown key {name}")
    } else if let Some(rest) = message.strip_prefix("missing field ") {
        format!("missing required field {rest}")
    } else {
        message.to_string()
    }
}

fn request_error(code: &str, message: String) -> Diagnostic {
    let mut diagnostic = Diagnostic {
        level: Level::Error,
        code: code.to_string(),
        message,
        primary_span: Span::request_document(),
        primary_label: None,
        secondary: Vec::new(),
        notes: Vec::new(),
        helps: Vec::new(),
        suggestions: Vec::new(),
        doc_url: Diagnostic::doc_url_for(code),
        rendered: String::new(),
    };
    diagnostic.rendered = render_cli(&diagnostic, &crate::diag::SourceCache::empty());
    diagnostic
}
