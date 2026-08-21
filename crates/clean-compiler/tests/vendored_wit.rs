//! The vendored-contract gate: `tests/fixtures/wit/host.wit` is a deliberate
//! copy of clean-server's authoritative contract, and this test pins its
//! exact bytes. Refreshing the copy is a two-file change — the new `host.wit`
//! plus the hash below — so a drive-by edit or a stale copy can never slip
//! through review unnamed (CLAUDE.md: "refresh deliberately, never casually").

mod common;

/// sha256 of the vendored copy, recorded at the last deliberate refresh.
const RECORDED_SHA256: &str = "c4aaba83494e63577cb798e1483ce6604c6e55660010c5d0ced3be0d2a6963de";

/// `contracts/host.wit` is the ecosystem-facing home of the same contract
/// (provenance in `contracts/SOURCE.md`); it must stay byte-identical to
/// the fixture copy the test suite compiles against.
#[test]
fn contracts_host_wit_matches_fixture() {
    assert_eq!(
        include_str!("../../../contracts/host.wit"),
        common::HOST_WIT,
        "contracts/host.wit and tests/fixtures/wit/host.wit diverged — \
         refresh both (and RECORDED_SHA256) in the same commit"
    );
}

#[test]
fn vendored_host_wit_matches_recorded_sha256() {
    let actual = common::sha256_hex(common::HOST_WIT.as_bytes());
    assert_eq!(
        actual, RECORDED_SHA256,
        "tests/fixtures/wit/host.wit no longer matches its recorded sha256. \
         If this refresh is deliberate, update RECORDED_SHA256 in this test \
         in the same commit; if not, restore the vendored copy."
    );
}
