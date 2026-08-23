# Plan — etapas de milestone tras el drenaje de briefs (2026-08-22)

Los follow-ups 16–18 del informe de round-trip (`clean-language-foundation/work/2026-08-22-drain-round-trip-report.md` §2/§3) más el ítem 7, que resultó ser de la misma escala. **Las decisiones de spec ya están aterrizadas a HEAD de foundation** — estas etapas son implementación pura contra spec cerrada; ninguna requiere escalado salvo lo anotado. No empezar una etapa sin cerrar la anterior de su cadena (16 y 18 son independientes entre sí; 17 depende del layout de 18 solo en la frontera de listas de `time`, que puede diferirse).

## Etapa A — `match`/`case` + typecheck de cuerpos `compiletime` (follow-up 16)

El más grande del round-trip. Spec: 21 §21.3.3 (cobertura exacta de variantes, sin brazo default, operando no-sum = SEM004 con su plantilla "cannot iterate…"-hermana), LEX-04 (+`match`, +`case` — **reserva rompedora**), gramática 21 §1b, SEM031 (ya registrado en codes.rs, en el ledger).

1. **Lexer**: reservar `match`/`case` como hard keywords. Rompedor: identificadores existentes con esos nombres pasan a SYN002. El grammar vendorizado ya los lleva (refresco 2026-08-22); revisar el corpus por reclasificaciones y regenerar snapshots deliberadamente.
2. **Parser**: producción de §1b — `match <expr>` + brazos `case <Variant> <binder>` indentados.
3. **Typecheck**: SEM031 con sus tres anclajes de label (falta un brazo → en `match`; duplicado → en el `case` duplicado; no-variante → en el `case` ofensor); operando no-sum → SEM004. Los únicos sums compile-time hoy: `BlockNode` (BlockAST). Con el constructo disponible, abrir el type-checking de cuerpos `compiletime` (TYP-04) y retirar el canal Unsupported "compiletime function bodies" — el desbloqueo real de la etapa.
4. **Fixtures**: DIA-06 de SEM031 (sale del ledger); pins del binding de `case`; los worked examples de 21 ahora legales entran al corpus.

## Etapa B — layout `datetime` + módulo `time` (follow-up 17)

Spec: Platform 03 §3.4.6 — epoch s64 + nanos u32 + offset-minutos s32; 16 bytes, **inline** (no boxed), compara el instante (offset excluido de la igualdad), `record` en la frontera WIT. Módulo `time` del cap. 15.

1. Layout en `layout.rs`/`elem_layout` (leaf de 16 bytes con tres campos; align 8).
2. Literales/constructores según cap. 15; comparaciones por instante.
3. Frontera WIT como `record` (proyección en typecheck/codegen como los records LBS-02).
4. Módulo `time`: solo las funciones que el mundo respalde (COM012 en el call site para las host-backed — mismo régimen que el resto del stdlib); el discriminador de clases nominales sigue **deferido por el dueño** (03 §3.4.4) — no tocarlo aquí.

## Etapa C — relayout de listas/pairs a handle estable (follow-up 18)

Spec: 03 §3.4.1/§3.4.2 — handle estable + `buffer_ptr` indirecto; crecimiento geométrico de factor **no observable**; FLW-02: `iterate` relee `buffer_ptr` en cada vuelta.

1. Relayout del header de lista (y de pairs — mismo mecanismo, decidido junto) a indirection por `buffer_ptr`.
2. Desbloquear `add`/`insert` (hoy "growing list methods" en Unsupported) con crecimiento geométrico; cap==len deja de ser invariante.
3. Lowering de `iterate` sobre listas: releer `buffer_ptr` por vuelta (el pin de aliasing TYP-08 de stdlib_list.rs debe seguir verde — es el contrato observable que el relayout preserva).
4. Revisar MEM/TIER: el trap de crecimiento sigue siendo MMD-02.

## Etapa D — cerrar el canal Unsupported de math guest (follow-up 7)

Decidido guest en el drenaje (BRG-05 reescrito; ADR-0004 local ratificado — ver su anotación). Es un **libm determinista emitido como funciones guest**: transcendentales (`sin`..`tanh`, `ln`/`log*`, `exp*`), `^` de number, y el case folding de string del cap. 15. Ninguna instrucción wasm los provee (solo `sqrt` es nativa); hay que emitir reducción de argumento + polinomios (estilo musl) con la misma disciplina de los helpers de ADR-0004, y pinear resultados byte-exactos (determinismo es el motivo de la decisión guest).

- Sub-etapa D1: `^` (number) vía `exp(ln)` + camino entero por cuadrados; `exp`/`ln` primero — desbloquean el resto.
- Sub-etapa D2: trigonometría e hiperbólicas.
- Sub-etapa D3: case folding de string según el texto exacto del cap. 15 (verificar si prescribe ASCII o Unicode simple antes de empezar; si es Unicode con tablas, evaluar coste de tabla en el guest y, si el texto de spec resulta ambiguo, bloquear-y-decidir).
- Los módulos stdlib host-backed (file/http/…) se abren "conforme el mundo los respalde" — fuera de esta etapa; dependen del ítem 15 del informe (obligación de hosts: restatear reloj/console en el mundo `server`).

## Registrado, no planificado aquí

- `request_sha256`: canonicalización serde vs §14.15.1 — "decidir o relajar" pendiente del dueño (anotado en ADR-0007 y en el registro de descubrimientos).
- Mundo `server` vendorizado no-conforme (reloj/console) — obligación host/framework, no de este repo.
