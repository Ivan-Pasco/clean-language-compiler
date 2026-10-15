# CLAUDE.md

## What this repo is (2026-09-09)

GitHub repo of the **implementation** of the Clean Language compiler.
It holds **only code** (crates, tests, CI).

The **documentation lives in the sibling repo `../clean-language-foundation`**
(language spec, architecture, decisions). Do not create or restore ADRs or working notes here: anything that needs documenting goes to foundation.

## Language (2026-10-14)

- This repo is written entirely in **English**: code, comments, tests, CI and
  this file. Anything found in Spanish is translated.
- Spec changes in foundation are **always written in English**, even when the
  user chats in Spanish.

## Methodology: spec-driven design (2026-10-14)


- **Never write code that is not in the specs.**
- Code without a spec is **surplus**: remove it, or specify it first.
- A spec without code is **missing**: it is pending work.
- An ambiguous or contradictory spec is resolved in foundation before coding.
- **Human in the loop:** never change a spec in foundation without the user's
  explicit approval. Propose the change and wait for the yes.
- **Every proposed change to a foundation document is shown as a diff in VS
  Code** before it is applied: write the proposed version to the scratchpad and
  open
  `"/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" --diff <original> <proposal>`.
  Only after the user approves is it applied to the real file.

