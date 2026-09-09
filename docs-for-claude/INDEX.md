# Índice de docs-for-claude

Este índice define **qué notas carga cada skill** al activarse.

## Cómo lo usan los skills

Cada skill, al activarse, lee:

1. La sección **Transversales (`*`)** — reglas que aplican a todo el repo.
2. Su propia sección en **Por skill**, con carga escalonada: por default solo las **★ (evergreen)**; el sub-grupo rotulado en **negrita** si la tarea cae en ese tema; una nota puntual cuando un grep la referencia.

## Cómo se mantiene

- Toda nota nueva creada por `/X40-capture` trae frontmatter (`title`, `captured`, `applies-to`, `topics`, `status`).
- Al agregar una nota, este `INDEX.md` se actualiza con la entrada en la sección de cada skill listado en `applies-to`.
- Una nota obsoleta se **borra** (git es el archivo) y su fila se elimina; la canónica que la consolida lo declara al pie.

Total al 2026-09-09: **1 nota activa**.

---

## Transversales (`*`) — 1 nota

- [Borrar carpetas top-level: los tests las incluyen por ruta](2026-09-09-borrar-carpetas-top-level-include-str.md)

---

## Por skill

### X10-explore (0)

### X40-capture (0)
