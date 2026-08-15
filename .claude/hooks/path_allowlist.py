#!/usr/bin/env python3
"""PreToolUse gate on Edit/Write (foundation 05 execution/elements/06-hooks §6.2).

A compiler session writes to the compiler repo only. The sibling checkouts —
foundation specs, clean-server, clean-host-core, the retired compiler — are
read-only context here: spec changes travel as task briefs (CT-H-16), and
other components own their own sessions (CT-H-15).
"""

import json
import pathlib
import sys

BLOCKED_SIBLINGS = (
    "clean-language-foundation",
    "clean-server",
    "clean-host-core",
    "clean-language-compiler-old",
)


def main() -> int:
    payload = json.load(sys.stdin)
    file_path = payload.get("tool_input", {}).get("file_path", "")
    if not file_path:
        return 0
    parts = pathlib.PurePath(file_path).parts
    # CT-H-06: a session MAY write task briefs under foundation/work/ —
    # that is where execution learnings (Discoveries) are recorded. Only
    # governance, specs, and ADRs are off-limits from here.
    if "clean-language-foundation" in parts:
        index = parts.index("clean-language-foundation")
        if len(parts) > index + 1 and parts[index + 1] == "work":
            return 0
    for sibling in BLOCKED_SIBLINGS:
        if sibling in parts:
            sys.stderr.write(
                f"blocked by path-allowlist (06-hooks §6.2): '{file_path}' is "
                f"inside '{sibling}', which is read-only for a compiler "
                "session. Spec or cross-component changes go through a task "
                "brief / team-prompt (CT-H-15, CT-H-16), not a direct edit.\n"
            )
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
