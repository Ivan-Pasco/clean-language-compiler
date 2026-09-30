/-
  Clean Language — piloto. Semántica dinámica de las expresiones.

  `evaluar` recibe la expresión ya tipada. Un fallo en tiempo de
  ejecución sale como `diagnostico` con su código RUN, que es lo que
  `onError` atrapa (ERH-03).
-/
import Clean.Ast
import Clean.Resultado
import Clean.Tipado

namespace Clean

inductive Valor where
  | int (v : Int)
  | num (v : Float)
  | bool (b : Bool)
  | str (s : String)
  | none
  /-- TYP-02: una lista de valores. -/
  | list (items : List Valor)
  /-- Declarada sin inicializar (STM-01 `string z`): la ley no dice qué
      pasa al leerla antes de asignarla. -/
  | indefinido
  deriving Inhabited

/-- La memoria es una lista de pares; el primero que lleva un nombre es
    su valor vigente. Un bloque recuerda la longitud a su entrada y la
    restaura a su salida: las declaraciones del bloque mueren con él y
    las asignaciones a nombres de fuera, que sustituyen en su sitio,
    sobreviven. -/
abbrev Memoria := List (String × Valor)

def Memoria.buscar (m : Memoria) (nombre : String) : Option Valor :=
  match m with
  | [] => Option.none
  | (n, v) :: resto => if n == nombre then some v else Memoria.buscar resto nombre

def Memoria.reemplazar (m : Memoria) (nombre : String) (v : Valor) : Memoria :=
  match m with
  | [] => []
  | (n, w) :: resto => if n == nombre then (n, v) :: resto else (n, w) :: Memoria.reemplazar resto nombre v

/-- Sustituye el valor del primer par con ese nombre — la asignación a
    una variable viva, en su sitio —; si no hay, lo añade al frente,
    en el ámbito actual. -/
def Memoria.poner (m : Memoria) (nombre : String) (v : Valor) : Memoria :=
  if (m.buscar nombre).isSome then m.reemplazar nombre v else (nombre, v) :: m

def enRango (v : Int) : Bool := minInteger ≤ v && v ≤ maxInteger

/-- RUL-136 (RUN003): un resultado integer que no cabe falla, no envuelve. -/
def desborda (op : String) : Resultado α :=
  .diagnostico "RUN003" s!"integer overflow in {op}"

def entero (op : String) (v : Int) : Resultado Valor :=
  if enRango v then .bien (.int v) else desborda op

/-- EXP-04, RUL-136 (RUN003): aritmética de integer. -/
def aritmeticaEntera (op : BinOp) (a b : Int) : Resultado Valor :=
  match op with
  | .add => entero "addition" (a + b)
  | .sub => entero "subtraction" (a - b)
  | .mul => entero "multiplication" (a * b)
  -- EXP-04: cociente truncado hacia cero; resto con el signo del dividendo.
  | .div =>
    if b == 0 then .diagnostico "RUN003" "division by zero"
    else if a == minInteger && b == -1 then
      .diagnostico "RUN003" "integer overflow in division"
    else .bien (.int (Int.tdiv a b))
  | .mod =>
    if b == 0 then .diagnostico "RUN003" "division by zero"
    else .bien (.int (Int.tmod a b))
  | .pow =>
    if b < 0 then .diagnostico "RUN003" "negative exponent on integer"
    else entero "exponentiation" (a ^ b.toNat)
  | _ => .fueraDeAlcance "aritmeticaEntera recibe solo operadores aritméticos"

/-- EXP-04, STD-14: aritmética de number, IEEE 754; nunca falla. -/
def aritmeticaDecimal (op : BinOp) (a b : Float) : Resultado Valor :=
  match op with
  | .add => .bien (.num (a + b))
  | .sub => .bien (.num (a - b))
  | .mul => .bien (.num (a * b))
  | .div => .bien (.num (a / b))
  | .pow => .bien (.num (Float.pow a b))
  | _ => .fueraDeAlcance "aritmeticaDecimal recibe solo operadores aritméticos"

def comparar (op : BinOp) (menor igual : Bool) : Bool :=
  match op with
  | .lt => menor
  | .le => menor || igual
  | .gt => !(menor || igual)
  | .ge => !menor
  | _ => false

/-- LEX-06, TYP-07: igualdad por valor; la de string es byte a byte. -/
def iguales : Valor → Valor → Resultado Bool
  | .none, .none => .bien true
  | .none, _ => .bien false
  | _, .none => .bien false
  | .int a, .int b => .bien (a == b)
  | .bool a, .bool b => .bien (a == b)
  | .str a, .str b => .bien (a == b)
  -- EXP-05: IEEE 754 — NaN no es igual a nada, ni a sí mismo.
  | .num a, .num b => .bien (a == b)
  | .list _, .list _ => .fueraDeAlcance "igualdad entre listas"
  | _, _ => .fueraDeAlcance "igualdad entre valores de tipos distintos"

def binarioEstricto (op : BinOp) (t : Ty) (a b : Valor) : Resultado Valor :=
  match op, a, b with
  | .add, .str x, .str y => .bien (.str (x ++ y))
  | .eq, x, y => do let r ← iguales x y; .bien (.bool r)
  | .ne, x, y => do let r ← iguales x y; .bien (.bool (!r))
  -- EXP-05: identidad es igualdad en los tipos valor; sobre un valor
  -- heap-shaped (string, list) es identidad de objeto, que este
  -- evaluador sin referencias no representa.
  | .is, .str _, .str _ => .fueraDeAlcance "identidad entre strings: pide un modelo de referencias"
  | .isNot, .str _, .str _ => .fueraDeAlcance "identidad entre strings: pide un modelo de referencias"
  | .is, .list _, .list _ => .fueraDeAlcance "identidad entre listas: pide un modelo de referencias"
  | .isNot, .list _, .list _ => .fueraDeAlcance "identidad entre listas: pide un modelo de referencias"
  | .is, x, y => do let r ← iguales x y; .bien (.bool r)
  | .isNot, x, y => do let r ← iguales x y; .bien (.bool (!r))
  | op, .int x, .int y =>
    match op with
    | .lt | .le | .gt | .ge => .bien (.bool (comparar op (x < y) (x == y)))
    | _ => aritmeticaEntera op x y
  | op, .num x, .num y =>
    match op with
    | .lt => .bien (.bool (x < y))
    | .le => .bien (.bool (x ≤ y))
    | .gt => .bien (.bool (x > y))
    | .ge => .bien (.bool (x ≥ y))
    | _ => aritmeticaDecimal op x y
  | _, _, _ =>
    .fueraDeAlcance s!"`{op.simbolo}` sobre `{t.nombre}` en evaluación"

/-- STD-07: `toString()`. `integer` en decimal llano, `boolean` como
    `true`/`false`, `string` tal cual. STD-08 (la forma más corta que
    hace el viaje de ida y vuelta) queda fuera del piloto. -/
def aTexto : Valor → Resultado String
  | .int v => .bien (toString v)
  | .bool b => .bien (if b then "true" else "false")
  | .str s => .bien s
  | .num _ => .fueraDeAlcance "number.toString(): la forma más corta (STD-08)"
  | .none => .fueraDeAlcance "none.toString()"
  | .list _ => .fueraDeAlcance "list.toString()"
  | .indefinido => .sinEspecificar "toString() de una variable declarada sin valor"

/-- STD-15, STD-16, STD-25: los métodos que el piloto evalúa. -/
def metodoValor (recv : Valor) (nombre : String) (_args : List Valor) : Resultado Valor :=
  match nombre, recv with
  | "toString", v => do let s ← aTexto v; .bien (.str s)
  -- STD-16: la longitud cuenta code points.
  | "length", .str s => .bien (.int s.length)
  | "length", .list xs => .bien (.int xs.length)
  | "isEmpty", .str s => .bien (.bool s.isEmpty)
  | "isEmpty", .list xs => .bien (.bool xs.isEmpty)
  | "isNotEmpty", .list xs => .bien (.bool (!xs.isEmpty))
  | _, _ => .fueraDeAlcance s!"método `{nombre}` en evaluación"

mutual

/-- Semántica dinámica de una expresión ya tipada. -/
partial def evaluar (m : Memoria) : TExpr → Resultado Valor
  | .int v => .bien (.int v)
  | .num v => .bien (.num v)
  | .str s => .bien (.str s)
  | .bool b => .bien (.bool b)
  | .none => .bien .none
  | .var nombre =>
    match m.buscar nombre with
    | some .indefinido => .sinEspecificar s!"lectura de `{nombre}`, declarada sin valor"
    | some v => .bien v
    | Option.none => .fueraDeAlcance s!"lectura de `{nombre}` sin valor"
  | .unary .neg _ e => do
    let v ← evaluar m e
    match v with
    | .int a => if a == minInteger then desborda "negation" else .bien (.int (-a))
    | .num a => .bien (.num (-a))
    | _ => .fueraDeAlcance "menos unario sobre un valor no numérico"
  | .unary .not _ e => do
    let v ← evaluar m e
    match v with
    | .bool b => .bien (.bool (!b))
    | _ => .fueraDeAlcance "not sobre un valor no boolean"
  -- ERH-02: si la expresión guardada falla, vale el respaldo.
  | .binary .onError _ l r =>
    match evaluar m l with
    | .diagnostico _ _ => evaluar m r
    | otro => otro
  -- EXP-03: el izquierdo si no es none; si no, el derecho.
  | .binary .default _ l r => do
    let a ← evaluar m l
    match a with
    | .none => evaluar m r
    | a => .bien a
  -- EXP-07: and, or.
  | .binary .and _ l r => do
    let a ← evaluar m l
    match a with
    | .bool false => .bien (.bool false)
    | .bool true => evaluar m r
    | _ => .fueraDeAlcance "and sobre un valor no boolean"
  | .binary .or _ l r => do
    let a ← evaluar m l
    match a with
    | .bool true => .bien (.bool true)
    | .bool false => evaluar m r
    | _ => .fueraDeAlcance "or sobre un valor no boolean"
  -- EXP-10 por analogía: el operando izquierdo se evalúa primero.
  | .binary op t l r => do
    let a ← evaluar m l
    let b ← evaluar m r
    binarioEstricto op t a b
  -- EXP-03, RUL-137 (RUN004).
  | .assertSome e => do
    let v ← evaluar m e
    match v with
    | .none => .diagnostico "RUN004" ""
    | v => .bien v
  -- EXP-10: los elementos, de izquierda a derecha.
  | .list _ items => do
    let vs ← evaluarLista m items
    .bien (.list vs)
  -- TYP-02: `items[i]`, con base cero. La ley no nombra el diagnóstico
  -- de un índice fuera de rango escrito con `[]` (RUN013 habla de
  -- `get`, `charAt`…).
  | .index e i => do
    let xs ← evaluar m e
    let k ← evaluar m i
    match xs, k with
    | .list vs, .int n =>
      if 0 ≤ n && n < vs.length then .bien (vs.getD n.toNat .none)
      else .sinEspecificar "índice fuera de rango con `[]`"
    | _, _ => .fueraDeAlcance "índice sobre un valor que no es lista"
  | .method recv _ nombre args => do
    let v ← evaluar m recv
    let vs ← evaluarLista m args
    metodoValor v nombre vs

partial def evaluarLista (m : Memoria) : List TExpr → Resultado (List Valor)
  | [] => .bien []
  | e :: resto => do
    let v ← evaluar m e
    let vs ← evaluarLista m resto
    .bien (v :: vs)

end

end Clean
