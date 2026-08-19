//! The M8 stage-3 contract (Platform 14 §14.14.6, build reproduction): over
//! the same request document, `repro_build` rebuilds the exact shipped
//! bytes from the manifest plus a resolver; every hash is re-verified on
//! the way in; a mismatch anywhere is a typed failure, never a "close
//! enough" outcome.

mod common;

use clean_compiler::repro::{
    divergence_diagnostic, repro_build, InputResolver, ReproFailure, RequestDocumentResolver,
    ResolveError,
};
use clean_compiler_types::request::LibraryManifest;

fn build_fixture(
    content: &str,
) -> (
    clean_compiler_types::request::CompileRequest,
    clean_compiler::CompileArtifact,
) {
    let mut request = common::minimal_valid_request();
    request.sources[0].content = content.to_string();
    request.sources[0].sha256 = common::sha256_hex(content.as_bytes());
    let artifact = clean_compiler::compile(request.clone()).expect("fixture compiles");
    (request, artifact)
}

const PROGRAM_A: &str = "functions:\n\tvoid init()\n\t\treturn\n";
const PROGRAM_B: &str =
    "functions:\n\tvoid init()\n\t\tinteger x = 41\n\t\tinteger y = x + 1\n\t\treturn\n";

/// The round trip: reproduce a build from its manifest and the request
/// document, and get back byte-identical shipped bytes (with and without
/// the original artifact on hand).
#[test]
fn reproduction_rebuilds_the_shipped_bytes_exactly() {
    let (request, artifact) = build_fixture(PROGRAM_A);
    let resolver = RequestDocumentResolver::new(&request);

    let rebuilt = repro_build(&artifact.manifest, None, &resolver)
        .expect("reproduction succeeds without the original artifact");
    assert_eq!(
        rebuilt.wasm, artifact.wasm,
        "rebuild is the shipped component"
    );

    let rebuilt = repro_build(&artifact.manifest, Some(&artifact.wasm), &resolver)
        .expect("reproduction succeeds with the original artifact");
    assert_eq!(rebuilt.wasm, artifact.wasm);
    assert_eq!(
        serde_json::to_string(&rebuilt.manifest.outputs).unwrap(),
        serde_json::to_string(&artifact.manifest.outputs).unwrap(),
        "the rebuilt manifest records the same outputs"
    );
}

/// A provided original that does not hash to `outputs.wasm_sha256` is a
/// corruption detected before any rebuild.
#[test]
fn corrupt_original_is_detected_before_compiling() {
    let (request, artifact) = build_fixture(PROGRAM_A);
    let mut tampered = artifact.wasm.clone();
    tampered[8] ^= 0xff;

    let failure = repro_build(
        &artifact.manifest,
        Some(&tampered),
        &RequestDocumentResolver::new(&request),
    )
    .expect_err("corrupt original must fail");
    match &failure {
        ReproFailure::OriginalCorrupt {
            expected_sha256,
            actual_sha256,
        } => {
            assert_eq!(expected_sha256, &artifact.manifest.outputs.wasm_sha256);
            assert_ne!(actual_sha256, expected_sha256);
        }
        other => panic!("expected OriginalCorrupt, got {other}"),
    }
    let diagnostic = divergence_diagnostic(&failure).expect("corruption maps to COM013");
    assert_eq!(diagnostic.code, "COM013");
}

/// A manifest whose `wasm_sha256` names a different build diverges, and
/// with the original on hand the report carries the first divergent byte.
#[test]
fn divergence_reports_the_first_differing_byte() {
    let (request_a, artifact_a) = build_fixture(PROGRAM_A);
    let (_request_b, artifact_b) = build_fixture(PROGRAM_B);
    assert_ne!(artifact_a.wasm, artifact_b.wasm);

    // The manifest claims A's inputs shipped B's bytes.
    let mut manifest = artifact_a.manifest.clone();
    manifest.outputs.wasm_sha256 = artifact_b.manifest.outputs.wasm_sha256.clone();

    let failure = repro_build(
        &manifest,
        Some(&artifact_b.wasm),
        &RequestDocumentResolver::new(&request_a),
    )
    .expect_err("the rebuild cannot hash to another build's record");
    match &failure {
        ReproFailure::Diverged {
            expected_sha256,
            actual_sha256,
            first_divergent_byte,
            rebuilt_len,
            original_len,
        } => {
            assert_eq!(expected_sha256, &artifact_b.manifest.outputs.wasm_sha256);
            assert_eq!(actual_sha256, &artifact_a.manifest.outputs.wasm_sha256);
            assert_eq!(*rebuilt_len, artifact_a.wasm.len());
            assert_eq!(*original_len, Some(artifact_b.wasm.len()));
            let offset = first_divergent_byte.expect("original provided, so the byte is known");
            let expected_offset = artifact_b
                .wasm
                .iter()
                .zip(artifact_a.wasm.iter())
                .position(|(x, y)| x != y)
                .unwrap_or_else(|| artifact_a.wasm.len().min(artifact_b.wasm.len()));
            assert_eq!(offset, expected_offset);
        }
        other => panic!("expected Diverged, got {other}"),
    }
    assert_eq!(
        divergence_diagnostic(&failure)
            .expect("divergence maps to COM013")
            .code,
        "COM013"
    );

    // Without the original, the same divergence still fails — by hash only.
    match repro_build(&manifest, None, &RequestDocumentResolver::new(&request_a)) {
        Err(ReproFailure::Diverged {
            first_divergent_byte: None,
            original_len: None,
            ..
        }) => {}
        other => panic!(
            "expected hash-only divergence, got {:?}",
            other.map(|a| a.wasm.len())
        ),
    }
}

/// A resolver that serves bytes not matching the recorded hash is a
/// resolver defect, refused before compiling — the operation trusts hashes,
/// never stores.
struct LyingResolver<'a> {
    inner: RequestDocumentResolver<'a>,
}

impl InputResolver for LyingResolver<'_> {
    fn fetch_source(&self, _path: &str, _sha256: &str) -> Result<String, ResolveError> {
        Ok("functions:\n\tvoid init()\n\t\tinteger evil = 666\n\t\treturn\n".to_string())
    }
    fn fetch_library(
        &self,
        name: &str,
        version: &str,
        sha256: &str,
    ) -> Result<LibraryManifest, ResolveError> {
        self.inner.fetch_library(name, version, sha256)
    }
    fn fetch_world(&self, host: &str, version: &str, sha256: &str) -> Result<String, ResolveError> {
        self.inner.fetch_world(host, version, sha256)
    }
}

#[test]
fn tampered_inputs_are_refused_by_hash_verification() {
    let (request, artifact) = build_fixture(PROGRAM_A);
    let resolver = LyingResolver {
        inner: RequestDocumentResolver::new(&request),
    };

    match repro_build(&artifact.manifest, None, &resolver) {
        Err(ReproFailure::Resolve(err)) => {
            assert!(
                err.what.contains("app/main.cln"),
                "names the tampered input: {err}"
            );
        }
        other => panic!(
            "expected a resolve failure, got {:?}",
            other.map(|a| a.wasm.len())
        ),
    }
}

/// A resolver miss names what could not be fetched.
#[test]
fn resolver_miss_names_the_missing_input() {
    let (request, artifact) = build_fixture(PROGRAM_A);
    let mut manifest = artifact.manifest.clone();
    manifest.inputs.sources[0].path = "app/other.cln".to_string();

    match repro_build(&manifest, None, &RequestDocumentResolver::new(&request)) {
        Err(ReproFailure::Resolve(err)) => assert!(err.what.contains("app/other.cln")),
        other => panic!(
            "expected a resolve failure, got {:?}",
            other.map(|a| a.wasm.len())
        ),
    }
}

/// A manifest without the local ADR 0007 records cannot reconstruct the
/// request, and says which record is missing.
#[test]
fn pre_adr0007_manifest_is_refused_as_incomplete() {
    let (request, artifact) = build_fixture(PROGRAM_A);

    let mut manifest = artifact.manifest.clone();
    manifest.inputs.project = None;
    match repro_build(&manifest, None, &RequestDocumentResolver::new(&request)) {
        Err(ReproFailure::ManifestIncomplete {
            missing: "inputs.project",
        }) => {}
        other => panic!(
            "expected incomplete manifest, got {:?}",
            other.map(|a| a.wasm.len())
        ),
    }

    let mut manifest = artifact.manifest.clone();
    manifest.inputs.target_world = None;
    match repro_build(&manifest, None, &RequestDocumentResolver::new(&request)) {
        Err(ReproFailure::ManifestIncomplete {
            missing: "inputs.target_world",
        }) => {}
        other => panic!(
            "expected incomplete manifest, got {:?}",
            other.map(|a| a.wasm.len())
        ),
    }
}

/// The manifest round-trips through JSON with the ADR 0007 records intact —
/// reproduction works from the persisted `build-manifest.json`, not only
/// from the in-memory value.
#[test]
fn reproduction_works_from_the_persisted_manifest() {
    let (request, artifact) = build_fixture(PROGRAM_A);
    let json = serde_json::to_string_pretty(&artifact.manifest).unwrap();
    let manifest: clean_compiler_types::manifest::BuildManifest =
        serde_json::from_str(&json).unwrap();

    let rebuilt = repro_build(&manifest, None, &RequestDocumentResolver::new(&request))
        .expect("reproduction from persisted manifest succeeds");
    assert_eq!(rebuilt.wasm, artifact.wasm);
}
