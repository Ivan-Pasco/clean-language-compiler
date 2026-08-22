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
| `01 governance/` | `01 governance/` (completo) | Gobernanza: principios de documentación (DOC), glosario (vocabulario controlado), principios de lenguaje (LANG), concerns (C), seguridad/perf/interop, playbook de calidad, y `decisions/` (todos los ADRs). Añadido 2026-08-21, mismo commit de origen. |
| `04 language/` | `04 language/` (completo) | Los 21 capítulos del lenguaje, `grammar/*.ebnf.md` (autoridad de sintaxis, DOC-15), `schema/` |
| `03 platform/` | `03 platform/` (completo) | Los 18 docs de plataforma: error codes (09), semantic rules (10), diagnostic format (13), compiler architecture (14, contrato CMP), host bridge (02), execution layers (01), component model (15/18), etc. |
| `02 components/compiler/` | `02 components/compiler/` | Frontera del componente compiler (CCMP) |
| `02 components/manager/automation.md` | `02 components/manager/automation.md` | Contrato de release/CI citado por `.github/workflows/release.yml` |
| `05 execution/testing/` | solo `00-testing-strategy-overview.md` y `01-compiler-testing.md` | Los dos docs que TESTING.md usa como plantilla |

No se copió el resto de foundation (componentes de hosts/bridges/framework,
testing de otros componentes): no es spec del compiler.

> **Revisión 2026-08-21:** la copia original excluyó `01 governance/` por la
> misma razón. Se revisó esa decisión: los capítulos vendoreados citan la
> gobernanza cientos de veces (glosario, principios LANG, concerns, ADRs), y
> una auditoría contra el best practice de especificación de lenguajes mostró
> que el "artefacto 1" (metas, principios, vocabulario controlado) vive ahí.
> Se vendoreó completa, del mismo commit `70f1571`.

## Notas

- `03 platform/wit/` contiene solo un README también en el origen: los
  archivos WIT nunca aterrizaron en foundation (ver `stub.rs` y
  DISCOVERIES-M8 §7; clean-server mantiene su propio `host.wit`).
- Dentro de `03 platform/` hay documentos que son **contrato compartido con
  los hosts** (01 execution layers, 02 host bridge, 08 bridge versioning,
  15/16/18 component model). Un cambio a esos documentos hecho aquí debe
  propagarse a los repos de hosts deliberadamente; no lo ven solos.
- Los enlaces relativos internos que apuntan fuera de las secciones copiadas
  quedan rotos por diseño; el texto citado es el que gobierna. Desde el
  añadido de `01 governance/` (2026-08-21) los enlaces a gobernanza y ADRs
  resuelven; siguen rotos solo los que apuntan a componentes no vendoreados
  (p. ej. `02 components/framework/`, hosts).

## Cambios locales desde la copia

Registrar aquí todo cambio hecho a `specs/` en este repo (fecha, archivo, qué
y por qué). Vacío = copia fiel al commit de origen.

- 2026-08-22 — **Overview endurecido** `04 language/01-overview.md`: las seis
  metas-viñeta ahora citan sus hogares normativos (`LANG-NN`, capítulos);
  sección nueva de No-Metas que apunta a `LANG-20` (sin duplicar la lista,
  disciplina de un-solo-hogar); puntero al capítulo 00 de conformidad. El
  overview sigue siendo no-normativo. **Pendiente: proponer upstream.**
- 2026-08-21 — **Capítulo nuevo** `04 language/00-scope-and-conformance.md`
  (prefijo `CNF-`; Accepted 2026-08-22): alcance de la spec, vocabulario normativo
  (MUST/SHOULD/MAY anclados a RFC 2119/8174) y cláusula de conformidad
  (programa válido, implementación conforme, sin dialectos, sin undefined
  behavior, supremacía del texto). Motivo: auditoría contra el best practice
  detectó que ninguna parte del ecosistema definía "implementación conforme"
  ni anclaba el vocabulario normativo. Cambios acompañantes: fila `CNF-` en
  el registro de prefijos de `01 governance/README.md` y fila del capítulo en
  `04 language/README.md`. **Pendiente: proponer upstream a foundation.**
- 2026-08-21 — Ampliación del alcance de la copia: se añadió `01 governance/`
  completo (11 docs + 30+ ADRs) desde el mismo commit de origen `70f1571`,
  sin modificaciones. Motivo: auditoría del "artefacto 1" de la spec (alcance
  y conformidad) — el glosario, los principios LANG-NN y los concerns que las
  specs vendoreadas citan vivían solo en foundation, dejando la autoridad
  local incompleta y con enlaces rotos. No es un cambio de contenido: la
  copia sigue fiel al commit de origen.
