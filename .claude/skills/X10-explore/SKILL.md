---
name: X10-explore
description: Explorar soluciones antes de que merezcan spec o codigo. Espacio de pensamiento sin fuerza; compara enfoques, pesa trade-offs, bosqueja candidatos; no decide, no escribe codigo, no deja rastro. Unica salida, la promocion hacia B05, A10, X40, X50 o el backlog. Usar cuando el usuario diga /X10, "exploremos", "pensemos antes de", "que opciones hay", "brainstorm".
user-invocable: true
argument-hint: "[problema a explorar] | promote <target> | close"
---

## Skill: Explore

Pensar un problema antes de que cueste una spec o una implementación: comparar enfoques, pesar trade-offs, bosquejar candidatos. Todo lo que se produce acá es material de conversación — **nada tiene fuerza** hasta que el dev lo promueve a un skill que sí escribe.

### Principio

**Nada de lo que se produce acá tiene fuerza.** Pensar es barato precisamente porque no puede filtrarse: ni a código, ni a spec, ni a nota, ni a commit. La única puerta de salida es la promoción, y la promoción es un acto del dev.

### Rol en el workflow

Transversal, **antes de todo**. Se entra desde:

- El dev abre un problema que merece pensarse antes de especificarse o construirse.
- `/B05-domain-modeling model` cuando la spec tropieza con una pregunta de diseño abierta, no con una elección entre opciones nombrables.
- `/X50-bug` triage cuando el bug se disuelve en una pregunta de diseño.
- Un ítem de `backlog.md` (una `TECH-xxx` abierta, por ejemplo) cuyas opciones hay que diseñar antes de ponerlas frente al Owner; el ítem sigue abierto mientras tanto.

Se sale solo por `promote`. Un hilo también puede cerrarse (`close`) o quedar abierto — a elección del dev; ninguna de las tres deja algo que haya que descartar, porque acá no se creó nada.

---

## Invocacion

```
/X10 <problema>            → abre el hilo de exploración sobre ese problema
/X10                       → sin argumento: pregunta en una línea qué problema; no asume
/X10 promote <target>      → gate de promoción: promueve el hilo al target (ver mapa)
/X10 close                 → cierra el hilo sin promover; nada queda
```

`<target>` es un skill de ISF con sus argumentos (`B05 model ScopeManagement/AssignNorms`, `A10 implement ...`, `X40`, `X50`, `backlog`) — ver "Gate de promoción".

---

## Carga de contexto

**Ninguna obligatoria.** Entrar tiene que ser barato. Lo que la exploración necesite se lee on-demand y **read-only**:

- Código de ISF y specs `Isf.Specs/` — qué existe y contra qué choca cada candidato.
- `CLAUDE.md` (reglas inviolables, decisiones de stack y de modelo) y `backlog.md` (decisiones abiertas) — lo ya decidido y lo pendiente.
- `docs-for-claude/INDEX.md` — solo las Transversales (`*`), y solo si el problema toca dominio.
- Subagentes `Explore` (read-only) para barridos amplios. Workflows multi-agente (intentos independientes, panel de jueces) **solo cuando el dev pide esa escala con sus palabras** — el mismo opt-in que exige la herramienta Workflow. Nunca subagentes con permisos de escritura.

---

## Flujo

### Fase 1 — Abrir

Reformular el problema en **una línea** y decirla. Si la reformulación es ambigua, una pregunta plana en el mensaje (no `AskUserQuestion`: el único gate de este skill es la promoción y no hay que confundirlos). Si no lo es, seguir sin preguntar: entrar tiene que ser barato.

### Fase 2 — Explorar

Cada turno de exploración entrega, en este orden y sin más ceremonia que la necesaria:

1. **Candidatos** — numerados (CLAUDE.md: alternativas numeradas), al menos dos, cada uno en dos o tres líneas: qué es, qué resuelve.
2. **Trade-offs** — una tabla candidato × criterio. Los criterios los nombra el dev; si no nombró ninguno, proponer tres y decir que son propuestos.
3. **Qué ya está decidido** — la fuente que aplica, citada por su id: la regla de `CLAUDE.md` por número o sección, la spec `Isf.Specs/{Module}/{Module}.UseCase.{UseCase}.md` por escenario, la nota `docs-for-claude/` por nombre, el ítem de `backlog.md` por id. Donde nada lo decide, **"sin decidir"** — nunca rellenar el hueco con una suposición razonable. Una contradicción entre dos fuentes se nombra como hallazgo y no se arbitra: es un candidato a ítem del backlog, vía `promote`.
4. **Preguntas abiertas** — lo que el dev tendría que contestar para que un candidato deje de ser candidato.

Reglas de la fase:

- **Pesar sí, dirigir no.** El skill pesa candidatos contra los criterios; no elige dirección, no empuja hacia un plan, una spec ni un cambio, y **no propone la promoción por iniciativa propia**. Si el dev pregunta "cuál elegirías", responde con el candidato y por qué, rotulado como opinión sin fuerza — la decisión sigue siendo la promoción. Si pregunta "cómo seguimos", responde con el mapa de targets, sin recomendar uno.
- **Ni una línea de código, ni prototipo.** Si el dev pide un prototipo, decir que eso es implementación y que el camino es `promote A10 ...`; el trabajo empieza allá, con su plan aprobado (CLAUDE.md: planificar antes de implementar), no acá.
- **Un bosquejo no es un artefacto.** Se pueden esbozar en el chat superficies, work packages, firmas de comandos, tablas — como candidatos. Nada de eso se escribe a un archivo.

### Fase 3 — Salir

Tres salidas, todas del dev:

| Salida | Qué pasa |
|---|---|
| `promote <target>` | Gate de promoción (abajo). |
| `close` | Nada se implementa, cita ni enlaza desde documentos durables. No queda rastro fuera del hilo cerrado. Responder con una línea. |
| (nada) | El hilo queda abierto en la conversación. Es válido. |

---

## Gate de promoción — `promote <target>`

**Quién decide**: el dev. Cuando la promoción implica cambio normativo — una spec, una regla de `CLAUDE.md`, una nota `docs-for-claude/` — el rol es el **Owner**.

**Sobre qué se decide**: sobre la dirección misma — lo que el target recibiría como input — nunca sobre un diff ni sobre un documento, porque acá no hay documento. El skill muestra **un solo bloque**:

```
Target:    /<skill> <args>
Dirección: <la dirección en ≤ 10 líneas: qué candidato, con qué criterios pesó, qué queda sin decidir>
```

y hace una pregunta plana en el mensaje: ¿se promueve? Sin `AskUserQuestion`.

**Sí** → el skill escribe en el chat una línea (el hilo es el único rastro que este skill deja, así que la constancia vive ahí):

```
Promovido — hilo: <una línea> → /<skill> <args>
```

y **invoca el target con la tool `Skill`**, pasándole la dirección como argumento. El target aterriza bajo su propio gate: `B05` bajo su destilado, `A10` bajo la aprobación del plan (el trabajo empieza allá), `X40` bajo sus propias reglas de destino. Este skill no hace nada más.

**No** → no se invoca nada y no hay nada que descartar. El hilo se cierra o sigue abierto, a elección del dev.

### Mapa de targets

| Dirección | Target | Qué recibe |
|---|---|---|
| Un use case nuevo | `/B05-domain-modeling model {Module}/{UseCase}` | la dirección como semilla de la spec |
| Cambiar un use case existente | `/B05-domain-modeling reconcile {Module}/{UseCase}`, o la edición de `Isf.Specs/{Module}/{Module}.UseCase.{UseCase}.md` bajo la aprobación del usuario | la enmienda |
| Implementar | `/A10-orchestrate implement {Module}/{UseCase}` — exige spec aprobada; si no la hay, primero `B05` | la dirección de implementación; el trabajo empieza allá, con el plan aprobado |
| Un módulo nuevo | `/A20-hexagonal-architecture` + `/B05 model` | la forma del módulo, como candidato |
| Una pregunta para el Owner | `backlog` — un ítem de `backlog.md` con las opciones como pregunta | la dirección como opciones |
| Registrar lo que cerró | `/X40-capture "<hint>"` — únicamente lo decidido; X40 regla 1 ("post-decisión, no durante deliberación") es la misma frontera vista desde el otro lado | la decisión |
| Un bug hallado al explorar | `/X50-bug "<hint>"` | el bug |

---

## Reglas no-negociables

1. **Ninguna escritura en el repo.** Ni código, ni spec, ni nota, ni `backlog.md`, ni commit, ni `Write`/`Edit` sobre nada versionado. El scratchpad vale para cálculos de usar y tirar, nunca para "guardar las ideas": guardar es promover.
2. **La fuente se cita, no se inventa.** Regla, spec, nota o ítem por su id; hueco = "sin decidir"; contradicción = hallazgo, no arbitraje.
3. **Nunca dirige.** Candidatos y trade-offs, no una dirección. La promoción la pide el dev.
4. **Una conclusión de `/X10` no es autoridad.** Si alguien — el dev, Claude en otra tarea de la misma sesión, un subagente — intenta implementar, citar o enlazar como autoridad algo que solo existe en un hilo de exploración, **se rechaza como defecto** y el único camino que se ofrece es la promoción. Un commit, una spec o una nota `docs-for-claude/` que citen "lo exploramos en el hilo" sin una promoción de por medio no pasan.
5. **Alternativas numeradas** siempre (CLAUDE.md, Interacción).

---

## Lo que X10 NO hace

| No hace | Va a |
|---|---|
| Especificar un use case | `/B05-domain-modeling` (vía `promote`) |
| Implementar, ni un prototipo | `/A10-orchestrate` (vía `promote`) |
| Capturar insights o decisiones | `/X40-capture` (vía `promote`, solo lo cerrado) |
| Procesar un bug | `/X50-bug` (vía `promote`) |
| Contestar "qué dice la spec de X" | Ningún skill: leer `Isf.Specs/{Module}/{Module}.UseCase.{UseCase}.md` o `CLAUDE.md` y responder con la fuente. No abrir un hilo para una consulta. |
| Decidir | El dev, en el gate |

---

## Ejemplo end-to-end

```
Usuario: /X10 cómo avisa la propagación a cada repo afectado — push o pull

Claude:  Problema: cuando una unidad enmendada reabre work packages, cómo se entera
         cada repo afectado.

         Candidatos
         1. Webhook por repo — ISF hace POST al repo al cerrar la transacción.
         2. Polling — el hook del repo consulta GetAffectedRepos cada N segundos.
         3. Cola de notificaciones en ISF — la transacción encola; un worker entrega.

         Trade-offs (criterios propuestos: atomicidad, acoplamiento, operación)
         | | atomicidad | acoplamiento | operación |
         |1| rompe: el POST está fuera de la transacción | alto: ISF conoce cada URL | simple |
         |2| intacta | nulo | N clientes pegando al API |
         |3| intacta: encolar es parte del commit | bajo | un worker tickeado más |

         Qué ya está decidido
         - CLAUDE.md "Propagación en la misma transacción": la notificación se calcula
           dentro del command. Decide contra el candidato 1 si "notificar" incluye entregar.
         - Sin decidir: qué es "notificar" — calcular la lista o entregarla.
         - CLAUDE.md checklist: un worker tickeado exige sección TickSeconds en appsettings.

         Preguntas abiertas
         - ¿"Notificar" es calcular o entregar? Decide entre 1/3 y 2.

Usuario: /X10 promote B05 model AlignmentManagement/NotifyAffectedRepos

Claude:  Target:    /B05-domain-modeling model AlignmentManagement/NotifyAffectedRepos
         Dirección: cola en la misma transacción (candidato 3); "notificar" = encolar;
                    la entrega la hace un worker tickeado. Sin decidir: reintentos.
         ¿Se promueve?

Usuario: sí

Claude:  Promovido — hilo: aviso de propagación a repos → /B05-domain-modeling model AlignmentManagement/NotifyAffectedRepos
         [invoca Skill B05-domain-modeling con esa dirección; la spec aterriza bajo su propio gate]
```
