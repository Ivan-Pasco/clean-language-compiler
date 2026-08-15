#!/usr/bin/env bash
# Builds the Milestone 1 acceptance guest (examples/acceptance-guest) into
# dist/: assembles the request document by hand — Clean Framework is not
# built yet, and the compiler reads nothing but the request (CMP-01) —
# then invokes the process adapter.
#
# Usage: scripts/build-acceptance.sh [path-to-host.wit]
# Default host.wit: the fixture vendored from clean-server.
set -euo pipefail
cd "$(dirname "$0")/.."

HOST_WIT="${1:-tests/fixtures/wit/host.wit}"

python3 - "$HOST_WIT" <<'EOF' > dist-request.json
import hashlib, json, sys, pathlib

def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()

host_wit = pathlib.Path(sys.argv[1]).read_text()
sources = []
for name in ["host_bridge.cln", "classes.cln", "main.cln"]:
    content = pathlib.Path("examples/acceptance-guest", name).read_text()
    sources.append({"path": f"app/{name}", "sha256": sha(content), "content": content})

request = {
    "spec_version": "1",
    "project": {"name": "acceptance-guest", "version": "0.1.0"},
    "build": {"target": "wasm32-server", "optimization": "debug"},
    "target_world": {
        "host": "clean-server",
        "version": "0.7.0",
        "world": "server",
        "sha256": sha(host_wit),
        "wit": host_wit,
    },
    "sources": sources,
}
print(json.dumps(request))
EOF

cargo run -q -p clean-compiler-bin --bin clean-compiler -- --request dist-request.json --out dist
rm dist-request.json

echo "== artifact set =="
ls -la dist/
echo "== header =="
xxd -l 8 dist/component.wasm
echo "== validate =="
wasm-tools validate dist/component.wasm && echo "component validates"
echo "== imports =="
wasm-tools component wit dist/component.wasm | head -12
