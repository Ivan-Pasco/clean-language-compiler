"""Oráculo de sintaxis: compone la gramática de la ley y decide sobre un texto.

Todo lo que decide vive en la gramática; aquí solo está el cableado de
Lark, la indentación de LEX-01 y la traducción de un fallo de Lark al
código de diagnóstico de la ley.
"""
from __future__ import annotations

import io
import logging
import re
from dataclasses import dataclass
from pathlib import Path

from lark import Lark, Token, Tree
from lark import logger as lark_logger
from lark.exceptions import (
    UnexpectedCharacters,
    UnexpectedEOF,
    UnexpectedToken,
)
from lark.indenter import Indenter

AQUI = Path(__file__).parent

import componer


class ErrorDeSintaxis(Exception):
    def __init__(self, codigo: str, linea: int, columna: int, detalle: str):
        super().__init__(f"{codigo} en {linea}:{columna} — {detalle}")
        self.codigo = codigo
        self.linea = linea
        self.columna = columna
        self.detalle = detalle


class CleanIndenter(Indenter):
    """LEX-01: un tab por nivel; los espacios no indentan."""

    NL_type = "_NEWLINE"
    OPEN_PAREN_types = ["LPAR"]
    CLOSE_PAREN_types = ["RPAR"]
    INDENT_type = "_INDENT"
    DEDENT_type = "_DEDENT"
    tab_len = 1
    # RESERVED no aparece en ninguna regla y las type keywords faltan en
    # los estados que solo esperan un nombre: el lexer debe reconocerlas
    # igualmente para que ni una palabra reservada ni una type keyword
    # pasen nunca por nombre (LEX-04).
    always_accept = ("_NEWLINE", "RESERVED", "PRIMITIVE_TYPE", "LIST_NAME",
                     "GENERIC_NAME")

    def handle_NL(self, token: Token):
        if self.paren_level > 0:
            return
        yield token
        indent_str = token.rsplit("\n", 1)[1]
        linea = (token.end_line or token.line or 0)
        if " " in indent_str:
            codigo = "SYN006" if "\t" in indent_str else "SYN003"
            raise ErrorDeSintaxis(codigo, linea, 1,
                                  "espacios en posición de indentación")
        nivel = len(indent_str)
        actual = self.indent_level[-1]
        if nivel > actual:
            if nivel != actual + 1:
                raise ErrorDeSintaxis(
                    "SYN003", linea, 1,
                    f"la indentación sube {nivel - actual} niveles de golpe")
            self.indent_level.append(nivel)
            yield Token.new_borrow_pos(self.INDENT_type, indent_str, token)
        else:
            while nivel < self.indent_level[-1]:
                self.indent_level.pop()
                yield Token.new_borrow_pos(self.DEDENT_type, indent_str, token)


class GramaticaConConflictos(Exception):
    """LALR resolvió algo en silencio: la gramática no es de una lectura."""


# Los símbolos de inicio de la gramática del lenguaje: una secuencia de
# sentencias (los fragmentos de los capítulos, STMG-01) y un archivo
# .cln entero (FILG-01).
INICIOS = ("statement_sequence", "source_file")


def inicios(texto: str) -> list[str]:
    """Los símbolos de inicio de una gramática: los del lenguaje, o los
    de una gramática de librería — su regla `start`, o las reglas que
    nombra su línea `// %start regla regla …` (componer)."""
    m = re.search(r"^\s*//\s*%start\s+(.+)$", texto, re.M)
    if m:
        return m.group(1).split()
    if re.search(r"^[?!]?start\s*:", texto, re.M):
        return ["start"]
    return list(INICIOS)


def _construir(parser: str, debug: bool, texto: str | None = None,
               **opciones) -> Lark:
    if texto is None:
        texto = componer.lenguaje()
    opciones.setdefault("start", inicios(texto))
    # `%import clean (...)`: la gramática del lenguaje, servida por
    # componer.cargador.
    opciones.setdefault("import_paths", [componer.cargador])
    return Lark(
        texto,
        parser=parser,
        lexer="contextual",
        postlex=CleanIndenter(),
        propagate_positions=True,
        maybe_placeholders=False,
        debug=debug,
        **opciones,
    )


def _sin_comentarios(texto: str) -> str:
    """El texto sin sus comentarios `//`: la línea entera, o la cola
    separada del código por espacio (un "//" entre comillas o en una
    regex no lo está)."""
    fuera = []
    for l in texto.split("\n"):
        if l.lstrip().startswith("//"):
            continue
        fuera.append(re.sub(r"\s//(\s.*)?$", "", l))
    return "\n".join(fuera)


def _fragmentos(nombres: list[str], texto: str) -> list[str]:
    """Los terminales que Lark da por no usados pero que otro terminal
    nombra: son fragmentos (LEXG-01), no terminales sobrantes.  Un
    terminal que una librería trae del lenguaje como dependencia llega
    con el prefijo del módulo (`clean__X`, `_clean__X`): es fragmento si
    X lo es en la gramática del lenguaje."""
    limpio = _sin_comentarios(texto)
    lenguaje = None
    fuera = []
    for n in nombres:
        m = re.match(r"^(_?)clean__(.+)$", n)
        if m:
            if lenguaje is None:
                lenguaje = _sin_comentarios(componer.lenguaje())
            nombre, donde = m.group(1) + m.group(2), lenguaje
        else:
            nombre, donde = n, limpio
        if len(re.findall(rf"(?<![A-Za-z0-9_]){re.escape(nombre)}(?![A-Za-z0-9_])",
                          donde)) > 1:
            fuera.append(n)
    return fuera


def cargar(parser: str = "lalr", **opciones) -> Lark:
    """Carga la gramática y exige que LALR no haya resuelto nada solo."""
    if opciones.get("texto") is None:
        opciones["texto"] = componer.lenguaje()
    registro = io.StringIO()
    manejador = logging.StreamHandler(registro)
    nivel = lark_logger.level
    # El registro de depuración va solo a `registro`: los manejadores
    # propios de Lark se apartan mientras dura la construcción.
    propios = list(lark_logger.handlers)
    for h in propios:
        lark_logger.removeHandler(h)
    lark_logger.addHandler(manejador)
    lark_logger.setLevel(logging.DEBUG)
    try:
        _construir(parser, True, **opciones)
    finally:
        lark_logger.removeHandler(manejador)
        for h in propios:
            lark_logger.addHandler(h)
        lark_logger.setLevel(nivel)
    texto = opciones["texto"]
    avisos = []
    for l in registro.getvalue().split("\n"):
        m = re.match(r"Unused terminals: (\[.*\])$", l)
        if m:
            # Los nombres, sean cadenas o Token('TERMINAL', ...) (los que
            # una librería importa llegan como Token).
            nombres = [a or b for a, b in re.findall(
                r"Token\('[A-Z]+', '([^']+)'\)|'([^']+)'", m.group(1))]
            sobran = [n for n in nombres if n not in _fragmentos(nombres, texto)]
            if sobran:
                avisos.append(f"Unused terminals: {sobran}")
        elif "onflict" in l or "ollision" in l or "Unused" in l:
            avisos.append(l)
    if avisos:
        raise GramaticaConConflictos("\n".join(avisos))
    return _construir(parser, False, **opciones)


@dataclass
class Veredicto:
    valido: bool
    arbol: Tree | None = None
    codigo: str | None = None
    linea: int | None = None
    columna: int | None = None
    detalle: str | None = None


def _normalizar(texto: str) -> str:
    # LEX-07: CRLF se normaliza a LF; un CR suelto es SYN001.
    texto = texto.replace("\r\n", "\n")
    if "\r" in texto:
        pos = texto.index("\r")
        linea = texto.count("\n", 0, pos) + 1
        raise ErrorDeSintaxis("SYN001", linea, 1, "CR suelto")
    if not texto.endswith("\n"):
        texto += "\n"
    return texto


def _dentro_de_print_block(e: UnexpectedToken) -> bool:
    """El `print` seguido de `:` aún sin reducir en la pila del parser
    dice que el fallo ocurrió dentro del cuerpo de un bloque `print:`."""
    ip = getattr(e, "interactive_parser", None)
    if ip is None:
        return False
    pila = list(ip.parser_state.value_stack)
    for i in range(len(pila) - 1, -1, -1):
        v = pila[i]
        if isinstance(v, Token) and v.type in ("PRINT", "_PRINT"):
            sig = pila[i + 1] if i + 1 < len(pila) else None
            return isinstance(sig, Token) and sig.type == "COLON"
    return False


_SECCIONES = frozenset({"import", "source", "constant", "constants", "state",
                        "class", "can", "functions", "watch", "tests", "start",
                        "public"})


def _otro_terminal(parser: Lark, texto: str) -> str | None:
    """El primer terminal que no es BLOCK_TEXT y casa al principio de
    `texto`, por prioridad; None si ninguno casa."""
    import re as _re
    for t in sorted(parser.terminals, key=lambda t: -t.priority):
        if t.name == "BLOCK_TEXT" or t.name.startswith("_"):
            continue
        if _re.match(t.pattern.to_regexp(), texto, _re.M):
            return t.name
    return None


def decidir(parser: Lark, texto: str, inicio: str = INICIOS[0]) -> Veredicto:
    try:
        arbol = parser.parse(_normalizar(texto), start=inicio)
        return Veredicto(True, arbol=arbol)
    except ErrorDeSintaxis as e:
        return Veredicto(False, codigo=e.codigo, linea=e.linea,
                         columna=e.columna, detalle=e.detalle)
    except UnexpectedCharacters as e:
        # Ningún terminal admite el carácter: SYN001 (RUL-04).
        return Veredicto(False, codigo="SYN001", linea=e.line,
                         columna=e.column,
                         detalle=f"carácter inesperado {e.char!r}")
    except UnexpectedEOF as e:
        return Veredicto(False, codigo="SYN004", linea=e.line,
                         columna=e.column, detalle="fin de archivo")
    except UnexpectedToken as e:
        tok = e.token
        if _dentro_de_print_block(e):
            # STM-04, STMG-06: cuerpo vacío o sentencia dentro de `print:`.
            return Veredicto(False, codigo="SYN008", linea=tok.line,
                             columna=tok.column,
                             detalle="`print:` exige una expresión por línea")
        if tok.type == "BLOCK_TEXT":
            # BLOCK_TEXT solo lo admite el cuerpo de un bloque de librería
            # (BLKG-02); si el lexer de rescate lo ofrece en otro sitio, el
            # código depende de si algún otro terminal empieza ahí: un
            # token fuera de lugar es SYN002, un carácter que ninguno
            # admite es SYN001 (RUL-04).
            otro = _otro_terminal(parser, str(tok))
            if otro is None:
                return Veredicto(False, codigo="SYN001", linea=tok.line,
                                 columna=tok.column,
                                 detalle=f"carácter inesperado {str(tok)[:1]!r}")
            return Veredicto(False, codigo="SYN002", linea=tok.line,
                             columna=tok.column,
                             detalle=f"token inesperado {otro}")
        if tok.type == "$END":
            # Algo quedó abierto al llegar al final: SYN004 (RUL-07).
            return Veredicto(False, codigo="SYN004", linea=tok.line,
                             columna=tok.column,
                             detalle="construcción sin cerrar al final")
        if inicio == "source_file" and tok.column == 1:
            # FIL-01, FIL-02: en la columna 1 (sin tab, LEX-01) solo hay nivel superior. Una
            # sección que el archivo admite, puesta donde no toca, es
            # SYN007; cualquier otra cosa no tiene sitio: SYN009.
            if str(tok) in _SECCIONES:
                return Veredicto(False, codigo="SYN007", linea=tok.line,
                                 columna=tok.column,
                                 detalle=f"sección {str(tok)!r} fuera de orden")
            return Veredicto(False, codigo="SYN009", linea=tok.line,
                             columna=tok.column,
                             detalle=f"{str(tok)!r} no es una forma de nivel superior")
        if tok.type == "RESERVED":
            detalle = f"palabra reservada {str(tok)!r} en posición de nombre"
        elif tok.type in ("PRIMITIVE_TYPE", "LIST_NAME", "GENERIC_NAME"):
            detalle = f"type keyword {str(tok)!r} en posición de nombre"
        else:
            detalle = f"token inesperado {tok.type} {str(tok)!r}"
        return Veredicto(False, codigo="SYN002", linea=tok.line,
                         columna=tok.column, detalle=detalle)


def unidades_de_la_gramatica(parser: Lark) -> set[str]:
    """Cada regla o alternativa con nombre: la unidad de cobertura."""
    unidades = set()
    for r in parser.rules:
        nombre = r.alias or r.origin.name
        nombre = getattr(nombre, "value", nombre)
        if str(nombre).startswith("_"):
            continue
        unidades.add(str(nombre))
    return unidades


def unidades_ejercidas(arbol: Tree) -> set[str]:
    return {str(t.data) for t in arbol.iter_subtrees()}
