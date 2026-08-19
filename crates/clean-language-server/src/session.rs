//! One editor session over one request document.
//!
//! The compiler's model is CMP-01: everything comes from the request
//! document, and the LSP is one of its callers (Platform 14 §14.1) — so the
//! server never discovers files on its own. The editor-mode construction is
//! not fixed by the spec; the decision recorded in DISCOVERIES-M7 is:
//!
//! - `initializationOptions.requestDocument` carries the request document
//!   verbatim — the same JSON `cln` would hand `clean-compiler --check`.
//! - `textDocument/didOpen` / `didChange` overlay `sources[].content` for the
//!   matching path (URI resolved against the workspace root); the server
//!   recomputes that entry's `sha256`, because the overlaid request is one
//!   the server is composing as a caller. `didClose` drops the overlay,
//!   reverting to the base document — never to the filesystem.
//! - Diagnostics whose span names no source file (`<request>`) publish under
//!   `initializationOptions.requestDocumentUri`, or `clean:request` when the
//!   caller did not name one.

use std::collections::BTreeMap;
use std::str::FromStr;

use clean_compiler::CompileError;
use clean_compiler_types::request::CompileRequest;
use clean_compiler_types::Diagnostic;
use sha2::{Digest, Sha256};

/// URI for request-level diagnostics when the caller names none.
const REQUEST_URI_FALLBACK: &str = "clean:request";
/// The pseudo-file request-level spans carry (`Span::request_document()`).
const REQUEST_PSEUDO_FILE: &str = "<request>";

pub struct Session {
    base: CompileRequest,
    /// Open-document contents keyed by project-relative source path.
    overlays: BTreeMap<String, String>,
    /// Workspace root URI (no trailing slash) source paths resolve against.
    root: Option<String>,
    /// Where request-level diagnostics publish.
    request_uri: String,
}

/// One `textDocument/publishDiagnostics` payload.
pub struct Publish {
    pub uri: lsp_types::Uri,
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

/// The outcome of a re-check: the publish set (every bucket, every time, so
/// stale diagnostics always clear) and any pre-v1 conditions that are not
/// diagnostics (DIA-01: no code, no publish) and surface as log messages —
/// the same split the batch adapter makes (stderr, exit 3).
pub struct CheckOutcome {
    pub publishes: Vec<Publish>,
    pub logs: Vec<String>,
}

impl Session {
    pub fn new(base: CompileRequest, root: Option<String>, request_uri: Option<String>) -> Self {
        Self {
            base,
            overlays: BTreeMap::new(),
            root: root.map(|r| r.trim_end_matches('/').to_string()),
            request_uri: request_uri.unwrap_or_else(|| REQUEST_URI_FALLBACK.to_string()),
        }
    }

    /// Project-relative source path for an editor URI, if the URI is inside
    /// the workspace root and names a `sources[]` entry.
    fn source_path(&self, uri: &str) -> Option<String> {
        let root = self.root.as_ref()?;
        let relative = uri.strip_prefix(root.as_str())?.strip_prefix('/')?;
        self.base
            .sources
            .iter()
            .any(|s| s.path == relative)
            .then(|| relative.to_string())
    }

    /// Applies a full-document overlay. Returns false when the URI names no
    /// request source — the request decides the compilation unit (CMP-01),
    /// so a foreign file is tracked by the editor, not by this session.
    pub fn open_or_change(&mut self, uri: &str, text: String) -> bool {
        match self.source_path(uri) {
            Some(path) => {
                self.overlays.insert(path, text);
                true
            }
            None => false,
        }
    }

    /// Drops the overlay for a closed document, reverting to the base
    /// request content (the only self-contained fallback under CMP-01).
    pub fn close(&mut self, uri: &str) {
        if let Some(path) = self.source_path(uri) {
            self.overlays.remove(&path);
        }
    }

    /// The request the pipeline sees right now: base plus overlays, each
    /// overlaid entry re-hashed because this server composes the request it
    /// submits (RQD001 keeps guarding the base document as delivered).
    fn effective_request(&self) -> CompileRequest {
        let mut request = self.base.clone();
        for source in &mut request.sources {
            if let Some(text) = self.overlays.get(&source.path) {
                source.content = text.clone();
                source.sha256 = sha256_hex(text.as_bytes());
            }
        }
        request
    }

    fn uri_for(&self, path: &str) -> lsp_types::Uri {
        let raw = match (&self.root, path) {
            (_, REQUEST_PSEUDO_FILE) => self.request_uri.clone(),
            (Some(root), _) => format!("{root}/{path}"),
            (None, _) => format!("clean:source/{path}"),
        };
        lsp_types::Uri::from_str(&raw).unwrap_or_else(|_| {
            lsp_types::Uri::from_str(REQUEST_URI_FALLBACK).expect("fallback uri parses")
        })
    }

    fn content_for(&self, path: &str) -> String {
        if path == REQUEST_PSEUDO_FILE {
            return String::new();
        }
        self.overlays.get(path).cloned().unwrap_or_else(|| {
            self.base
                .sources
                .iter()
                .find(|s| s.path == path)
                .map(|s| s.content.clone())
                .unwrap_or_default()
        })
    }

    /// Runs the same `check` operation `cln check` dispatches (passes 1–9)
    /// and buckets the resulting diagnostics per file. The parity gate rests
    /// here: one entry point, one diagnostic list, converted — never
    /// re-derived.
    pub fn check(&self) -> CheckOutcome {
        let request = self.effective_request();
        let (diagnostics, logs) = match clean_compiler::check(request) {
            Ok(diagnostics) => (diagnostics, Vec::new()),
            Err(CompileError::Rejected(diagnostics)) => (diagnostics, Vec::new()),
            Err(CompileError::Unsupported(unsupported)) => (
                Vec::new(),
                unsupported
                    .iter()
                    .map(|u| {
                        format!(
                            "not compilable yet: {} ({}:{}:{})",
                            u.construct, u.span.file, u.span.start.line, u.span.start.column
                        )
                    })
                    .collect(),
            ),
            Err(err @ CompileError::Incomplete { .. }) => (Vec::new(), vec![err.to_string()]),
        };
        CheckOutcome {
            publishes: self.bucket(&diagnostics),
            logs,
        }
    }

    /// Intake failures precede any source (Platform 10 §16): every span is
    /// `<request>`, and everything publishes under the request-document URI.
    pub fn bucket_for_intake(
        &self,
        diagnostics: &[Diagnostic],
    ) -> (lsp_types::Uri, Vec<lsp_types::Diagnostic>) {
        let locate = |file: &str| -> Option<(lsp_types::Uri, String)> {
            Some((self.uri_for(file), self.content_for(file)))
        };
        let converted = diagnostics
            .iter()
            .map(|d| crate::convert::diagnostic(d, &self.content_for(&d.primary_span.file), locate))
            .collect();
        (self.uri_for(REQUEST_PSEUDO_FILE), converted)
    }

    /// Buckets diagnostics by primary file: the request bucket first, then
    /// every source in `sources[]` order — every bucket published every
    /// round, so a fixed file's stale squiggles clear deterministically.
    fn bucket(&self, diagnostics: &[Diagnostic]) -> Vec<Publish> {
        let locate = |file: &str| -> Option<(lsp_types::Uri, String)> {
            Some((self.uri_for(file), self.content_for(file)))
        };
        let mut buckets: BTreeMap<&str, Vec<lsp_types::Diagnostic>> = BTreeMap::new();
        buckets.insert(REQUEST_PSEUDO_FILE, Vec::new());
        for source in &self.base.sources {
            buckets.insert(&source.path, Vec::new());
        }
        for diagnostic in diagnostics {
            let file = diagnostic.primary_span.file.as_str();
            let content = self.content_for(file);
            let converted = crate::convert::diagnostic(diagnostic, &content, locate);
            // A span naming neither a source nor `<request>` has no URI to
            // live at; it joins the request bucket rather than vanish
            // (parity over placement — recorded in DISCOVERIES-M7).
            let key = if buckets.contains_key(file) {
                file
            } else {
                REQUEST_PSEUDO_FILE
            };
            buckets.get_mut(key).expect("bucket exists").push(converted);
        }
        let mut publishes = Vec::new();
        let request_bucket = buckets
            .remove(REQUEST_PSEUDO_FILE)
            .expect("request bucket exists");
        publishes.push(Publish {
            uri: self.uri_for(REQUEST_PSEUDO_FILE),
            diagnostics: request_bucket,
        });
        for source in &self.base.sources {
            let diagnostics = buckets.remove(source.path.as_str()).unwrap_or_default();
            publishes.push(Publish {
                uri: self.uri_for(&source.path),
                diagnostics,
            });
        }
        publishes
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
