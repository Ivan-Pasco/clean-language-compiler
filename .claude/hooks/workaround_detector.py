#!/usr/bin/env python3
"""PreToolUse gate on Edit/Write (foundation 05 execution/elements/06-hooks §6.1).

Blocks source edits whose content carries workaround markers. The ecosystem
rule it enforces: a bug is reported and its thread stopped — never patched
around (control-tower CT-H-09). Scope is limited to source-like files so that
documentation *about* this policy does not trip it.
"""

import json
import re
import sys

SOURCE_SUFFIXES = (".rs", ".cln", ".wit", ".toml", ".ebnf", ".pest")

PATTERNS = [
    re.compile(r"workaround", re.IGNORECASE),
    re.compile(r"TODO fix later", re.IGNORECASE),
    re.compile(r"until .{1,60} is fixed", re.IGNORECASE),
    re.compile(r"\bfor now\b", re.IGNORECASE),
    re.compile(r"\btemporary\b", re.IGNORECASE),
]


def main() -> int:
    payload = json.load(sys.stdin)
    tool_input = payload.get("tool_input", {})
    file_path = tool_input.get("file_path", "")
    if not file_path.endswith(SOURCE_SUFFIXES):
        return 0
    content = tool_input.get("content") or tool_input.get("new_string") or ""
    for pattern in PATTERNS:
        match = pattern.search(content)
        if match:
            sys.stderr.write(
                "blocked by workaround-detector (06-hooks §6.1): the edit to "
                f"'{file_path}' contains '{match.group(0)}'.\n"
                "Ecosystem rule (CT-H-09): report the bug and stop that thread; "
                "do not code around it. Fix the root cause or rephrase if this "
                "is not a workaround marker.\n"
            )
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
