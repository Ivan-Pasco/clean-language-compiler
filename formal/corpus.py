"""Corpus del piloto: los ejemplos de la ley con la marca que se espera.

origen:
  ley-ejemplo    el código y la marca están los dos en la ley
  ley-sin-marca  el código está en la ley; la marca la propone Claude
  ley-prosa      la ley lo afirma en prosa; el código se arma de esa frase
  sonda          código de Claude, para ejercer una regla o preguntar

Un caso tomado de la ley no copia el código: nombra archivo y líneas, y
el arnés lo lee de allí.

decidido: el caso discrepa de la ley por un defecto que el Owner ya
decidió (ver decisiones.md) y que todavía no se aterrizó.
"""

E = "language-principles/06-expressions.md"
S = "language-principles/07-statements.md"
F = "language-principles/12-control-flow.md"
L = "language-principles/03-lexical-structure.md"
T = "language-principles/04-type-system.md"
H = "language-principles/13-error-handling.md"
M = "language-principles/16-method-style-syntax.md"
N = "language-principles/09-functions.md"
R = "interoperability-principles/platform/10-semantic-rules.md"


def ley(id, unidad, archivo, desde, hasta, origen, sintaxis="valida",
        **mas):
    return dict(id=id, unidad=unidad, archivo=archivo,
                lineas=(desde, hasta), origen=origen, sintaxis=sintaxis,
                **mas)


def texto(id, unidad, codigo, origen, sintaxis="valida", **mas):
    return dict(id=id, unidad=unidad, codigo=codigo, origen=origen,
                sintaxis=sintaxis, **mas)


def valor(tipo, v):
    return dict(clase="valor", tipo=tipo, valor=v)


def fallo(codigo, mensaje=None):
    return dict(clase="fallo", codigo=codigo, mensaje=mensaje)


def estatico(codigo):
    return dict(clase="estatico", codigo=codigo)


def pregunta(que):
    """La ley no lo decide: el caso existe para preguntarlo."""
    return dict(clase="pregunta", que=que)


def sinvalor():
    return dict(clase="sin-valor")


# Los capítulos cuyas líneas de código el arnés exige cubiertas.
CAPITULOS = (E, S, F)

# Los companions de gramática de los capítulos de los que el corpus toma
# casos (L, T, E, S, F): cada regla y cada alias que definen debe quedar
# ejercido.  Las reglas de los demás companions se informan sin fallar.
GRAMATICAS = ("03-lexical-structure", "04-type-system", "06-expressions",
              "07-statements", "12-control-flow")


CASOS = [
    # ---- EXP-01 (Operator precedence and associativity) -------------
    texto("EXP-01.p1", "EXP-01", "2^3^2", "ley-prosa",
          agrupa_como="2^(3^2)", semantica=valor("integer", "512"),
          afirma="06-expressions.md:30"),
    texto("EXP-01.p2", "EXP-01", "(2^3)^2", "ley-prosa",
          semantica=valor("integer", "64"),
          afirma="06-expressions.md:30"),
    texto("EXP-01.p3", "EXP-01", "a - b - c", "ley-prosa",
          agrupa_como="(a - b) - c", afirma="06-expressions.md:41"),
    texto("EXP-01.p4", "EXP-01", "a = b = c", "ley-prosa",
          sintaxis="invalida", afirma="06-expressions.md:41"),
    texto("EXP-01.p5", "EXP-01", "x = f() onError 0", "ley-prosa",
          agrupa_como="x = (f() onError 0)",
          afirma="06-expressions.md:41"),

    # ---- EXP-02 (A multi-line expression is parenthesized) ----------
    ley("EXP-02.s1", "EXP-02", E, 65, 66, "ley-sin-marca"),
    ley("EXP-02.s2", "EXP-02", E, 67, 67, "ley-sin-marca"),
    ley("EXP-02.s3", "EXP-02", E, 69, 71, "ley-sin-marca"),
    ley("EXP-02.s4", "EXP-02", E, 73, 75, "ley-sin-marca"),
    ley("EXP-02.s5", "EXP-02", E, 77, 79, "ley-sin-marca"),
    ley("EXP-02.e1", "EXP-02", E, 92, 93, "ley-ejemplo"),
    ley("EXP-02.e2", "EXP-02", E, 95, 97, "ley-ejemplo"),
    ley("EXP-02.e3", "EXP-02", E, 99, 102, "ley-ejemplo"),
    ley("EXP-02.e4", "EXP-02", E, 104, 109, "ley-ejemplo"),
    ley("EXP-02.e5", "EXP-02", E, 111, 113, "ley-ejemplo",
        sintaxis="invalida"),
    ley("EXP-02.e6", "EXP-02", E, 115, 116, "ley-ejemplo",
        sintaxis="invalida", codigo_sintaxis="SYN004"),
    texto("EXP-02.p1", "EXP-02", "total = a + b)", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN002",
          afirma="06-expressions.md:87"),
    texto("EXP-02.p2", "EXP-02", "total = (a + b", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN004",
          afirma="06-expressions.md:87"),

    # ---- EXP-04 (Arithmetic operators) ------------------------------
    ley("EXP-04.n1", "EXP-04", E, 144, 149, "ley-sin-marca"),

    # ---- EXP-05 (Comparison operators) ------------------------------
    ley("EXP-05.n1", "EXP-05", E, 179, 186, "ley-sin-marca"),

    # ---- EXP-06 (`not` in two positions) ----------------------------
    ley("EXP-06.n1", "EXP-06", E, 224, 224, "ley-sin-marca"),
    ley("EXP-06.n2", "EXP-06", E, 225, 225, "ley-sin-marca"),
    ley("EXP-06.n3", "EXP-06", E, 226, 226, "ley-ejemplo",
        agrupa_como="(not a) not b"),
    ley("EXP-06.n4", "EXP-06", E, 227, 227, "ley-ejemplo",
        agrupa_como="a not (not b)"),

    # ---- EXP-07 (Logical operators) ---------------------------------
    ley("EXP-07.n1", "EXP-07", E, 253, 255, "ley-sin-marca"),

    # ---- EXP-03 (The none-handling operators `default` and `!`) -----
    ley("EXP-03.a1", "EXP-03", E, 285, 285, "ley-sin-marca"),
    ley("EXP-03.b1", "EXP-03", E, 294, 294, "ley-ejemplo",
        semantica=valor("string", "x")),
    ley("EXP-03.b2", "EXP-03", E, 295, 295, "ley-ejemplo",
        semantica=valor("string", "y")),
    ley("EXP-03.b3", "EXP-03", E, 298, 298, "ley-ejemplo",
        semantica=valor("boolean", "false")),
    ley("EXP-03.b4", "EXP-03", E, 299, 299, "ley-ejemplo",
        semantica=valor("integer", "0")),
    ley("EXP-03.b5", "EXP-03", E, 300, 300, "ley-ejemplo",
        semantica=valor("string", "")),
    ley("EXP-03.b6", "EXP-03", E, 303, 303, "ley-ejemplo",
        semantica=valor("boolean", "true")),
    ley("EXP-03.b7", "EXP-03", E, 304, 304, "ley-ejemplo",
        semantica=valor("boolean", "true")),
    ley("EXP-03.c1", "EXP-03", E, 310, 312, "ley-sin-marca"),
    ley("EXP-03.c2", "EXP-03", E, 315, 315, "ley-sin-marca"),
    ley("EXP-03.e1", "EXP-03", E, 324, 324, "ley-ejemplo"),
    ley("EXP-03.e2", "EXP-03", E, 325, 325, "ley-ejemplo",
        sintaxis="invalida"),
    ley("EXP-03.f1", "EXP-03", E, 336, 336, "ley-sin-marca"),
    ley("EXP-03.f2", "EXP-03", E, 339, 339, "ley-sin-marca"),
    ley("EXP-03.f3", "EXP-03", E, 342, 342, "ley-sin-marca"),
    ley("EXP-03.f4", "EXP-03", E, 345, 345, "ley-sin-marca"),
    texto("EXP-03.p1", "EXP-03",
          "integer? v = none\ninteger w = v!", "ley-prosa",
          semantica=fallo("RUN004"), afirma="06-expressions.md:314"),

    # ---- EXP-08 (Operators on built-in types; no overloading) -------
    ley("EXP-08.n1", "EXP-08", E, 386, 388, "ley-sin-marca"),

    # ---- EXP-09 (Method calls and property access) ------------------
    ley("EXP-09.n1", "EXP-09", E, 412, 416, "ley-sin-marca"),

    # ---- EXP-10 (Function arguments evaluate left to right) ---------
    ley("EXP-10.n1", "EXP-10", E, 438, 440, "ley-sin-marca"),

    # ---- CALL-03 (Namespace functions: for utilities only) ----------
    ley("CALL-03.e1", "CALL-03", M, 84, 87, "ley-sin-marca"),
    ley("CALL-03.e2", "CALL-03", M, 89, 92, "ley-sin-marca"),
    ley("CALL-03.e4", "CALL-03", M, 95, 95, "ley-sin-marca"),
    texto("LEX-04.s1", "LEX-04", "list = 1", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN002",
          afirma="03-lexical-structure.md (LEX-04: una type keyword "
                 "nunca es un identificador)"),
    texto("LEX-04.s2", "LEX-04", "x = list", "ley-prosa",
          sintaxis="invalida",
          afirma="03-lexical-structure.md (LEX-04: sola no es un valor)"),
    texto("LEX-04.s3", "LEX-04", "n = list.concat(a, b).length()",
          "sonda"),
    # La forma admite cualquier type keyword; si `boolean` tiene esa
    # función es asunto del checker (EXPG-02).
    texto("LEX-04.s4", "LEX-04", 'b = boolean.parse("true")', "sonda"),

    # ---- LEX-06 (Literal forms) -------------------------------------
    ley("LEX-06.i1", "LEX-06", L, 320, 320, "ley-sin-marca",
        semantica=valor("integer", "42")),
    ley("LEX-06.i2", "LEX-06", L, 321, 321, "ley-sin-marca",
        semantica=valor("integer", "255")),
    ley("LEX-06.i3", "LEX-06", L, 322, 322, "ley-sin-marca",
        semantica=valor("integer", "10")),
    ley("LEX-06.i4", "LEX-06", L, 323, 323, "ley-sin-marca",
        semantica=valor("integer", "511")),
    ley("LEX-06.f1", "LEX-06", L, 328, 328, "ley-sin-marca",
        semantica=valor("number", "3.14")),
    ley("LEX-06.f2", "LEX-06", L, 329, 329, "ley-ejemplo",
        semantica=valor("number", "0.5")),
    ley("LEX-06.f3", "LEX-06", L, 330, 330, "ley-sin-marca",
        semantica=valor("number", "6.02e23")),
    texto("LEX-06.p1", "LEX-06", "3.14.toInteger()", "ley-prosa",
          afirma="03-lexical-structure.md:331"),
    texto("LEX-06.p2", "LEX-06", "3.toInteger()", "ley-prosa",
          afirma="03-lexical-structure.md:332"),
    texto("LEX-06.p3", "LEX-06", "3.", "ley-prosa",
          sintaxis="invalida", afirma="03-lexical-structure.md:333"),
    texto("LEX-06.p4", "LEX-06", "1_000_000", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN001",
          afirma="03-lexical-structure.md:341"),
    ley("LEX-06.s1", "LEX-06", L, 365, 367, "ley-sin-marca"),
    ley("LEX-06.b1", "LEX-06", L, 478, 479, "ley-sin-marca"),
    ley("LEX-06.n1", "LEX-06", L, 487, 487, "ley-sin-marca"),
    texto("LEX-06.n2", "LEX-06", "none == none", "ley-prosa",
          semantica=valor("boolean", "true"),
          afirma="03-lexical-structure.md:478"),
    texto("LEX-06.n3", "LEX-06", "none == 0", "ley-prosa",
          semantica=valor("boolean", "false"),
          afirma="03-lexical-structure.md:479"),
    ley("LEX-06.y1", "LEX-06", L, 469, 471, "ley-sin-marca"),
    ley("LEX-06.t1", "LEX-06", L, 422, 426, "ley-sin-marca"),
    ley("LEX-06.l1", "LEX-06", L, 499, 502, "ley-sin-marca"),
    ley("LEX-06.m1", "LEX-06", L, 507, 509, "ley-sin-marca"),

    # ---- TYP-05 (list behaviors), TYPG-02, TYPG-04 -----------------
    texto("TYP-05.p1", "TYP-05", "list<string>.line queue = []", "ley-prosa",
          afirma="04-type-system.md:214 (`list<string>.line`)"),
    texto("TYP-05.p2", "TYP-05", "list<integer>.line.unique seen = []",
          "ley-prosa", afirma="04-type-system.lark.md (TYPG-03: la cadena "
                              "la admite la gramática; SEM009 es del checker)"),
    texto("TYP-05.p3", "TYP-05", "matrix<integer>.line m = []", "ley-prosa",
          sintaxis="invalida",
          afirma="04-type-system.lark.md (TYPG-03: solo list<T> lleva "
                 "comportamientos)"),
    texto("TYP-05.p4", "TYP-05", "void f(list<string>.line q)\n\treturn",
          "ley-prosa", sintaxis="invalida",
          afirma="04-type-system.lark.md (TYPG-03: solo en una declaración "
                 "de variable)"),
    # Un campo de clase es una typed_declaration, explícita o inferida.
    texto("TYPG-02.s1", "TYPG-02", "class Point\n\tinteger x\n\ty = 0\n",
          "sonda", inicio="source_file"),
    # return_type: la firma de flecha de un método de capacidad.
    ley("FNC-03.e1", "FNC-03", N, 156, 158, "ley-sin-marca",
        inicio="source_file"),

    # FIL-01, FIL-02: sección fuera de orden (SYN007); forma que no es de
    # nivel superior (SYN009).
    texto("FIL-01.o1", "FIL-01", "functions:\n\tvoid f()\n\t\treturn\nimport:\n\tmath\n",
          "ley-prosa", sintaxis="invalida", codigo_sintaxis="SYN007",
          inicio="source_file", afirma="08-file-structure.md (FIL-01)"),
    texto("FIL-02.t1", "FIL-02", "integer x = 5\n", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN009", inicio="source_file",
          afirma="08-file-structure.md (FIL-02)"),

    # ---- TYP-03, TYP-06, TYP-07, TYP-11 -----------------------------
    texto("TYP-03.p1", "TYP-03", "integer x = none", "ley-prosa",
          semantica=estatico(None), afirma="04-type-system.md:155"),
    texto("TYP-06.p1", "TYP-06", "number x = 1", "ley-prosa",
          semantica=valor("number", "1.0"),
          afirma="04-type-system.md:440"),
    ley("TYP-06.e1", "TYP-06", T, 454, 458, "ley-ejemplo"),
    ley("TYP-07.e1", "TYP-07", T, 484, 486, "ley-ejemplo",
        semantica=valor("boolean", "false")),
    ley("TYP-11.e1", "TYP-11", T, 411, 415, "ley-sin-marca",
        semantica=valor("string", "Alice")),

    # ---- ERH-02 (onError, forma sufijo) -----------------------------
    ley("ERH-02.e1", "ERH-02", H, 64, 65, "ley-sin-marca"),
    texto("ERH-02.p1", "ERH-02", "number x = 1 onError 2.5", "ley-prosa",
          semantica=valor("number", "1.0"),
          afirma="13-error-handling.md:81"),

    # ---- RUL-16 (SEM001), RUL-19 (SEM004), RUL-41 (SEM026) ----------
    ley("RUL-16.e1", "RUL-16", R, 417, 418, "ley-ejemplo",
        semantica=valor("string", "Alice")),
    ley("RUL-16.e2", "RUL-16", R, 424, 424, "ley-ejemplo",
        semantica=estatico("SEM001")),
    ley("RUL-19.e1", "RUL-19", R, 527, 527, "ley-ejemplo",
        semantica=estatico("SEM004")),
    ley("RUL-19.e2", "RUL-19", R, 533, 533, "ley-ejemplo",
        semantica=valor("string", "helloworld")),
    texto("RUL-41.p1", "RUL-41",
          "integer floor = -9223372036854775808", "ley-prosa",
          semantica=valor("integer", "-9223372036854775808"),
          afirma="10-semantic-rules.md:1290"),
    texto("RUL-41.p2", "RUL-41",
          "integer n = 9223372036854775808", "ley-prosa",
          semantica=estatico("SEM026"),
          afirma="10-semantic-rules.md:1292"),

    # ---- RUL-136 (RUN003), STD-14 -----------------------------------
    ley("RUL-136.e1", "RUL-136", R, 3885, 3887, "ley-ejemplo",
        semantica=fallo("RUN003", "division by zero")),
    ley("RUL-136.e2", "RUL-136", R, 3893, 3893, "ley-ejemplo",
        semantica=valor("number", "Infinity")),
    texto("RUL-136.p1", "RUL-136",
          "integer m = -9223372036854775808\ninteger q = m / -1",
          "ley-prosa",
          semantica=fallo("RUN003", "integer overflow in division"),
          afirma="10-semantic-rules.md:3831"),
    texto("RUL-136.p2", "RUL-136",
          "integer m = -9223372036854775808\ninteger r = m % -1",
          "ley-prosa", semantica=valor("integer", "0"),
          afirma="10-semantic-rules.md:3831"),
    texto("RUL-136.p3", "RUL-136", "integer r = 7 % 0", "ley-prosa",
          semantica=fallo("RUN003", "division by zero"),
          afirma="10-semantic-rules.md:3830"),
    texto("STD-14.p1", "STD-14", "number x = -1.0 / 0.0", "ley-prosa",
          semantica=valor("number", "-Infinity"),
          afirma="15-standard-library.md:410"),
    texto("STD-14.p2", "STD-14", "number x = 0.0 / 0.0", "ley-prosa",
          semantica=valor("number", "NaN"),
          afirma="15-standard-library.md:410"),

    # ---- STM-02 (destinos de asignación), EXPG-01 (índice): sondas ---
    texto("STM-02.s1", "STM-02", "obj.property = val", "ley-prosa",
          afirma="07-statements.lark.md (MemberAssignmentTarget)"),
    texto("STM-02.s2", "STM-02", "arr[0] = value", "ley-prosa",
          afirma="07-statements.lark.md (IndexedAssignmentTarget)"),
    texto("STM-02.s3", "STM-02", "f() = 1", "ley-prosa",
          sintaxis="invalida",
          afirma="07-statements.lark.md (STM-02: un call no es destino)"),
    texto("STM-02.s4", "STM-02", "x! = 1", "ley-prosa",
          sintaxis="invalida",
          afirma="07-statements.lark.md (STM-02: un ! no es destino)"),
    texto("EXPG-01.s1", "EXPG-01", "first = items[0]", "sonda"),
    texto("TYP-02.s1", "TYP-02", "User owner = makeUser()", "sonda"),

    # ---- LEX-01 (Indentation is one tab per level): sonda -----------
    texto("LEX-01.s1", "LEX-01",
          "if x > 0\n\ty = 1\nelse if x < 0\n\ty = 2\nelse\n\ty = 3\n"
          "while y > 0\n\ty = y - 1",
          "sonda"),
    texto("LEX-01.s2", "LEX-01", "if x > 0\n    y = 1", "sonda",
          sintaxis="invalida", codigo_sintaxis="SYN003"),
    texto("LEX-01.s3", "LEX-01", "if x > 0\n \ty = 1", "sonda",
          sintaxis="invalida", codigo_sintaxis="SYN006"),
    texto("LEX-01.s4", "LEX-01", "if x > 0\n\t\ty = 1", "sonda",
          sintaxis="invalida", codigo_sintaxis="SYN003"),

    # ---- Las dos formas de RUL-19 sin la palabra reservada ----------
    texto("RUL-19.s1", "RUL-19", 'string r = "hello" - "world"', "sonda",
          semantica=estatico("SEM004")),
    texto("RUL-19.s2", "RUL-19", 'string r = "hello" + "world"', "sonda",
          semantica=valor("string", "helloworld")),

    # ---- Sondas que la ley sí decide --------------------------------
    texto("EXP-01.s1", "EXP-01", "1 + 2 * 3", "sonda",
          semantica=valor("integer", "7")),
    texto("EXP-01.s2", "EXP-01", "10 - 4 - 3", "sonda",
          semantica=valor("integer", "3")),
    texto("EXP-01.s3", "EXP-01", "-2 ^ 2", "sonda",
          agrupa_como="(-2) ^ 2", semantica=valor("integer", "4")),
    texto("EXP-01.s4", "EXP-01", "not true == false", "sonda",
          agrupa_como="(not true) == false",
          semantica=valor("boolean", "true")),
    texto("EXP-01.s5", "EXP-01", "1 + 2 < 4 and 2 * 2 == 4", "sonda",
          agrupa_como="((1 + 2) < 4) and ((2 * 2) == 4)",
          semantica=valor("boolean", "true")),
    texto("EXP-01.s6", "EXP-01",
          "integer? a = none\ninteger b = a default 1 + 2", "sonda",
          semantica=valor("integer", "3")),
    texto("EXP-03.s1", "EXP-03",
          "integer? v = 5\ninteger w = v!", "sonda",
          semantica=valor("integer", "5")),
    texto("EXP-03.s2", "EXP-03",
          "integer? a = none\ninteger b = a default 3", "sonda",
          semantica=valor("integer", "3")),
    texto("EXP-04.s1", "EXP-04", "0.1 + 0.2", "sonda",
          semantica=valor("number", "0.30000000000000004")),
    texto("EXP-04.s2", "EXP-04", "2.0 ^ 0.5", "sonda",
          semantica=valor("number", "1.4142135623730951")),
    texto("EXP-04.s3", "EXP-04", "6 / 3", "sonda",
          semantica=valor("integer", "2")),
    texto("EXP-04.s4", "EXP-04", "7 % 3", "sonda",
          semantica=valor("integer", "1")),
    texto("EXP-05.s1", "EXP-05", "7 > 3 and 2 <= 2", "sonda",
          semantica=valor("boolean", "true")),
    texto("EXP-05.s2", "EXP-05",
          "integer? a = none\nboolean b = a is none", "sonda",
          semantica=valor("boolean", "true")),
    texto("EXP-06.s1", "EXP-06",
          "integer? a = none\ninteger? b = 1\nboolean c = a not b",
          "sonda", semantica=valor("boolean", "true")),
    texto("EXP-06.s2", "EXP-06",
          "integer? a = none\ninteger? b = 1\n"
          "boolean c = not (a is b)",
          "sonda", semantica=valor("boolean", "true")),
    texto("EXP-07.s1", "EXP-07", "not 1", "sonda",
          semantica=estatico("SEM004")),
    texto("EXP-07.s2", "EXP-07", "1 and true", "sonda",
          semantica=estatico("SEM004")),
    texto("ERH-02.s1", "ERH-02",
          "integer x = (1 / 0) onError 7", "sonda",
          semantica=valor("integer", "7")),
    texto("ERH-02.s2", "ERH-02",
          'integer x = (1 / 0) onError "siete"', "sonda",
          semantica=estatico("SEM001")),
    texto("ERH-02.s3", "ERH-02",
          "integer? v = none\ninteger w = v! onError 9", "sonda",
          semantica=valor("integer", "9")),
    texto("RUL-17.s1", "RUL-17", "integer x = y + 1", "sonda",
          semantica=estatico("SEM002")),
    texto("TYP-03.s1", "TYP-03",
          "integer? a = 5\ninteger b = a", "sonda",
          semantica=estatico("SEM001")),
    texto("TYP-06.s1", "TYP-06", "integer x = 3.14", "sonda",
          semantica=estatico("SEM001")),
    texto("TYP-06.s2", "TYP-06", "number x = -17", "sonda",
          semantica=valor("number", "-17.0")),

    # ---- Las diecinueve preguntas, ya decididas (2026-09-29) ---------
    # EXP-04, EXP-05, EXP-07, EXP-03, TYP-06, TYP-11, RUL-136, RUL-19,
    # RUL-24: cada una tiene ahora respuesta en la ley.
    texto("P-01", "RUL-136", "integer x = 9223372036854775807 + 1",
          "sonda", semantica=fallo("RUN003", "integer overflow in addition")),
    texto("P-02", "EXP-04", "integer q = 7 / 2", "sonda",
          semantica=valor("integer", "3")),
    texto("P-02b", "EXP-04", "integer q = -7 / 2", "sonda",
          semantica=valor("integer", "-3")),
    texto("P-03", "EXP-04", "integer r = -7 % 2", "sonda",
          semantica=valor("integer", "-1")),
    texto("P-04", "RUL-136", "integer p = 2 ^ -1", "sonda",
          semantica=fallo("RUN003", "negative exponent on integer")),
    texto("P-05", "TYP-06", "number n = 1 + 2.5", "sonda",
          semantica=valor("number", "3.5")),
    texto("P-06", "RUL-19",
          "integer a = 1\nnumber b = 2.5\nnumber c = a + b", "sonda",
          semantica=estatico("SEM004")),
    texto("P-07", "EXP-08", "number r = 5.5 % 2.0", "sonda",
          semantica=estatico("SEM004")),
    texto("P-08", "EXP-05", 'boolean b = "a" < "b"', "sonda",
          semantica=estatico("SEM004")),
    texto("P-09", "EXP-05", 'boolean b = 1 == "1"', "sonda",
          semantica=estatico("SEM004")),
    texto("P-10", "EXP-05", "boolean b = 3 is 3", "sonda",
          semantica=valor("boolean", "true")),
    texto("P-11", "EXP-03", "integer x = 5\ninteger y = x!", "sonda",
          semantica=estatico("SEM004")),
    texto("P-12", "EXP-07", "boolean b = false and (1 / 0 == 1)",
          "sonda", semantica=valor("boolean", "false")),
    texto("P-13", "EXP-07", "boolean b = true or (1 / 0 == 1)",
          "sonda", semantica=valor("boolean", "true")),
    texto("P-14", "EXP-03", "integer x = 5 default (1 / 0)", "sonda",
          semantica=valor("integer", "5")),
    texto("P-15", "TYP-06", "number x = 1 + 2", "sonda",
          semantica=valor("number", "3.0")),
    texto("P-16", "TYP-11", "x = none", "sonda",
          semantica=estatico("SEM009")),
    texto("P-17", "EXP-03",
          'integer? a = none\nstring s = a default "x"', "sonda",
          semantica=estatico("SEM001")),
    texto("P-18", "EXP-05",
          "number n = 0.0 / 0.0\nboolean b = n == n", "sonda",
          semantica=valor("boolean", "false")),
    texto("P-19", "RUL-136",
          "integer m = -9223372036854775808\ninteger x = -m", "sonda",
          semantica=fallo("RUN003", "integer overflow in negation")),
    texto("P-20", "RUL-136",
          "integer x = 4611686018427387904 * 2", "sonda",
          semantica=fallo("RUN003", "integer overflow in multiplication")),
    texto("P-21", "RUL-136", "integer x = 2 ^ 63", "sonda",
          semantica=fallo("RUN003", "integer overflow in exponentiation")),
    texto("P-22", "TYP-06", "number n = 2.5 + 1", "sonda",
          semantica=valor("number", "3.5")),

    # =================================================================
    # 07 statements
    # =================================================================
    # ---- STM-01 (Declarations are type-first) -----------------------
    ley("STM-01.e1", "STM-01", S, 27, 30, "ley-sin-marca",
        semantica=valor("boolean", "true")),
    texto("STM-01.s1", "STM-01", "string z\nz = \"a\"\nreturn z", "sonda",
          semantica=valor("string", "a")),
    texto("STM-01.s2", "STM-01", "integer x = 1\ninteger x = 2", "sonda",
          semantica=estatico("SCOPE002")),

    # ---- STM-02 (Assignment is a statement, never an expression) ----
    ley("STM-02.e1", "STM-02", S, 50, 52, "ley-sin-marca"),
    texto("STM-02.p1", "STM-02", "if x = 5\n\ty = 1", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN002",
          afirma="07-statements.md:55 (`if x = 5` no lee como comparación)"),

    # ---- STM-04 (Console output and input: the language-level parts)
    ley("STM-04.e1", "STM-04", S, 80, 82, "ley-sin-marca", dedenta=2),
    texto("STM-04.p1", "STM-04", "print:\nx = 1", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN008",
          afirma="07-statements.md:77 (cuerpo vacío)"),
    texto("STM-04.p2", "STM-04", "print:\n\tx = 1", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN008",
          afirma="07-statements.md:77 (una sentencia en el cuerpo)"),
    texto("STM-04.p3", "STM-04", "print = 1", "ley-prosa",
          sintaxis="invalida", codigo_sintaxis="SYN002",
          afirma="07-statements.md:85 (`print` es hard keyword)"),
    texto("STM-04.s1", "STM-04", 'print:\n\t"a"\n\t1 + 2\n\ttrue', "sonda",
          imprime="a\n3\ntrue\n"),

    # ---- STM-03 (`return` and its forms) ----------------------------
    # `value` y `expression` son marcadores de posición: solo sintaxis.
    ley("STM-03.e1", "STM-03", S, 104, 106, "ley-sin-marca"),
    texto("STM-03.s1", "STM-03", "integer x = 5\nreturn x + 1\nx = 9",
          "sonda", semantica=valor("integer", "6")),
    texto("STM-03.s2", "STM-03", "return\nprint(1)", "sonda",
          semantica=sinvalor(), imprime=""),

    # ---- STD-04 (Console output with `print`): la llamada -----------
    texto("STD-04.p1", "STD-04", "integer age = 25\nprint(age)", "ley-prosa",
          imprime="25\n", afirma="15-standard-library.md:81"),
    texto("STD-04.p2", "STD-04",
          'integer age = 25\nprint("Age: " + age.toString())', "ley-prosa",
          imprime="Age: 25\n", afirma="15-standard-library.md:82"),
    texto("STD-04.p3", "STD-04", 'print("a", newline: false)\nprint("b")',
          "ley-prosa", imprime="ab\n", afirma="15-standard-library.md:73"),
    texto("STD-04.p4", "STD-04", 'print "text"', "ley-prosa",
          sintaxis="invalida", afirma="15-standard-library.md:75"),
    texto("STD-04.s1", "STD-04", "print(true)\nprint(false)", "sonda",
          imprime="true\nfalse\n"),
    texto("STD-04.s2", "STD-04", 'print(1, newline: 2)', "sonda",
          semantica=estatico("SEM001")),

    # =================================================================
    # 12 control flow
    # =================================================================
    # ---- FLW-01 (Conditional statements) ----------------------------
    # H-10: un cuerpo hecho solo de un comentario no es un cuerpo.
    ley("FLW-01.e1", "FLW-01", F, 27, 28, "ley-ejemplo"),
    ley("FLW-01.e2", "FLW-01", F, 31, 34, "ley-ejemplo"),
    ley("FLW-01.e3", "FLW-01", F, 37, 42, "ley-ejemplo"),
    texto("FLW-01.s1", "FLW-01",
          "integer x = 7\nif x > 5\n\tprint(1)\nelse if x > 2\n\tprint(2)\n"
          "else\n\tprint(3)", "sonda", imprime="1\n"),
    texto("FLW-01.s2", "FLW-01",
          "integer x = 3\nif x > 5\n\tprint(1)\nelse if x > 2\n\tprint(2)\n"
          "else\n\tprint(3)", "sonda", imprime="2\n"),
    texto("FLW-01.s3", "FLW-01",
          "integer x = 0\nif x > 5\n\tprint(1)\nelse if x > 2\n\tprint(2)\n"
          "else\n\tprint(3)", "sonda", imprime="3\n"),
    texto("FLW-01.s4", "FLW-01", "if 1\n\tprint(1)", "sonda",
          semantica=estatico("SEM023")),
    texto("FLW-01.s5", "FLW-01", "if true\n\tinteger y = 1\nprint(y)",
          "sonda", semantica=estatico("SEM002")),
    # STM-05: el ejemplo entero es SEM002 en su última línea; lo que
    # imprime antes lo comprueba STM-05.p1.
    ley("STM-05.e1", "STM-05", S, 128, 133, "ley-ejemplo",
        semantica=estatico("SEM002")),
    texto("STM-05.p1", "STM-05",
          "integer total = 0\niterate n in 1 to 3\n\tinteger twice = n * 2\n"
          "\ttotal = total + twice\nprint(total)", "ley-prosa",
          imprime="12\n", afirma="07-statements.md (STM-05, ejemplo)"),
    texto("STM-05.p2", "STM-05",
          "integer x = 1\nif true\n\tinteger x = 2", "ley-prosa",
          semantica=estatico("SCOPE002"),
          afirma="07-statements.md (STM-05: un nombre visible no se redeclara)"),
    texto("STM-05.p3", "STM-05",
          "iterate i in 1 to 2\n\tinteger k = i\nprint(k)", "ley-prosa",
          semantica=estatico("SEM002"),
          afirma="07-statements.md (STM-05: el binder y el cuerpo mueren con la pasada)"),

    # ---- FLW-02 (Loops) ---------------------------------------------
    ley("FLW-02.e1", "FLW-02", F, 67, 68, "ley-sin-marca"),
    texto("FLW-02.e1b", "FLW-02",
          'list<string> items = ["x", "y"]\niterate item in items\n\tprint(item)',
          "sonda", imprime="x\ny\n"),
    ley("FLW-02.e2", "FLW-02", F, 71, 72, "ley-sin-marca",
        imprime="h\ne\nl\nl\no\n"),
    ley("FLW-02.r0", "FLW-02", F, 78, 81, "ley-sin-marca"),
    ley("FLW-02.r1", "FLW-02", F, 84, 85, "ley-sin-marca",
        imprime="1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"),
    ley("FLW-02.r2", "FLW-02", F, 87, 88, "ley-ejemplo",
        imprime="10\n8\n6\n4\n2\n"),
    ley("FLW-02.r3", "FLW-02", F, 90, 91, "ley-sin-marca",
        imprime="C\nl\ne\na\nn\n"),
    ley("FLW-02.r4", "FLW-02", F, 93, 95, "ley-sin-marca"),
    texto("FLW-02.r4b", "FLW-02",
          "matrix<integer> grid = [[1, 2], [3, 4]]\niterate row in grid\n"
          "\titerate value in row\n\t\tprint(value)", "sonda",
          imprime="1\n2\n3\n4\n"),
    ley("FLW-02.r5", "FLW-02", F, 97, 98, "ley-ejemplo",
        imprime="".join(f"{k}\n" for k in range(0, 101, 5))),
    # Range semantics, línea a línea.
    texto("FLW-02.n1", "FLW-02", "iterate i in 0 to 100 step 3\n\tprint(i)",
          "ley-prosa", imprime="".join(f"{k}\n" for k in range(0, 100, 3)),
          afirma="12-control-flow.md:101 (`0 to 100 step 3` acaba en 99)"),
    texto("FLW-02.n2", "FLW-02", "iterate i in 5 to 1\n\tprint(i)",
          "ley-prosa", imprime="5\n4\n3\n2\n1\n",
          afirma="12-control-flow.md:102 (paso -1 por defecto)"),
    texto("FLW-02.n3", "FLW-02", "iterate i in 4 to 4\n\tprint(i)",
          "ley-prosa", imprime="4\n",
          afirma="12-control-flow.md:102 (`from == to` visita una vez)"),
    texto("FLW-02.n4", "FLW-02",
          "integer n = 3\niterate i in 1 to n\n\tn = 10\n\tprint(i)",
          "ley-prosa", imprime="1\n2\n3\n",
          afirma="12-control-flow.md:103 (los límites se evalúan una vez)"),
    texto("FLW-02.n5", "FLW-02",
          'list<integer> xs = [1, 2]\niterate x in xs step 1\n\tprint(x)',
          "ley-prosa", sintaxis="invalida", codigo_sintaxis="SYN002",
          afirma="12-control-flow.md:104 (`step` solo en la forma de rango)"),
    texto("FLW-02.n6", "FLW-02", "iterate i in 1 to 10 step 0\n\tprint(i)",
          "ley-prosa", semantica=estatico("SEM030"),
          afirma="12-control-flow.md:106"),
    texto("FLW-02.n7", "FLW-02",
          "integer s = 0\niterate i in 1 to 3 step s\n\tprint(i)",
          "ley-prosa", semantica=fallo("RUN020"),
          afirma="12-control-flow.md:106"),
    texto("FLW-02.n8", "FLW-02", "iterate i in 1 to 0\n\tprint(i)",
          "sonda", imprime="1\n0\n"),
    texto("FLW-02.n9", "FLW-02", "iterate x in 5\n\tprint(x)", "sonda",
          semantica=estatico("SEM004")),
    texto("FLW-02.n10", "FLW-02", "iterate i in 1.0 to 3.0\n\tprint(i)",
          "sonda", semantica=estatico("SEM004")),
    # While.
    ley("FLW-02.w0", "FLW-02", F, 116, 117, "ley-sin-marca"),
    ley("FLW-02.w1", "FLW-02", F, 124, 127, "ley-ejemplo",
        imprime="0\n1\n2\n3\n4\n"),
    ley("FLW-02.w2", "FLW-02", F, 131, 136, "ley-ejemplo"),
    texto("FLW-02.w2b", "FLW-02",
          "boolean running = true\ninteger iterations = 0\nwhile running\n"
          "\titerations = iterations + 1\n\tif iterations >= 3\n"
          "\t\trunning = false\nprint(iterations)", "ley-ejemplo",
          imprime="3\n", afirma="12-control-flow.md:135"),
    ley("FLW-02.w3", "FLW-02", F, 140, 146, "ley-sin-marca",
        imprime="".join(f"outer: {o}, inner: {i}\n"
                        for o in range(3) for i in range(2))),
    ley("FLW-02.w4", "FLW-02", F, 149, 156, "ley-sin-marca",
        imprime="".join(("Even: " if k % 2 == 0 else "Odd: ") + f"{k}\n"
                        for k in range(10))),
    texto("FLW-02.w5", "FLW-02", "while 1\n\tbreak", "ley-prosa",
          semantica=estatico("SEM023"), afirma="12-control-flow.md:158"),
    texto("FLW-02.w6", "FLW-02",
          "integer n = 0\nwhile n < 3\n\tn = n + 1\nprint(n)", "ley-prosa",
          imprime="3\n", afirma="12-control-flow.md:160 (el valor persiste)"),

    # ---- FLW-03 (`break` and `continue`) ----------------------------
    ley("FLW-03.e1", "FLW-03", F, 193, 198, "ley-sin-marca"),
    texto("FLW-03.e1b", "FLW-03",
          'list<string> items = ["a", "", "b", "stop", "c"]\n'
          "iterate item in items\n\tif item.isEmpty()\n\t\tcontinue\n"
          '\tif item == "stop"\n\t\tbreak\n\tprint(item)', "ley-ejemplo",
          imprime="a\nb\n", afirma="12-control-flow.md:190"),
    texto("FLW-03.p1", "FLW-03", "break", "ley-prosa",
          semantica=estatico("SEM025"), afirma="12-control-flow.md:184"),
    texto("FLW-03.p2", "FLW-03", "if true\n\tcontinue", "ley-prosa",
          semantica=estatico("SEM025"), afirma="12-control-flow.md:184"),
    texto("FLW-03.p3", "FLW-03",
          "iterate i in 1 to 10 step 3\n\tif i == 4\n\t\tcontinue\n\tprint(i)",
          "ley-prosa", imprime="1\n7\n10\n",
          afirma="12-control-flow.md:179 (continue aplica el step)"),
    texto("FLW-03.p4", "FLW-03",
          "iterate i in 1 to 3\n\titerate j in 1 to 3\n\t\tif j == 2\n"
          "\t\t\tcontinue\n\t\tprint(i * 10 + j)", "ley-prosa",
          imprime="11\n13\n21\n23\n31\n33\n",
          afirma="12-control-flow.md:187 (Check: continue en el bucle interno)"),
    texto("FLW-03.p5", "FLW-03",
          "iterate i in 1 to 3\n\titerate j in 1 to 3\n\t\tif j == 2\n"
          "\t\t\tbreak\n\t\tprint(i * 10 + j)", "sonda",
          imprime="11\n21\n31\n"),
    texto("FLW-03.p6", "FLW-03", "break 1", "ley-prosa",
          sintaxis="invalida", afirma="12-control-flow.md:176 (sin operando)"),
    texto("FLW-03.p7", "FLW-03", "x = break", "ley-prosa",
          sintaxis="invalida", afirma="12-control-flow.lark.md (FLWG-04)"),
]
