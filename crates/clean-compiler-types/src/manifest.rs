//! `build-manifest.json` — the reproducibility record for one compilation
//! (Platform 14 §14.8). A first-class output: CI, dashboards, and harnesses
//! compare `outputs.wasm_sha256`, not file bytes.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::request::ConfigOverride;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildManifest {
    pub spec_version: String,
    pub compiler: CompilerId,
    /// SHA-256 of the canonically-serialized request document.
    pub request_sha256: String,
    pub inputs: Inputs,
    /// The request's build/memory/folders/dependencies/compile_limits/
    /// telemetry, verbatim.
    pub resolved_config: serde_json::Value,
    pub overrides: Vec<ConfigOverride>,
    pub outputs: Outputs,
    /// Every warning and info emitted, in emission order.
    pub diagnostics: Vec<Diagnostic>,
    /// Per-pass timings in milliseconds (`lex_ms`, `parse_ms`, …). Wall time
    /// never appears anywhere else in the outputs (CMP-02).
    pub timings: IndexMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompilerId {
    pub version: String,
    /// SHA-256 of the compiler binary that produced this build.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inputs {
    pub sources: Vec<SourceHash>,
    pub library_manifests: Vec<LibraryHash>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryHash {
    pub name: String,
    pub version: String,
    pub wit_sha256: String,
    pub compiletime_wasm_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outputs {
    pub wasm_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map_sha256: Option<String>,
}
