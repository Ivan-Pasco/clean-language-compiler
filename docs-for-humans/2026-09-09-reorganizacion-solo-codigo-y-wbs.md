# Reorganización: solo código aquí, docs en foundation, avance por WBS

_Capturado 2026-09-09 mientras se vaciaba el repo de documentación y se preparaba el seguimiento del avance._

## Qué motivó la sesión

El repo del compilador arrastraba documentación propia: README, TESTING, `docs/` con siete ADRs y nueve DISCOVERIES-M1..M9, `work/`, `examples/acceptance-guest`, `scripts/build-acceptance.sh`, y un CLAUDE.md de 53 líneas con reglas de gobernanza (protocolo bloquear-y-decidir, políticas del owner, principios DDD, "dónde vive la verdad"). También los hooks de `.claude/` que forzaban solo-lectura sobre los checkouts hermanos (`clean-server`, `clean-host-core`, `clean-language-compiler-old`).

El dueño decidió partir: **este repo es solo código**; toda la documentación vive en el hermano `../clean-language-foundation`. Borró todo lo anterior, vació el CLAUDE.md y, en un segundo paso, borró también `contracts/` (`host.wit` + `SOURCE.md`), que el CLAUDE.md viejo declaraba autoridad de este repo.

## Lo que se rompió y cómo se detectó

El commit de limpieza se pusheó a `main` antes de compilar. Un grep posterior mostró que `crates/clean-compiler/tests/vendored_wit.rs` hacía `include_str!("../../../contracts/host.wit")`: el crate de tests dejó de compilar. Se confirmó con `cargo test --no-run`, se quitó la aserción (queda el pin sha256 del fixture `tests/fixtures/wit/host.wit`) y se pusheó un segundo commit. Lección: grep de `include_str!` antes de borrar carpetas (nota en `docs-for-claude/`).

Quedó una consecuencia sin resolver: **el `host.wit` ya no tiene un hogar "de cara al ecosistema"**. Foundation no tiene ninguna copia; la única que existe en este repo es el fixture de tests, cuyo comentario dice que es copia del contrato autoritativo de `clean-server`. Va al backlog.

## Decisiones de la sesión

1. **Solo código aquí; docs en foundation.** Anotado en `CLAUDE.md`.
2. **El avance del desarrollo se sigue con un Work Breakdown Structure (WBS) en `Scope/`**, única excepción declarada a "solo código". La carpeta se creó vacía (`.gitkeep`); el WBS en sí está pendiente.
3. **Se importan dos skills de Indra Software Factory**: `X10-explore` y `X40-capture`, copia literal. X40 se corrió por primera vez en esta sesión, y para eso se crearon `docs-for-claude/`, `docs-for-humans/` y `backlog.md` en este repo.

## Tensión abierta

La decisión 1 ("solo código") y la 3 (X40 escribe `docs-for-claude/`, `docs-for-humans/` y `backlog.md` en el repo) se contradicen en la letra. La captura se hizo aquí porque el dueño pidió correr X40 en este repo y foundation no tiene esas carpetas. Falta que el dueño diga si estas carpetas son una segunda excepción (como `Scope/`) o si X40 debe escribir en foundation.
