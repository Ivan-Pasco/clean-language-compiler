# specs/ — Vendored specification

Copia vendoreada de las secciones de `clean-language-foundation` relevantes
para el compiler. **Esta copia es la autoridad para este repositorio**: el
compiler se implementa contra `specs/`, no contra el checkout hermano.

## Origen

- Repo: `clean-language-foundation` (checkout hermano `../clean-language-foundation`)
- Commit: `70f15711a114c1f0fa13008f82c2b0910b2af108` (2026-08-20, working tree limpio)
- Fecha de la copia: 2026-08-21

## Qué se copió

| Directorio aquí | Origen en foundation | Contenido |
|---|---|---|
| `04 language/` | `04 language/` (completo) | Los 21 capítulos del lenguaje, `grammar/*.ebnf.md` (autoridad de sintaxis, DOC-15), `schema/` |
| `03 platform/` | `03 platform/` (completo) | Los 18 docs de plataforma: error codes (09), semantic rules (10), diagnostic format (13), compiler architecture (14, contrato CMP), host bridge (02), execution layers (01), component model (15/18), etc. |
| `02 components/compiler/` | `02 components/compiler/` | Frontera del componente compiler (CCMP) |
| `02 components/manager/automation.md` | `02 components/manager/automation.md` | Contrato de release/CI citado por `.github/workflows/release.yml` |
| `05 execution/testing/` | solo `00-testing-strategy-overview.md` y `01-compiler-testing.md` | Los dos docs que TESTING.md usa como plantilla |

No se copió el resto de foundation (gobernanza, componentes de hosts/bridges/
framework, testing de otros componentes): no es spec del compiler.

## Notas

- `03 platform/wit/` contiene solo un README también en el origen: los
  archivos WIT nunca aterrizaron en foundation (ver `stub.rs` y
  DISCOVERIES-M8 §7; clean-server mantiene su propio `host.wit`).
- Dentro de `03 platform/` hay documentos que son **contrato compartido con
  los hosts** (01 execution layers, 02 host bridge, 08 bridge versioning,
  15/16/18 component model). Un cambio a esos documentos hecho aquí debe
  propagarse a los repos de hosts deliberadamente; no lo ven solos.
- Los enlaces relativos internos que apuntan fuera de las secciones copiadas
  (p. ej. a `01 governance/decisions/`) quedan rotos por diseño; el texto
  citado es el que gobierna.

## Cambios locales desde la copia

Registrar aquí todo cambio hecho a `specs/` en este repo (fecha, archivo, qué
y por qué). Vacío = copia fiel al commit de origen.

(ninguno)
