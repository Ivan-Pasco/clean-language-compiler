# Vendored conformance corpora — JSON

| Upstream | Pinned commit | Vendored |
|---|---|---|
| [nst/JSONTestSuite](https://github.com/nst/JSONTestSuite) | `1ef36fa01286573e846ac449e8683f8833c5b26a` | `JSONTestSuite/test_parsing/` (318 files) |

Vendoring discipline (05 execution/testing/06-stdlib-conformance-testing.md
§4): never a floating clone. Refreshing the corpus is a deliberate PR that
(a) updates the SHA here, (b) re-pins any new `i_*` verdicts in
`i_verdicts.txt`, and (c) records the pass/fail delta in the commit
message. Post-ADR-0010 there is no separate decisions document: the
accept/reject boundary for implementation-defined cases is what the
RUN007/RUN009/RUN010 rule conditions state, and `i_verdicts.txt` pins this
parser's observed verdict per `i_*` file so CI catches drift.

Upstream license: MIT (Nicolas Seriot). The corpus is test data only.

## Expected divergences from upstream verdicts

- `y_object_duplicated_key.json`, `y_object_duplicated_key_and_value.json`
  — rejected here by design: RUN009's Accepted condition rejects
  duplicate object keys ("last-wins and first-wins both discard data"),
  deliberately stricter than the corpus oracle.
- 25 files are not valid UTF-8 and are outside the parser's input domain
  (its input is a Clean string, well-formed UTF-8 by TXT-01); the runner
  pins the exact count and names them.
