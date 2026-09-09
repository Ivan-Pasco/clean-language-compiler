# CLAUDE.md

## Qué es este repo (2026-09-09)

Repo GitHub de la **implementación** del compilador de Clean Language.
Aquí va **solo código** (crates, tests, CI).

La **documentación vive en el repo hermano `../clean-language-foundation`**
(spec del lenguaje, arquitectura, decisiones). No crear ni restaurar aquí
`docs/`, `README.md`, ADRs ni notas de trabajo: si algo hay que documentar,
va a foundation.

## Avance del desarrollo (2026-09-09)

El avance se sigue con un **Work Breakdown Structure (WBS)** en la carpeta
`Scope/` de este repo.

## Excepciones a "solo código aquí" (2026-09-09)

Memoria operativa del trabajo sobre este código, no documentación del lenguaje:

- `Scope/` — el WBS.
- `docs-for-claude/` — notas que Claude relee; `INDEX.md` dice qué carga cada skill.
- `docs-for-humans/` — narrativa de sesiones (Claude no la lee).
- `backlog.md` — pendientes accionables.

Las tres últimas las alimenta `/X40-capture`. Todo lo demás documental va a foundation.
