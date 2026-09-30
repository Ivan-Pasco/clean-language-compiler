/-
  Clean Language — piloto. Un programa es un cuerpo de sentencias:
  primero se comprueba entero, después se ejecuta. El resultado de un
  programa es lo que devuelve un `return`, o el valor de su última
  sentencia con valor; lo que `print` escribe sale aparte.

  Ámbitos (STM-05): cada cuerpo indentado es un ámbito. Una declaración
  vive hasta el final de su cuerpo; una asignación a un nombre de fuera
  lo cambia fuera (FLW-02); un cuerpo de bucle se entra de nuevo en cada
  pasada; un nombre todavía visible no se declara otra vez (SCOPE002).
-/
import Clean.Ast
import Clean.Resultado
import Clean.Tipado
import Clean.Evaluacion

namespace Clean

/-- Sentencia ya comprobada. -/
inductive TStmt where
  | guardar (nombre : String) (e : TExpr)
  | declarar (nombre : String)
  | evaluar (e : TExpr)
  | si (ramas : List (TExpr × List TStmt)) (sino : Option (List TStmt))
  | mientras (cond : TExpr) (cuerpo : List TStmt)
  | recorrer (nombre : String) (fuente : TExpr) (cuerpo : List TStmt)
  | rango (nombre : String) (desde hasta : TExpr) (paso : Option TExpr) (cuerpo : List TStmt)
  | romper
  | continuar
  | devolver (e : Option TExpr)
  | imprimir (e : TExpr) (salto : Option TExpr)
  | imprimirBloque (items : List TExpr)
  deriving Inhabited

/-- RUL-16 (SEM001): el lado derecho debe ser asignable al tipo
    declarado. -/
def comprobarAsignacion (declarado obtenido : Ty) : Resultado Unit :=
  if asignable obtenido declarado then .bien ()
  else .diagnostico "SEM001" "type mismatch in assignment"

/-- ¿El nombre está declarado en el ámbito actual — las `n` entradas
    más recientes del entorno? -/
def enAmbitoActual (env : Entorno) (n : Nat) (nombre : String) : Bool :=
  (env.take n).any (fun (k, _) => k == nombre)

/-- RUL-44 (SEM030): `step` con la constante 0. -/
def esCeroConstante : Expr → Bool
  | .intLit 0 => true
  | .paren e => esCeroConstante e
  | .unary .neg e => esCeroConstante e
  | _ => false

/-- El contexto de una comprobación: `base` es cuántas entradas del
    entorno pertenecen al ámbito actual; `bucles` cuántos bucles
    encierran la sentencia dentro de este cuerpo (FLW-03, SEM025). -/
structure Ctx where
  base : Nat
  bucles : Nat
  /-- Nombres declarados en cuerpos ya cerrados (STM-05 los saca de
      alcance; se conservan por si un diagnóstico quiere nombrarlos). -/
  cerrados : List String := []

/-- STM-05: un nombre declarado en un cuerpo ya cerrado no es visible;
    usarlo es SEM002, como cualquier nombre sin declaración en alcance. -/
def conCerrados (_ctx : Ctx) (r : Resultado α) : Resultado α := r

/-- RUL-38 (SEM023): la condición de un `if` o un `while` es boolean. -/
def comprobarCondicion (ctx : Ctx) (env : Entorno) (e : Expr) : Resultado TExpr := do
  let (te, t) ← conCerrados ctx (elaborar env Option.none e)
  if t == .boolean then .bien te
  else .diagnostico "SEM023" s!"Condition must be a boolean expression, found {t.nombre}"

/-- Los nombres que un cuerpo declara, para recordarlos al cerrarlo. -/
def declaradosEn : List Stmt → List String
  | [] => []
  | .decl _ n _ :: resto => n :: declaradosEn resto
  | .assign n _ :: resto => n :: declaradosEn resto
  | _ :: resto => declaradosEn resto

mutual

partial def comprobarSentencia (ctx : Ctx) (env : Entorno) : Stmt → Resultado (TStmt × Entorno × Nat)
  -- TYP-11: declaración con el tipo primero.
  | .decl t nombre inicial =>
    match t.base with
    | .other n => .fueraDeAlcance s!"tipo `{n}`"
    | _ => do
      -- STM-05, RUL-47 (SCOPE002): un nombre todavía visible — del mismo
      -- cuerpo o de uno que lo encierra — no se declara otra vez.
      if (env.buscar nombre).isSome then
        .diagnostico "SCOPE002" s!"`{nombre}` is already declared and still visible"
      else
        match inicial with
        | Option.none => .bien (.declarar nombre, (nombre, t) :: env, ctx.base + 1)
        | some e => do
          let (te, obtenido) ← conCerrados ctx (elaborar env (some t) e)
          comprobarAsignacion t obtenido
          .bien (.guardar nombre te, (nombre, t) :: env, ctx.base + 1)
  -- STM-02, TYP-11: asignación, o declaración con tipo inferido.
  | .assign nombre e =>
    match env.buscar nombre with
    | some declarado => do
      let (te, obtenido) ← conCerrados ctx (elaborar env (some declarado) e)
      comprobarAsignacion declarado obtenido
      .bien (.guardar nombre te, env, ctx.base)
    | Option.none => do
      let (te, obtenido) ← conCerrados ctx (elaborar env Option.none e)
      match obtenido with
      -- TYP-11, RUL-24 (SEM009): `none` no fija ningún tipo.
      | .noneLit =>
        .diagnostico "SEM009" s!"`{nombre}` needs a type: `none` fixes none"
      | .emptyList =>
        .sinEspecificar s!"`{nombre} = []` sin tipo declarado"
      | t => .bien (.guardar nombre te, (nombre, t) :: env, ctx.base + 1)
  | .expr e => do
    let (te, _) ← conCerrados ctx (elaborar env Option.none e)
    .bien (.evaluar te, env, ctx.base)
  -- FLW-01: cada rama, un cuerpo nuevo; la cadena no declara nada fuera.
  | .ifStmt ramas sino => do
    let tramas ← comprobarRamas ctx env ramas
    let tsino ← match sino with
      | Option.none => pure Option.none
      | some cuerpo => do
        let c ← comprobarCuerpo { base := 0, bucles := ctx.bucles } env cuerpo
        pure (some c)
    .bien (.si tramas tsino, env, ctx.base)
  -- FLW-02: `while`, condición boolean, cuerpo dentro de un bucle más.
  | .whileStmt cond cuerpo => do
    let tc ← comprobarCondicion ctx env cond
    let tcuerpo ← comprobarCuerpo { base := 0, bucles := ctx.bucles + 1 } env cuerpo
    .bien (.mientras tc tcuerpo, env, ctx.base)
  -- FLW-02: `iterate` sobre lista o cadena; RUL-19 (SEM004) si la
  -- fuente no es iterable.
  | .iterateElems nombre fuente cuerpo => do
    let (tf, t) ← elaborar env Option.none fuente
    let elem ← match t with
      | .list u => pure u
      | .string => pure Ty.string
      | .emptyList => .sinEspecificar "iterate sobre `[]` sin tipo"
      | t => .diagnostico "SEM004" s!"cannot iterate over a value of type `{t.nombre}`"
    let tcuerpo ← comprobarCuerpo { base := 1, bucles := ctx.bucles + 1 } ((nombre, elem) :: env) cuerpo
    .bien (.recorrer nombre tf tcuerpo, env, ctx.base)
  -- FLW-02: rango de integer; SEM030 con `step 0` constante.
  | .iterateRange nombre desde hasta paso cuerpo => do
    let (td, tyd) ← elaborar env (some .integer) desde
    let (th, tyh) ← elaborar env (some .integer) hasta
    let _ ← limiteDeRango tyd
    let _ ← limiteDeRango tyh
    let tpaso ← match paso with
      | Option.none => pure Option.none
      | some p => do
        if esCeroConstante p then
          .diagnostico "SEM030" "a range cannot advance with step 0"
        else
          let (tp, typ) ← elaborar env (some .integer) p
          let _ ← limiteDeRango typ
          pure (some tp)
    let tcuerpo ← comprobarCuerpo { base := 1, bucles := ctx.bucles + 1 } ((nombre, Ty.integer) :: env) cuerpo
    .bien (.rango nombre td th tpaso tcuerpo, env, ctx.base)
  -- FLW-03, RUL-40 (SEM025).
  | .breakStmt =>
    if ctx.bucles == 0 then .diagnostico "SEM025" "'break' is not inside a loop"
    else .bien (.romper, env, ctx.base)
  | .continueStmt =>
    if ctx.bucles == 0 then .diagnostico "SEM025" "'continue' is not inside a loop"
    else .bien (.continuar, env, ctx.base)
  -- STM-03.
  | .returnStmt Option.none => .bien (.devolver Option.none, env, ctx.base)
  | .returnStmt (some e) => do
    let (te, _) ← conCerrados ctx (elaborar env Option.none e)
    .bien (.devolver (some te), env, ctx.base)
  -- STD-04: `print(value)`; `newline:` es boolean.
  | .print e salto => do
    let (te, _) ← conCerrados ctx (elaborar env Option.none e)
    let ts ← match salto with
      | Option.none => pure Option.none
      | some s => do
        let (tsalto, t) ← elaborar env (some .boolean) s
        if t == .boolean then pure (some tsalto)
        else .diagnostico "SEM001" "type mismatch in assignment"
    .bien (.imprimir te ts, env, ctx.base)
  -- STM-04: cada línea del bloque es una expresión.
  | .printBlock items => do
    let titems ← conCerrados ctx (elaborarLista env Option.none items)
    .bien (.imprimirBloque (titems.map (·.1)), env, ctx.base)
  | .unsupported que => .fueraDeAlcance que

/-- FLW-02, RUL-19 (SEM004): `from`, `to` y `step` son `integer`. -/
partial def limiteDeRango (t : Ty) : Resultado Unit :=
  match t with
  | .integer => .bien ()
  | t => .diagnostico "SEM004" s!"a range bound or step must be `integer`, found `{t.nombre}`"

partial def comprobarRamas (ctx : Ctx) (env : Entorno)
    : List (Expr × List Stmt) → Resultado (List (TExpr × List TStmt))
  | [] => .bien []
  | (cond, cuerpo) :: resto => do
    let tc ← comprobarCondicion ctx env cond
    let tcuerpo ← comprobarCuerpo { base := 0, bucles := ctx.bucles } env cuerpo
    let tresto ← comprobarRamas ctx env resto
    .bien ((tc, tcuerpo) :: tresto)

/-- Un cuerpo: sus declaraciones no salen de él. -/
partial def comprobarCuerpo (ctx : Ctx) (env : Entorno) : List Stmt → Resultado (List TStmt)
  | [] => .bien []
  | s :: resto => do
    let (ts, env', base') ← comprobarSentencia ctx env s
    let cerrados := ctx.cerrados ++ cuerposDe s
    let tresto ← comprobarCuerpo { ctx with base := base', cerrados := cerrados } env' resto
    .bien (ts :: tresto)

/-- Los nombres que los cuerpos de una sentencia compuesta declaran. -/
partial def cuerposDe : Stmt → List String
  | .ifStmt ramas sino =>
    (ramas.map (fun (_, c) => declaradosEn c)).flatten
      ++ (match sino with | some c => declaradosEn c | Option.none => [])
  | .whileStmt _ c => declaradosEn c
  | .iterateElems n _ c => n :: declaradosEn c
  | .iterateRange n _ _ _ c => n :: declaradosEn c
  | _ => []

end

def comprobar (p : List Stmt) : Resultado (List TStmt) :=
  comprobarCuerpo { base := 0, bucles := 0 } [] p

-- ---------------------------------------------------------------
-- Ejecución
-- ---------------------------------------------------------------

/-- Cómo termina un cuerpo: siguiendo, por `break`, por `continue` o
    por `return`. -/
inductive Flujo where
  | sigue
  | rompe
  | continua
  | devuelve (v : Option Valor)
  deriving Inhabited

/-- El estado de la ejecución: la memoria, lo impreso hasta ahora, el
    valor de la última sentencia con valor, y el combustible que acota
    las pasadas de bucle para que un programa que no termina no cuelgue
    el arnés. -/
structure Estado where
  m : Memoria
  salida : String
  ultimo : Option Valor
  combustible : Nat
  deriving Inhabited

def combustibleInicial : Nat := 100000

/-- STD-04: lo que `print` escribe de un valor. -/
def escribir (st : Estado) (v : Valor) (salto : Bool) : Resultado Estado := do
  let s ← aTexto v
  .bien { st with salida := st.salida ++ s ++ (if salto then "\n" else "") }

/-- Salida de un cuerpo: la memoria vuelve a su longitud de entrada;
    las asignaciones a nombres de fuera ya sustituyeron en su sitio. -/
def cerrarAmbito (st : Estado) (n0 : Nat) : Estado :=
  { st with m := st.m.drop (st.m.length - n0) }

mutual

partial def ejecutarSentencia (st : Estado) : TStmt → Resultado (Flujo × Estado)
  | .guardar nombre e => do
    let v ← evaluar st.m e
    .bien (.sigue, { st with m := st.m.poner nombre v, ultimo := some v })
  | .declarar nombre => .bien (.sigue, { st with m := (nombre, .indefinido) :: st.m })
  | .evaluar e => do
    let v ← evaluar st.m e
    .bien (.sigue, { st with ultimo := some v })
  | .si ramas sino => ejecutarRamas st ramas sino
  | .mientras cond cuerpo => bucleMientras st cond cuerpo
  | .recorrer nombre fuente cuerpo => do
    let v ← evaluar st.m fuente
    match v with
    | .list xs => bucleElementos st nombre xs cuerpo
    -- FLW-02, STD-16: una cadena se recorre por code points, cada uno
    -- una cadena de un carácter.
    | .str s => bucleElementos st nombre (s.toList.map (fun c => Valor.str (String.singleton c))) cuerpo
    | _ => .fueraDeAlcance "iterate sobre un valor que no es lista ni cadena"
  -- FLW-02: `from`, `to` y `step` se evalúan una vez cada uno, en ese
  -- orden, antes de la primera pasada; el paso por defecto sigue la
  -- dirección; RUN020 con paso 0 calculado.
  | .rango nombre desde hasta paso cuerpo => do
    let vd ← evaluar st.m desde
    let vh ← evaluar st.m hasta
    let vp ← match paso with
      | Option.none => pure Option.none
      | some p => do let v ← evaluar st.m p; pure (some v)
    match vd, vh, vp with
    | .int a, .int b, Option.none =>
      bucleRango st nombre a b (if a ≤ b then 1 else -1) cuerpo
    | .int a, .int b, some (.int 0) =>
      let _ := a; let _ := b
      .diagnostico "RUN020" "a range cannot advance with step 0"
    | .int a, .int b, some (.int s) => bucleRango st nombre a b s cuerpo
    | _, _, _ => .fueraDeAlcance "rango con límites que no son integer"
  | .romper => .bien (.rompe, st)
  | .continuar => .bien (.continua, st)
  | .devolver Option.none => .bien (.devuelve Option.none, st)
  | .devolver (some e) => do
    let v ← evaluar st.m e
    .bien (.devuelve (some v), st)
  | .imprimir e salto => do
    let v ← evaluar st.m e
    let s ← match salto with
      | Option.none => pure true
      | some se => do
        let sv ← evaluar st.m se
        match sv with
        | .bool b => pure b
        | _ => .fueraDeAlcance "`newline:` con un valor que no es boolean"
    let st' ← escribir st v s
    .bien (.sigue, st')
  | .imprimirBloque items => do
    let st' ← imprimirLineas st items
    .bien (.sigue, st')

partial def imprimirLineas (st : Estado) : List TExpr → Resultado Estado
  | [] => .bien st
  | e :: resto => do
    let v ← evaluar st.m e
    let st' ← escribir st v true
    imprimirLineas st' resto

partial def ejecutarRamas (st : Estado) (ramas : List (TExpr × List TStmt)) (sino : Option (List TStmt))
    : Resultado (Flujo × Estado) :=
  match ramas with
  | [] =>
    match sino with
    | Option.none => .bien (.sigue, st)
    | some cuerpo => ejecutarCuerpo st cuerpo
  | (cond, cuerpo) :: resto => do
    let c ← evaluar st.m cond
    match c with
    | .bool true => ejecutarCuerpo st cuerpo
    | .bool false => ejecutarRamas st resto sino
    | _ => .fueraDeAlcance "condición que no es boolean en ejecución"

/-- Un cuerpo es un ámbito: al salir, la memoria vuelve a su longitud. -/
partial def ejecutarCuerpo (st : Estado) (cuerpo : List TStmt) : Resultado (Flujo × Estado) := do
  let n0 := st.m.length
  let (f, st') ← ejecutarSecuencia st cuerpo
  .bien (f, cerrarAmbito st' n0)

partial def ejecutarSecuencia (st : Estado) : List TStmt → Resultado (Flujo × Estado)
  | [] => .bien (.sigue, st)
  | s :: resto => do
    let (f, st') ← ejecutarSentencia st s
    match f with
    | .sigue => ejecutarSecuencia st' resto
    | otro => .bien (otro, st')

/-- Una pasada de cuerpo de bucle, con el combustible descontado. -/
partial def pasada (st : Estado) (cuerpo : List TStmt) : Resultado (Flujo × Estado) :=
  if st.combustible == 0 then
    .fueraDeAlcance s!"más de {combustibleInicial} pasadas de bucle"
  else ejecutarCuerpo { st with combustible := st.combustible - 1 } cuerpo

partial def bucleMientras (st : Estado) (cond : TExpr) (cuerpo : List TStmt) : Resultado (Flujo × Estado) := do
  let c ← evaluar st.m cond
  match c with
  | .bool false => .bien (.sigue, st)
  | .bool true => do
    let (f, st') ← pasada st cuerpo
    match f with
    | .rompe => .bien (.sigue, st')
    | .devuelve v => .bien (.devuelve v, st')
    | _ => bucleMientras st' cond cuerpo
  | _ => .fueraDeAlcance "condición de while que no es boolean en ejecución"

/-- FLW-02: la cuenta de pasadas es la longitud a la entrada; el
    binder es una variable del cuerpo, ligada de nuevo en cada pasada. -/
partial def bucleElementos (st : Estado) (nombre : String) (xs : List Valor) (cuerpo : List TStmt)
    : Resultado (Flujo × Estado) :=
  match xs with
  | [] => .bien (.sigue, st)
  | x :: resto => do
    let (f, st') ← pasada { st with m := (nombre, x) :: st.m } (cuerpo)
    let st'' := cerrarAmbito st' st.m.length
    match f with
    | .rompe => .bien (.sigue, st'')
    | .devuelve v => .bien (.devuelve v, st'')
    | _ => bucleElementos st'' nombre resto cuerpo

/-- FLW-02: los dos extremos inclusivos cuando se alcanzan; `to` se
    visita exactamente cuando algún `from + k·step` lo iguala. -/
partial def bucleRango (st : Estado) (nombre : String) (v hasta paso : Int) (cuerpo : List TStmt)
    : Resultado (Flujo × Estado) :=
  let pasado := if paso > 0 then v > hasta else v < hasta
  if pasado then .bien (.sigue, st)
  else do
    let (f, st') ← pasada { st with m := (nombre, .int v) :: st.m } cuerpo
    let st'' := cerrarAmbito st' st.m.length
    match f with
    | .rompe => .bien (.sigue, st'')
    | .devuelve w => .bien (.devuelve w, st'')
    | _ => bucleRango st'' nombre (v + paso) hasta paso cuerpo

end

inductive Salida where
  | valor (v : Valor)
  | sinValor
  | estatico (codigo mensaje : String)
  | fallo (codigo mensaje : String)
  | sinEspecificar (que : String)
  | fueraDeAlcance (que : String)

/-- El resultado de un programa y lo que imprimió. -/
def correr (p : List Stmt) : Salida × String :=
  match comprobar p with
  | .diagnostico c msg => (.estatico c msg, "")
  | .sinEspecificar q => (.sinEspecificar q, "")
  | .fueraDeAlcance q => (.fueraDeAlcance q, "")
  | .bien tp =>
    let st0 : Estado := { m := [], salida := "", ultimo := Option.none, combustible := combustibleInicial }
    match ejecutarSecuencia st0 tp with
    | .bien (.devuelve (some v), st) => (.valor v, st.salida)
    | .bien (.devuelve Option.none, st) => (.sinValor, st.salida)
    | .bien (_, st) =>
      match st.ultimo with
      | some v => (.valor v, st.salida)
      | Option.none => (.sinValor, st.salida)
    | .diagnostico c msg => (.fallo c msg, "")
    | .sinEspecificar q => (.sinEspecificar q, "")
    | .fueraDeAlcance q => (.fueraDeAlcance q, "")

-- ---------------------------------------------------------------
-- Salida en texto, para que el arnés la lea sin interpretar nada.
-- Cinco campos: clase, tipo o código, valor o mensaje, y lo impreso.
-- ---------------------------------------------------------------

def hex (s : String) : String :=
  let digito (n : Nat) : Char :=
    if n < 10 then Char.ofNat (48 + n) else Char.ofNat (87 + n)
  s.toUTF8.foldl
    (fun acc b => (acc.push (digito (b.toNat / 16))).push (digito (b.toNat % 16)))
    ""

def Valor.texto : Valor → String
  | .int v => s!"integer\t{v}"
  -- Los 64 bits tal cual: el texto decimal de Lean pierde precisión.
  | .num v => s!"number\t{v.toBits}"
  | .bool b => s!"boolean\t{b}"
  | .str s => s!"string\t{hex s}"
  | .none => "none\t"
  | .list _ => "list\t"
  | .indefinido => "indefinido\t"

def Salida.texto : Salida → String
  | .valor v => s!"valor\t{v.texto}"
  | .sinValor => "sin-valor\t\t"
  | .estatico c msg => s!"estatico\t{c}\t{hex msg}"
  | .fallo c msg => s!"fallo\t{c}\t{hex msg}"
  | .sinEspecificar q => s!"sin-especificar\t\t{hex q}"
  | .fueraDeAlcance q => s!"fuera-de-alcance\t\t{hex q}"

def textoDe (r : Salida × String) : String :=
  r.1.texto ++ "\t" ++ hex r.2

end Clean
