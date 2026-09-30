"""Del árbol de Lark al AST de Clean, y del AST al término de Lean.

El AST es una tupla por nodo. Es la misma forma que Clean/Ast.lean
declara como tipos inductivos: este módulo es el puente entre los dos.
"""
from __future__ import annotations

from lark import Tree


class FueraDelPuente(Exception):
    """El árbol tiene una forma que el puente no sabe traducir."""


# ------------------------------------------------------------------
# Lark -> AST
# ------------------------------------------------------------------

_BINARIOS = {
    "op_eq": "eq", "op_ne": "ne", "op_is": "is", "op_is_not": "isNot",
    "op_lt": "lt", "op_gt": "gt", "op_le": "le", "op_ge": "ge",
    "op_add": "add", "op_sub": "sub",
    "op_mul": "mul", "op_div": "div", "op_mod": "mod",
}
_UNARIOS = {"op_not": "not", "op_neg": "neg"}


def _desescapar(cuerpo: str) -> str:
    out = []
    i = 0
    simples = {'"': '"', "\\": "\\", "n": "\n", "t": "\t", "r": "\r",
               "{": "{", "}": "}", "0": "\0"}
    while i < len(cuerpo):
        c = cuerpo[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        sig = cuerpo[i + 1]
        if sig == "u":
            out.append(chr(int(cuerpo[i + 2:i + 8], 16)))
            i += 8
        else:
            out.append(simples[sig])
            i += 2
    return "".join(out)


def _numero(texto: str) -> tuple:
    mantisa, _, exp = texto.lower().partition("e")
    entera, _, fraccion = mantisa.partition(".")
    digitos = (entera + fraccion) or "0"
    exp10 = (int(exp) if exp else 0) - len(fraccion)
    return ("num", int(digitos), exp10)


def _plegar_izquierda(op: str, hijos: list) -> tuple:
    nodo = hijos[0]
    for h in hijos[1:]:
        nodo = ("bin", op, nodo, h)
    return nodo


def _plegar_con_operadores(hijos: list) -> tuple:
    nodo = expr(hijos[0])
    for i in range(1, len(hijos), 2):
        op = _BINARIOS[hijos[i].data]
        nodo = ("bin", op, nodo, expr(hijos[i + 1]))
    return nodo


def expr(t) -> tuple:
    d = t.data
    h = t.children
    if d == "expression":
        return expr(h[0])
    if d == "on_error_expression":
        return _plegar_izquierda("onError", [expr(x) for x in h])
    if d == "default_expression":
        return _plegar_izquierda("default", [expr(x) for x in h])
    if d == "or_expression":
        return _plegar_izquierda("or", [expr(x) for x in h])
    if d == "and_expression":
        return _plegar_izquierda("and", [expr(x) for x in h])
    if d in ("equality_expression", "comparison_expression",
             "additive_expression", "multiplicative_expression"):
        return _plegar_con_operadores(h)
    if d == "exponentiation_expression":
        if len(h) == 1:
            return expr(h[0])
        return ("bin", "pow", expr(h[0]), expr(h[1]))
    if d == "unary_expression":
        nodo = expr(h[-1])
        for op in reversed(h[:-1]):
            nodo = ("un", _UNARIOS[op.data], nodo)
        return nodo
    if d == "postfix_expression":
        return expr(h[0])
    if d == "op_assert":
        return ("assert", expr(h[0]))
    if d == "member_access":
        return ("member", expr(h[0]), str(h[1]))
    if d == "call":
        args = ()
        if len(h) > 1:
            args = tuple(expr(a) for a in h[1].children)
        return ("call", expr(h[0]), args)
    if d == "index_access":
        return ("index", expr(h[0]), expr(h[1]))
    if d in ("primary_expression", "literal", "interpolation"):
        return expr(h[0])
    if d == "namespace_call":
        espacio = str(h[0].children[0])
        args = ()
        if len(h) > 2:
            args = tuple(expr(a) for a in h[2].children)
        return ("nscall", espacio, str(h[1]), args)
    if d == "identifier":
        return ("var", str(h[0]))
    if d == "parenthesized":
        return ("paren", expr(h[0]))
    if d == "hex_literal":
        return ("int", int(str(h[0])[2:], 16))
    if d == "binary_literal":
        return ("int", int(str(h[0])[2:], 2))
    if d == "octal_literal":
        return ("int", int(str(h[0])[2:], 8))
    if d == "decimal_literal":
        return ("int", int(str(h[0])))
    if d == "number_literal":
        return _numero(str(h[0]))
    if d == "string_literal":
        if h[0].type == "MULTI_LINE_STRING":
            return ("str", _multilinea(str(h[0])))
        return ("str", _desescapar(str(h[0])[1:-1]))
    if d == "interpolated_string":
        # EXPG-03: las piezas de texto y las interpolaciones, en orden.
        partes = []
        for x in h:
            if isinstance(x, Tree):
                partes.append(expr(x))
            else:
                partes.append(("str", _desescapar(str(x)[1:-1])))
        return ("interp", tuple(partes))
    if d == "bytes_literal":
        return ("bytes", str(h[0])[2:-1])
    if d == "error_value":
        return ("var", "error")
    if d == "true_literal":
        return ("bool", True)
    if d == "false_literal":
        return ("bool", False)
    if d == "none_literal":
        return ("none",)
    if d == "list_literal":
        return ("list", tuple(expr(x) for x in h))
    raise FueraDelPuente(d)


def _multilinea(texto: str) -> str:
    """LEX-06: el contenido de un \"\"\" sin interpretar, sin los saltos
    de apertura y cierre, con el margen del cierre quitado."""
    cuerpo = texto[3:-3]
    cuerpo = cuerpo.split("\n", 1)[1]
    lineas = cuerpo.split("\n")
    margen = lineas[-1]
    return "\n".join(l[len(margen):] if l.startswith(margen) else l
                     for l in lineas[:-1])


def tipo(t) -> tuple:
    d = t.data
    h = t.children
    if d == "list_behavior_type":
        # TYP-05: list<T> con su cadena de comportamientos.
        args = tuple(tipo(a) for a in h[1].children)
        nombres = tuple(str(x.children[0]) for x in h[2:])
        return ("behav", ("generic", str(h[0]), args), nombres)
    if d == "type_expression":
        base = tipo(h[0])
        if len(h) > 1:
            return ("opt", base)
        return base
    if d == "primitive_type":
        return ("prim", str(h[0]))
    if d == "generic_type":
        args = tuple(tipo(a) for a in h[1].children)
        return ("generic", str(h[0]), args)
    if d == "class_type":
        return ("class", str(h[0]))
    raise FueraDelPuente(d)


def _destino(t) -> tuple:
    d = t.data
    h = t.children
    if d == "simple_target":
        return ("var", str(h[0]))
    if d == "member_target":
        return ("member", expr(h[0]), str(h[1]))
    if d == "index_target":
        return ("index", expr(h[0]), expr(h[1]))
    raise FueraDelPuente(d)


def sentencia(t) -> tuple:
    d = t.data
    h = t.children
    if d in ("statement", "simple_statement", "compound_statement",
             "control_flow_statement"):
        return sentencia(h[0])
    if d == "variable_declaration" and h[0].data == "explicit_declaration":
        return sentencia(h[0])
    if d in ("explicit_declaration", "variable_declaration"):
        inicial = expr(h[2]) if len(h) > 2 else None
        return ("decl", tipo(h[0]), str(h[1]), inicial)
    if d == "assignment":
        return ("assign", _destino(h[0]), expr(h[1]))
    if d == "expression_statement":
        return ("expr", expr(h[0]))
    if d == "if_statement":
        # FLWG-01: la cadena es una lista de (condición, cuerpo) y un
        # `else` opcional.
        ramas = [(expr(h[0]), bloque(h[1]))]
        otro = None
        for x in h[2:]:
            if x.data == "else_if_clause":
                ramas.append((expr(x.children[0]), bloque(x.children[1])))
            else:
                otro = bloque(x.children[0])
        return ("if", tuple(ramas), otro)
    if d == "while_statement":
        return ("while", expr(h[0]), bloque(h[1]))
    if d == "iterate_statement":
        nombre = str(h[0])
        fuente = h[1]
        cuerpo = bloque(h[2])
        if fuente.data == "range_source":
            # FLWG-02: range_expression (a to b) y el step opcional.
            desde, hasta = fuente.children[0].children
            fh = fuente.children
            paso = expr(fh[1].children[0]) if len(fh) > 1 else None
            return ("iterateRange", nombre, expr(desde), expr(hasta), paso,
                    cuerpo)
        return ("iterate", nombre, expr(fuente.children[0]), cuerpo)
    if d == "break_statement":
        return ("break",)
    if d == "continue_statement":
        return ("continue",)
    if d == "return_statement":
        return ("return", expr(h[0]) if h else None)
    if d == "print_statement":
        # STD-04: `print(value)` o `print(value, newline: false)`.
        salto = expr(h[2]) if len(h) > 2 else None
        return ("print", expr(h[0]), salto)
    if d == "print_block":
        return ("printBlock", tuple(expr(x.children[0]) for x in h))
    raise FueraDelPuente(d)


def bloque(t) -> tuple:
    return tuple(sentencia(x) for x in t.children)


def programa(t: Tree) -> tuple:
    return tuple(sentencia(x) for x in t.children)


def sin_parentesis(n):
    """El mismo AST sin los nodos de paréntesis, para comparar agrupación."""
    if isinstance(n, tuple):
        if n and n[0] == "paren":
            return sin_parentesis(n[1])
        return tuple(sin_parentesis(x) for x in n)
    return n


# ------------------------------------------------------------------
# AST -> término de Lean
# ------------------------------------------------------------------

def _cadena_lean(s: str) -> str:
    out = ['"']
    for c in s:
        cp = ord(c)
        if c == '"':
            out.append('\\"')
        elif c == "\\":
            out.append("\\\\")
        elif c == "\n":
            out.append("\\n")
        elif c == "\t":
            out.append("\\t")
        elif c == "\r":
            out.append("\\r")
        elif 0x20 <= cp < 0x7F:
            out.append(c)
        elif cp < 0x100:
            out.append(f"\\x{cp:02x}")
        elif cp <= 0xFFFF:
            out.append(f"\\u{cp:04x}")
        else:
            out.append(c)
    out.append('"')
    return "".join(out)


def _lista(elems) -> str:
    return "[" + ", ".join(elems) + "]"


def tipo_lean(t) -> str:
    k = t[0]
    if k == "prim":
        if t[1] in ("integer", "number", "boolean", "string"):
            return f"Ty.{t[1]}"
        return f"(Ty.other {_cadena_lean(t[1])})"
    if k == "opt":
        return f"(Ty.optional {tipo_lean(t[1])})"
    if k == "generic":
        # TYP-02; D-04 H-03: una matrix<T> es una list<list<T>>.
        if t[1] == "list" and len(t[2]) == 1:
            return f"(Ty.list {tipo_lean(t[2][0])})"
        if t[1] == "matrix" and len(t[2]) == 1:
            return f"(Ty.list (Ty.list {tipo_lean(t[2][0])}))"
        return f"(Ty.other {_cadena_lean(t[1])})"
    if k == "class":
        return f"(Ty.other {_cadena_lean(t[1])})"
    if k == "behav":
        return f"(Ty.other {_cadena_lean('list.' + '.'.join(t[2]))})"
    raise FueraDelPuente(k)


def expr_lean(n) -> str:
    k = n[0]
    if k == "int":
        return f"(Expr.intLit {n[1]})"
    if k == "num":
        return f"(Expr.numLit {n[1]} ({n[2]}))"
    if k == "str":
        return f"(Expr.strLit {_cadena_lean(n[1])})"
    if k == "bool":
        return f"(Expr.boolLit {'true' if n[1] else 'false'})"
    if k == "none":
        return "Expr.noneLit"
    if k == "var":
        return f"(Expr.var {_cadena_lean(n[1])})"
    if k == "un":
        return f"(Expr.unary UnOp.{n[1]} {expr_lean(n[2])})"
    if k == "bin":
        return (f"(Expr.binary BinOp.{n[1]} {expr_lean(n[2])} "
                f"{expr_lean(n[3])})")
    if k == "assert":
        return f"(Expr.assertSome {expr_lean(n[1])})"
    if k == "paren":
        return f"(Expr.paren {expr_lean(n[1])})"
    if k == "member":
        return f"(Expr.member {expr_lean(n[1])} {_cadena_lean(n[2])})"
    if k == "call":
        return (f"(Expr.call {expr_lean(n[1])} "
                f"{_lista(expr_lean(a) for a in n[2])})")
    if k == "nscall":
        return (f"(Expr.namespaceCall {_cadena_lean(n[1])} "
                f"{_cadena_lean(n[2])} "
                f"{_lista(expr_lean(a) for a in n[3])})")
    if k == "index":
        return f"(Expr.index {expr_lean(n[1])} {expr_lean(n[2])})"
    if k == "list":
        return f"(Expr.listLit {_lista(expr_lean(a) for a in n[1])})"
    raise FueraDelPuente(k)


def _opcional(e) -> str:
    return "none" if e is None else f"(some {expr_lean(e)})"


def bloque_lean(b) -> str:
    return _lista(sentencia_lean(s) for s in b)


def sentencia_lean(s) -> str:
    """Una sentencia cuya expresión el modelo de Lean no tiene todavía
    (una cadena interpolada, un literal bytes) se entrega como no
    soportada, igual que las sentencias que el modelo no cubre."""
    try:
        return _sentencia_lean(s)
    except FueraDelPuente as e:
        return f"(Stmt.unsupported {_cadena_lean('expresión ' + str(e))})"


def _sentencia_lean(s) -> str:
    k = s[0]
    if k == "decl":
        return (f"(Stmt.decl {tipo_lean(s[1])} {_cadena_lean(s[2])} "
                f"{_opcional(s[3])})")
    if k == "assign":
        if s[1][0] != "var":
            return (f"(Stmt.unsupported "
                    f"{_cadena_lean('asignación a miembro o índice')})")
        return f"(Stmt.assign {_cadena_lean(s[1][1])} {expr_lean(s[2])})"
    if k == "expr":
        return f"(Stmt.expr {expr_lean(s[1])})"
    if k == "if":
        ramas = _lista(f"({expr_lean(c)}, {bloque_lean(b)})" for c, b in s[1])
        otro = "none" if s[2] is None else f"(some {bloque_lean(s[2])})"
        return f"(Stmt.ifStmt {ramas} {otro})"
    if k == "while":
        return f"(Stmt.whileStmt {expr_lean(s[1])} {bloque_lean(s[2])})"
    if k == "iterate":
        return (f"(Stmt.iterateElems {_cadena_lean(s[1])} {expr_lean(s[2])} "
                f"{bloque_lean(s[3])})")
    if k == "iterateRange":
        return (f"(Stmt.iterateRange {_cadena_lean(s[1])} {expr_lean(s[2])} "
                f"{expr_lean(s[3])} {_opcional(s[4])} {bloque_lean(s[5])})")
    if k == "break":
        return "Stmt.breakStmt"
    if k == "continue":
        return "Stmt.continueStmt"
    if k == "return":
        return f"(Stmt.returnStmt {_opcional(s[1])})"
    if k == "print":
        return f"(Stmt.print {expr_lean(s[1])} {_opcional(s[2])})"
    if k == "printBlock":
        return f"(Stmt.printBlock {_lista(expr_lean(e) for e in s[1])})"
    return f"(Stmt.unsupported {_cadena_lean('sentencia ' + k)})"


def programa_lean(p) -> str:
    return _lista(sentencia_lean(s) for s in p)
