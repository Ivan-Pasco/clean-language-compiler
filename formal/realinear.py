"""Realinea las líneas que el corpus cita cuando la ley se enmienda.

Cada caso tomado de la ley se ancla en la primera línea de su código;
este script busca el ancla en el capítulo tal como está hoy y reescribe
el par de líneas del caso en corpus.py. Se corre después de cada
aterrizaje; si un ancla no aparece exactamente una vez, lo dice.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

from etapa_a import LEY

AQUI = Path(__file__).parent

# id del caso -> (archivo relativo a LEY, ancla de la primera línea, líneas)
E = "language-principles/06-expressions.md"
R = "interoperability-principles/platform/10-semantic-rules.md"
M = "language-principles/16-method-style-syntax.md"
S = "language-principles/07-statements.md"
F = "language-principles/12-control-flow.md"
L = "language-principles/03-lexical-structure.md"
N = "language-principles/09-functions.md"
ANCLAS = {
    "LEX-06.i1": (L, "42          // Decimal", 1),
    "LEX-06.i2": (L, "0xff        // Hexadecimal", 1),
    "LEX-06.i3": (L, "0b1010      // Binary", 1),
    "LEX-06.i4": (L, "0o777       // Octal", 1),
    "LEX-06.f1": (L, "3.14        // Standard decimal", 1),
    "LEX-06.f2": (L, ".5          // Leading zero optional", 1),
    "LEX-06.f3": (L, "6.02e23     // Scientific notation", 1),
    "LEX-06.s1": (L, '"Hello, World!"\n"Line 1\\nLine 2"', 3),
    "LEX-06.b1": (L, "true\nfalse", 2),
    "LEX-06.n1": (L, "none        // The none value", 1),
    "LEX-06.y1": (L, 'b"GET / HTTP/1.1\\r\\n"', 3),
    "LEX-06.t1": (L, 'string text = """', 5),
    "FNC-03.e1": (N, "can Describe:", 3),
    "LEX-06.l1": (L, "[1, 2, 3, 4]           // Integer list", 4),
    "LEX-06.m1": (L, "[[1, 2], [3, 4]]                    // 2x2 matrix", 3),
    "STM-01.e1": (S, "integer x = 10", 4),
    "STM-02.e1": (S, "x = 42              // Simple assignment", 3),
    "STM-04.e1": (S, "  print:", 3),
    "STM-03.e1": (S, "return              // Return void", 3),
    "STM-05.e1": (S, "integer total = 0", 6),
    "FLW-01.e1": (F, "if condition\n\tstatements\n", 2),
    "FLW-01.e2": (F, "if condition\n\tstatements\nelse\n\tstatements", 4),
    "FLW-01.e3": (F, "if condition1", 6),
    "FLW-02.e1": (F, "iterate item in items\n\tprint(item)", 2),
    "FLW-02.e2": (F, 'iterate char in "hello"', 2),
    "FLW-02.r0": (F, "iterate name in source", 4),
    "FLW-02.r1": (F, "iterate i in 1 to 10", 2),
    "FLW-02.r2": (F, "iterate k in 10 to 1 step -2", 2),
    "FLW-02.r3": (F, 'iterate ch in "Clean"', 2),
    "FLW-02.r4": (F, "iterate row in grid", 3),
    "FLW-02.r5": (F, "iterate idx in 0 to 100 step 5", 2),
    "FLW-02.w0": (F, "while condition\n\tbody                // executed while condition is true", 2),
    "FLW-02.w1": (F, "integer count = 0", 4),
    "FLW-02.w2": (F, "boolean running = true", 6),
    "FLW-02.w3": (F, "integer outer = 0", 7),
    "FLW-02.w4": (F, "integer i = 0", 8),
    "FLW-03.e1": (F, "iterate item in items\n\tif item.isEmpty()", 6),
    "EXP-02.s1": (E, "// Single line expressions (no parentheses required)", 2),
    "EXP-02.s2": (E, "value = functionCall(arg1, arg2)", 1),
    "EXP-02.s3": (E, "// Multi-line expressions (parentheses required)", 3),
    "EXP-02.s4": (E, "complex = (functionCall(arg1, arg2) +", 3),
    "EXP-02.s5": (E, "calculation = (matrix1 * matrix2 +", 3),
    "EXP-02.e1": (E, "// ✅ Valid: Single line, no parentheses needed", 2),
    "EXP-02.e2": (E, "// ✅ Valid: Multi-line with parentheses", 3),
    "EXP-02.e3": (E, "// ✅ Valid: Complex multi-line expression", 4),
    "EXP-02.e4": (E, "// ✅ Valid: Multi-line function call", 6),
    "EXP-02.e5": (E, "// ❌ Invalid: Multi-line without parentheses", 3),
    "EXP-02.e6": (E, "// ❌ Invalid: Unmatched parentheses", 2),
    "EXP-04.n1": (E, "a + b       // Addition", 6),
    "EXP-05.n1": (E, "a == b      // Equal", 8),
    "EXP-06.n1": (E, "not a           // unary", 1),
    "EXP-06.n2": (E, "a not b         // binary", 1),
    "EXP-06.n3": (E, "not a not b     //", 1),
    "EXP-06.n4": (E, "a not not b     //", 1),
    "EXP-07.n1": (E, "a and b     // Logical AND", 3),
    "EXP-03.a1": (E, "value default fallback    //", 1),
    "EXP-03.b1": (E, 'none default "x"', 1),
    "EXP-03.b2": (E, '"y" default "x"', 1),
    "EXP-03.b3": (E, "false default true", 1),
    "EXP-03.b4": (E, "0 default 10", 1),
    "EXP-03.b5": (E, '"" default "fallback"', 1),
    "EXP-03.b6": (E, "false or true", 1),
    "EXP-03.b7": (E, "true or false", 1),
    "EXP-03.c1": (E, "string username = userData.name default", 3),
    "EXP-03.c2": (E, "string value = primary default secondary", 1),
    "EXP-03.e1": (E, "value!    // ✅", 1),
    "EXP-03.e2": (E, "!value    // ❌", 1),
    "EXP-03.f1": (E, "string? maybeNone = getUser()", 1),
    "EXP-03.f2": (E, "string name = maybeNone!", 1),
    "EXP-03.f3": (E, "string upper = getText()!", 1),
    "EXP-03.f4": (E, "integer count = items.find(item)!", 1),
    "EXP-08.n1": (E, "matrix<number> product = a * b", 3),
    "EXP-09.n1": (E, "obj.method()            // Method call", 5),
    "EXP-10.n1": (E, "functionName()                     // No arguments", 3),
    "RUL-16.e1": (R, "integer x = 42", 2),
    "RUL-16.e2": (R, 'integer x = "hello"', 1),
    "RUL-19.e1": (R, 'string outcome = "hello" - "world"', 1),
    "RUL-19.e2": (R, 'string outcome = "hello" + "world"', 1),
    "RUL-136.e1": (R, "integer a = 10", 3),
    "RUL-136.e2": (R, "number x = 1.0 / 0.0", 1),
    "CALL-03.e1": (M, "// Math utilities (no single owner)", 4),
    "CALL-03.e2": (M, "// Creating new collections", 4),
    "CALL-03.e4": (M, 'list.join(words, ", ")', 1),
}


def main() -> int:
    corpus = (AQUI / "corpus.py").read_text(encoding="utf-8")
    cambios = 0
    problemas = 0
    for caso, (archivo, ancla, n) in ANCLAS.items():
        lineas = (LEY / archivo).read_text(encoding="utf-8").split("\n")
        # Un ancla de varias líneas exige que cada una esté en la suya.
        # Una de varias líneas se compara entera, línea a línea.
        partes = ancla.split("\n")
        def casa(l: str, p: str) -> bool:
            return l.rstrip() == p if len(partes) > 1 else p in l
        hallazgos = [i + 1 for i in range(len(lineas) - len(partes) + 1)
                     if all(casa(lineas[i + k], p) for k, p in enumerate(partes))]
        if len(hallazgos) != 1:
            print(f"  {caso}: el ancla aparece {len(hallazgos)} veces")
            problemas += 1
            continue
        a = hallazgos[0]
        b = a + n - 1
        patron = re.compile(r'(ley\("' + re.escape(caso) + r'", "[^"]+", [A-Z], )(\d+), (\d+)')
        nuevo, k = patron.subn(lambda m: f"{m.group(1)}{a}, {b}", corpus)
        if k != 1:
            print(f"  {caso}: no está en corpus.py una sola vez ({k})")
            problemas += 1
            continue
        if nuevo != corpus:
            cambios += 1
        corpus = nuevo
    (AQUI / "corpus.py").write_text(corpus, encoding="utf-8")
    print(f"realineados: {cambios}   con problema: {problemas}")
    return 1 if problemas else 0


if __name__ == "__main__":
    sys.exit(main())
