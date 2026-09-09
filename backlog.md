# Backlog

Trabajo pendiente accionable del compilador. Lo alimenta `/X40-capture`; lo cierra el flujo que ejecuta cada item.

## Convenciones

**Prefijos**:
- `BUG-XXX`   bugs
- `TECH-XXX`  deuda técnica (refactors, stubs, violaciones de reglas)
- `TEST-XXX`  gaps de cobertura
- `DOC-XXX`   deuda de documentación (WBS, foundation, comentarios)
- `CHORE-XXX` housekeeping (inconsistencias menores, skills, CLAUDE.md)

**Prioridad**: P0 (urgente / viola reglas inviolables) · P1 (estructural) · P2 (menor)

**Estado**: `open` · `in-progress` · `done` (los `done` se borran cuando ya están reflejados en el código)

**Formato de item**: `### {ID} · {título corto}` + bullets **Origen** / **Problema** / **Alcance/Acción** / **Estado**. Items nuevos van al final de su sección de prioridad con el siguiente número libre del prefijo; no renumerar al borrar.

## P0 — Violaciones de reglas inviolables

(ninguno)

## P1 — Estructural

### TECH-001 · Dar hogar al `host.wit` fuera del fixture de tests
- **Origen**: 2026-09-09, borrado de `contracts/` en la reorganización "solo código".
- **Problema**: la única copia del contrato compiler↔hosts en este repo es `tests/fixtures/wit/host.wit` (pinneada por sha256 en `vendored_wit.rs`). Foundation no tiene ninguna. El fixture dice ser copia del autoritativo de `clean-server`, pero nada lo verifica ya.
- **Alcance/Acción**: decidir dónde vive la autoridad (foundation, `clean-server`, o el fixture mismo) y, si es un repo externo, reponer un gate que compare el fixture contra esa fuente (auto-skip local, obligatorio en CI como los tests de spec).
- **Estado**: open

### DOC-001 · Diseñar el subárbol de alcance del compilador (en foundation)
- **Origen**: 2026-09-09, decisión de seguir el avance con un Work Breakdown Structure. El mismo día se decidió que el árbol vive en foundation (SCM-05) y no en una carpeta `Scope/` de este repo; la carpeta se borró.
- **Problema**: el compilador no tiene subárbol de alcance: sin superficies ni paquetes de trabajo no hay medida de avance. Foundation tampoco tiene todavía el tronco (familias, componentes, capítulos que gobiernan a cada uno) ni el skill `/scope-design` de UC-13.
- **Alcance/Acción**: en foundation, crear el tronco y diseñar el subárbol del compilador en `02 components/compiler/` (UC-13): superficies alineadas a capítulos, paquetes nombrados por comportamiento y dimensionados a una sesión, cada unidad vigente llevada por al menos un paquete. En este repo queda solo la evidencia (tests que citan unidades); pendiente aquí un checker que compruebe que cada unidad de un paquete `done` está citada por un test verde.
- **Estado**: open

## P2 — Menor

### CHORE-001 · Comentarios que citan `docs/adr/*` y `docs/DISCOVERIES-M*.md` borrados
- **Origen**: 2026-09-09, grep tras la limpieza.
- **Problema**: `Cargo.toml`, `crates/clean-compiler/Cargo.toml`, `blocks/ir.rs`, `blocks/abi.rs`, `blocks/sandbox.rs`, `parser/ast.rs`, `parser/parse.rs`, `tests/modules.rs`, `tests/render_cli.rs` y `examples/perf_budget.rs` citan rutas que ya no existen. `perf_budget.rs` además le pide al usuario anotar números en DISCOVERIES-M9.
- **Alcance/Acción**: verificar que el contenido de esos ADRs y DISCOVERIES llegó a foundation (si no, recuperarlo de git y llevarlo); reapuntar los comentarios a su ubicación en foundation o quitarlos.
- **Estado**: open

### CHORE-002 · Adaptar `X10-explore` y `X40-capture` a este repo
- **Origen**: 2026-09-09, copia literal desde Indra Software Factory.
- **Problema**: X10 promueve hacia `B05`, `A10` y `X50`, que aquí no existen. X40 exige una fila en `audits.md` por cada convención (no existe).
- **Alcance/Acción**: podar las salidas de X10 a lo que existe, y crear `audits.md` o quitar la regla de X40. (Los destinos de captura ya se decidieron el 2026-09-09: viven aquí, como excepción declarada en CLAUDE.md.)
- **Estado**: open

### CHORE-003 · Decidir si vuelven los hooks de solo-lectura sobre los checkouts hermanos
- **Origen**: 2026-09-09, borrado de `.claude/settings.json`, `path_allowlist.py` y `workaround_detector.py`.
- **Problema**: el CLAUDE.md anterior decía que `../clean-server`, `../clean-host-core` y `../clean-language-compiler-old` eran solo lectura "forzado por hook", y que foundation solo se escribía tras aprobación. Sin los hooks, esa protección es solo una regla de texto, y hoy ni siquiera está escrita.
- **Alcance/Acción**: el dueño decide si se restauran (desde git, commit `e621757`) o si la regla se reescribe en CLAUDE.md sin enforcement.
- **Estado**: open
