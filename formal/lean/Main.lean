import Clean
open Clean

def caso0 : List Stmt := [(Stmt.expr (Expr.binary BinOp.pow (Expr.intLit 2) (Expr.binary BinOp.pow (Expr.intLit 3) (Expr.intLit 2))))]
def caso1 : List Stmt := [(Stmt.expr (Expr.binary BinOp.pow (Expr.paren (Expr.binary BinOp.pow (Expr.intLit 2) (Expr.intLit 3))) (Expr.intLit 2)))]
def caso2 : List Stmt := [(Stmt.expr (Expr.binary BinOp.sub (Expr.binary BinOp.sub (Expr.var "a") (Expr.var "b")) (Expr.var "c")))]
def caso3 : List Stmt := [(Stmt.assign "x" (Expr.binary BinOp.onError (Expr.call (Expr.var "f") []) (Expr.intLit 0)))]
def caso4 : List Stmt := [(Stmt.assign "outcome" (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.var "a") (Expr.var "b")) (Expr.var "c")))]
def caso5 : List Stmt := [(Stmt.assign "value" (Expr.call (Expr.var "functionCall") [(Expr.var "arg1"), (Expr.var "arg2")]))]
def caso6 : List Stmt := [(Stmt.assign "outcome" (Expr.paren (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.var "a") (Expr.var "b")) (Expr.var "c")) (Expr.var "d")) (Expr.var "e")) (Expr.var "f"))))]
def caso7 : List Stmt := [(Stmt.assign "complex" (Expr.paren (Expr.binary BinOp.add (Expr.call (Expr.var "functionCall") [(Expr.var "arg1"), (Expr.var "arg2")]) (Expr.binary BinOp.mul (Expr.call (Expr.var "anotherFunction") [(Expr.var "arg3")]) (Expr.paren (Expr.binary BinOp.add (Expr.var "nested") (Expr.var "expression")))))))]
def caso8 : List Stmt := [(Stmt.assign "calculation" (Expr.paren (Expr.binary BinOp.add (Expr.binary BinOp.mul (Expr.var "matrix1") (Expr.var "matrix2")) (Expr.binary BinOp.mul (Expr.call (Expr.member (Expr.var "matrix3") "transpose") []) (Expr.var "scalar_value")))))]
def caso9 : List Stmt := [(Stmt.assign "total" (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.var "price") (Expr.var "tax")) (Expr.var "shipping")))]
def caso10 : List Stmt := [(Stmt.assign "total" (Expr.paren (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.var "price") (Expr.var "tax")) (Expr.var "shipping")) (Expr.var "handling"))))]
def caso11 : List Stmt := [(Stmt.assign "outcome" (Expr.paren (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.call (Expr.var "calculateBase") [(Expr.var "width"), (Expr.var "height")]) (Expr.call (Expr.var "calculateTax") [(Expr.var "subtotal")])) (Expr.paren (Expr.binary BinOp.mul (Expr.var "shippingCost") (Expr.var "quantity"))))))]
def caso12 : List Stmt := [(Stmt.assign "value" (Expr.call (Expr.var "functionCall") [(Expr.paren (Expr.binary BinOp.add (Expr.var "arg1") (Expr.var "arg2"))), (Expr.paren (Expr.binary BinOp.mul (Expr.var "arg3") (Expr.var "arg4"))), (Expr.var "defaultValue")]))]
def caso13 : List Stmt := [(Stmt.expr (Expr.binary BinOp.add (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.sub (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.mul (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.div (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.mod (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.pow (Expr.var "a") (Expr.var "b")))]
def caso14 : List Stmt := [(Stmt.expr (Expr.binary BinOp.eq (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.ne (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.lt (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.gt (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.le (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.ge (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.is (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.isNot (Expr.var "a") (Expr.var "b")))]
def caso15 : List Stmt := [(Stmt.expr (Expr.unary UnOp.not (Expr.var "a")))]
def caso16 : List Stmt := [(Stmt.expr (Expr.binary BinOp.isNot (Expr.var "a") (Expr.var "b")))]
def caso17 : List Stmt := [(Stmt.expr (Expr.binary BinOp.isNot (Expr.unary UnOp.not (Expr.var "a")) (Expr.var "b")))]
def caso18 : List Stmt := [(Stmt.expr (Expr.binary BinOp.isNot (Expr.var "a") (Expr.unary UnOp.not (Expr.var "b"))))]
def caso19 : List Stmt := [(Stmt.expr (Expr.binary BinOp.and (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.binary BinOp.or (Expr.var "a") (Expr.var "b"))), (Stmt.expr (Expr.unary UnOp.not (Expr.var "a")))]
def caso20 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default (Expr.var "value") (Expr.var "fallback")))]
def caso21 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default Expr.noneLit (Expr.strLit "x")))]
def caso22 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default (Expr.strLit "y") (Expr.strLit "x")))]
def caso23 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default (Expr.boolLit false) (Expr.boolLit true)))]
def caso24 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default (Expr.intLit 0) (Expr.intLit 10)))]
def caso25 : List Stmt := [(Stmt.expr (Expr.binary BinOp.default (Expr.strLit "") (Expr.strLit "fallback")))]
def caso26 : List Stmt := [(Stmt.expr (Expr.binary BinOp.or (Expr.boolLit false) (Expr.boolLit true)))]
def caso27 : List Stmt := [(Stmt.expr (Expr.binary BinOp.or (Expr.boolLit true) (Expr.boolLit false)))]
def caso28 : List Stmt := [(Stmt.decl Ty.string "username" (some (Expr.binary BinOp.default (Expr.member (Expr.var "userData") "name") (Expr.strLit "Guest")))), (Stmt.decl Ty.integer "count" (some (Expr.binary BinOp.default (Expr.member (Expr.var "config") "maxItems") (Expr.intLit 100)))), (Stmt.decl Ty.number "price" (some (Expr.binary BinOp.default (Expr.member (Expr.var "product") "price") (Expr.numLit 0 (-1)))))]
def caso29 : List Stmt := [(Stmt.decl Ty.string "value" (some (Expr.binary BinOp.default (Expr.binary BinOp.default (Expr.var "primary") (Expr.var "secondary")) (Expr.strLit "final fallback"))))]
def caso30 : List Stmt := [(Stmt.expr (Expr.assertSome (Expr.var "value")))]
def caso31 : List Stmt := [(Stmt.decl (Ty.optional Ty.string) "maybeNone" (some (Expr.call (Expr.var "getUser") [])))]
def caso32 : List Stmt := [(Stmt.decl Ty.string "name" (some (Expr.assertSome (Expr.var "maybeNone"))))]
def caso33 : List Stmt := [(Stmt.decl Ty.string "upper" (some (Expr.call (Expr.member (Expr.assertSome (Expr.call (Expr.var "getText") [])) "toUpperCase") [])))]
def caso34 : List Stmt := [(Stmt.decl Ty.integer "count" (some (Expr.assertSome (Expr.call (Expr.member (Expr.var "items") "find") [(Expr.var "item")]))))]
def caso35 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "v" (some Expr.noneLit)), (Stmt.decl Ty.integer "w" (some (Expr.assertSome (Expr.var "v"))))]
def caso36 : List Stmt := [(Stmt.decl (Ty.list (Ty.list Ty.number)) "product" (some (Expr.binary BinOp.mul (Expr.var "a") (Expr.var "b")))), (Stmt.decl Ty.number "scaled" (some (Expr.binary BinOp.mul (Expr.var "x") (Expr.var "y")))), (Stmt.decl Ty.string "greeting" (some (Expr.binary BinOp.add (Expr.strLit "hola ") (Expr.var "name"))))]
def caso37 : List Stmt := [(Stmt.expr (Expr.call (Expr.member (Expr.var "obj") "method") [])), (Stmt.expr (Expr.member (Expr.var "obj") "property")), (Stmt.expr (Expr.call (Expr.member (Expr.var "obj") "method") [(Expr.var "arg1"), (Expr.var "arg2")])), (Stmt.expr (Expr.call (Expr.member (Expr.strLit "string") "length") [])), (Stmt.expr (Expr.call (Expr.member (Expr.var "myList") "get") [(Expr.intLit 0)]))]
def caso38 : List Stmt := [(Stmt.expr (Expr.call (Expr.var "functionName") [])), (Stmt.expr (Expr.call (Expr.var "functionName") [(Expr.var "arg1")])), (Stmt.expr (Expr.call (Expr.var "functionName") [(Expr.var "arg1"), (Expr.var "arg2"), (Expr.var "arg3")]))]
def caso39 : List Stmt := [(Stmt.expr (Expr.call (Expr.member (Expr.var "math") "sqrt") [(Expr.intLit 16)])), (Stmt.expr (Expr.call (Expr.member (Expr.var "math") "max") [(Expr.intLit 10), (Expr.intLit 20)])), (Stmt.expr (Expr.call (Expr.member (Expr.var "math") "absInteger") [(Expr.unary UnOp.neg (Expr.intLit 5))]))]
def caso40 : List Stmt := [(Stmt.expr (Expr.namespaceCall "list" "concat" [(Expr.var "listA"), (Expr.var "listB")])), (Stmt.expr (Expr.namespaceCall "list" "range" [(Expr.intLit 1), (Expr.intLit 10)])), (Stmt.expr (Expr.namespaceCall "list" "fill" [(Expr.intLit 5), (Expr.intLit 0)]))]
def caso41 : List Stmt := [(Stmt.expr (Expr.namespaceCall "list" "join" [(Expr.var "words"), (Expr.strLit ", ")]))]
def caso42 : List Stmt := [(Stmt.assign "n" (Expr.call (Expr.member (Expr.namespaceCall "list" "concat" [(Expr.var "a"), (Expr.var "b")]) "length") []))]
def caso43 : List Stmt := [(Stmt.assign "b" (Expr.namespaceCall "boolean" "parse" [(Expr.strLit "true")]))]
def caso44 : List Stmt := [(Stmt.expr (Expr.intLit 42))]
def caso45 : List Stmt := [(Stmt.expr (Expr.intLit 255))]
def caso46 : List Stmt := [(Stmt.expr (Expr.intLit 10))]
def caso47 : List Stmt := [(Stmt.expr (Expr.intLit 511))]
def caso48 : List Stmt := [(Stmt.expr (Expr.numLit 314 (-2)))]
def caso49 : List Stmt := [(Stmt.expr (Expr.numLit 5 (-1)))]
def caso50 : List Stmt := [(Stmt.expr (Expr.numLit 602 (21)))]
def caso51 : List Stmt := [(Stmt.expr (Expr.call (Expr.member (Expr.numLit 314 (-2)) "toInteger") []))]
def caso52 : List Stmt := [(Stmt.expr (Expr.call (Expr.member (Expr.intLit 3) "toInteger") []))]
def caso53 : List Stmt := [(Stmt.expr (Expr.strLit "Hello, World!")), (Stmt.expr (Expr.strLit "Line 1\nLine 2")), (Stmt.expr (Expr.strLit ""))]
def caso54 : List Stmt := [(Stmt.expr (Expr.boolLit true)), (Stmt.expr (Expr.boolLit false))]
def caso55 : List Stmt := [(Stmt.expr Expr.noneLit)]
def caso56 : List Stmt := [(Stmt.expr (Expr.binary BinOp.eq Expr.noneLit Expr.noneLit))]
def caso57 : List Stmt := [(Stmt.expr (Expr.binary BinOp.eq Expr.noneLit (Expr.intLit 0)))]
def caso58 : List Stmt := [(Stmt.unsupported "expresi\xf3n bytes"), (Stmt.unsupported "expresi\xf3n bytes"), (Stmt.unsupported "expresi\xf3n bytes")]
def caso59 : List Stmt := [(Stmt.decl Ty.string "text" (some (Expr.strLit "data UserData:\n\tfields:\n\t\tinteger id primary")))]
def caso60 : List Stmt := [(Stmt.expr (Expr.listLit [(Expr.intLit 1), (Expr.intLit 2), (Expr.intLit 3), (Expr.intLit 4)])), (Stmt.expr (Expr.listLit [(Expr.strLit "a"), (Expr.strLit "b"), (Expr.strLit "c")])), (Stmt.expr (Expr.listLit [])), (Stmt.expr (Expr.listLit [(Expr.boolLit true), (Expr.boolLit false), (Expr.boolLit true)]))]
def caso61 : List Stmt := [(Stmt.expr (Expr.listLit [(Expr.listLit [(Expr.intLit 1), (Expr.intLit 2)]), (Expr.listLit [(Expr.intLit 3), (Expr.intLit 4)])])), (Stmt.expr (Expr.listLit [(Expr.listLit [(Expr.intLit 1), (Expr.intLit 2), (Expr.intLit 3)]), (Expr.listLit [(Expr.intLit 4), (Expr.intLit 5), (Expr.intLit 6)]), (Expr.listLit [(Expr.intLit 7), (Expr.intLit 8), (Expr.intLit 9)])])), (Stmt.expr (Expr.listLit [(Expr.listLit [])]))]
def caso62 : List Stmt := [(Stmt.decl (Ty.other "list.line") "queue" (some (Expr.listLit [])))]
def caso63 : List Stmt := [(Stmt.decl (Ty.other "list.line.unique") "seen" (some (Expr.listLit [])))]
def caso64 : List Stmt := [(Stmt.decl Ty.integer "x" (some Expr.noneLit))]
def caso65 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.intLit 1)))]
def caso66 : List Stmt := [(Stmt.decl Ty.integer "num" (some (Expr.intLit 42))), (Stmt.decl Ty.number "numFloat" (some (Expr.call (Expr.member (Expr.var "num") "toNumber") []))), (Stmt.decl Ty.integer "piInt" (some (Expr.call (Expr.member (Expr.numLit 314 (-2)) "toInteger") []))), (Stmt.decl Ty.boolean "flag" (some (Expr.call (Expr.member (Expr.intLit 0) "toBoolean") []))), (Stmt.decl Ty.boolean "nonZero" (some (Expr.call (Expr.member (Expr.intLit 5) "toBoolean") [])))]
def caso67 : List Stmt := [(Stmt.decl Ty.string "a" (some (Expr.strLit "caf\xe9"))), (Stmt.decl Ty.string "b" (some (Expr.strLit "cafe\u0301"))), (Stmt.expr (Expr.binary BinOp.eq (Expr.var "a") (Expr.var "b")))]
def caso68 : List Stmt := [(Stmt.decl Ty.integer "count" (some (Expr.intLit 0))), (Stmt.decl Ty.number "temperature" (some (Expr.numLit 235 (-1)))), (Stmt.decl Ty.boolean "isActive" (some (Expr.boolLit true))), (Stmt.decl Ty.string "name" (some (Expr.strLit "Alice")))]
def caso69 : List Stmt := [(Stmt.decl Ty.integer "value" (some (Expr.binary BinOp.onError (Expr.call (Expr.var "riskyCall") []) (Expr.intLit 0)))), (Stmt.decl Ty.string "name" (some (Expr.binary BinOp.onError (Expr.call (Expr.var "lookupName") [(Expr.var "id")]) (Expr.strLit "unknown"))))]
def caso70 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.binary BinOp.onError (Expr.intLit 1) (Expr.numLit 25 (-1)))))]
def caso71 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 42))), (Stmt.decl Ty.string "name" (some (Expr.strLit "Alice")))]
def caso72 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.strLit "hello")))]
def caso73 : List Stmt := [(Stmt.decl Ty.string "outcome" (some (Expr.binary BinOp.sub (Expr.strLit "hello") (Expr.strLit "world"))))]
def caso74 : List Stmt := [(Stmt.decl Ty.string "outcome" (some (Expr.binary BinOp.add (Expr.strLit "hello") (Expr.strLit "world"))))]
def caso75 : List Stmt := [(Stmt.decl Ty.integer "floor" (some (Expr.unary UnOp.neg (Expr.intLit 9223372036854775808))))]
def caso76 : List Stmt := [(Stmt.decl Ty.integer "n" (some (Expr.intLit 9223372036854775808)))]
def caso77 : List Stmt := [(Stmt.decl Ty.integer "a" (some (Expr.intLit 10))), (Stmt.decl Ty.integer "b" (some (Expr.intLit 0))), (Stmt.decl Ty.integer "q" (some (Expr.binary BinOp.div (Expr.var "a") (Expr.var "b"))))]
def caso78 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.binary BinOp.div (Expr.numLit 10 (-1)) (Expr.numLit 0 (-1)))))]
def caso79 : List Stmt := [(Stmt.decl Ty.integer "m" (some (Expr.unary UnOp.neg (Expr.intLit 9223372036854775808)))), (Stmt.decl Ty.integer "q" (some (Expr.binary BinOp.div (Expr.var "m") (Expr.unary UnOp.neg (Expr.intLit 1)))))]
def caso80 : List Stmt := [(Stmt.decl Ty.integer "m" (some (Expr.unary UnOp.neg (Expr.intLit 9223372036854775808)))), (Stmt.decl Ty.integer "r" (some (Expr.binary BinOp.mod (Expr.var "m") (Expr.unary UnOp.neg (Expr.intLit 1)))))]
def caso81 : List Stmt := [(Stmt.decl Ty.integer "r" (some (Expr.binary BinOp.mod (Expr.intLit 7) (Expr.intLit 0))))]
def caso82 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.binary BinOp.div (Expr.unary UnOp.neg (Expr.numLit 10 (-1))) (Expr.numLit 0 (-1)))))]
def caso83 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.binary BinOp.div (Expr.numLit 0 (-1)) (Expr.numLit 0 (-1)))))]
def caso84 : List Stmt := [(Stmt.unsupported "asignaci\xf3n a miembro o \xedndice")]
def caso85 : List Stmt := [(Stmt.unsupported "asignaci\xf3n a miembro o \xedndice")]
def caso86 : List Stmt := [(Stmt.assign "first" (Expr.index (Expr.var "items") (Expr.intLit 0)))]
def caso87 : List Stmt := [(Stmt.decl (Ty.other "User") "owner" (some (Expr.call (Expr.var "makeUser") [])))]
def caso88 : List Stmt := [(Stmt.ifStmt [((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 0)), [(Stmt.assign "y" (Expr.intLit 1))]), ((Expr.binary BinOp.lt (Expr.var "x") (Expr.intLit 0)), [(Stmt.assign "y" (Expr.intLit 2))])] (some [(Stmt.assign "y" (Expr.intLit 3))])), (Stmt.whileStmt (Expr.binary BinOp.gt (Expr.var "y") (Expr.intLit 0)) [(Stmt.assign "y" (Expr.binary BinOp.sub (Expr.var "y") (Expr.intLit 1)))])]
def caso89 : List Stmt := [(Stmt.decl Ty.string "r" (some (Expr.binary BinOp.sub (Expr.strLit "hello") (Expr.strLit "world"))))]
def caso90 : List Stmt := [(Stmt.decl Ty.string "r" (some (Expr.binary BinOp.add (Expr.strLit "hello") (Expr.strLit "world"))))]
def caso91 : List Stmt := [(Stmt.expr (Expr.binary BinOp.add (Expr.intLit 1) (Expr.binary BinOp.mul (Expr.intLit 2) (Expr.intLit 3))))]
def caso92 : List Stmt := [(Stmt.expr (Expr.binary BinOp.sub (Expr.binary BinOp.sub (Expr.intLit 10) (Expr.intLit 4)) (Expr.intLit 3)))]
def caso93 : List Stmt := [(Stmt.expr (Expr.binary BinOp.pow (Expr.unary UnOp.neg (Expr.intLit 2)) (Expr.intLit 2)))]
def caso94 : List Stmt := [(Stmt.expr (Expr.binary BinOp.eq (Expr.unary UnOp.not (Expr.boolLit true)) (Expr.boolLit false)))]
def caso95 : List Stmt := [(Stmt.expr (Expr.binary BinOp.and (Expr.binary BinOp.lt (Expr.binary BinOp.add (Expr.intLit 1) (Expr.intLit 2)) (Expr.intLit 4)) (Expr.binary BinOp.eq (Expr.binary BinOp.mul (Expr.intLit 2) (Expr.intLit 2)) (Expr.intLit 4))))]
def caso96 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl Ty.integer "b" (some (Expr.binary BinOp.default (Expr.var "a") (Expr.binary BinOp.add (Expr.intLit 1) (Expr.intLit 2)))))]
def caso97 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "v" (some (Expr.intLit 5))), (Stmt.decl Ty.integer "w" (some (Expr.assertSome (Expr.var "v"))))]
def caso98 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl Ty.integer "b" (some (Expr.binary BinOp.default (Expr.var "a") (Expr.intLit 3))))]
def caso99 : List Stmt := [(Stmt.expr (Expr.binary BinOp.add (Expr.numLit 1 (-1)) (Expr.numLit 2 (-1))))]
def caso100 : List Stmt := [(Stmt.expr (Expr.binary BinOp.pow (Expr.numLit 20 (-1)) (Expr.numLit 5 (-1))))]
def caso101 : List Stmt := [(Stmt.expr (Expr.binary BinOp.div (Expr.intLit 6) (Expr.intLit 3)))]
def caso102 : List Stmt := [(Stmt.expr (Expr.binary BinOp.mod (Expr.intLit 7) (Expr.intLit 3)))]
def caso103 : List Stmt := [(Stmt.expr (Expr.binary BinOp.and (Expr.binary BinOp.gt (Expr.intLit 7) (Expr.intLit 3)) (Expr.binary BinOp.le (Expr.intLit 2) (Expr.intLit 2))))]
def caso104 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.is (Expr.var "a") Expr.noneLit)))]
def caso105 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl (Ty.optional Ty.integer) "b" (some (Expr.intLit 1))), (Stmt.decl Ty.boolean "c" (some (Expr.binary BinOp.isNot (Expr.var "a") (Expr.var "b"))))]
def caso106 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl (Ty.optional Ty.integer) "b" (some (Expr.intLit 1))), (Stmt.decl Ty.boolean "c" (some (Expr.unary UnOp.not (Expr.paren (Expr.binary BinOp.is (Expr.var "a") (Expr.var "b"))))))]
def caso107 : List Stmt := [(Stmt.expr (Expr.unary UnOp.not (Expr.intLit 1)))]
def caso108 : List Stmt := [(Stmt.expr (Expr.binary BinOp.and (Expr.intLit 1) (Expr.boolLit true)))]
def caso109 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.onError (Expr.paren (Expr.binary BinOp.div (Expr.intLit 1) (Expr.intLit 0))) (Expr.intLit 7))))]
def caso110 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.onError (Expr.paren (Expr.binary BinOp.div (Expr.intLit 1) (Expr.intLit 0))) (Expr.strLit "siete"))))]
def caso111 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "v" (some Expr.noneLit)), (Stmt.decl Ty.integer "w" (some (Expr.binary BinOp.onError (Expr.assertSome (Expr.var "v")) (Expr.intLit 9))))]
def caso112 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.add (Expr.var "y") (Expr.intLit 1))))]
def caso113 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some (Expr.intLit 5))), (Stmt.decl Ty.integer "b" (some (Expr.var "a")))]
def caso114 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.numLit 314 (-2))))]
def caso115 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.unary UnOp.neg (Expr.intLit 17))))]
def caso116 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.add (Expr.intLit 9223372036854775807) (Expr.intLit 1))))]
def caso117 : List Stmt := [(Stmt.decl Ty.integer "q" (some (Expr.binary BinOp.div (Expr.intLit 7) (Expr.intLit 2))))]
def caso118 : List Stmt := [(Stmt.decl Ty.integer "q" (some (Expr.binary BinOp.div (Expr.unary UnOp.neg (Expr.intLit 7)) (Expr.intLit 2))))]
def caso119 : List Stmt := [(Stmt.decl Ty.integer "r" (some (Expr.binary BinOp.mod (Expr.unary UnOp.neg (Expr.intLit 7)) (Expr.intLit 2))))]
def caso120 : List Stmt := [(Stmt.decl Ty.integer "p" (some (Expr.binary BinOp.pow (Expr.intLit 2) (Expr.unary UnOp.neg (Expr.intLit 1)))))]
def caso121 : List Stmt := [(Stmt.decl Ty.number "n" (some (Expr.binary BinOp.add (Expr.intLit 1) (Expr.numLit 25 (-1)))))]
def caso122 : List Stmt := [(Stmt.decl Ty.integer "a" (some (Expr.intLit 1))), (Stmt.decl Ty.number "b" (some (Expr.numLit 25 (-1)))), (Stmt.decl Ty.number "c" (some (Expr.binary BinOp.add (Expr.var "a") (Expr.var "b"))))]
def caso123 : List Stmt := [(Stmt.decl Ty.number "r" (some (Expr.binary BinOp.mod (Expr.numLit 55 (-1)) (Expr.numLit 20 (-1)))))]
def caso124 : List Stmt := [(Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.lt (Expr.strLit "a") (Expr.strLit "b"))))]
def caso125 : List Stmt := [(Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.eq (Expr.intLit 1) (Expr.strLit "1"))))]
def caso126 : List Stmt := [(Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.is (Expr.intLit 3) (Expr.intLit 3))))]
def caso127 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 5))), (Stmt.decl Ty.integer "y" (some (Expr.assertSome (Expr.var "x"))))]
def caso128 : List Stmt := [(Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.and (Expr.boolLit false) (Expr.paren (Expr.binary BinOp.eq (Expr.binary BinOp.div (Expr.intLit 1) (Expr.intLit 0)) (Expr.intLit 1))))))]
def caso129 : List Stmt := [(Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.or (Expr.boolLit true) (Expr.paren (Expr.binary BinOp.eq (Expr.binary BinOp.div (Expr.intLit 1) (Expr.intLit 0)) (Expr.intLit 1))))))]
def caso130 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.default (Expr.intLit 5) (Expr.paren (Expr.binary BinOp.div (Expr.intLit 1) (Expr.intLit 0))))))]
def caso131 : List Stmt := [(Stmt.decl Ty.number "x" (some (Expr.binary BinOp.add (Expr.intLit 1) (Expr.intLit 2))))]
def caso132 : List Stmt := [(Stmt.assign "x" Expr.noneLit)]
def caso133 : List Stmt := [(Stmt.decl (Ty.optional Ty.integer) "a" (some Expr.noneLit)), (Stmt.decl Ty.string "s" (some (Expr.binary BinOp.default (Expr.var "a") (Expr.strLit "x"))))]
def caso134 : List Stmt := [(Stmt.decl Ty.number "n" (some (Expr.binary BinOp.div (Expr.numLit 0 (-1)) (Expr.numLit 0 (-1))))), (Stmt.decl Ty.boolean "b" (some (Expr.binary BinOp.eq (Expr.var "n") (Expr.var "n"))))]
def caso135 : List Stmt := [(Stmt.decl Ty.integer "m" (some (Expr.unary UnOp.neg (Expr.intLit 9223372036854775808)))), (Stmt.decl Ty.integer "x" (some (Expr.unary UnOp.neg (Expr.var "m"))))]
def caso136 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.mul (Expr.intLit 4611686018427387904) (Expr.intLit 2))))]
def caso137 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.binary BinOp.pow (Expr.intLit 2) (Expr.intLit 63))))]
def caso138 : List Stmt := [(Stmt.decl Ty.number "n" (some (Expr.binary BinOp.add (Expr.numLit 25 (-1)) (Expr.intLit 1))))]
def caso139 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 10))), (Stmt.decl Ty.number "y" (some (Expr.numLit 314 (-2)))), (Stmt.decl Ty.string "z" none), (Stmt.decl Ty.boolean "flag" (some (Expr.boolLit true)))]
def caso140 : List Stmt := [(Stmt.decl Ty.string "z" none), (Stmt.assign "z" (Expr.strLit "a")), (Stmt.returnStmt (some (Expr.var "z")))]
def caso141 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 1))), (Stmt.decl Ty.integer "x" (some (Expr.intLit 2)))]
def caso142 : List Stmt := [(Stmt.assign "x" (Expr.intLit 42)), (Stmt.unsupported "asignaci\xf3n a miembro o \xedndice"), (Stmt.unsupported "asignaci\xf3n a miembro o \xedndice")]
def caso143 : List Stmt := [(Stmt.unsupported "expresi\xf3n interp")]
def caso144 : List Stmt := [(Stmt.printBlock [(Expr.strLit "a"), (Expr.binary BinOp.add (Expr.intLit 1) (Expr.intLit 2)), (Expr.boolLit true)])]
def caso145 : List Stmt := [(Stmt.returnStmt none), (Stmt.returnStmt (some (Expr.var "value"))), (Stmt.returnStmt (some (Expr.var "expression")))]
def caso146 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 5))), (Stmt.returnStmt (some (Expr.binary BinOp.add (Expr.var "x") (Expr.intLit 1)))), (Stmt.assign "x" (Expr.intLit 9))]
def caso147 : List Stmt := [(Stmt.returnStmt none), (Stmt.print (Expr.intLit 1) none)]
def caso148 : List Stmt := [(Stmt.decl Ty.integer "age" (some (Expr.intLit 25))), (Stmt.print (Expr.var "age") none)]
def caso149 : List Stmt := [(Stmt.decl Ty.integer "age" (some (Expr.intLit 25))), (Stmt.print (Expr.binary BinOp.add (Expr.strLit "Age: ") (Expr.call (Expr.member (Expr.var "age") "toString") [])) none)]
def caso150 : List Stmt := [(Stmt.print (Expr.strLit "a") (some (Expr.boolLit false))), (Stmt.print (Expr.strLit "b") none)]
def caso151 : List Stmt := [(Stmt.print (Expr.boolLit true) none), (Stmt.print (Expr.boolLit false) none)]
def caso152 : List Stmt := [(Stmt.print (Expr.intLit 1) (some (Expr.intLit 2)))]
def caso153 : List Stmt := [(Stmt.ifStmt [((Expr.var "condition"), [(Stmt.expr (Expr.var "statements"))])] none)]
def caso154 : List Stmt := [(Stmt.ifStmt [((Expr.var "condition"), [(Stmt.expr (Expr.var "statements"))])] (some [(Stmt.expr (Expr.var "statements"))]))]
def caso155 : List Stmt := [(Stmt.ifStmt [((Expr.var "condition1"), [(Stmt.expr (Expr.var "statements"))]), ((Expr.var "condition2"), [(Stmt.expr (Expr.var "statements"))])] (some [(Stmt.expr (Expr.var "statements"))]))]
def caso156 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 7))), (Stmt.ifStmt [((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 5)), [(Stmt.print (Expr.intLit 1) none)]), ((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 2)), [(Stmt.print (Expr.intLit 2) none)])] (some [(Stmt.print (Expr.intLit 3) none)]))]
def caso157 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 3))), (Stmt.ifStmt [((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 5)), [(Stmt.print (Expr.intLit 1) none)]), ((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 2)), [(Stmt.print (Expr.intLit 2) none)])] (some [(Stmt.print (Expr.intLit 3) none)]))]
def caso158 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 0))), (Stmt.ifStmt [((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 5)), [(Stmt.print (Expr.intLit 1) none)]), ((Expr.binary BinOp.gt (Expr.var "x") (Expr.intLit 2)), [(Stmt.print (Expr.intLit 2) none)])] (some [(Stmt.print (Expr.intLit 3) none)]))]
def caso159 : List Stmt := [(Stmt.ifStmt [((Expr.intLit 1), [(Stmt.print (Expr.intLit 1) none)])] none)]
def caso160 : List Stmt := [(Stmt.ifStmt [((Expr.boolLit true), [(Stmt.decl Ty.integer "y" (some (Expr.intLit 1)))])] none), (Stmt.print (Expr.var "y") none)]
def caso161 : List Stmt := [(Stmt.decl Ty.integer "total" (some (Expr.intLit 0))), (Stmt.iterateRange "n" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.decl Ty.integer "twice" (some (Expr.binary BinOp.mul (Expr.var "n") (Expr.intLit 2)))), (Stmt.assign "total" (Expr.binary BinOp.add (Expr.var "total") (Expr.var "twice")))]), (Stmt.print (Expr.var "total") none), (Stmt.print (Expr.var "twice") none)]
def caso162 : List Stmt := [(Stmt.decl Ty.integer "total" (some (Expr.intLit 0))), (Stmt.iterateRange "n" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.decl Ty.integer "twice" (some (Expr.binary BinOp.mul (Expr.var "n") (Expr.intLit 2)))), (Stmt.assign "total" (Expr.binary BinOp.add (Expr.var "total") (Expr.var "twice")))]), (Stmt.print (Expr.var "total") none)]
def caso163 : List Stmt := [(Stmt.decl Ty.integer "x" (some (Expr.intLit 1))), (Stmt.ifStmt [((Expr.boolLit true), [(Stmt.decl Ty.integer "x" (some (Expr.intLit 2)))])] none)]
def caso164 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 2) none [(Stmt.decl Ty.integer "k" (some (Expr.var "i")))]), (Stmt.print (Expr.var "k") none)]
def caso165 : List Stmt := [(Stmt.iterateElems "item" (Expr.var "items") [(Stmt.print (Expr.var "item") none)])]
def caso166 : List Stmt := [(Stmt.decl (Ty.list Ty.string) "items" (some (Expr.listLit [(Expr.strLit "x"), (Expr.strLit "y")]))), (Stmt.iterateElems "item" (Expr.var "items") [(Stmt.print (Expr.var "item") none)])]
def caso167 : List Stmt := [(Stmt.iterateElems "char" (Expr.strLit "hello") [(Stmt.print (Expr.var "char") none)])]
def caso168 : List Stmt := [(Stmt.iterateElems "name" (Expr.var "source") [(Stmt.expr (Expr.var "body"))]), (Stmt.iterateRange "name" (Expr.var "a") (Expr.var "b") (some (Expr.var "n")) [(Stmt.expr (Expr.var "body"))])]
def caso169 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 10) none [(Stmt.print (Expr.var "i") none)])]
def caso170 : List Stmt := [(Stmt.iterateRange "k" (Expr.intLit 10) (Expr.intLit 1) (some (Expr.unary UnOp.neg (Expr.intLit 2))) [(Stmt.print (Expr.var "k") none)])]
def caso171 : List Stmt := [(Stmt.iterateElems "ch" (Expr.strLit "Clean") [(Stmt.print (Expr.var "ch") none)])]
def caso172 : List Stmt := [(Stmt.iterateElems "row" (Expr.var "grid") [(Stmt.iterateElems "value" (Expr.var "row") [(Stmt.print (Expr.var "value") none)])])]
def caso173 : List Stmt := [(Stmt.decl (Ty.list (Ty.list Ty.integer)) "grid" (some (Expr.listLit [(Expr.listLit [(Expr.intLit 1), (Expr.intLit 2)]), (Expr.listLit [(Expr.intLit 3), (Expr.intLit 4)])]))), (Stmt.iterateElems "row" (Expr.var "grid") [(Stmt.iterateElems "value" (Expr.var "row") [(Stmt.print (Expr.var "value") none)])])]
def caso174 : List Stmt := [(Stmt.iterateRange "idx" (Expr.intLit 0) (Expr.intLit 100) (some (Expr.intLit 5)) [(Stmt.print (Expr.var "idx") none)])]
def caso175 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 0) (Expr.intLit 100) (some (Expr.intLit 3)) [(Stmt.print (Expr.var "i") none)])]
def caso176 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 5) (Expr.intLit 1) none [(Stmt.print (Expr.var "i") none)])]
def caso177 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 4) (Expr.intLit 4) none [(Stmt.print (Expr.var "i") none)])]
def caso178 : List Stmt := [(Stmt.decl Ty.integer "n" (some (Expr.intLit 3))), (Stmt.iterateRange "i" (Expr.intLit 1) (Expr.var "n") none [(Stmt.assign "n" (Expr.intLit 10)), (Stmt.print (Expr.var "i") none)])]
def caso179 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 10) (some (Expr.intLit 0)) [(Stmt.print (Expr.var "i") none)])]
def caso180 : List Stmt := [(Stmt.decl Ty.integer "s" (some (Expr.intLit 0))), (Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 3) (some (Expr.var "s")) [(Stmt.print (Expr.var "i") none)])]
def caso181 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 0) none [(Stmt.print (Expr.var "i") none)])]
def caso182 : List Stmt := [(Stmt.iterateElems "x" (Expr.intLit 5) [(Stmt.print (Expr.var "x") none)])]
def caso183 : List Stmt := [(Stmt.iterateRange "i" (Expr.numLit 10 (-1)) (Expr.numLit 30 (-1)) none [(Stmt.print (Expr.var "i") none)])]
def caso184 : List Stmt := [(Stmt.whileStmt (Expr.var "condition") [(Stmt.expr (Expr.var "body"))])]
def caso185 : List Stmt := [(Stmt.decl Ty.integer "count" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.binary BinOp.lt (Expr.var "count") (Expr.intLit 5)) [(Stmt.print (Expr.call (Expr.member (Expr.var "count") "toString") []) none), (Stmt.assign "count" (Expr.binary BinOp.add (Expr.var "count") (Expr.intLit 1)))])]
def caso186 : List Stmt := [(Stmt.decl Ty.boolean "running" (some (Expr.boolLit true))), (Stmt.decl Ty.integer "iterations" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.var "running") [(Stmt.assign "iterations" (Expr.binary BinOp.add (Expr.var "iterations") (Expr.intLit 1))), (Stmt.ifStmt [((Expr.binary BinOp.ge (Expr.var "iterations") (Expr.intLit 3)), [(Stmt.assign "running" (Expr.boolLit false))])] none)])]
def caso187 : List Stmt := [(Stmt.decl Ty.boolean "running" (some (Expr.boolLit true))), (Stmt.decl Ty.integer "iterations" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.var "running") [(Stmt.assign "iterations" (Expr.binary BinOp.add (Expr.var "iterations") (Expr.intLit 1))), (Stmt.ifStmt [((Expr.binary BinOp.ge (Expr.var "iterations") (Expr.intLit 3)), [(Stmt.assign "running" (Expr.boolLit false))])] none)]), (Stmt.print (Expr.var "iterations") none)]
def caso188 : List Stmt := [(Stmt.decl Ty.integer "outer" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.binary BinOp.lt (Expr.var "outer") (Expr.intLit 3)) [(Stmt.decl Ty.integer "inner" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.binary BinOp.lt (Expr.var "inner") (Expr.intLit 2)) [(Stmt.print (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.binary BinOp.add (Expr.strLit "outer: ") (Expr.call (Expr.member (Expr.var "outer") "toString") [])) (Expr.strLit ", inner: ")) (Expr.call (Expr.member (Expr.var "inner") "toString") [])) none), (Stmt.assign "inner" (Expr.binary BinOp.add (Expr.var "inner") (Expr.intLit 1)))]), (Stmt.assign "outer" (Expr.binary BinOp.add (Expr.var "outer") (Expr.intLit 1)))])]
def caso189 : List Stmt := [(Stmt.decl Ty.integer "i" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.binary BinOp.lt (Expr.var "i") (Expr.intLit 10)) [(Stmt.decl Ty.integer "remainder" (some (Expr.binary BinOp.mod (Expr.var "i") (Expr.intLit 2)))), (Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "remainder") (Expr.intLit 0)), [(Stmt.print (Expr.binary BinOp.add (Expr.strLit "Even: ") (Expr.call (Expr.member (Expr.var "i") "toString") [])) none)])] (some [(Stmt.print (Expr.binary BinOp.add (Expr.strLit "Odd: ") (Expr.call (Expr.member (Expr.var "i") "toString") [])) none)])), (Stmt.assign "i" (Expr.binary BinOp.add (Expr.var "i") (Expr.intLit 1)))])]
def caso190 : List Stmt := [(Stmt.whileStmt (Expr.intLit 1) [Stmt.breakStmt])]
def caso191 : List Stmt := [(Stmt.decl Ty.integer "n" (some (Expr.intLit 0))), (Stmt.whileStmt (Expr.binary BinOp.lt (Expr.var "n") (Expr.intLit 3)) [(Stmt.assign "n" (Expr.binary BinOp.add (Expr.var "n") (Expr.intLit 1)))]), (Stmt.print (Expr.var "n") none)]
def caso192 : List Stmt := [(Stmt.iterateElems "item" (Expr.var "items") [(Stmt.ifStmt [((Expr.call (Expr.member (Expr.var "item") "isEmpty") []), [Stmt.continueStmt])] none), (Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "item") (Expr.strLit "stop")), [Stmt.breakStmt])] none), (Stmt.print (Expr.var "item") none)])]
def caso193 : List Stmt := [(Stmt.decl (Ty.list Ty.string) "items" (some (Expr.listLit [(Expr.strLit "a"), (Expr.strLit ""), (Expr.strLit "b"), (Expr.strLit "stop"), (Expr.strLit "c")]))), (Stmt.iterateElems "item" (Expr.var "items") [(Stmt.ifStmt [((Expr.call (Expr.member (Expr.var "item") "isEmpty") []), [Stmt.continueStmt])] none), (Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "item") (Expr.strLit "stop")), [Stmt.breakStmt])] none), (Stmt.print (Expr.var "item") none)])]
def caso194 : List Stmt := [Stmt.breakStmt]
def caso195 : List Stmt := [(Stmt.ifStmt [((Expr.boolLit true), [Stmt.continueStmt])] none)]
def caso196 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 10) (some (Expr.intLit 3)) [(Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "i") (Expr.intLit 4)), [Stmt.continueStmt])] none), (Stmt.print (Expr.var "i") none)])]
def caso197 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.iterateRange "j" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "j") (Expr.intLit 2)), [Stmt.continueStmt])] none), (Stmt.print (Expr.binary BinOp.add (Expr.binary BinOp.mul (Expr.var "i") (Expr.intLit 10)) (Expr.var "j")) none)])])]
def caso198 : List Stmt := [(Stmt.iterateRange "i" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.iterateRange "j" (Expr.intLit 1) (Expr.intLit 3) none [(Stmt.ifStmt [((Expr.binary BinOp.eq (Expr.var "j") (Expr.intLit 2)), [Stmt.breakStmt])] none), (Stmt.print (Expr.binary BinOp.add (Expr.binary BinOp.mul (Expr.var "i") (Expr.intLit 10)) (Expr.var "j")) none)])])]

def casos : List (String × List Stmt) := [
  ("EXP-01.p1", caso0),
  ("EXP-01.p2", caso1),
  ("EXP-01.p3", caso2),
  ("EXP-01.p5", caso3),
  ("EXP-02.s1", caso4),
  ("EXP-02.s2", caso5),
  ("EXP-02.s3", caso6),
  ("EXP-02.s4", caso7),
  ("EXP-02.s5", caso8),
  ("EXP-02.e1", caso9),
  ("EXP-02.e2", caso10),
  ("EXP-02.e3", caso11),
  ("EXP-02.e4", caso12),
  ("EXP-04.n1", caso13),
  ("EXP-05.n1", caso14),
  ("EXP-06.n1", caso15),
  ("EXP-06.n2", caso16),
  ("EXP-06.n3", caso17),
  ("EXP-06.n4", caso18),
  ("EXP-07.n1", caso19),
  ("EXP-03.a1", caso20),
  ("EXP-03.b1", caso21),
  ("EXP-03.b2", caso22),
  ("EXP-03.b3", caso23),
  ("EXP-03.b4", caso24),
  ("EXP-03.b5", caso25),
  ("EXP-03.b6", caso26),
  ("EXP-03.b7", caso27),
  ("EXP-03.c1", caso28),
  ("EXP-03.c2", caso29),
  ("EXP-03.e1", caso30),
  ("EXP-03.f1", caso31),
  ("EXP-03.f2", caso32),
  ("EXP-03.f3", caso33),
  ("EXP-03.f4", caso34),
  ("EXP-03.p1", caso35),
  ("EXP-08.n1", caso36),
  ("EXP-09.n1", caso37),
  ("EXP-10.n1", caso38),
  ("CALL-03.e1", caso39),
  ("CALL-03.e2", caso40),
  ("CALL-03.e4", caso41),
  ("LEX-04.s3", caso42),
  ("LEX-04.s4", caso43),
  ("LEX-06.i1", caso44),
  ("LEX-06.i2", caso45),
  ("LEX-06.i3", caso46),
  ("LEX-06.i4", caso47),
  ("LEX-06.f1", caso48),
  ("LEX-06.f2", caso49),
  ("LEX-06.f3", caso50),
  ("LEX-06.p1", caso51),
  ("LEX-06.p2", caso52),
  ("LEX-06.s1", caso53),
  ("LEX-06.b1", caso54),
  ("LEX-06.n1", caso55),
  ("LEX-06.n2", caso56),
  ("LEX-06.n3", caso57),
  ("LEX-06.y1", caso58),
  ("LEX-06.t1", caso59),
  ("LEX-06.l1", caso60),
  ("LEX-06.m1", caso61),
  ("TYP-05.p1", caso62),
  ("TYP-05.p2", caso63),
  ("TYP-03.p1", caso64),
  ("TYP-06.p1", caso65),
  ("TYP-06.e1", caso66),
  ("TYP-07.e1", caso67),
  ("TYP-11.e1", caso68),
  ("ERH-02.e1", caso69),
  ("ERH-02.p1", caso70),
  ("RUL-16.e1", caso71),
  ("RUL-16.e2", caso72),
  ("RUL-19.e1", caso73),
  ("RUL-19.e2", caso74),
  ("RUL-41.p1", caso75),
  ("RUL-41.p2", caso76),
  ("RUL-136.e1", caso77),
  ("RUL-136.e2", caso78),
  ("RUL-136.p1", caso79),
  ("RUL-136.p2", caso80),
  ("RUL-136.p3", caso81),
  ("STD-14.p1", caso82),
  ("STD-14.p2", caso83),
  ("STM-02.s1", caso84),
  ("STM-02.s2", caso85),
  ("EXPG-01.s1", caso86),
  ("TYP-02.s1", caso87),
  ("LEX-01.s1", caso88),
  ("RUL-19.s1", caso89),
  ("RUL-19.s2", caso90),
  ("EXP-01.s1", caso91),
  ("EXP-01.s2", caso92),
  ("EXP-01.s3", caso93),
  ("EXP-01.s4", caso94),
  ("EXP-01.s5", caso95),
  ("EXP-01.s6", caso96),
  ("EXP-03.s1", caso97),
  ("EXP-03.s2", caso98),
  ("EXP-04.s1", caso99),
  ("EXP-04.s2", caso100),
  ("EXP-04.s3", caso101),
  ("EXP-04.s4", caso102),
  ("EXP-05.s1", caso103),
  ("EXP-05.s2", caso104),
  ("EXP-06.s1", caso105),
  ("EXP-06.s2", caso106),
  ("EXP-07.s1", caso107),
  ("EXP-07.s2", caso108),
  ("ERH-02.s1", caso109),
  ("ERH-02.s2", caso110),
  ("ERH-02.s3", caso111),
  ("RUL-17.s1", caso112),
  ("TYP-03.s1", caso113),
  ("TYP-06.s1", caso114),
  ("TYP-06.s2", caso115),
  ("P-01", caso116),
  ("P-02", caso117),
  ("P-02b", caso118),
  ("P-03", caso119),
  ("P-04", caso120),
  ("P-05", caso121),
  ("P-06", caso122),
  ("P-07", caso123),
  ("P-08", caso124),
  ("P-09", caso125),
  ("P-10", caso126),
  ("P-11", caso127),
  ("P-12", caso128),
  ("P-13", caso129),
  ("P-14", caso130),
  ("P-15", caso131),
  ("P-16", caso132),
  ("P-17", caso133),
  ("P-18", caso134),
  ("P-19", caso135),
  ("P-20", caso136),
  ("P-21", caso137),
  ("P-22", caso138),
  ("STM-01.e1", caso139),
  ("STM-01.s1", caso140),
  ("STM-01.s2", caso141),
  ("STM-02.e1", caso142),
  ("STM-04.e1", caso143),
  ("STM-04.s1", caso144),
  ("STM-03.e1", caso145),
  ("STM-03.s1", caso146),
  ("STM-03.s2", caso147),
  ("STD-04.p1", caso148),
  ("STD-04.p2", caso149),
  ("STD-04.p3", caso150),
  ("STD-04.s1", caso151),
  ("STD-04.s2", caso152),
  ("FLW-01.e1", caso153),
  ("FLW-01.e2", caso154),
  ("FLW-01.e3", caso155),
  ("FLW-01.s1", caso156),
  ("FLW-01.s2", caso157),
  ("FLW-01.s3", caso158),
  ("FLW-01.s4", caso159),
  ("FLW-01.s5", caso160),
  ("STM-05.e1", caso161),
  ("STM-05.p1", caso162),
  ("STM-05.p2", caso163),
  ("STM-05.p3", caso164),
  ("FLW-02.e1", caso165),
  ("FLW-02.e1b", caso166),
  ("FLW-02.e2", caso167),
  ("FLW-02.r0", caso168),
  ("FLW-02.r1", caso169),
  ("FLW-02.r2", caso170),
  ("FLW-02.r3", caso171),
  ("FLW-02.r4", caso172),
  ("FLW-02.r4b", caso173),
  ("FLW-02.r5", caso174),
  ("FLW-02.n1", caso175),
  ("FLW-02.n2", caso176),
  ("FLW-02.n3", caso177),
  ("FLW-02.n4", caso178),
  ("FLW-02.n6", caso179),
  ("FLW-02.n7", caso180),
  ("FLW-02.n8", caso181),
  ("FLW-02.n9", caso182),
  ("FLW-02.n10", caso183),
  ("FLW-02.w0", caso184),
  ("FLW-02.w1", caso185),
  ("FLW-02.w2", caso186),
  ("FLW-02.w2b", caso187),
  ("FLW-02.w3", caso188),
  ("FLW-02.w4", caso189),
  ("FLW-02.w5", caso190),
  ("FLW-02.w6", caso191),
  ("FLW-03.e1", caso192),
  ("FLW-03.e1b", caso193),
  ("FLW-03.p1", caso194),
  ("FLW-03.p2", caso195),
  ("FLW-03.p3", caso196),
  ("FLW-03.p4", caso197),
  ("FLW-03.p5", caso198)
]

def main : IO Unit := do
  for (id, p) in casos do
    IO.println s!"{id}\t{textoDe (correr p)}"
