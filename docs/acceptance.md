# Milestone 1 acceptance runbook

The brief: `foundation/work/2026-08-11-compiler-component-model-emission.md`.
Checks 1–5 run entirely in this repo; check 6 needs the clean-server
sibling checkout, which needs the private `clean-host-core` repo.

## Checks 1–5 (all runnable here)

```bash
# Compile the acceptance guest and inspect the artifact set:
scripts/build-acceptance.sh
#   → dist/component.wasm  (header 0061 736d 0d00 0100, validates,
#     imports clean:host/{routing,request,response,log}@0.1.0,
#     exports init + handle)
#   → dist/build-manifest.json, dist/diagnostics.json

# The same assertions, plus conformance and determinism, as tests:
cargo test -p clean-compiler --test component_pipeline   # checks 1, 2, 2b, 5
cargo test -p clean-compiler --test world_import_check   # check 3 (COM012)
cargo test -p clean-compiler --test canonical_abi        # check 4 (round-trips)
```

Note on check 2b: `wasm-tools component targets host.wit -w server` cannot
hold for a guest — `server` is the world the *host* implements. The guest
targets the mirror world (`clean:guest/app`), exactly like clean-server's
own `testing/fake-guest`; the test verifies conformance against that world
with host.wit as a dep. The host-side gate is clean-server's Moment 3
check at instantiation.

## Check 6 — end to end against clean-server ✅ PASSED 2026-08-15

Run on this machine with `clean-host-core` cloned as a sibling: the
component built by `scripts/build-acceptance.sh ../clean-server/host.wit`
was served by clean-server via a fixture without the fake-bridge entry
(`dist/host.toml` — the 9a guest imports no `clean:fake-bridge/store`).
Observed: init registered all 5 routes at startup; `GET /` → `hello world`
with the guest-set content-type header; `GET /users/42` → `42` and
`GET /users/` → the `default` fallback; `POST /echo` echoed the body;
`POST /hook` → `hook received`; `GET /log` → `logged` with the structured
record (`hello from the guest fields=route=log-demo`) in the host log;
unknown routes → 404.

Reproduce (clean-server depends by path on `../clean-host-core`):

```bash
cd "../"    # the Clean Language folder
git clone git@github.com:Ivan-Pasco/clean-host-core.git
cd clean-language-compiler
scripts/build-acceptance.sh ../clean-server/host.wit

# Point clean-server's hello-world fixture at dist/component.wasm (see
# ../clean-server/testing/fixtures/hello-world/host.toml), then:
cd ../clean-server
cargo run --bin clean-server -- testing/fixtures/hello-world/host.toml
curl http://127.0.0.1:3000/          # expect: hello world, from a cln guest
curl http://127.0.0.1:3000/users/42  # expect: 42
curl -d 'hi' http://127.0.0.1:3000/echo   # expect: hi
```

The milestone is Done when that curl answers from a component this
compiler produced (brief: "not before, and not with the WAT fixture in
place"). Routes `/events` (SSE), `/ws`, and `/counter` are the 9b set,
deferred to M6 with `result`/`variant` support.
