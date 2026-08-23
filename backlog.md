# Backlog

Trabajo pendiente accionable. Convenciones: prefijos `BUG-` (bugs), `TECH-`
(deuda técnica), `UC-` (use cases sin código), `TEST-` (gaps de cobertura),
`DOC-` (deuda de documentación), `CHORE-` (housekeeping). Prioridades: P0
(urgente, viola regla inviolable) · P1 (estructural) · P2 (menor). Los IDs son
citables: nunca renumerar. Numeración: siguiente número libre por prefijo.
Estado inicial `open`; quien cierra un item lo marca `done` con fecha.

## P0

_(vacío)_

## P1

### TECH-001 · Refactor de visibilidad en clean-compiler
- **Origen**: sesión 2026-08-22 — adopción de principios DDD estratégico (CLAUDE.md §4)
- **Problema**: `crates/clean-compiler/src/lib.rs` expone todos los módulos del pipeline como `pub mod` (lexer, parser, hir, mir, codegen, …); cualquier consumidor puede acoplarse al IR interno, contra el principio 4.
- **Alcance**: analizar qué importan realmente `clean-compiler-bin` y `clean-language-server`; bajar el resto a `pub(crate)`; verificar workspace completo (build + tests).
- **Estado**: open

## P2

### TECH-002 · Forzar invariantes documentados de Span/Position
- **Origen**: sesión 2026-08-22 — adopción de principios DDD estratégico (CLAUDE.md §6)
- **Problema**: `Span` documenta `end >= start` y 1-based pero nada lo fuerza: `Span::new` acepta cualquier valor y la deserialización serde no pasa por constructores.
- **Alcance**: `debug_assert!` en `Span::new` + validación en el intake del request; revisar si otros tipos de `clean-compiler-types` documentan invariantes sin forzar.
- **Estado**: open
