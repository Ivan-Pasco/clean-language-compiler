# CLAUDE.md

## Reglas

1. **No usar nunca registros en memoria** (el directorio de auto-memoria de Claude). Si hay que recordar algo, escribirlo aquí, en CLAUDE.md.
2. **Toda verificación es doble: spec y código.** Cuando el usuario pida verificar algo, comprobarlo tanto en la spec de foundation como en el código fuente, mostrar las discrepancias encontradas entre ambos, y dar recomendaciones para cerrar esas diferencias.
3. **La spec vive en foundation; el contrato compiler↔hosts en `contracts/`.** La autoridad de la spec es el checkout hermano `../clean-language-foundation` (pinneado: implementado contra foundation @ `e042f96`, 2026-08-22; actualizar este pin en cada round-trip). Los checkouts hermanos (`../clean-language-foundation`, `../clean-server`, `../clean-host-core`, `../clean-language-compiler-old`) son **de solo lectura** desde sesiones de este repo (forzado por hook); la única excepción es foundation `work/`, donde se escriben task briefs. Una ambigüedad o silencio de spec no se resuelve aquí: se convierte en task brief en foundation `work/` y se resuelve allá, desde una sesión de foundation. `contracts/` (el `host.wit`, procedencia en `contracts/SOURCE.md`) sí es autoridad de este repo — decisión de gobernanza separada, no revertida —; un cambio ahí exige propagarlo a los repos de hosts a mano.

## Dónde vive la verdad

Rutas relativas a `../clean-language-foundation/`:

- `03 platform/14-compiler-architecture.md` — el contrato (`CMP-01..06`): esquema del request, salidas, pipeline de 10 pasadas, determinismo, build manifest, operaciones de la API v1.
- `02 components/compiler/01-specification.md` — frontera del componente (`CCMP-`): qué posee este componente y las 20 cosas que rehúsa hacer.
- `04 language/` — capítulos Accepted; `04 language/grammar/*.ebnf.md` es la **fuente de verdad de la sintaxis** (DOC-15) — parsear desde el EBNF, nunca desde la prosa. `04 language/00-scope-and-conformance.md` (CNF-01..07) define programa válido e implementación conforme.
- `03 platform/09-error-codes.md` + `10-semantic-rules.md` — cada código, con la **plantilla literal del mensaje** que el compiler debe emitir. Copiar plantillas al pie de la letra; nunca redactar.
- `03 platform/13-diagnostic-format.md` — el valor `Diagnostic`, serialización NDJSON, render CLI, disciplina de fixtures DIA-06.
- `01 governance/` — glosario, principios LANG, concerns (C-NN) y `decisions/` (ADRs). Decisiones locales que desvían o refinan un ADR viven en `docs/adr/`.

Los tests dependientes de spec (`registry_spec.rs`, la pata de deriva de `grammar_fuzz.rs`) leen el checkout hermano y se auto-skipean (ruidosamente) cuando no está — en CI siempre se skipean: el checkout de foundation es privado y no se clona ahí.

**Round-trips a foundation:** un round-trip lleva **solo lo que los documentos de foundation prescriben como su alcance**: contratos cross-component, texto de spec, huecos/silencios de spec, ADRs y los *números o forma* de sus implementaciones de referencia. Mantenimiento local del repo (bumps de dependencias/actions, lints) y decisiones operativas del dueño (billing, visibilidad) nunca van en un brief de foundation — aun cuando toquen un archivo etiquetado "reference implementation".

## Principios de arquitectura (DDD estratégico)

4. **Encapsulamiento — la superficie pública es una decisión, no un accidente.**
   Cada crate expone solo su API curada. En `clean-compiler`, los módulos del pipeline (`lexer`, `parser`, `hir`, `mir`, `codegen`, …) son `pub(crate)`; lo público es únicamente lo re-exportado en `lib.rs` (`check`, `compile`, `repro_build`, `why`, y `types`). Los consumidores (`clean-compiler-bin`, `clean-language-server`) importan solo esos re-exports, nunca módulos internos. `clean-compiler-types` es la excepción deliberada: sus campos son `pub` porque es formato de cable (wire format), no modelo interno.
   *Verificación:* agregar un ítem `pub` nuevo a `clean-compiler` es un cambio de API y se anota en el mensaje de commit; en revisión, todo import de un consumidor que no venga de un re-export es un hallazgo.

5. **Dependencias acíclicas — el grafo de crates es un DAG fijo.**
   `clean-compiler-types` ← `clean-compiler` ← {`clean-compiler-bin`, `clean-language-server`}. No se agregan aristas nuevas entre crates internos sin un ADR. Dentro de `clean-compiler`, las etapas respetan el orden del pipeline: una etapa nunca importa de una etapa posterior (el lexer no conoce al parser, el parser no conoce a hir, etc.); los tipos compartidos entre etapas suben a un módulo común, no cruzan hacia atrás.
   *Verificación:* Cargo garantiza el nivel crate; el nivel módulo se revisa en cada cambio que toque `use` entre etapas.

6. **Invariantes — "parse, don't validate": el tipo atestigua la validación.**
   Todo dato externo cruza la frontera una sola vez y sale como un tipo testigo (`ValidatedRequest` es el patrón de referencia); el pipeline interior nunca re-valida lo que un tipo testigo ya garantiza. Todo invariante documentado en un tipo (ej. `Span`: `end >= start`, 1-based) debe estar forzado en algún punto: `debug_assert!` en el constructor y/o validación en el intake — un invariante que solo vive en un comentario es un bug latente.
   *Verificación:* al declarar o documentar un invariante nuevo, señalar en qué línea se fuerza; si no se fuerza en ninguna, es un hallazgo.

7. **Design by contract — todo contrato observable tiene un test que falla al romperlo.**
   La superficie observable es contrato: el formato de `diagnostics.json`, los códigos de diagnóstico y sus mensajes exactos, `contracts/host.wit`, y el `BuildManifest`. Cada contrato tiene al menos un test que lo fija (snapshot, conformance, o equivalencia — como LSP ≡ `check`). Cambiar un contrato exige, en el mismo cambio: actualizar el test que lo fija, llevar el cambio de spec a foundation como round-trip (regla 3), y (si toca `contracts/`) propagar a los hosts.
   *Verificación:* un diff que cambia salida observable sin tocar un test de contrato es un hallazgo.
