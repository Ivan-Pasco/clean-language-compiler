# X40 — referencia: `audit` y `conflicts`

Detalle operativo de los dos subcomandos on-demand de `/X40-capture`. Leer este archivo completo antes de ejecutar cualquiera de los dos.

---

## Audit on-demand (`/X40 audit`)

Subcomando **separado del flujo de captura**. No corre automáticamente. Lo dispara el usuario cuando quiere un sweep total de `docs-for-claude/`. Casos típicos:

- Después de un refactor grande del codebase (puede haber notas que mencionan código que ya no existe).
- Periódicamente (mensualmente, o cada 50 notas nuevas) para detectar drift acumulado que pre-check + session sweep no atrapan.
- Antes de empezar una sesión que va a tocar un módulo poco visitado (verificar que las notas vigentes son confiables).

### Invocación

```
/X40 audit                  → recorre toda la colección
/X40 audit <topic>          → audita solo notas con ese topic en frontmatter
/X40 audit <skill-name>     → audita solo notas que tienen ese skill en applies-to
```

### Qué chequea

1. **Notas potencialmente duplicadas**: pares de notas que comparten ≥2 topics y tienen `applies-to` solapado. Heurística simple, no infalible — el output es "candidatos a revisar", no "duplicados confirmados".
2. **Orphan archives**: notas con `status: archived` sin `superseded-by:` (la decisión nueva no se documentó, o se documentó en otro lado y se perdió la trazabilidad). Con la política actual (borrar las superadas — git es el archivo), toda nota `archived` que persista es un estado transicional sin resolver: candidata a borrado una vez confirmado el reversal.
3. **Refs muertas al codebase**: notas que mencionan archivos (`{App}.Hexagon.Domain/...`, paths absolutos, `docs-for-claude/X.md`) que ya no existen.
4. **`applies-to` rotos**: notas que listan skills que ya no existen en `.claude/skills/` (típicamente porque el skill fue renombrado o borrado).
5. **Notas potencialmente stale**: `captured` > 60 días, `status: active`, sin ninguna referencia explícita en INDEX desde otras notas más nuevas. Candidatas a "¿sigue siendo verdad?" — el usuario decide.
6. **INDEX desincronizado**: notas con frontmatter cuyo `applies-to` no coincide con las secciones donde aparecen listadas en INDEX.md (drift entre nota y índice).

### Output

Reporte estructurado, **sin auto-fix**. El usuario decide qué consolidar/borrar/fixear invocando capturas o ediciones manuales después.

```
## Audit de docs-for-claude/ — 2026-05-10

Total notas activas: 98 | archivadas: 1

### Duplicados potenciales (3)
- "Render determinista por versión" ↔ "Render: reintentos ante push rechazado"
  Topics comunes: render, git. Skills solapados: B30-use-cases, B40-ef-core.
  → Revisar manualmente si una refina/contradice la otra.

### Orphan archives (1)
- "Render por cron cada 5 min" (archived) — sin superseded-by
  → Probable reemplazada por "Render dentro de la transacción de approve". Confirmar y borrar (la canónica declara qué consolidó).

### Refs muertas al codebase (2)
- "Propagación en la misma transacción" línea 47 menciona `Isf.Hexagon.Domain.Entities.PropagationRun` — clase no existe (probable rename).
- ...

### applies-to rotos (0)
Sin gaps.

### Notas potencialmente stale (5)
- "Alcance del haystack en tablas de unidades" (no editada en 25 días)
- ...

### INDEX desincronizado (1)
- "El diff del gate se renderiza server-side" tiene applies-to: [C20-client-web] pero NO aparece en la sección C20 del INDEX.
  → Agregar entrada al INDEX.

---
Sugerencia: revisar duplicados primero (mayor impacto sobre coherencia), luego orphan archives, luego refs muertas. Stale puede esperar.
```

### Costo del audit

Variable según tamaño de la colección. Para 100 notas:
- Lectura completa: ~30 segundos
- Análisis: ~30-60 segundos
- Reporte: instantáneo

No bloquea — el usuario puede pedirlo y seguir trabajando en otra cosa, o dedicar una sesión al audit. Recomendado correr en sesión propia, no mezclado con feature work.

### Lo que `audit` NO hace

- **No edita ni borra nada.** Solo reporta. Las acciones correctivas (consolidar, borrar, fixear refs) son turnos separados, decisión del usuario.
- **No repite el flujo de captura.** Es un subcomando aparte. No genera notas nuevas.
- **No detecta contradicciones lógicas entre notas.** El audit estructural sólo mira frontmatter, refs y filesystem. Para detectar que dos notas dicen cosas incompatibles, usar `/X40 conflicts` (sección abajo).
- **No reemplaza al pre-check ni al session sweep.**

---

## Conflicts — audit semántico on-demand (`/X40 conflicts`)

Subcomando **separado tanto del flujo de captura como del audit estructural**. Razona sobre los **cuerpos** de las notas (no solo frontmatter) buscando contradicciones lógicas.

### Cuándo usarlo

- Después de un refactor grande del modelo de dominio (varias decisiones cambiaron, algunas notas viejas pueden contradecir las nuevas).
- Antes de empezar trabajo en un módulo poco visitado (verificar que las reglas vigentes son consistentes entre sí).
- Periódicamente, **menos frecuente que `audit`** (por ejemplo, cada 2-3 meses) — el costo es alto.
- Cuando una sesión reciente reveló contradicciones (señal de que probablemente hay más invisibles).

### Dos modos

#### Modo 1: Por topic (default — `/X40 conflicts` o `/X40 conflicts <topic>`)

Agrupa notas por topic y razona sobre cada subgrupo de forma aislada. Detecta contradicciones **intra-topic** (notas del mismo dominio conceptual que se contradicen).

**Algoritmo:**

1. Listar todos los topics activos en la colección (parsear frontmatter `topics:` de notas con `status: active`).
2. Si se pasó `<topic>` como argumento, filtrar a ese topic. Si no, recorrer todos.
3. Para cada topic con ≥ 2 notas:
   a. Leer los cuerpos completos de las notas de ese subgrupo.
   b. Razonar buscando:
      - **Contradicciones directas**: una nota afirma X, otra afirma ¬X (ej. "siempre usar enfoque A" vs "evitar enfoque A en este escenario").
      - **Decisiones reemplazadas sin dar de baja**: nota nueva propone Y como reemplazo de X, pero la nota X sigue `status: active`.
      - **Definiciones inconsistentes**: el mismo concepto descrito con criterios distintos en dos notas.
      - **Reglas que se aplican al mismo escenario con criterios distintos**: dos guards/heurísticas para el mismo caso que dan resultados distintos.
4. Acumular candidatos. Pasar al reporte.

**Costo**: con 100 notas en ~25 topics, promedio 4 notas por topic → 25 razonamientos pequeños. ~30 segundos a 2 minutos según volumen.

**Limitación**: NO detecta contradicciones entre notas de **topics distintos**. Si la regla "el estado solo lo escribe una transición" vive en topic `domain` y una nota en topic `persistence` dice "el importador fija el estado", el modo 1 no las cruza. Para eso, modo 2.

#### Modo 2: Global (`/X40 conflicts --global`)

Razona sobre toda la colección sin agrupar por topic. Detecta contradicciones **cross-topic**.

**Algoritmo (map-reduce para no leer 100 notas en una sola pasada):**

1. **Map**: leer cada nota activa y generar un "claim summary" estructurado (1-3 afirmaciones clave, qué scope/condición aplican).
   - Ej: `{ note: "X.md", scope: "WorkPackage.State", claim: "solo lo escribe una transición", condition: "always" }`
   - Ej: `{ note: "Y.md", scope: "WorkPackage.State", claim: "el import puede fijarlo", condition: "durante la migración" }`
2. **Reduce**: comparar claims entre sí. Agrupar por `scope` (heurística: substring match o concept overlap).
3. Para cada grupo de claims que comparten scope, razonar si las claims son compatibles, refinamientos, o contradictorias.
4. Reportar los pares contradictorios.

**Costo**: con 100 notas, lectura completa + map (~1 minuto) + reduce con N² claims (~1-3 minutos según overlap). Total: 2-5 minutos.

**Cuándo justifica**: post-refactor de modelo, post-cambio de stack, o cada 2-3 meses. NO correr en cada captura ni en cada audit estructural.

### Verificar contra el código antes de clasificar (ambos modos)

Un candidato es "refinamiento" o "reversal" según **qué está vigente**, y eso muchas veces lo dice el código, no las notas. Antes de redactar el veredicto de cada candidato, grepear lo que lo decide: el campo existe o no (`StackId` en un AR), la variable de config sigue o no (`.env.example`), el port vive en tal carpeta, el script de auditoría existe. Una pasada de `grep`/`find` por candidato, en paralelo. Con eso el reporte **afirma** la dirección del arreglo en vez de preguntar "¿cuál vale?"; en el primer audit de ISF resolvió 5 de 8 candidatos. Lo que el código no puede decidir (una decisión de diseño no implementada) se deja como pregunta al usuario.

### Output (mismo formato para ambos modos)

```
## Conflicts — semántico [por topic | global] — 2026-05-10

Notas analizadas: 98 | topics revisados: 25 | candidatos detectados: 4

### Contradicciones potenciales

#### 1. Scope: WorkPackage.State  [cross-topic]
- "El estado solo lo escribe una transición" (topic: domain)
  Afirma: "ningún Input lleva state"
- "Import de foundation" (topic: persistence)
  Afirma: "el importador fija el estado inicial de los work packages"

  → ¿Reversal o refinamiento? Probable refinamiento (la 2 matiza la 1 para el caso EF). Sugerencia: editar la 1 para incluir el matiz, o referenciar la 2 desde la 1.

#### 2. Scope: Render retry  [topic: render]
- "Render — Implementation": "retry inmediato hasta 3 intentos"
- "Push rechazado por GitHub": "retry con backoff exponencial empezando en 30s"

  → Contradicción directa. Sugerencia: revisar cuál es el comportamiento vigente, borrar la superada (la vigente declara qué consolidó).

#### 3-4. ...

---
Sugerencia: tratar candidatos con etiqueta "contradicción directa" primero. "Refinamiento" puede esperar a la próxima captura del area.
```

### Lo que `conflicts` NO hace

- **No edita ni borra nada.** Igual que `audit`. Reporta candidatos y deja la decisión al usuario.
- **No es infalible.** Razonamiento semántico tiene falsos positivos (dos notas que parecen contradictorias pero hablan de scopes distintos) y falsos negativos (contradicciones sutiles que el modelo no detecta). Tratá el output como pista, no como verdad.
- **No reemplaza captura cuidadosa.** Si el pre-check + session sweep funcionan bien, conflicts encuentra muy poco. Es backstop, no compensación.
