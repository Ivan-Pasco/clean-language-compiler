"""Check the Lark grammar of grammar/*.md against the corpus.

Usage:
    python3 -I check.py [<grammar dir>] [--verbose]

1. Composes the ```lark fences of the grammar chapters in the fixed order
   of COMPOSITION, and builds an LALR(1) parser with Lark's contextual
   lexer.  Any shift/reduce or reduce/reduce conflict is reported and the
   check fails.
2. Parses every case of corpus.json whose expected is not `illustrative`
   with the entry the case names (chosen by extract.py, see `entry_of`
   there), and compares with the expected result: `passes` must parse;
   `fails:SYNxxx` must not (any syntax error will do).
3. Reports the productions no passing case uses: a production without an
   example is surplus.

Exit 0 only when there is no conflict, no mismatch and no unbacked
production.  The report names each mismatch by file:line of foundation.
"""
from __future__ import annotations

import io
import json
import logging
import re
import sys
from pathlib import Path

from lark import Lark, Token, Tree, logger as lark_logger
from lark.exceptions import LarkError, UnexpectedInput
from lark.indenter import Indenter

COMPOSITION = [
    "lexical", "file-structure", "expressions", "statements", "control-flow",
    "functions", "classes", "contracts", "errors", "state", "testing", "ai",
    "modules", "async", "apply-blocks", "method-syntax", "block-handlers",
]
# entry name (corpus and heuristic) -> start rule of the grammar
ENTRY_RULE = {
    "file": "file",
    "fragment": "fragment",
    "expression": "expression_entry",
    "outer_block": "outer_block",
    "identifier": "identifier_entry",
}
ENTRIES = list(ENTRY_RULE.values())

LARK_FENCE = re.compile(r"^```lark[ \t]*\n(.*?)^```[ \t]*$", re.S | re.M)


class IndentationError_(Exception):
    """SYN003: indentation is not one tab per level."""


class CleanIndenter(Indenter):
    """Lexical Structure — Whitespace and Indentation.

    One tab per level.  A space in the leading whitespace of a line, or a
    rise of more than one level, is SYN003.  Inside (), [] and {} no
    newline or indentation event is emitted (Expressions — Multi-Line
    Expressions).
    """

    NL_type = "_NL"
    OPEN_PAREN_types = ["LPAR", "LSQB", "LBRACE"]
    CLOSE_PAREN_types = ["RPAR", "RSQB", "RBRACE"]
    INDENT_type = "_INDENT"
    DEDENT_type = "_DEDENT"
    tab_len = 1

    def handle_NL(self, token):
        if self.paren_level > 0:
            return
        yield token
        lead = token.rsplit("\n", 1)[1]
        if " " in lead:
            raise IndentationError_(f"line {token.end_line}: indentation with spaces")
        level = len(lead)
        if level > self.indent_level[-1]:
            if level != self.indent_level[-1] + 1:
                raise IndentationError_(f"line {token.end_line}: indentation rises by more than one tab")
            self.indent_level.append(level)
            yield Token.new_borrow_pos(self.INDENT_type, lead, token)
        else:
            while level < self.indent_level[-1]:
                self.indent_level.pop()
                yield Token.new_borrow_pos(self.DEDENT_type, lead, token)
            if level != self.indent_level[-1]:
                raise IndentationError_(f"line {token.end_line}: dedent to an unknown level")


def compose(grammar_dir: Path) -> str:
    parts = []
    for name in COMPOSITION:
        path = grammar_dir / f"{name}.md"
        text = path.read_text(encoding="utf-8")
        for m in LARK_FENCE.finditer(text):
            parts.append(f"// ---- {name}.md\n{m.group(1)}")
    return "\n".join(parts)


def build(grammar: str, debug_conflicts: bool = True):
    """Build the parser; return (parser, conflicts)."""
    stream = io.StringIO()
    handler = logging.StreamHandler(stream)
    lark_logger.addHandler(handler)
    old = lark_logger.level
    lark_logger.setLevel(logging.DEBUG)
    try:
        parser = Lark(grammar, parser="lalr", lexer="contextual", postlex=CleanIndenter(),
                      start=ENTRIES, debug=debug_conflicts, propagate_positions=False,
                      maybe_placeholders=False)
    finally:
        lark_logger.removeHandler(handler)
        lark_logger.setLevel(old)
    conflicts = [ln for ln in stream.getvalue().splitlines() if "conflict" in ln.lower() or ln.startswith("\t")]
    return parser, conflicts


def parse(parser, case):
    entry = case["entry"]
    code = case["code"]
    try:
        tree = parser.parse(code + "\n", start=ENTRY_RULE[entry])
        return entry, tree, None
    except (UnexpectedInput, IndentationError_, LarkError) as e:
        msg = str(e).strip().split("\n")[0]
        return entry, None, msg


def rule_names(grammar: str):
    names = set()
    for m in re.finditer(r"^[?!]?([a-z_][a-z0-9_]*)\s*(\.\d+)?\s*:", grammar, re.M):
        if not m.group(1).startswith("_"):
            names.add(m.group(1))
    return names


def used_rules(tree, acc):
    if isinstance(tree, Tree):
        acc.add(tree.data if isinstance(tree.data, str) else tree.data.value)
        for ch in tree.children:
            used_rules(ch, acc)


def main(argv):
    root = Path(__file__).resolve().parents[2]
    args = [a for a in argv if not a.startswith("--")]
    gdir = Path(args[0]) if args else root / "grammar"
    verbose = "--verbose" in argv
    grammar = compose(gdir)
    try:
        _, conflicts = build(grammar)
        parser, _ = build(grammar, debug_conflicts=False)
    except LarkError as e:
        print(f"grammar does not build: {e}")
        return 1
    if conflicts:
        print("LALR conflicts:")
        for c in conflicts:
            print("  " + c)
    cases = json.loads((gdir / "corpus.json").read_text(encoding="utf-8"))["cases"]

    # Coverage parser: the same grammar with every rule kept in the tree.
    keep = re.sub(r"^\?([a-z_])", r"\1", grammar, flags=re.M)
    cover_parser, _ = build(keep, debug_conflicts=False)

    mismatches = []
    used = set()
    checked = 0
    for c in cases:
        if c["expected"] == "illustrative":
            continue
        checked += 1
        entry, tree, err = parse(parser, c)
        ok_expected = c["expected"] == "passes"
        if (tree is not None) != ok_expected:
            mismatches.append((c, entry, err))
        elif tree is not None:
            _, t2, _ = parse(cover_parser, c)
            used_rules(t2, used)

    by_file = {}
    for c, entry, err in mismatches:
        by_file.setdefault(c["file"], []).append((c, entry, err))
    for f in sorted(by_file):
        print(f"{f}:")
        for c, entry, err in by_file[f]:
            what = "should parse" if c["expected"] == "passes" else f"should fail ({c['expected']})"
            print(f"  {f}:{c['line']}  [{entry}] {what}" + (f": {err}" if err else ""))
            if verbose:
                for ln in c["code"].split("\n"):
                    print("      | " + ln.replace("\t", "→   "))

    unbacked = sorted(rule_names(grammar) - used - set(ENTRIES))
    if unbacked:
        print("productions with no passing example: " + ", ".join(unbacked))

    print(f"checked {checked} cases: {checked - len(mismatches)} as expected, {len(mismatches)} mismatches; "
          f"{len(conflicts)} conflict lines; {len(unbacked)} unbacked productions")
    return 1 if (mismatches or conflicts or unbacked) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
