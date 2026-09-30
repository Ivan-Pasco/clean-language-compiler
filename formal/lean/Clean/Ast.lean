/-
  Clean Language — piloto. Sintaxis abstracta.

  Es el contrato entre la gramática (clean.lark) y la semántica: el
  puente de Python entrega términos de estos tipos y nada más.
-/
namespace Clean

/-- TYP-01, TYP-03. `noneLit` es el tipo del literal `none`, que no
    tiene tipo propio y es asignable a todo `T?` (TYP-03). `other` es
    un tipo que el piloto no cubre. -/
inductive Ty where
  | integer
  | number
  | boolean
  | string
  | optional (t : Ty)
  /-- TYP-02: `list<T>`, homogénea. Una `matrix<T>` es `list<list<T>>`
      (D-04, H-03). -/
  | list (t : Ty)
  | noneLit
  /-- El tipo de un literal `[]` sin contexto: lo fija el destino. -/
  | emptyList
  | other (name : String)
  deriving Repr, BEq, Inhabited

/-- EXP-01 nivel 3. -/
inductive UnOp where
  | neg
  | not
  deriving Repr, BEq, Inhabited

/-- EXP-01 niveles 4 a 13, sin la asignación, que es sentencia. -/
inductive BinOp where
  | add | sub | mul | div | mod | pow
  | lt | gt | le | ge
  | eq | ne | is | isNot
  | and | or
  | default
  | onError
  deriving Repr, BEq, Inhabited

/-- EXPG-01, EXPG-02. Un literal numérico no lleva signo (LEX-06):
    por eso `intLit` guarda un natural. `namespaceCall` es la llamada
    sobre una type keyword (EXPG-02 NamespaceCall, LEX-04). -/
inductive Expr where
  | intLit (n : Nat)
  | numLit (mantissa : Nat) (exp10 : Int)
  | strLit (s : String)
  | boolLit (b : Bool)
  | noneLit
  | var (name : String)
  | unary (op : UnOp) (e : Expr)
  | binary (op : BinOp) (l r : Expr)
  | assertSome (e : Expr)
  | paren (e : Expr)
  | member (e : Expr) (name : String)
  | call (f : Expr) (args : List Expr)
  | namespaceCall (ns fn : String) (args : List Expr)
  | index (e i : Expr)
  | listLit (items : List Expr)
  deriving Inhabited

/-- STM-01..STM-04, TYP-11, FLW-01..FLW-03. Las sentencias compuestas
    llevan sus cuerpos como listas: STMG-01. -/
inductive Stmt where
  | decl (t : Ty) (name : String) (init : Option Expr)
  | assign (name : String) (e : Expr)
  | expr (e : Expr)
  /-- FLW-01: la cadena `if` / `else if` / `else`, cada rama con su
      condición y su cuerpo, y el `else` sin condición al final. -/
  | ifStmt (branches : List (Expr × List Stmt)) (elseBody : Option (List Stmt))
  /-- FLW-02: `while`. -/
  | whileStmt (cond : Expr) (body : List Stmt)
  /-- FLW-02: `iterate name in source` sobre una lista, una cadena o
      una matriz. -/
  | iterateElems (name : String) (source : Expr) (body : List Stmt)
  /-- FLW-02: `iterate name in from to to [step n]`. -/
  | iterateRange (name : String) (desde hasta : Expr) (step : Option Expr) (body : List Stmt)
  /-- FLW-03. -/
  | breakStmt
  | continueStmt
  /-- STM-03: `return`, con o sin expresión. -/
  | returnStmt (e : Option Expr)
  /-- STD-04: `print(value)` y `print(value, newline: false)`. -/
  | print (e : Expr) (newline : Option Expr)
  /-- STM-04: el bloque `print:`, una expresión por línea. -/
  | printBlock (items : List Expr)
  | unsupported (what : String)
  deriving Inhabited

def Ty.nombre : Ty → String
  | .integer => "integer"
  | .number => "number"
  | .boolean => "boolean"
  | .string => "string"
  | .optional t => t.nombre ++ "?"
  | .list t => "list<" ++ t.nombre ++ ">"
  | .noneLit => "none"
  | .emptyList => "list<>"
  | .other n => n

def UnOp.simbolo : UnOp → String
  | .neg => "-"
  | .not => "not"

def BinOp.simbolo : BinOp → String
  | .add => "+" | .sub => "-" | .mul => "*" | .div => "/"
  | .mod => "%" | .pow => "^"
  | .lt => "<" | .gt => ">" | .le => "<=" | .ge => ">="
  | .eq => "==" | .ne => "!=" | .is => "is" | .isNot => "not"
  | .and => "and" | .or => "or"
  | .default => "default"
  | .onError => "onError"

end Clean
