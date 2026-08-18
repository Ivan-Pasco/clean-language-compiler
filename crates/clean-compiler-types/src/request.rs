//! The compilation request document (Platform 14 §14.1.1) — the single JSON
//! payload the compiler is handed. The compiler derives every behaviour from
//! this document and reads nothing else (CMP-01).
//!
//! Schema discipline: unknown top-level keys and unknown keys inside a
//! well-known section are hard errors (`RQD002`), so every struct here is
//! `deny_unknown_fields`. Missing required fields surface through serde as
//! `RQD002` too; sections that the schema allows to be absent get
//! `#[serde(default)]` with the defaults Platform 07 defines.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileRequest {
    /// Request-schema version. Incrementing is a breaking change governed by
    /// the ADR process (Platform 14 §14.1.1; currently `"1"`).
    pub spec_version: String,
    pub project: Project,
    pub build: Build,
    #[serde(default)]
    pub folders: IndexMap<String, Vec<String>>,
    #[serde(default)]
    pub dependencies: IndexMap<String, Dependency>,
    #[serde(default)]
    pub compile_limits: CompileLimits,
    #[serde(default)]
    pub telemetry: Telemetry,
    /// The host contract, carried by value (ADR-0033). Required: a request
    /// without it is refused with `RQD002`; there is no fallback.
    pub target_world: TargetWorld,
    /// The complete set of `.cln` files to compile, project-relative POSIX
    /// paths. No filesystem discovery happens downstream.
    pub sources: Vec<SourceFile>,
    #[serde(default)]
    pub library_manifests: Vec<LibraryManifest>,
    /// Audit trail of values that came from outside `clean.toml`; recorded
    /// verbatim in the build manifest (§14.8).
    #[serde(default)]
    pub overrides: Vec<ConfigOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// e.g. `wasm32-server`; maps to a world name via the caller (§07 §7.2).
    pub target: String,
    /// `debug` | `release` | `size` (§07 §7.5).
    pub optimization: String,
    #[serde(default)]
    pub memory: Memory,
    #[serde(default)]
    pub strip: bool,
    #[serde(default = "default_true")]
    pub component_model: bool,
    #[serde(default)]
    pub memory64: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub tier: String,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            tier: "standard".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub version: String,
    pub resolved_from: String,
}

/// Compile-time limits (§07 §7.8 `[compile.limits]`); defaults are the
/// spec's published defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileLimits {
    #[serde(default = "default_handler_timeout_ms")]
    pub handler_timeout_ms: u64,
    #[serde(default = "default_handler_memory_mb")]
    pub handler_memory_mb: u64,
    #[serde(default = "default_total_timeout_min")]
    pub total_timeout_min: u64,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    #[serde(default = "default_max_import_depth")]
    pub max_import_depth: u32,
}

fn default_handler_timeout_ms() -> u64 {
    5000
}
fn default_handler_memory_mb() -> u64 {
    128
}
fn default_total_timeout_min() -> u64 {
    10
}
fn default_max_file_size_mb() -> u64 {
    4
}
fn default_max_import_depth() -> u32 {
    32
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            handler_timeout_ms: default_handler_timeout_ms(),
            handler_memory_mb: default_handler_memory_mb(),
            total_timeout_min: default_total_timeout_min(),
            max_file_size_mb: default_max_file_size_mb(),
            max_import_depth: default_max_import_depth(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Telemetry {
    pub consent_level: String,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            consent_level: "error-with-code".to_string(),
        }
    }
}

/// The target host contract, by value (ADR-0033). Five fields, all required.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetWorld {
    /// Host name from the project's `[target]` block, e.g. `"clean-server"`.
    pub host: String,
    /// The resolved host version — never a semver constraint (CMP-02).
    pub version: String,
    /// Which world inside `wit` to validate against.
    pub world: String,
    /// Hex-lowercase SHA-256 of `wit`, as pinned in `.cln/lock.toml`.
    pub sha256: String,
    /// The host's `host.wit`, verbatim as published — never a path or URL.
    pub wit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    /// Project-relative POSIX path.
    pub path: String,
    /// Hex-lowercase SHA-256 of the decoded UTF-8 content; verified on
    /// intake, mismatch is `RQD001`.
    pub sha256: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryManifest {
    pub name: String,
    pub version: String,
    /// The library's WIT, by value.
    pub wit: String,
    #[serde(default)]
    pub handles_blocks: Vec<String>,
    /// SHA-256 of the framework-compiled compile-time handler wasm
    /// (ADR-0004); the sandbox instantiates exactly this artifact.
    pub compiletime_wasm_sha256: String,
    /// The handler wasm itself, base64-encoded (standard alphabet, padded).
    /// CMP-01 leaves the compiler nowhere else to obtain the artifact the
    /// hash above names — Platform 14 §14.1.1 models only the hash, so this
    /// field is a provisional schema addition recorded in DISCOVERIES-M5.
    /// Absent when the library registers no `handles_blocks`; a manifest
    /// that registers blocks but carries no wasm cannot expand them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiletime_wasm: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOverride {
    /// Dotted config path, e.g. `build.optimization`.
    pub path: String,
    pub value: serde_json::Value,
    /// Where the override came from: `cli`, `env`, `programmatic`.
    pub source: String,
}
