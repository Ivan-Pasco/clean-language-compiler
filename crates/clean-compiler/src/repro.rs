//! Build reproduction (Platform 14 §14.14.6, the operation behind
//! `cln repro build`): read a build manifest, obtain every input at its
//! recorded SHA-256 through a caller-provided resolver, invoke `compile()`
//! with the reconstructed request, and assert the output `wasm_sha256`
//! matches the manifest's. On match the rebuilt bytes *are* the originally
//! shipped `component.wasm`; on mismatch the divergence is a compiler bug
//! (COM013) or a manifest corruption — never a "close enough" outcome.
//!
//! The resolver is pluggable (§14.14.6: the compiler embeds no package
//! manager and reads no cache on its own — CMP-01). The spec's two-method
//! trait cannot rebuild a request on its own: the §14.8 manifest records
//! neither `project` nor `target_world`, and the world's WIT must be
//! refetched by hash. Local adoptions: the manifest additions of local
//! ADR 0007 plus the `fetch_world` method (DISCOVERIES-M8 §4).
//!
//! First-byte-of-divergence reporting needs the original artifact, which
//! the manifest names only by hash — so the caller may hand over the
//! original `component.wasm`; without it a divergence reports the two
//! hashes (DISCOVERIES-M8 §5).

use clean_compiler_types::manifest::BuildManifest;
use clean_compiler_types::request::{CompileRequest, LibraryManifest, SourceFile, TargetWorld};
use clean_compiler_types::Diagnostic;
use sha2::{Digest, Sha256};

use crate::driver::{CompileArtifact, CompileError};

/// One failed fetch: what was asked for, and why it could not be served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    /// What was being fetched, e.g. `source app/main.cln@e3b0c4…`.
    pub what: String,
    pub reason: String,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot resolve {}: {}", self.what, self.reason)
    }
}

/// The caller-provided input store (§14.14.6). Every method returns the
/// bytes *at the recorded hash* — the operation re-verifies each hash and
/// treats a mismatch as a resolver defect, so a resolver may serve from
/// anywhere (a request document, `~/.cln`, CI artifact storage) without
/// being trusted.
pub trait InputResolver {
    fn fetch_source(&self, path: &str, sha256: &str) -> Result<String, ResolveError>;
    fn fetch_library(
        &self,
        name: &str,
        version: &str,
        sha256: &str,
    ) -> Result<LibraryManifest, ResolveError>;
    /// Local extension over the spec's trait (DISCOVERIES-M8 §4): the
    /// target world's WIT text, verbatim as published, at the hash the
    /// manifest recorded.
    fn fetch_world(&self, host: &str, version: &str, sha256: &str) -> Result<String, ResolveError>;
}

/// Why a reproduction did not produce the shipped bytes.
#[derive(Debug)]
pub enum ReproFailure {
    /// The manifest predates the local ADR 0007 additions (or was written
    /// by another tool): it does not carry the named record, so the
    /// request cannot be reconstructed.
    ManifestIncomplete { missing: &'static str },
    /// An input could not be fetched, or the fetched bytes do not hash to
    /// the manifest's record.
    Resolve(ResolveError),
    /// The reconstructed request did not compile to a component. Carries
    /// the diagnostics (or pre-v1 channel) exactly as `compile` raised them.
    Compile(CompileError),
    /// The caller-provided original artifact does not hash to
    /// `outputs.wasm_sha256`: the manifest and the artifact disagree before
    /// any rebuild — a corruption, not a compiler divergence.
    OriginalCorrupt {
        expected_sha256: String,
        actual_sha256: String,
    },
    /// The rebuild completed but its bytes do not hash to the manifest's
    /// record (§14.14.6: a compiler bug — COM013 — or manifest corruption).
    Diverged {
        expected_sha256: String,
        actual_sha256: String,
        rebuilt_len: usize,
        /// Byte offset of the first difference against the original
        /// artifact, when the caller provided it. `None` when only hashes
        /// are known, or `Some(min_len)` when one artifact is a prefix of
        /// the other.
        first_divergent_byte: Option<usize>,
        original_len: Option<usize>,
    },
}

impl std::fmt::Display for ReproFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestIncomplete { missing } => {
                write!(f, "build manifest records no {missing}; cannot reconstruct the request")
            }
            Self::Resolve(err) => err.fmt(f),
            Self::Compile(err) => write!(f, "reconstructed request did not compile: {err}"),
            Self::OriginalCorrupt {
                expected_sha256,
                actual_sha256,
            } => write!(
                f,
                "provided component.wasm hashes to {actual_sha256} but the manifest records {expected_sha256}: manifest or artifact is corrupt"
            ),
            Self::Diverged {
                expected_sha256,
                actual_sha256,
                first_divergent_byte,
                ..
            } => match first_divergent_byte {
                Some(offset) => write!(
                    f,
                    "rebuild diverged from the shipped component at byte {offset}: manifest records {expected_sha256}, rebuild hashes to {actual_sha256} — a compiler bug (COM013) or a manifest corruption"
                ),
                None => write!(
                    f,
                    "rebuild diverged from the shipped component: manifest records {expected_sha256}, rebuild hashes to {actual_sha256} — a compiler bug (COM013) or a manifest corruption"
                ),
            },
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The reproduction operation. On success the returned artifact's bytes
/// hash to the manifest's `outputs.wasm_sha256` — they are the exact
/// component originally shipped.
pub fn repro_build(
    manifest: &BuildManifest,
    original_wasm: Option<&[u8]>,
    resolver: &dyn InputResolver,
) -> Result<CompileArtifact, ReproFailure> {
    // A provided original that disagrees with the manifest is detected
    // before any compile work: nothing the rebuild does can arbitrate
    // between two records that already contradict each other.
    if let Some(original) = original_wasm {
        let actual = sha256_hex(original);
        if actual != manifest.outputs.wasm_sha256 {
            return Err(ReproFailure::OriginalCorrupt {
                expected_sha256: manifest.outputs.wasm_sha256.clone(),
                actual_sha256: actual,
            });
        }
    }

    let request = reconstruct_request(manifest, resolver)?;
    let artifact = crate::driver::compile(request).map_err(ReproFailure::Compile)?;

    let actual = sha256_hex(&artifact.wasm);
    if actual != manifest.outputs.wasm_sha256 {
        let (first_divergent_byte, original_len) = match original_wasm {
            Some(original) => (
                Some(first_divergence(original, &artifact.wasm)),
                Some(original.len()),
            ),
            None => (None, None),
        };
        return Err(ReproFailure::Diverged {
            expected_sha256: manifest.outputs.wasm_sha256.clone(),
            actual_sha256: actual,
            rebuilt_len: artifact.wasm.len(),
            first_divergent_byte,
            original_len,
        });
    }
    Ok(artifact)
}

/// Byte offset of the first difference; when one side is a strict prefix
/// of the other, the divergence is at the shorter length.
fn first_divergence(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()))
}

/// Rebuilds the request document from the manifest's records plus the
/// resolver's inputs, re-verifying every hash on the way in.
fn reconstruct_request(
    manifest: &BuildManifest,
    resolver: &dyn InputResolver,
) -> Result<CompileRequest, ReproFailure> {
    let project = manifest
        .inputs
        .project
        .clone()
        .ok_or(ReproFailure::ManifestIncomplete {
            missing: "inputs.project",
        })?;
    let world_ref =
        manifest
            .inputs
            .target_world
            .as_ref()
            .ok_or(ReproFailure::ManifestIncomplete {
                missing: "inputs.target_world",
            })?;

    let section = |name: &'static str| -> Result<&serde_json::Value, ReproFailure> {
        manifest
            .resolved_config
            .get(name)
            .ok_or(ReproFailure::ManifestIncomplete {
                missing: "resolved_config section",
            })
    };
    fn parse_section<T: serde::de::DeserializeOwned>(
        name: &'static str,
        value: &serde_json::Value,
    ) -> Result<T, ReproFailure> {
        serde_json::from_value(value.clone()).map_err(|e| {
            ReproFailure::Resolve(ResolveError {
                what: format!("resolved_config.{name}"),
                reason: format!("does not match the request schema: {e}"),
            })
        })
    }
    let build = parse_section("build", section("build")?)?;
    let folders = parse_section("folders", section("folders")?)?;
    let dependencies = parse_section("dependencies", section("dependencies")?)?;
    let compile_limits = parse_section("compile_limits", section("compile_limits")?)?;
    let telemetry = parse_section("telemetry", section("telemetry")?)?;

    let wit = resolver
        .fetch_world(&world_ref.host, &world_ref.version, &world_ref.sha256)
        .map_err(ReproFailure::Resolve)?;
    let wit_hash = sha256_hex(wit.as_bytes());
    if wit_hash != world_ref.sha256 {
        return Err(ReproFailure::Resolve(ResolveError {
            what: format!("world {}@{}", world_ref.host, world_ref.version),
            reason: format!(
                "resolver returned WIT hashing to {wit_hash}, manifest records {}",
                world_ref.sha256
            ),
        }));
    }

    let mut sources = Vec::with_capacity(manifest.inputs.sources.len());
    for record in &manifest.inputs.sources {
        let content = resolver
            .fetch_source(&record.path, &record.sha256)
            .map_err(ReproFailure::Resolve)?;
        let content_hash = sha256_hex(content.as_bytes());
        if content_hash != record.sha256 {
            return Err(ReproFailure::Resolve(ResolveError {
                what: format!("source {}", record.path),
                reason: format!(
                    "resolver returned content hashing to {content_hash}, manifest records {}",
                    record.sha256
                ),
            }));
        }
        sources.push(SourceFile {
            path: record.path.clone(),
            sha256: record.sha256.clone(),
            content,
        });
    }

    let mut library_manifests = Vec::with_capacity(manifest.inputs.library_manifests.len());
    for record in &manifest.inputs.library_manifests {
        let library = resolver
            .fetch_library(&record.name, &record.version, &record.wit_sha256)
            .map_err(ReproFailure::Resolve)?;
        let wit_hash = sha256_hex(library.wit.as_bytes());
        if wit_hash != record.wit_sha256
            || library.compiletime_wasm_sha256 != record.compiletime_wasm_sha256
        {
            return Err(ReproFailure::Resolve(ResolveError {
                what: format!("library {}@{}", record.name, record.version),
                reason:
                    "resolver returned a manifest whose hashes disagree with the build manifest"
                        .to_string(),
            }));
        }
        library_manifests.push(library);
    }

    Ok(CompileRequest {
        spec_version: manifest.spec_version.clone(),
        project,
        build,
        folders,
        dependencies,
        compile_limits,
        telemetry,
        target_world: TargetWorld {
            host: world_ref.host.clone(),
            version: world_ref.version.clone(),
            world: world_ref.world.clone(),
            sha256: world_ref.sha256.clone(),
            wit,
        },
        sources,
        library_manifests,
        overrides: manifest.overrides.clone(),
    })
}

/// A resolver serving from a request document the caller already holds —
/// the `cln repro` bug bundle carries one, and the process adapter's
/// `--repro-build` mode reads one. Fetches are answered from the document's
/// own entries; anything the document does not carry is a miss.
pub struct RequestDocumentResolver<'a> {
    request: &'a CompileRequest,
}

impl<'a> RequestDocumentResolver<'a> {
    pub fn new(request: &'a CompileRequest) -> Self {
        Self { request }
    }
}

impl InputResolver for RequestDocumentResolver<'_> {
    fn fetch_source(&self, path: &str, sha256: &str) -> Result<String, ResolveError> {
        self.request
            .sources
            .iter()
            .find(|s| s.path == path && s.sha256 == sha256)
            .map(|s| s.content.clone())
            .ok_or_else(|| ResolveError {
                what: format!("source {path}@{sha256}"),
                reason: "the request document carries no source with this path and hash"
                    .to_string(),
            })
    }

    fn fetch_library(
        &self,
        name: &str,
        version: &str,
        sha256: &str,
    ) -> Result<LibraryManifest, ResolveError> {
        self.request
            .library_manifests
            .iter()
            .find(|l| {
                l.name == name && l.version == version && sha256_hex(l.wit.as_bytes()) == sha256
            })
            .cloned()
            .ok_or_else(|| ResolveError {
                what: format!("library {name}@{version}"),
                reason: "the request document carries no such library manifest".to_string(),
            })
    }

    fn fetch_world(&self, host: &str, version: &str, sha256: &str) -> Result<String, ResolveError> {
        let world = &self.request.target_world;
        if world.host == host && world.version == version && world.sha256 == sha256 {
            Ok(world.wit.clone())
        } else {
            Err(ResolveError {
                what: format!("world {host}@{version}"),
                reason: "the request document's target_world does not match".to_string(),
            })
        }
    }
}

/// The §14.14.6 presentation of a divergence or corruption as a registered
/// diagnostic: COM013, because a rebuild that does not reproduce the
/// manifest is a compiler bug until proven a corruption — either way it is
/// never the user's program. `None` for the failures that are the caller's
/// to fix (missing records, resolver misses, a request that no longer
/// compiles).
pub fn divergence_diagnostic(failure: &ReproFailure) -> Option<Diagnostic> {
    match failure {
        ReproFailure::Diverged { .. } | ReproFailure::OriginalCorrupt { .. } => {
            Some(crate::diag::build(
                clean_compiler_types::Level::Error,
                clean_compiler_types::codes::COM013,
                format!(
                    "internal compiler invariant violated: {failure} — this is a compiler bug, please report it"
                ),
                clean_compiler_types::Span::request_document(),
                None,
            ))
        }
        _ => None,
    }
}
