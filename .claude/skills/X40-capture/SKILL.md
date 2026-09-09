---
name: X40-capture
description: Captura de requerimientos emergentes e insights durante una tarea, con friccion minima. Claude decide destino (docs-for-claude vs docs-for-humans vs items de backlog.md), escribe sin pedir aprobacion y retoma la tarea sin ceremonia.
user-invocable: true
argument-hint: "[hint corto opcional]"
---

## Skill: Capture

Registro rapido de decisiones e insights que surgen **en la mitad de una tarea**, para que no se pierdan al cambiar de sesion.

### Principio

**Capturar sucio es mejor que no capturar.** El skill nunca rechaza por falta de pulido. Si el draft esta crudo, se guarda crudo.

### Rol en el workflow

Transversal. Invocable desde cualquier otro skill o tarea. No interrumpe — captura y devuelve el control.

---

## Cuándo invocar /X40

El skill optimiza por **captura confiable, no por captura completa**. Cuatro reglas operativas:

1. **Captura post-decisión, no durante deliberación.** El brainstorming es ruido; la decisión cerrada es señal. Cuando una conversación cierra ("ok, hagámoslo así"), ese es el momento — vos o Claude pueden disparar `/capture` explícitamente.
2. **Captura incremental, no acumulativa.** Cada 30-50 mensajes que aporten señal, `/capture`. NO esperar al final de la sesión: los matices se pierden y el contexto se compacta. Mejor varias capturas chicas que una grande al final.
3. **Distinguir hecho de hipótesis.** "Decidimos X" → `docs-for-claude/` (regla activa). "Estamos considerando X" → `docs-for-humans/` (exploración). Si no se cerró la decisión, NO escribir en `docs-for-claude/`.
4. **Versionar narrativa, idempotente reglas.** Las notas para humans pueden acumular versiones (cada captura agrega un párrafo). Las notas para Claude son **idempotentes**: la última versión gana, las anteriores se editan o se borran (ver "Manejo de discrepancias" abajo).

---

## Invocacion

```
/capture                       → flujo normal de captura (default)
/capture <hint corto>          → captura focalizada en ese tema

/X40 audit                     → audit ESTRUCTURAL: drift mecánico (refs muertas, orphans, INDEX desync). Barato.
/X40 audit <topic>             → audit estructural limitado a notas con ese topic
/X40 audit <skill-name>        → audit estructural limitado a notas con ese skill en applies-to

/X40 conflicts                 → audit SEMÁNTICO POR TOPIC: contradicciones lógicas dentro de cada subgrupo de topic. Costo medio.
/X40 conflicts <topic>         → audit semántico limitado a un topic
/X40 conflicts --global        → audit SEMÁNTICO GLOBAL: contradicciones cross-topic en toda la colección. Caro.
```

Sin `hint` → Claude infiere los insights de los mensajes posteriores a la última invocación de `/X40` en esta sesión (ver paso 1 del Flujo).
Con `hint` → Claude enfoca la captura en ese tema.
`audit` y `conflicts` son subcomandos **separados del flujo de captura**. Ver "Audit y Conflicts" abajo + `references/audit-and-conflicts.md`.

---

## Flujo (1 turno — sin aprobación)

Claude decide, **escribe y reporta en el mismo turno**. No hay turno de propuesta ni espera de OK: el usuario ajusta post-hoc si algo no le cierra (ver "Ajustes post-hoc" abajo). La única concesión: en ambigüedades destructivas (¿refinamiento o reversal?) Claude aplica el default no destructivo y deja la pregunta señalada en el reporte — nunca bloquea esperando respuesta.

### Pasos — Claude, en un solo turno

1. **Detectar el rango de re-lectura (filesystem-based).** En la misma sesión `/X40` puede invocarse varias veces. La señal confiable de "última invocación" no es el contexto (puede haberse compactado) sino el filesystem:
   ```bash
   ls -lt docs-for-claude/ docs-for-humans/ 2>/dev/null | head -10
   ```
   - Si hay archivos creados/modificados con la fecha de hoy → el más reciente marca el corte. **Releer solo los mensajes posteriores** a ese timestamp.
   - Si no hay archivos de hoy pero sí recientes (sesión que cruza medianoche) → usar el más reciente sin importar la fecha.
   - Si no hay nada reciente → primera invocación, releer **desde el inicio de la sesión actual**.

   No usar ventanas fijas tipo "los últimos 20 mensajes" — rompen en sesiones largas y omiten decisiones de hace 30+ mensajes.
2. **Identifica insights / decisiones / requerimientos emergentes** dentro del rango detectado. Puede ser 1, pueden ser varios.
3. **Aplica el test de destino** a cada uno (ver abajo) y decide solo:
   - `docs-for-claude/` → reglas, invariantes, decisiones cerradas que gatillan comportamiento en código
   - `docs-for-humans/` → narrativa, historia, hipótesis no cerradas, contexto, onboarding
   - `backlog.md` → trabajo pendiente accionable (bug detectado, deuda nueva, gap a llenar)
4. **Pre-check de duplicados — dos pasadas** (solo para notas que van a `docs-for-claude/`):

   **Pasada A — match directo por keyword/topic:**
   - Para cada nota propuesta, grepear keywords del título o tópicos en `INDEX.md` y/o leer notas existentes que parezcan relacionadas.
   - Si hay match plausible → clasificar la discrepancia según la tabla "Manejo de discrepancias con notas existentes" (abajo) y aplicar la acción correspondiente (editar, borrar+nueva, o nueva independiente) en vez de blind-add.

   **Pasada B — listado de notas con topic relacionado:**
   - Si los `topics` de la nota propuesta aparecen en INDEX.md con N≥3 notas existentes, **listar los títulos en el reporte** (no leer cada una — solo títulos del INDEX), como revisión opcional post-hoc.
   - Si el usuario después indica "la #3 se relaciona, revisala" → Claude lee esa nota específica, reaplica la tabla de discrepancias y corrige.
   - Esto cubre el caso "el lenguaje cambió, mi grep no engancha pero el usuario sí lo recuerda".

   **Reglas comunes:**
   - Si hay ambigüedad sobre el caso → **default no destructivo** (ver "Manejo de discrepancias" abajo) y señalarla en el reporte con la pregunta de una palabra. No bloquear.
   - Si no hay match (pasada A) y los topics tienen <3 notas existentes → nota nueva limpia, sin listar.

5. **Session sweep — coherencia entre notas creadas hoy en esta sesión:**
   - Listar archivos en `docs-for-claude/` con mtime >= inicio de sesión (los creados/editados en invocaciones previas de `/capture` durante esta misma sesión).
   - Si hay ≥ 2, leer todos los frescos de la sesión y verificar entre sí:
     - ¿Dos notas con `applies-to`/`topics` solapados que dicen cosas distintas?
     - ¿La conversación posterior contradijo o refinó alguna?
   - Si encuentra inconsistencia con resolución clara → resolverla (editar/borrar la nota fresca desactualizada). Si es ambigua → default no destructivo + bloque "**Coherencia intra-sesión**" en el reporte con la observación, para que el usuario resuelva post-hoc.
   - Si no hay inconsistencia o hay <2 notas frescas → omitir.

6. **Para cada nota destinada a `docs-for-claude/`**, aplicá las "Reglas de captura (anti-fragmentación)" (abajo) y decidí su `applies-to` (qué skills la cargan) y `topics` (tags de búsqueda). Ver "Cómo decidir `applies-to`" abajo.
7. **Escribe los archivos** (notas + INDEX actualizado + items de backlog) y **reporta en bloque compacto**, mostrando applies-to + acción (nueva/editar/borrar) de cada nota docs-for-claude, listado de notas relacionadas si aplica, y bloque de coherencia intra-sesión si aplica:

```
Capturado — N notas:

  [NUEVA]    docs-for-claude/2026-04-13-slug-uno.md     → applies-to: [B20-ddd, B30-use-cases]
  [EDITADA]  docs-for-claude/2026-04-13-existente-X.md  → refina decisión sobre Y
  [BORRADA]  docs-for-claude/2026-03-20-vieja-Z.md      → superada por la NUEVA (git es el archivo)
  [NUEVA]    docs-for-humans/2026-04-13-slug-tres.md    (sin frontmatter)

INDEX.md actualizado con las altas y bajas en docs-for-claude/.

Notas existentes con topics relacionados (revisión opcional post-hoc):
  - "Propagación en la misma transacción" (topic: propagation)
  - "Render determinista por versión" (topic: render)
  Si alguna se relaciona con la NUEVA #1, decime cuál y la reviso.

Coherencia intra-sesión (quedó sin resolver, default no destructivo aplicado):
  La nota "X" creada hoy a las 10:42 dijo "decidimos A"; la conversación
  posterior discutió que A no funciona. Dejé ambas; ¿reemplazo "X" con la
  NUEVA, o son independientes?

Y M items al backlog:

  TECH-XXX (P1) · titulo corto
  BUG-YYY  (P0) · titulo corto
```

El paso 6 incluye, para cada nota, decidir si es **★ evergreen** y en qué **sub-grupo** de su sección del INDEX cae (ver "Mantenimiento del INDEX" abajo).

Si no hay items para el backlog, omitir la seccion "Y M items al backlog". Cero items es valido — la mayoria de las sesiones no descubren trabajo pendiente nuevo.

Si no hay notas para `docs-for-claude/`, omitir el aviso de INDEX.md.

Si no hay notas relacionadas (Pasada B sin matches o topics con <3 notas existentes), omitir el bloque "Notas existentes con topics relacionados".

Si el session sweep no encontró inconsistencia (o hay <2 notas frescas en la sesión), omitir el bloque "Coherencia intra-sesión".

### Ajustes post-hoc

El usuario puede ajustar **después de escrito**: cambios de `applies-to` ("la 1 no aplica a B30, sacalo"), de acción ("la 2 era independiente, restaurá X"), de scope ("borrá la 3"), o resolver las ambigüedades señaladas en el reporte. Claude aplica el ajuste en el momento — git conserva todo, nada es irreversible.

---

## Test de destino

**Filtro previo (el test de una línea de `docs-for-claude/README.md`):** antes de mandar algo a `docs-for-claude/`, preguntarse si **el código ya lo dice**.

> Capturar solo lo que el código NO puede decir: el *por qué*, el conocimiento negativo (qué se intentó y falló), invariantes no-locales y restricciones de sistemas externos. Si un ingeniero competente puede reconstruirlo leyendo el código —o lo responde `graphify query`— **no se captura como nota**.

Pasado ese filtro, Claude rutea:

> **¿Necesito leer esto en una sesion futura para implementar o modificar codigo correctamente?**

- **Si** y pasa el filtro → `docs-for-claude/`. Formato declarativo, rule-like, conciso.
- Es **trabajo pendiente / deuda** (hay una acción concreta que lo cierra) → `backlog.md` (ver abajo). No a `docs-for-claude/`.
- Es **narrativa / historia / log de milestone** → `docs-for-humans/`.

Casos tipicos:

| Tipo de insight | Destino |
|---|---|
| El *por qué* de una decisión + alternativa descartada | `docs-for-claude/` |
| Invariante de dominio no-local / bounded context | `docs-for-claude/` |
| Restricción de sistema externo (git/GitHub, foundation legacy, EF/Npgsql, API de terceros) | `docs-for-claude/` |
| Conocimiento negativo (enfoque que se probó y falló) | `docs-for-claude/` |
| Convencion de naming / estructura no obvia | `docs-for-claude/` |
| **Patrón visible en un único archivo / reconstruible del código / "dónde vive X"** | **no capturar** (código o graphify) |
| **Trabajo pendiente, deuda, gap, TODO** | **`backlog.md`** |
| Historia / evolución / log "Chunk N completado" / validación end-to-end | `docs-for-humans/` |
| Debate largo con pros/cons — solo la **conclusion** va a Claude | conclusion: `docs-for-claude/`, debate: `docs-for-humans/` |
| Meeting notes crudas | `docs-for-humans/` |

Si hay duda entre `docs-for-claude/` y `docs-for-humans/`: preferir `docs-for-humans/` salvo que pase el test de una línea. **No** sobre-preservar en `docs-for-claude/`: cada nota de más es costo de tokens recurrente en cada skill que la carga.

---

## Reglas de captura (anti-fragmentación)

La colección se fragmentó una vez (276 → ~120 notas en la consolidación de 2026-07) porque se capturaba por **sesión** (cuándo se aprendió) y no por **tema** (qué explica). Estas reglas se aplican a cada nota propuesta, antes de escribirla:

1. **Skill primero.** Antes de crear una nota que corrige una convención de capa, verificar si el `SKILL.md` correspondiente la contradice — si sí, el destino es el skill, no la nota.
2. **Editar antes que crear.** Si el insight es un caso nuevo de una regla ya escrita, editar la nota/regla existente. Una nota nueva requiere un TEMA nuevo, no una sesión nueva.
3. **Prohibido en notas**: secciones de estado ("pendiente", "GAP", "próximos pasos"), IDs de backlog como contenido (el pendiente va a `backlog.md`), inventarios de símbolos/rutas ("Archivos clave" — eso es graphify/grep), y valores copiados de archivos grepeables (appsettings, package.json, csproj — referenciar, no copiar).
4. **Formato objetivo**: regla (1-3 líneas) + por qué el código no puede decirlo (2-3) + síntoma reconocible (1-2). Máximo ~20 líneas salvo canónicas de arquitectura.
5. **La nota nombra la regla, no el símbolo.** Los nombres de tipos van como ejemplo desechable, nunca como definición — mueren con el rename.
6. **`applies-to: "*"` es caro** (se carga en TODOS los skills): solo si su violación es detectable en cualquier capa. Si nombra un skill/carpeta/tecnología concreta, es de esa capa. Si es regla inviolable de proyecto → proponer CLAUDE.md.
7. **Excepciones deliberadas.** Una nota cuyo propósito es "esto viola el skill a propósito, no lo refactorices" lleva `exception-to: <skill> <regla>` en el frontmatter; la acción validate de los skills debe poder encontrarlas por ese campo.

### Regla: toda convención nueva nace con su fila en `audits.md`

Si lo capturado es una **convención** (una regla que alguien podría violar en el futuro), además de escribirla en su destino hay que agregarle una fila al registro `audits.md` (raíz del repo): dónde quedó escrita, qué la audita, con qué disparador (`V10` o `campaña`) y de qué tipo es (`mecánica` o `juicio`).

Vale perfectamente que la fila diga *"no auditable — solo review"*. El punto no es instrumentar todo: es que la pregunta **"¿y esto quién lo verifica?"** se conteste mientras se escribe la convención, que es cuando sale barata. Una convención sin disparador se rompe en silencio hasta que alguien audita por casualidad.

No aplica a lo que no es convención: historia, conocimiento negativo, restricciones de sistemas externos, items de backlog. Tampoco a las convenciones que gobiernan la **conducta de una sesión** (orden de operaciones entre skills, cuándo cortar por contexto, cómo redactar un handoff): su producto vive en el chat, no en el repo, y una fila sin auditoría real hace creer que algo verifica. Alcance completo en `audits.md` § Mantenimiento.

### Regla: siempre generar al menos una nota `docs-for-humans/`

**Obligatorio**: toda invocacion de `/capture` debe producir **minimo una nota para `docs-for-humans/`**. Claude tiene sesgo natural hacia `docs-for-claude/` (reglas tecnicas) y tiende a ignorar el valor narrativo de la sesion. Pero las sesiones de trabajo contienen material humano valioso que se pierde si no se captura:

- **Lecciones aprendidas**: que se rompio, por que, como se detecto, que se hizo para evitarlo en el futuro
- **Evolucion de criterios**: como un requerimiento empezo siendo X y termino siendo Y (y por que)
- **Decisiones que cambiaron de rumbo**: el usuario corrigio el approach, se descubrio un caso edge, se expandio el scope
- **Contexto de "por que ahora"**: que motivo la tarea, que dolor resolvio, que alternativas se descartaron

Si la sesion no tiene nada de lo anterior (raro), Claude lo dice explicitamente: *"No encuentro material narrativo en esta sesion — solo capturo notas tecnicas."* Pero eso es la excepcion, no el default.

---

## Backlog: cuando un insight es trabajo pendiente

Algunos insights no son **reglas a recordar** (`docs-for-claude/`) ni **historia para humanos** (`docs-for-humans/`) sino **trabajo concreto que hay que hacer**. Esos van como item nuevo en `backlog.md`.

### Test de "esto va al backlog"

Para cada insight, Claude se pregunta:

> **¿Hay una accion concreta pendiente que cierra esto, distinta de "recordar la regla en futuras sesiones"?**

- **Si** → tambien crear item de backlog (puede coexistir con una nota en `docs-for-claude/` que documente el _por que_).
- **No** → solo nota.

Casos tipicos al backlog:
- Bug detectado durante la sesion que no se va a arreglar ahora.
- Deuda tecnica descubierta (stub deliberado, regla violada, refactor pendiente).
- Use case nuevo que el usuario describio pero no se implementa todavia.
- Gap de cobertura de tests detectado.
- Endpoint/feature placeholder con TODO declarado al usuario (ej. "boton X queda disabled, agregarlo en fase Y").

### Convenciones

**Prefijos, numeración, prioridad y formato de item**: viven en el header de `backlog.md` → sección `## Convenciones` — leerla antes de proponer items; no duplicar un gap ya cubierto por un ID existente (buscarlo primero, referenciarlo).

---

## Formato de archivo

### Nombre (mismo para ambas carpetas)
`YYYY-MM-DD-<slug>.md`

- Fecha actual (absoluta, no relativa).
- Slug kebab-case, corto (3-6 palabras).
- Si ya existe uno con ese nombre, sufijar con `-2`, `-3`, etc.

### Contenido — `docs-for-claude/` (frontmatter OBLIGATORIO)

```markdown
---
title: "{Titulo corto}"
captured: YYYY-MM-DD
applies-to:
  - <skill-name>
  - <skill-name>
topics:
  - <tag>
  - <tag>
status: active
---

# {Titulo corto}

_Capturado YYYY-MM-DD mientras {1 linea de contexto — que se estaba haciendo}._

{cuerpo — 3-12 lineas, bullets OK, code blocks OK, enlaces a archivos OK}
```

**Reglas del frontmatter:**

- `title`: igual al `# H1`. Encerrar entre comillas dobles si tiene `:`, `—`, `→`, `↔` u otros caracteres YAML especiales.
- `captured`: la misma fecha del nombre del archivo.
- `applies-to`: lista de skills que deben cargar esta nota. Valor especial `"*"` (entre comillas) = transversal — la cargan **todos** los skills. Válido = cualquier directorio de `.claude/skills/` (excepto vendor: `graphify`, `shadcn`); no mantener lista acá — verificar con `ls .claude/skills/` si hay duda.
- `topics`: 2-4 tags libres (vocabulario sugerido en `INDEX.md` sección "Búsqueda alternativa por topic"). Sirven para búsqueda, no para carga.
- `status`: `active` (default). `archived` (con `superseded-by: <archivo.md>`) existe solo como **estado transicional**: el estado estable de una nota superada es **borrarla** — git es el archivo — y que la nota canónica declare qué consolidó.
- `exception-to: <skill> <regla>` (opcional): marca una excepción deliberada a una regla de skill (ver Reglas de captura, #7).

**Cómo decidir `applies-to`:**

- ¿La nota habla de una capa específica (Domain, UseCases, Persistence, API, UI)? → listar el/los skills de esa capa.
- ¿Es una regla del proyecto que aplica a cualquier capa (convenciones de backlog, anti-patrones globales, infraestructura compartida)? → `"*"`.
- ¿Toca varias capas? → listar todas. Una nota puede aplicar a 2-5 skills sin problema.
- **NO usar `"*"` por defecto.** Solo cuando es genuinamente transversal. Si dudás entre `"*"` y "varios skills específicos", elegí los específicos — reduce ruido en cargas de skills no relacionados.

### Contenido — `docs-for-humans/` (formato libre, sin frontmatter)

```markdown
# {Titulo corto}

_Capturado YYYY-MM-DD mientras {1 linea de contexto — que se estaba haciendo}._

{cuerpo — narrativa libre, sin restricciones de tamaño}
```

Las notas para humanos no llevan frontmatter (Claude no las lee, no hay carga selectiva que mantener).

---

## Mantenimiento del INDEX (`docs-for-claude/INDEX.md`)

Toda vez que `/X40-capture` crea o modifica una nota en `docs-for-claude/`, **debe actualizar `INDEX.md` en el mismo turno**. Sin esto, la nota nueva no la carga ningún skill (queda invisible).

### Cuándo actualizar

| Acción sobre la nota | Acción sobre el INDEX |
|---|---|
| Crear nota nueva | Insertar entrada en cada sección listada en `applies-to` (o en "Transversales (`*`)" si aplica a todos). Decidir ★ y sub-grupo (ver abajo). |
| Renombrar archivo | Actualizar todos los links del INDEX que apunten al viejo nombre. |
| Cambiar `applies-to` | Mover la entrada de las viejas secciones a las nuevas. |
| Borrar nota superada | Eliminar todas las entradas del INDEX que la referencian. La nota canónica que la consolidó declara qué reemplazó — git conserva la historia. |
| Marcar `status: archived` (transicional) | Mover entrada a "Archivadas / superseded" abajo en el INDEX. Solo para transiciones (reversal pendiente de confirmar); el estado estable es borrar. |

### ★ evergreen y sub-grupos — decidir al insertar la entrada

Al agregar una entrada a una sección del INDEX, decidir dos cosas:

1. **¿Es ★ evergreen?** Sí solo si es una regla always-relevant de esa capa que debe cargarse **por default** en cada activación del skill (no "importante en su tema" — eso es carga por sub-grupo). Marcarla con `★` antes del link. El piso de carga de cada skill = Transversales + ★ de su sección; una ★ de más encarece **todas** las activaciones de ese skill.
2. **¿A qué sub-grupo pertenece?** Si la sección del skill tiene sub-grupos rotulados en **negrita** (ej. *"Bulk ops"*, *"Render y git"*), insertar la entrada dentro del rótulo cuyo tema la cubre — no al final suelto. Si ningún rótulo la cubre y hay ≥3 notas del mismo tema nuevo, crear el rótulo nuevo y señalarlo en el reporte.

### Formato de cada entrada en el INDEX

```markdown
- [{title}](YYYY-MM-DD-slug.md)
- ★ [{title}](YYYY-MM-DD-slug.md)     ← solo evergreen
```

Una línea por sección. Si la nota aparece en 3 skills + 1 transversal, hay 4 líneas distribuidas — pero **una nota con `"*"` solo se lista en "Transversales", no se duplica** en los demás skills (los skills cargan ambas secciones).

### Mantener el contador

Cada sección por skill del INDEX tiene `(N)` en el header (ej. `### B30-use-cases (32)`); la sección Transversales usa `## Transversales (\`*\`) — N notas`. Actualizar el número al agregar/quitar entradas. El total en la línea introductoria del INDEX (`Total al YYYY-MM-DD: N activas + M archivadas`) también se actualiza.

### Si el INDEX no existe (proyecto sin migrar todavía)

Crear `docs-for-claude/INDEX.md` siguiendo el formato existente (ver el archivo actual como referencia canónica). No es opcional — el sistema de carga selectiva por skill lo requiere.

---

## Manejo de discrepancias con notas existentes

Cuando el pre-check de duplicados (paso 4 del flujo) detecta superposición entre una nota propuesta y una existente, clasificar el caso y aplicar la acción correspondiente.

**Regla de oro**: `docs-for-claude/` refleja el **estado actual de la verdad**, no la evolución del razonamiento. La narrativa de "cómo llegamos acá" va a `docs-for-humans/`.

| Caso | Cómo se ve | Acción sobre `docs-for-claude/` | Acción opcional sobre `docs-for-humans/` |
|---|---|---|---|
| **Refinamiento** | La nueva matiza/detalla la vieja sin contradecirla. Ej: vieja "decidimos X", nueva "X aplica en caso Y; en Z usamos W". | **Editar la nota original** para reflejar la decisión refinada. NO crear nota nueva. | — |
| **Reversal** | La nueva contradice la vieja. Ej: vieja "vamos por A", nueva "A no funciona, vamos por B". | **Borrar la vieja** (git es el archivo) y crear la nota nueva con la decisión vigente, declarando qué consolidó. `archived` + `superseded-by:` solo como estado transicional si el reversal no está confirmado. | Captura honesta con la historia: "exploramos A, descartamos por Z, ahora B". |
| **Adición no contradictoria** | La nueva habla de un topic distinto, no se superpone con la vieja. | Nota nueva sin tocar las viejas. **Caso default** — la mayoría de capturas. | — |
| **Brainstorming sin cierre** | La conversación tira hacia X y Y sin cerrar. | **NO escribir** (no hay regla todavía). | Captura honesta con las dos opciones y el estado abierto. Cuando se cierre, futura `/capture` la promueve a `docs-for-claude/`. |

**Si hay ambigüedad sobre el caso** (ej: "¿esto es refinamiento o reversal?"), NO bloquear esperando respuesta: aplicar el **default no destructivo** y señalar la ambigüedad en el reporte con la pregunta de una palabra, para que el usuario la resuelva post-hoc:

- Default no destructivo = la nota vieja queda **intacta** (o `archived` + `superseded-by:` transicional si la contradicción es evidente pero no confirmada), la nueva se escribe. Nunca borrar ni sobreescribir una nota existente en un caso ambiguo.
- Respuestas post-hoc de una palabra: "refinás X" → refinement (consolidar en la vieja, borrar la nueva) · "reemplazás X" → reversal (borrar la vieja) · "es independiente" → adición (quitar el flag) · "no cerramos eso todavía" → la nueva baja a humans.

El costo de equivocarse destructivamente (sobreescribir una decisión válida) es alto; el de una nota duplicada transitoria, bajo — por eso el default preserva y el usuario poda.

### Edición de nota existente — flujo concreto

Cuando la acción es "editar":

1. Leer la nota completa.
2. Modificar el cuerpo para reflejar la decisión refinada (sobreescribir, NO agregar "Refinamiento posterior" — esto contamina la idempotencia).
3. Si el `applies-to` o `topics` cambiaron, actualizar el frontmatter.
4. Actualizar el INDEX si el title cambió o si los `applies-to` cambiaron (mover entradas de sección).
5. NO cambiar el `captured` original — es la fecha de la primera captura, no del refinamiento. Si querés trazabilidad temporal del cambio, va a `docs-for-humans/`.

### Baja de nota superada — flujo concreto

Cuando la acción es "borrar" (default para toda nota superada — git es el archivo):

1. Leer la nota vieja (para no perder ningún claim vigente al consolidar).
2. Crear la nota nueva (frontmatter completo) con la decisión vigente, declarando en su cuerpo qué consolidó (título de la superada basta).
3. Borrar el archivo viejo.
4. En el INDEX: eliminar todas las entradas de la vieja, agregar las de la nueva (con decisión ★/sub-grupo).

**Variante transicional** (solo si el reversal no está confirmado): en vez de borrar, `status: active` → `status: archived` + `superseded-by: <nuevo-archivo.md>`, sin modificar el cuerpo, y mover la entrada del INDEX a "Archivadas / superseded". Es un estado a resolver: en cuanto se confirme, la nota se borra.

---

## Audit y Conflicts (subcomandos on-demand)

- **`/X40 audit [<topic>|<skill-name>]`** — audit ESTRUCTURAL de `docs-for-claude/`: drift mecánico (duplicados candidatos por frontmatter, orphan archives, refs muertas al codebase, `applies-to` rotos, INDEX desincronizado). Barato. Solo reporta, no edita.
- **`/X40 conflicts [<topic>|--global]`** — audit SEMÁNTICO: contradicciones lógicas entre los cuerpos de las notas (por topic, o global cross-topic con map-reduce). Costo medio/alto. Solo reporta, no edita.

Ambos son complementarios del pre-check y el session sweep del flujo de captura, no los reemplazan.

**Antes de ejecutar cualquiera de los dos, leer `references/audit-and-conflicts.md`** (algoritmos, qué chequea cada uno, formato de reporte, costos y límites).

---

## Regla absoluta: retomar la tarea

Despues de escribir los archivos, Claude **retoma la tarea original** en la misma respuesta, sin ceremonia. Ejemplo:

```
Capturado. Retomando auditoria de ScopeManagement.

[sigue con la tarea]
```

Si el usuario invoco `/capture` sin haber una tarea previa clara, Claude confirma: "¿algo mas que quieras capturar, o cerramos?".

### Variante: captura en corte de sesión → ofrecer el handoff

Si la captura ocurre en un **corte de sesión** en vez de en medio de una tarea, no hay tarea que retomar: lo que sigue es el prompt de handoff. Señales de corte: el usuario dice "cerramos" / "handoff" / "me quedo sin contexto" / "seguimos en otra sesión", o aparecen los disparadores de `docs-for-claude/2026-08-05-anatomia-del-prompt-de-handoff.md` → "Cuándo escribirlo" (síntomas de degradación, cambio de tema, volumen).

En ese caso, **después de escribir los archivos y en la misma respuesta**, Claude ofrece redactar el handoff. El orden importa: `/X40` primero, handoff después — el handoff se redacta contra **lo que quedó** tras la captura, no contra la conversación entera. Esto lo encoge: las decisiones cerradas ya son notas (va la ruta, no la decisión), el pendiente que excede la tarea ya es item de `backlog.md` (va el ID), y los callejones sin salida durables ya son notas (queda en el prompt solo el dead-end propio de esta implementación).

Antes de redactarlo, leer la nota de anatomía completa — define las seis partes, los anti-patrones y la plantilla. Dos partes del handoff **nunca** se capturan como nota (son estado transicional, prohibido por la regla anti-fragmentación #3): "estado verificado vs. asumido" y "próximo paso concreto".

```
Capturado (notas + INDEX.md actualizado).

Esto parece corte de sesión más que pausa. ¿Te redacto el prompt de handoff?
```

---

## Lo que NO hace este skill

- **No cura / promueve**: los archivos quedan en `docs-for-claude/` o `docs-for-humans/` directamente. No hay inbox/curate split. Si despues hace falta ordenar, se hace en otra pasada.
- **No lee `docs-for-humans/`** en futuras sesiones (regla de CLAUDE.md).
- **No interrumpe**: el flujo es estrictamente captura → continuacion. Nunca abre subdiscusiones.
- **No edita CLAUDE.md**: si una captura amerita convertirse en regla, el usuario decide moverla o referenciarla desde CLAUDE.md manualmente.
- **No cierra items del backlog**: solo los crea. Cerrar / marcar `done` es del flujo del skill que ejecuta el item.

---

## Ejemplo end-to-end

**Contexto**: estamos en plena discusion sobre refactor de modulos. Surgen 3 decisiones importantes.

```
Usuario: /capture

Claude: Capturado — 3 notas:

  [NUEVA] docs-for-claude/2026-09-01-propagacion-misma-transaccion.md     → applies-to: [B20-ddd, B30-use-cases]
  [NUEVA] docs-for-claude/2026-09-01-render-derivado-nunca-fuente.md      → applies-to: [B30-use-cases, A20-hexagonal-architecture]
  [NUEVA] docs-for-humans/2026-09-01-evolucion-del-modelo-de-versiones.md (sin frontmatter)

  INDEX.md actualizado con las dos nuevas entradas en docs-for-claude/.

  Y 1 item al backlog:

    TECH-042 (P1) · Migrar el importador al parser por bloques

  Retomando la discusion del modelo de versiones.

  [sigue con la tarea]
```
