"""La gramática de Clean, compuesta desde la ley (FS-01, DOC-15).

La ley escribe la gramática en los companions `<capítulo>.lark.md` de
foundation, en bloques ```lark. Este módulo los lee y los compone:

- el lenguaje: todos los companions de `language-principles/`, en orden
  de archivo, forman una sola gramática;
- cada librería: su companion, una gramática propia.  Lo que toma del
  lenguaje lo pide con la directiva de Lark `%import`, escrita en un
  bloque ```lark del companion, sobre el módulo `clean`:

      %import clean (IDENTIFIER, expression, _NEWLINE, _INDENT, _DEDENT)

  El módulo `clean` es la gramática del lenguaje compuesta (`lenguaje()`),
  que `cargador` sirve a Lark como si fuera el archivo `clean.lark`; Lark
  trae cada nombre pedido con todo aquello de lo que depende.  Un nombre
  que la librería no pide pero que llega como dependencia queda con el
  prefijo `clean__`; por eso un terminal que el arnés o el indentador
  nombran (`_NEWLINE`, `_INDENT`, `_DEDENT`, `LPAR`, `RPAR`, `RESERVED`, las
  type keywords) se pide siempre por su nombre.  Los `%ignore` no se
  importan: la librería pide `INLINE_SPACE`, `LINE_COMMENT` y
  `BLOCK_COMMENT` y los ignora ella.  El símbolo de inicio de una
  librería es su regla `start`; una librería que lee textos de varias
  clases (un bloque, una clave, una plantilla) nombra en cambio sus
  reglas de inicio en una línea de comentario de sus bloques, que Lark
  ignora y el arnés lee como la lista `start` de Lark:

      // %start locale_block translation_key

Foundation se busca en $CLEAN_FOUNDATION o, si no está, como repositorio
hermano de este.
"""
from __future__ import annotations

import os
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FOUNDATION = Path(os.environ.get("CLEAN_FOUNDATION",
                                 REPO.parent / "clean-language-foundation"))
LEY = FOUNDATION / "governance" / "product"
LENGUAJE = LEY / "language-principles"
LIBRERIAS = LEY / "architecture-boundaries" / "framework"

_ABRE = re.compile(r"^\s*```lark\s*$")
_CIERRA = re.compile(r"^\s*```\s*$")


def bloques(ruta: Path) -> str:
    """El texto de los bloques ```lark de un companion, en orden."""
    out, dentro = [], False
    for linea in ruta.read_text(encoding="utf-8").split("\n"):
        if not dentro and _ABRE.match(linea):
            dentro = True
            out.append(f"// --- {ruta.name}")
            continue
        if dentro and _CIERRA.match(linea):
            dentro = False
            continue
        if dentro:
            out.append(linea)
    return "\n".join(out) + "\n"


def companions_del_lenguaje() -> list[Path]:
    return sorted(LENGUAJE.glob("*.lark.md"))


def companions_de_librerias() -> list[Path]:
    return sorted(LIBRERIAS.rglob("*.lark.md"))


def lenguaje() -> str:
    return "".join(bloques(p) for p in companions_del_lenguaje())


def libreria(ruta: Path) -> str:
    """La gramática de un companion de librería: sus bloques ```lark, con
    sus `%import clean (...)`, que Lark resuelve con `cargador`."""
    return bloques(ruta)


MODULO = "clean.lark"


def cargador(base, ruta: str):
    """Fuente de `%import` para Lark (su opción `import_paths`): sirve la
    gramática del lenguaje compuesta como el módulo `clean`."""
    if ruta != MODULO:
        raise IOError(ruta)
    return f"<{MODULO}>", lenguaje()
