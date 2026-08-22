# clean-language-compiler

The reference implementation of the Clean compiler: one self-contained
compilation request document in, one WebAssembly Component Model component
out. The compiler is specified by the
[Clean Language foundation](https://github.com/Ivan-Pasco/clean-language-foundation)
(`03 platform/14-compiler-architecture.md`, `02 components/compiler/01-specification.md`;
pinned commit in CLAUDE.md); this repository is spec-driven — the spec
decides, the code follows. The compiler ↔ host binary contract lives in
[contracts/](contracts/).

## What ships

Each release archive contains two binaries built from the same source at the
same version:

- **`clean-compiler`** — the batch compiler, a thin process adapter over the
  `clean-compiler` library crate. It is **not a user-facing command**
  (CCMP-04): every developer verb belongs to `cln` (Clean Manager), which
  resolves and invokes this binary internally.
- **`clean-language-server`** — the LSP server (CCMP-25/26). It shares the
  compiler's lexer, parser, and type checker, so what the editor understands
  and what compiles are the same code at the same version. Editor extensions
  never bundle it; they resolve it through `cln` at the project's pin.

## API stability guarantees

- **The stable API is the process boundary the spec defines** — request JSON
  in; `component.wasm`, `build-manifest.json`, or `diagnostics.json` out
  (Platform 14, CMP-01..06; diagnostics per Platform 13). Anything observable
  but not specified is not a contract and may change without notice.
- **Determinism is guaranteed and release-blocking** (CMP-02): a byte-identical
  request produces byte-identical outputs on every released target. A
  determinism regression blocks the release; it is never shipped as a known
  issue.
- **Emitted bytes are versioned** (CCMP-24): any change in the bytes emitted
  for an unchanged request — including one caused by a Rust toolchain bump —
  never ships under an existing version, because it invalidates every build
  cache entry and every reproduction keyed on the prior version. Such changes
  are called out in release notes.
- **Published versions are immutable** (CCMP-23): projects pin an exact
  version, and a published archive is never modified or replaced. Fixes ship
  as new versions.
- **The Rust crates carry no stability guarantee.** They are not published to
  crates.io; internal structure and dependency choices are implementation
  decisions recorded in [docs/adr/](docs/adr/).

## Distribution

Pushing a tag `v<version>` runs [release.yml](.github/workflows/release.yml),
which verifies the tag matches the workspace version in `Cargo.toml`, then
builds and publishes one archive per target plus a `.sha256` sidecar:

```
clean-compiler-<version>-<target>.tar.gz   # linux-x64, macos-x64, macos-arm64
clean-compiler-<version>-windows-x64.zip
```

Clean Manager installs an archive into `~/.cln/versions/compiler/<version>/`
and dispatches to it from there. The filename convention and target matrix are
owned by the foundation (`02 components/manager/automation.md`).

## Layout

- `crates/clean-compiler-types` — stable value types: spans, diagnostics,
  request, manifest.
- `crates/clean-compiler` — the pipeline behind `compile()`; one module per
  pass.
- `crates/clean-compiler-bin` — the process adapter; the only crate that sees
  argv or TOML.
- `crates/clean-language-server` — the LSP surface over the same pipeline.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Working conventions, fixture discipline, and spec pointers live in
[CLAUDE.md](CLAUDE.md) and [TESTING.md](TESTING.md). Spec-dependent tests
read the sibling `../clean-language-foundation` checkout and self-skip
(loudly) when it is absent, as in CI.

## License

MIT OR Apache-2.0.
