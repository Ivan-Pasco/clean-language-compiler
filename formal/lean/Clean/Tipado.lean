/-
  Clean Language — piloto. Semántica estática de las expresiones.

  `elaborar` comprueba una expresión y la devuelve con cada literal ya
  en su tipo y cada operador ya resuelto sobre el tipo de sus operandos.
  Cada caso cita la unidad de la ley de la que sale.
-/
import Clean.Ast
import Clean.Resultado

namespace Clean

/-- Expresión ya tipada: lo que `evaluar` recibe. -/
inductive TExpr where
  | int (v : Int)
  | num (v : Float)
  | str (s : String)
  | bool (b : Bool)
  | none
  | var (name : String)
  | unary (op : UnOp) (t : Ty) (e : TExpr)
  | binary (op : BinOp) (t : Ty) (l r : TExpr)
  | assertSome (e : TExpr)
  /-- TYP-02: literal de lista, con el tipo de sus elementos. -/
  | list (t : Ty) (items : List TExpr)
  /-- EXPG-01 IndexAccess sobre una lista. -/
  | index (e i : TExpr)
  /-- CALL-01: llamada a método sobre un valor, con el tipo del receptor. -/
  | method (recv : TExpr) (t : Ty) (name : String) (args : List TExpr)
  deriving Inhabited

abbrev Entorno := List (String × Ty)

def Entorno.buscar (env : Entorno) (nombre : String) : Option Ty :=
  match env with
  | [] => Option.none
  | (n, t) :: resto => if n == nombre then some t else Entorno.buscar resto nombre

/-- TYP-01: `integer` es de 64 bits con signo. -/
def maxInteger : Int := 9223372036854775807
def minInteger : Int := -9223372036854775808

def Ty.base : Ty → Ty
  | .optional t => t
  | t => t

def Ty.esOpcional : Ty → Bool
  | .optional _ => true
  | _ => false

/-- TYP-03: `T` es asignable a `T?`, no al revés; `none` es asignable a
    todo tipo opcional y a nada más. TYP-02: `[]` toma el tipo de lista
    que su destino declara. -/
def asignable (origen destino : Ty) : Bool :=
  if origen == destino then true
  else match origen, destino with
    | .noneLit, .optional _ => true
    | .emptyList, .list _ => true
    | .emptyList, .optional (.list _) => true
    | t, .optional u => t == u || (t == .emptyList && (match u with | .list _ => true | _ => false))
    | _, _ => false

def fueraDeRango (n : Nat) (signo : String) : Resultado α :=
  .diagnostico "SEM026"
    s!"literal {signo}{n} does not fit integer (range {minInteger} to {maxInteger})"

def noDefinido (op : String) (t : Ty) : Resultado α :=
  .diagnostico "SEM004" s!"operator `{op}` is not defined for type `{t.nombre}`"

/-- RUL-19: dos tipos que admiten el operador por separado, no juntos. -/
def noDefinidoEntre (op : String) (t u : Ty) : Resultado α :=
  .diagnostico "SEM004"
    s!"operator `{op}` is not defined between `{t.nombre}` and `{u.nombre}`"

def esLiteralEntero : Expr → Bool
  | .intLit _ => true
  | .paren e => esLiteralEntero e
  | .unary .neg e => esLiteralEntero e
  | _ => false

/-- Un literal numérico, o una operación hecha solo de literales: lo
    que TYP-06 deja tomar su tipo del contexto. -/
def esLiteralNumerico : Expr → Bool
  | .intLit _ => true
  | .numLit _ _ => true
  | .paren e => esLiteralNumerico e
  | .unary .neg e => esLiteralNumerico e
  | .binary _ l r => esLiteralNumerico l && esLiteralNumerico r
  | _ => false

def esOperadorNumerico : BinOp → Bool
  | .add | .sub | .mul | .div | .mod | .pow => true
  | _ => false

def Float.deLiteral (mantisa : Nat) (exp10 : Int) : Float :=
  if exp10 < 0 then OfScientific.ofScientific mantisa true exp10.natAbs
  else OfScientific.ofScientific mantisa false exp10.natAbs

/-- Tipado de un operador binario aritmético, de comparación o lógico,
    ya con los tipos de sus dos operandos. Devuelve el tipo sobre el que
    opera y el tipo del resultado. -/
def tiparBinario (op : BinOp) (tl tr : Ty) : Resultado (Ty × Ty) :=
  let mezclaNumerica :=
    (tl == .integer && tr == .number) || (tl == .number && tr == .integer)
  match op with
  | .add | .sub | .mul | .div | .mod | .pow =>
    -- TYP-06: el literal junto a un operando de tipo conocido toma ese
    -- tipo; `elaborar` ya lo hizo, así que aquí la mezcla es SEM004
    -- (EXP-04, RUL-19).
    if mezclaNumerica then noDefinidoEntre op.simbolo tl tr
    else if tl != tr then
      -- RUL-19 (SEM004): el operador no tiene significado sobre el tipo.
      if tl == .integer || tl == .number || (op == .add && tl == .string)
      then noDefinido op.simbolo tr else noDefinido op.simbolo tl
    else match tl, op with
      -- EXP-08: aritmética sobre integer y number.
      | .integer, _ => .bien (.integer, .integer)
      -- EXP-08: `%` es de integer; sobre number es SEM004.
      | .number, .mod => noDefinido "%" .number
      | .number, _ => .bien (.number, .number)
      -- EXP-08: `+` sobre string es concatenación.
      | .string, .add => .bien (.string, .string)
      | t, _ => noDefinido op.simbolo t
  | .lt | .gt | .le | .ge =>
    -- EXP-05: orden solo entre dos integer o dos number.
    if mezclaNumerica then noDefinidoEntre op.simbolo tl tr
    else if tl != tr then noDefinidoEntre op.simbolo tl tr
    else match tl with
      | .integer => .bien (.integer, .boolean)
      | .number => .bien (.number, .boolean)
      | t => noDefinido op.simbolo t
  | .eq | .ne | .is | .isNot =>
    -- EXP-05: dos operandos de un tipo — T contra T o T? — o none
    -- contra cualquier opcional; cualquier otro par es SEM004.
    if tl == .noneLit || tr == .noneLit || tl.esOpcional || tr.esOpcional then
      if tl.base == tr.base || tl == .noneLit || tr == .noneLit
      then .bien (tl.base, .boolean)
      else noDefinidoEntre op.simbolo tl tr
    else if tl != tr then noDefinidoEntre op.simbolo tl tr
    else .bien (tl, .boolean)
  | .and | .or =>
    -- EXP-07; RUL-19 para el operando que no es boolean.
    if tl != .boolean then noDefinido op.simbolo tl
    else if tr != .boolean then noDefinido op.simbolo tr
    else .bien (.boolean, .boolean)
  | .default | .onError =>
    .fueraDeAlcance "tiparBinario no recibe default ni onError"

/-- STD-07, STD-15, STD-25: los métodos que el piloto conoce, por tipo
    del receptor: (nombre, número de argumentos, tipo del resultado). -/
def metodo (recv : Ty) (nombre : String) (n : Nat) : Resultado Ty :=
  match nombre, n, recv with
  -- STD-07: `toString()` existe en todo tipo.
  | "toString", 0, .integer => .bien .string
  | "toString", 0, .boolean => .bien .string
  | "toString", 0, .string => .bien .string
  | "toString", 0, .number => .bien .string
  -- STD-15, STD-16: `length()` cuenta code points; STD-25: elementos.
  | "length", 0, .string => .bien .integer
  | "length", 0, .list _ => .bien .integer
  | "isEmpty", 0, .string => .bien .boolean
  | "isEmpty", 0, .list _ => .bien .boolean
  | "isNotEmpty", 0, .list _ => .bien .boolean
  | _, _, .optional _ => .fueraDeAlcance s!"método `{nombre}` sobre un opcional"
  | _, _, _ => .fueraDeAlcance s!"método `{nombre}/{n}` sobre `{recv.nombre}`"

mutual

/-- Semántica estática de una expresión. `esperado` es el tipo que el
    contexto espera, del que un literal numérico toma el suyo (TYP-06). -/
partial def elaborar (env : Entorno) (esperado : Option Ty) : Expr → Resultado (TExpr × Ty)
  -- TYP-06: el literal toma el tipo numérico que su contexto espera.
  -- RUL-41 (SEM026): el literal debe caber en su tipo.
  | .intLit n =>
    if (esperado.map Ty.base) == some .number then
      .bien (.num (Float.ofNat n), .number)
    else if (n : Int) ≤ maxInteger then .bien (.int n, .integer)
    else fueraDeRango n ""
  -- TYP-01, RUL-41: el rango se mide con el menos unario ya aplicado.
  | .unary .neg (.intLit n) =>
    if (esperado.map Ty.base) == some .number then
      .bien (.num (-(Float.ofNat n)), .number)
    else if -(n : Int) ≥ minInteger then .bien (.int (-(n : Int)), .integer)
    else fueraDeRango n "-"
  | .numLit m e => .bien (.num (Float.deLiteral m e), .number)
  | .strLit s => .bien (.str s, .string)
  | .boolLit b => .bien (.bool b, .boolean)
  -- TYP-03: `none` no tiene tipo propio.
  | .noneLit => .bien (.none, .noneLit)
  -- RUL-17 (SEM002).
  | .var nombre =>
    match env.buscar nombre with
    | some t => .bien (.var nombre, t)
    | Option.none => .diagnostico "SEM002" s!"undefined variable `{nombre}`"
  -- EXPG-02: el paréntesis no cambia el significado.
  | .paren e => elaborar env esperado e
  | .unary .neg e => do
    let (te, t) ← elaborar env esperado e
    match t with
    | .integer => .bien (.unary .neg .integer te, .integer)
    | .number => .bien (.unary .neg .number te, .number)
    | t => noDefinido "-" t
  | .unary .not e => do
    let (te, t) ← elaborar env Option.none e
    match t with
    | .boolean => .bien (.unary .not .boolean te, .boolean)
    | t => noDefinido "not" t
  -- ERH-02: la expresión guardada se tipa en el contexto de toda la
  -- expresión; el respaldo debe tener su tipo, admitiendo T → T?.
  | .binary .onError l r => do
    let (tl, tyl) ← elaborar env esperado l
    let (tr, tyr) ← elaborar env (some tyl) r
    let salida : Resultado (TExpr × Ty) :=
      if asignable tyr tyl then .bien (TExpr.binary .onError tyl tl tr, tyl)
      else .diagnostico "SEM001" "type mismatch in assignment"
    salida
  -- EXP-03: `default` da el izquierdo si no es none, y si no el derecho.
  | .binary .default l r => do
    let (tl, tyl) ← elaborar env Option.none l
    let contexto := if tyl == .noneLit then esperado else some tyl.base
    let (tr, tyr) ← elaborar env contexto r
    let salida : Resultado (TExpr × Ty) :=
      if tyl == .noneLit || tyr == tyl.base || tyr == tyl then
        .bien (TExpr.binary .default tyr tl tr, tyr)
      else .diagnostico "SEM001" "type mismatch in assignment"
    salida
  -- TYP-06: el contexto de un literal es, en este orden, el otro
  -- operando de tipo conocido, el tipo declarado, y el contexto de la
  -- expresión entera cuando los dos operandos son literales.
  | .binary op l r => do
    let contexto := if esOperadorNumerico op then esperado.map Ty.base else Option.none
    let (tl, tyl) ← elaborar env (if esLiteralNumerico l then contexto else Option.none) l
    let ctxR := if esLiteralNumerico r
                then (if esLiteralNumerico l then contexto else some tyl.base)
                else Option.none
    let (tr, tyr) ← elaborar env ctxR r
    -- un literal a la izquierda de un operando de tipo conocido
    let (tl, tyl) ← if esLiteralNumerico l && !esLiteralNumerico r && tyl != tyr
                    then elaborar env (some tyr.base) l else pure (tl, tyl)
    let (sobre, resultado) ← tiparBinario op tyl tyr
    .bien (TExpr.binary op sobre tl tr, resultado)
  -- EXP-03: `!` afirma que el valor no es none y estrecha su tipo.
  | .assertSome e => do
    let (te, t) ← elaborar env Option.none e
    match t with
    | .optional u => .bien (.assertSome te, u)
    | .noneLit => noDefinido "!" .noneLit
    | t => noDefinido "!" t
  -- TYP-02: una lista es homogénea; `[]` toma el tipo de su destino.
  | .listLit items =>
    let contexto : Option Ty := match esperado.map Ty.base with
      | some (.list t) => some t
      | _ => Option.none
    match items with
    | [] =>
      match contexto with
      | some t => .bien (.list t [], .list t)
      | Option.none => .bien (.list .emptyList [], .emptyList)
    | primero :: resto => do
      let (tp, t) ← elaborar env contexto primero
      let tresto ← elaborarLista env (some t) resto
      -- La ley no nombra el diagnóstico de una lista heterogénea.
      if tresto.any (fun (_, u) => u != t) then
        .sinEspecificar "literal de lista con elementos de tipos distintos"
      else .bien (.list t (tp :: tresto.map (·.1)), .list t)
  -- EXPG-01 IndexAccess: `items[i]` sobre una lista, con índice integer.
  | .index e i => do
    let (te, t) ← elaborar env Option.none e
    let (ti, tyi) ← elaborar env (some .integer) i
    match t with
    | .list u =>
      if tyi == .integer then .bien (.index te ti, u)
      else noDefinido "[]" tyi
    | .string => .fueraDeAlcance "índice sobre una cadena"
    | t => noDefinido "[]" t
  -- CALL-01: llamada a método sobre un valor.
  | .call (.member recv nombre) args => do
    let (tr, t) ← elaborar env Option.none recv
    let resultado ← metodo t nombre args.length
    let targs ← elaborarLista env Option.none args
    .bien (.method tr t nombre (targs.map (·.1)), resultado)
  | .member _ _ => .fueraDeAlcance "acceso a miembro"
  | .call _ _ => .fueraDeAlcance "llamada"
  | .namespaceCall _ _ _ => .fueraDeAlcance "llamada a un espacio de nombres"

partial def elaborarLista (env : Entorno) (esperado : Option Ty)
    : List Expr → Resultado (List (TExpr × Ty))
  | [] => .bien []
  | e :: resto => do
    let te ← elaborar env esperado e
    let tresto ← elaborarLista env esperado resto
    .bien (te :: tresto)

end

end Clean
