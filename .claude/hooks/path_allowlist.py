#!/usr/bin/env python3
"""PreToolUse gate on Edit/Write.

A compiler session writes to the compiler repo — and to the sibling
foundation checkout, where approved spec amendments land in the moment
(CLAUDE.md rule 3: block-and-decide; the guarantee that a spec write only
follows the user's approval lives in that rule and in diff review, not
here). The other siblings — clean-server, clean-host-core, the retired
compiler — are read-only context: other components own their own sessions.
"""

import json
import pathlib
import sys

BLOCKED_SIBLINGS = (
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
    for sibling in BLOCKED_SIBLINGS:
        if sibling in parts:
            sys.stderr.write(
                f"blocked by path-allowlist: '{file_path}' is inside "
                f"'{sibling}', which is read-only for a compiler session. "
                "That component owns its own sessions; changes to it are "
                "asked for there, not edited from here.\n"
            )
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
