# Descubrimientos del round-trip del drenaje (2026-08-22)

Registro de lo hallado al implementar los follow-ups del informe
`clean-language-foundation/work/2026-08-22-drain-round-trip-report.md` §2
contra foundation `f842c8e`. No es una cola de decisiones (ese régimen
está retirado — CLAUDE.md regla 3): las **discrepancias de spec** de la
sección 1 esperan un brief de vuelta a foundation o la decisión del dueño
en sesión; las **adopciones locales** de la sección 2 son interpretaciones
mínimas donde la spec calla en un detalle no bloqueante, listadas para su
ratificación o corrección.

## 1. Discrepancias de spec (para brief de vuelta / decisión del dueño)

1. **El ejemplo de §SEM030 usa sintaxis ilegal.** Platform 10 §SEM030
   ilustra el fallo con `print(i)`, pero la gramática (autoridad DOC-15)
   solo define `print` como bloque `print:` (grammar/07 §6); `print(i)`
   parsea como SYN002/SYN008. El fixture DIA-06 de SEM030 usa un cuerpo
   neutro. El ejemplo del capítulo debería reescribirse con sintaxis
   legal.
2. **LIB021 no enumera el leg de inicial minúscula en su condición.** El
   changelog de LBS-02 asigna la validación del requisito de inicial
   minúscula a LIB021 ("implement the clause, the lowercase-initial
   check, and LIB021"), pero la **Condition** de 10 §LIB021 solo enumera
   colisión y string inválido. Implementado como LIB021 con el brazo
   "is not a valid WIT identifier" sobre el nombre declarado sin
   proyección sancionada (test-pinneado). La condición de la regla
   debería ganar el tercer leg explícito.
3. **`max-import-depth` no tiene regla de conteo.** 07 §7.8 lo lista sin
   definir qué mide (a diferencia de `max-nesting-depth`, que tiene su
   regla en 10 §BLD001), y su comentario dice "library dependency
   graphs" cuando el límite viaja al compilador, que ve el grafo de
   módulos. Adopción local: aristas de la cadena de imports acíclica más
   larga sobre el grafo de módulos; BLD001 una vez, anclado en la
   primera arista (orden de entrada) sobre una cadena excesiva.
   Necesita regla normativa.
4. **`request_sha256`: canonicalización pendiente** (ítem 14 del
   informe, "decidir o relajar"): la serialización serde del request no
   sigue la canonicalización de §14.15.1. Registrado también en la
   anotación de ADR-0007. Decisión del dueño.
5. **Params de `clean/requestDocument` sin especificar.** 04 §4.10 dice
   "carrying a complete replacement document… same intake as the
   bootstrap" sin fijar el sobre. Adoptado: el mismo sobre de
   `initializationOptions` (`{requestDocument, requestDocumentUri}`).
   Además: una notificación **sin** `requestDocument` se trata como
   defecto del caller (log una vez, la sesión viva se conserva) — el
   paralelo bootstrap habría destruido la sesión; elegido lo menos
   destructivo. Ambos puntos piden confirmación de spec.

## 2. Adopciones locales menores (para ratificar en el siguiente ciclo)

6. **Anclaje de BLOCK007–009**: la spec da plantillas pero no spans ni
   labels. Adoptado: los tres anclan en la declaración `handles block`
   (como BLOCK003/SEM019), sin primary label; BLOCK009 corta en seco los
   demás checks del registro. Pineado por los fixtures DIA-06.
7. **Leg de heap de LIB014**: la disciplina DIA-06 es un triple por
   código y el de LIB014 ejercita el leg de nodos IR; el pin del leg de
   heap que foundation pidió vive como test byte-exacto en
   `blocks_expansion.rs` (mismo patrón que el leg fuente de BLOCK003).
8. **Stub de referencia del grammar**: 08-file-structure dejó
   `LibraryBlock = ? defined in 21-… §1a ? ;` — el loader del fuzzer
   aprendió ese segundo deletreo de stub (junto a `? see … ?`) para no
   contarlo como duplicado DOC-15. Si foundation prefiere el patrón
   WatchBlock (eliminación total), el stub sobra.
9. **`..` más allá de la raíz**: resolvido como "sin match" → IMPORT002
   (los paths del request son root-relativos, MOD-03); antes el clamping
   silencioso podía inventar un match de raíz. Coherente con el ítem 10
   del informe; sin texto de spec propio aún.

## 3. Notas de estado (sin acción en este repo)

- El follow-up 7 (transcendentales/`^`/case folding guest) resultó de
  escala de milestone — libm determinista emitido como guest; planificado
  como Etapa D en `docs/plan-2026-08-22-post-drain-milestones.md`, junto
  a las etapas de los follow-ups 16–18.
- El mundo `server` vendorizado sigue no-conforme para el stdlib
  host-backed (reloj/console) — obligación host/framework (ítem 15 del
  informe).
- SEM029 (`SecretStringComparison`) llegó registrado por la verificación
  de retiro de ADRs del mismo día (no venía en el informe); alta en
  codes.rs, en el ledger hasta que exista el tipo `secret`.
