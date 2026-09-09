# CLAUDE.md

## Qué es este repo (2026-09-09)

Repo GitHub de la **implementación** del compilador de Clean Language.
Aquí va **solo código** (crates, tests, CI).

La **documentación vive en el repo hermano `../clean-language-foundation`**
(spec del lenguaje, arquitectura, decisiones). No crear ni restaurar aquí
`docs/`, `README.md`, ADRs ni notas de trabajo: si algo hay que documentar,
va a foundation.

## Avance del desarrollo (2026-09-09)

El avance se mide con el **árbol de alcance (WBS)** que vive en foundation,
junto a la spec del componente (`../clean-language-foundation/02 components/compiler/`,
regla SCM-05). Este repo no tiene carpeta de scope ni copia del árbol: aquí
queda solo la **evidencia**, los tests que citan las unidades (reglas, códigos
de diagnóstico, producciones de la gramática) que cada paquete de trabajo lleva.

## Excepciones a "solo código aquí" (2026-09-09)

Memoria operativa del trabajo sobre este código, no documentación del lenguaje:

- `docs-for-claude/` — notas que Claude relee; `INDEX.md` dice qué carga cada skill.
- `docs-for-humans/` — narrativa de sesiones (Claude no la lee).
- `backlog.md` — pendientes accionables.

Las tres las alimenta `/X40-capture`. Todo lo demás documental va a foundation.
