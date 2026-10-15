"""Extract the ```clean examples of foundation into the grammar corpus.

Usage:
    python3 -I extract.py <foundation/specs> <grammar/corpus.json> [--write | --check]

Reads every ```clean fence under the specs directory, seals each block by
the SHA-256 of its code, classifies it with the relevance rules below, and
compares the result with the existing manifest: blocks new, changed
(a seal gone and a seal born in the same file) and deleted, per file.
Writes the manifest only with --write.  Exit 0, except: 2 when the
arguments are wrong, and 1 with --check when the manifest is not up to
date (for CI).

Relevance rules (they live here, never in foundation):

  R1  A block under language/ is a case.
  R2  A block under components/ only tests the outer form of a generic
      `identifier:` block (Block Handlers §3): it is a case with entry
      `outer_block` when its first code line is such a header, and
      illustrative otherwise.
  R3  A block with an ellipsis (`...` or `…`) in its code — outside string
      literals and comments — is illustrative: the ellipsis stands for
      code left out.  One inside a string or a comment is text.
  R4  A block containing both `✅` and `❌` is illustrative (mixed) and is
      listed for the human.
  R5  The result marker is the line before the fence:
      `**Example (passes):**` -> passes; `**Example (fails — SYNxxx…):**`
      -> fails:SYNxxx; `**Example (fails — <other code>…):**` -> passes
      (a rule of the checker: the parser accepts the code).  No marker ->
      passes.
  R6  A block whose code (outside strings and comments) holds a
      metavariable `<name>` not attached to a type (a syntax template such
      as `before:` / `<boolean_expression>`, or `maxAttempts: <integer>`) is
      illustrative.
  R7  A block containing several `// file: <path>` lines is split into one
      case per file (each a whole file).
  R8  The specs show each tab as four spaces (Lexical Structure —
      Whitespace and Indentation) and as `⇥` in one example.  Each `⇥` and
      each leading group of four spaces becomes one tab, except in a block
      marked `fails — SYN003`, whose spaces are the point of the example.
  R9  A block under a paragraph `**Valid Examples:**` or
      `**Invalid Examples:**` (Lexical Structure — Identifiers) is a list of
      independent names: each line is a case with entry `identifier`,
      `passes` or `fails:SYN` (the paragraph names no code).
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

FENCE_OPEN = re.compile(r"^([ \t]*)```clean[ \t]*$")
FENCE_CLOSE = re.compile(r"^[ \t]*```[ \t]*$")
MARK_PASSES = re.compile(r"^\*\*Example \(passes\):\*\*\s*$")
MARK_FAILS = re.compile(r"^\*\*Example \(fails — ([A-Z]+[0-9]+)[^)]*\):\*\*\s*$")
MARK_NAMES = re.compile(r"^\*\*(Valid|Invalid) Examples:\*\*\s*$")
METAVAR = re.compile(r"(?<![\w>])<[a-z][a-z_-]*>")
FILE_LINE = re.compile(r"^//\s*file:\s*(\S+)")
GENERIC_HEADER = re.compile(
    r"^[a-z][A-Za-z0-9_]*(?:\.[a-z][A-Za-z0-9_]*)*"   # name or name.name
    r"(?:[ \t]+[A-Za-z][A-Za-z0-9_]*)?"               # optional block title
    r"(?:[ \t]*\(.*\))?"                              # optional arguments
    r"[ \t]*:[ \t]*(?://.*)?$"
)

# R2: words that open a language construct, never a generic block
# (sections of File Structure, typed apply-blocks, print:).
LANGUAGE_HEADS = {
    "libraries", "import", "source", "constant", "state", "can", "class",
    "functions", "screen", "watch", "tests", "start", "compiletime",
    "handles", "host", "print", "boolean", "integer", "number", "string",
    "bytes", "datetime", "list", "matrix", "pairs", "any",
}

# foundation file -> grammar chapter (composition order of grammar/README.md)
CHAPTERS = {
    "language/syntax/lexical.md": "lexical",
    "language/syntax/file-structure.md": "file-structure",
    "language/syntax/expressions.md": "expressions",
    "language/syntax/statements.md": "statements",
    "language/syntax/control-flow.md": "control-flow",
    "language/functions.md": "functions",
    "language/types/classes.md": "classes",
    "language/types/contracts.md": "contracts",
    "language/errors.md": "errors",
    "language/state.md": "state",
    "language/testing.md": "testing",
    "language/ai.md": "ai",
    "language/modules.md": "modules",
    "language/async.md": "async",
    "language/syntax/apply-blocks.md": "apply-blocks",
    "language/syntax/method-syntax.md": "method-syntax",
    "language/syntax/block-handlers.md": "block-handlers",
}


def chapter_of(rel: str) -> str:
    if rel in CHAPTERS:
        return CHAPTERS[rel]
    if rel.startswith("components/"):
        return "block-handlers"
    return "-"  # no chapter of its own (type-system, semantics, stdlib, ...)


def sha(code: str) -> str:
    return hashlib.sha256(code.encode("utf-8")).hexdigest()


def normalize(code: str, keep_spaces: bool) -> str:
    """R8: the source a displayed example stands for."""
    out = []
    for line in code.split("\n"):
        line = line.replace("⇥", "\t")
        if not keep_spaces:
            m = re.match(r"^[ \t]*", line)
            lead = m.group(0)
            tabs = lead.count("\t")
            spaces = lead.count(" ")
            lead = "\t" * (tabs + spaces // 4) + " " * (spaces % 4)
            line = lead + line[m.end():]
        out.append(line)
    return "\n".join(out)


def fences(text: str):
    """Yield (line_no_of_fence, marker_line, code) for each ```clean fence."""
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        m = FENCE_OPEN.match(lines[i])
        if not m:
            i += 1
            continue
        indent = len(m.group(1))
        start = i
        body = []
        i += 1
        while i < len(lines) and not FENCE_CLOSE.match(lines[i]):
            ln = lines[i]
            body.append(ln[indent:] if ln[:indent].strip() == "" else ln.lstrip())
            i += 1
        j = start - 1
        while j >= 0 and lines[j].strip() == "":
            j -= 1
        marker = lines[j].strip() if j >= 0 else ""
        # strip the trailing blank lines of the block
        while body and body[-1].strip() == "":
            body.pop()
        yield start + 1, marker, "\n".join(body)
        i += 1


def split_files(code: str):
    """R7: blocks with several `// file:` lines become one part per file."""
    lines = code.split("\n")
    heads = [k for k, ln in enumerate(lines) if FILE_LINE.match(ln)]
    if len(heads) < 2:
        return [(0, code)]
    parts = []
    if any(ln.strip() and not ln.lstrip().startswith("//") for ln in lines[: heads[0]]):
        parts.append((0, "\n".join(lines[: heads[0]]).rstrip("\n")))
    for a, b in zip(heads, heads[1:] + [len(lines)]):
        part = "\n".join(lines[a:b]).rstrip("\n").rstrip()
        parts.append((a, part))
    return parts


# ---- entry of a case ---------------------------------------------------------
#
# Every case names the entry it is parsed with: `outer_block` (R2) and
# `identifier` (R9) by their rule; any other case by its code:
#   file        its first code line opens a top-level section: one of the
#               section words of File Structure (libraries, import, source,
#               constant, state, can, class, functions, screen, watch, tests,
#               start followed by `:` or `(`) or a library declaration
#               (compiletime, handles, host), or a block header with a title
#               (`data UserData:`);
#   file        it holds no code line, only comments (an empty file);
#   expression  it is one code line that holds no statement word, no `=`
#               assignment, no `onError` and is not a declaration;
#   fragment    otherwise: a sequence of statements.

SECTION = re.compile(
    r"^(libraries\s*:|import\s*:|source\s*:|constant\s*:|state\s*:|can\s+\w+\s*:|class\s|"
    r"functions\s*:|screen\s|watch[\s(]|tests\s*:|start\s*[:(]|compiletime\s|handles\s|host\s|"
    r"[a-z]\w*(\.\w+)*\s+[A-Z]\w*\s*(\(.*\))?\s*:)"
)
STATEMENT_WORD = re.compile(
    r"^(if|else|iterate|while|return|break|continue|print|reset|assert|later|background|"
    r"description|spec|before|after|constructor|always)\b"
)
ASSIGN = re.compile(r'(?<![=!<>])=(?!=)')


def code_lines(code: str):
    code = re.sub(r"/\*[\s\S]*?\*/", "", code)
    return [ln for ln in code.split("\n") if ln.strip() and not ln.strip().startswith("//")]


def entry_of(code: str) -> str:
    lines = code_lines(code)
    if not lines:
        return "file"   # only comments: an empty file
    if SECTION.match(lines[0]):
        return "file"
    if len(lines) == 1:
        one = re.sub(r'"(\\.|[^"\\])*"', '""', lines[0].split("//")[0])
        if not STATEMENT_WORD.match(one.strip()) and not ASSIGN.search(one) and "onError" not in one \
                and not re.match(r"^\s*[A-Za-z_][\w<>, .?:]*\s+[A-Za-z_]\w*\s*$", one):
            return "expression"
    return "fragment"


def first_block(code: str) -> str:
    """R2: the header of the first generic block and its indented body."""
    out, started = [], False
    for ln in code.split("\n"):
        bare = ln.strip()
        if not started:
            out.append(ln)
            if bare and not bare.startswith("//"):
                started = True
            continue
        if bare and not ln[0].isspace() and not bare.startswith("//"):
            break
        out.append(ln)
    return "\n".join(out)


def bare_code(code: str) -> str:
    """The code without string literals and comments."""
    code = re.sub(r'"(\\.|[^"\\\n])*"', '""', code)
    code = re.sub(r"/\*[\s\S]*?\*/", "", code)
    return re.sub(r"//[^\n]*", "", code)


def first_code_line(code: str) -> str:
    for ln in code.split("\n"):
        s = ln.strip()
        if s and not s.startswith("//") and not s.startswith("/*"):
            return ln
    return ""


def classify(rel: str, line: int, marker: str, code: str):
    """Return a list of case dicts for one fence."""
    base = {"file": rel, "line": line, "chapter": chapter_of(rel)}
    m_fail = MARK_FAILS.match(marker)
    keep_spaces = bool(m_fail and m_fail.group(1) == "SYN003")

    def case(code_part, expected, offset=0, **extra):
        d = dict(base)
        d["line"] = line + 1 + offset
        d["sha256"] = sha(code_part)
        d["expected"] = expected
        d["code"] = normalize(code_part, keep_spaces)
        d.update(extra)
        if expected != "illustrative" and "entry" not in d:
            d["entry"] = entry_of(d["code"])
        return d

    if "..." in bare_code(code) or "…" in bare_code(code):
        return [case(code, "illustrative", reason="ellipsis")]
    if "✅" in code and "❌" in code:
        return [case(code, "illustrative", reason="mixed")]
    if METAVAR.search(bare_code(code)):
        return [case(code, "illustrative", reason="metavariable")]

    if rel.startswith("components/"):
        head = first_code_line(code)
        word = re.match(r"[A-Za-z]*", head).group(0)
        if head and not head[0].isspace() and GENERIC_HEADER.match(head) \
                and word not in LANGUAGE_HEADS:
            d = case(code, "passes", entry="outer_block")
            d["code"] = normalize(first_block(code), keep_spaces)
            return [d]
        return [case(code, "illustrative", reason="components")]

    if not rel.startswith("language/"):
        return [case(code, "illustrative", reason="outside language/")]

    m_names = MARK_NAMES.match(marker)
    if m_names:
        expected = "passes" if m_names.group(1) == "Valid" else "fails:SYN"
        out = []
        for k, ln in enumerate(code.split("\n")):
            if ln.strip() and not ln.strip().startswith("//"):
                out.append(case(ln.rstrip(), expected, offset=k, entry="identifier"))
        return out

    if MARK_PASSES.match(marker):
        expected, extra = "passes", {}
    elif m_fail:
        code_id = m_fail.group(1)
        if code_id.startswith("SYN"):
            expected, extra = f"fails:{code_id}", {}
        else:
            expected, extra = "passes", {"checker": code_id}
    else:
        expected, extra = "passes", {}

    parts = split_files(code)
    if len(parts) == 1:
        return [case(code, expected, **extra)]
    return [case(p, expected, offset=off, part=n, **extra) for n, (off, p) in enumerate(parts)]


def extract(specs: Path):
    cases = []
    stats = {"fences": 0, "language": 0, "components": 0, "passes_marks": 0, "fails_marks": 0}
    for path in sorted(specs.rglob("*.md")):
        rel = path.relative_to(specs).as_posix()
        text = path.read_text(encoding="utf-8")
        for line, marker, code in fences(text):
            stats["fences"] += 1
            top = rel.split("/")[0]
            if top in ("language", "components"):
                stats[top] += 1
            if MARK_PASSES.match(marker):
                stats["passes_marks"] += 1
            if MARK_FAILS.match(marker):
                stats["fails_marks"] += 1
            cases.extend(classify(rel, line, marker, code))
    return cases, stats


def key(c):
    return (c["file"], c["sha256"], c.get("part", 0), c["code"])


def diff(old, new):
    """Report new / changed / deleted seals per file."""
    old_by = {}
    for c in old:
        old_by.setdefault(c["file"], set()).add(c["sha256"])
    new_by = {}
    for c in new:
        new_by.setdefault(c["file"], set()).add(c["sha256"])
    report = {}
    for f in sorted(set(old_by) | set(new_by)):
        born = new_by.get(f, set()) - old_by.get(f, set())
        gone = old_by.get(f, set()) - new_by.get(f, set())
        if not born and not gone:
            continue
        lines_of = {c["sha256"]: c["line"] for c in new if c["file"] == f}
        n_changed = min(len(born), len(gone))
        report[f] = {
            "born": sorted(lines_of[s] for s in born),
            "gone": len(gone),
            "changed": n_changed,
        }
    return report


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    specs, manifest = Path(args[0]), Path(args[1])
    cases, stats = extract(specs)
    old = json.loads(manifest.read_text(encoding="utf-8"))["cases"] if manifest.exists() else []

    counts = {}
    for c in cases:
        k = c["expected"] if not c["expected"].startswith("fails") else "fails"
        counts[k] = counts.get(k, 0) + 1
    print(f"fences: {stats['fences']}  (language/ {stats['language']}, components/ {stats['components']})")
    print(f"markers: passes {stats['passes_marks']}, fails {stats['fails_marks']}")
    print("cases: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    mixed = [c for c in cases if c.get("reason") == "mixed"]
    if mixed:
        print(f"mixed ✅/❌ blocks (illustrative, for the human): {len(mixed)}")
        for c in mixed:
            print(f"  {c['file']}:{c['line']}")

    report = diff(old, cases)
    if not report:
        print("manifest: up to date")
    else:
        print("manifest: differs")
        for f, r in report.items():
            print(f"  {f}: +{len(r['born'])} new/changed at lines {r['born']}, -{r['gone']} gone ({r['changed']} changed)")

    if "--check" in argv and report:
        return 1
    if "--write" in argv:
        data = {"schema_version": 1, "cases": sorted(cases, key=lambda c: (c["file"], c["line"], c.get("part", 0)))}
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if manifest.exists() and manifest.read_text(encoding="utf-8") == text:
            print(f"{manifest} unchanged")
        else:
            manifest.write_text(text, encoding="utf-8")
            print(f"wrote {manifest}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
