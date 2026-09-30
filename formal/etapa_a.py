"""Etapa A del piloto: la gramática Lark contra los ejemplos de la ley."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import arbol
import componer
import sintaxis
from corpus import CASOS, CAPITULOS, GRAMATICAS

from componer import LEY
AQUI = Path(__file__).parent


def codigo_de(caso) -> str:
    if "codigo" in caso:
        return caso["codigo"]
    lineas = (LEY / caso["archivo"]).read_text(encoding="utf-8").split("\n")
    a, b = caso["lineas"]
    trozo = lineas[a - 1:b]
    # Un bloque de código dentro de una viñeta de markdown lleva la
    # sangría de la viñeta; el código rendido no la tiene.
    n = caso.get("dedenta", 0)
    if n:
        trozo = [l[n:] if l.startswith(" " * n) else l for l in trozo]
    return "\n".join(trozo)


def lineas_de_codigo(archivo: str) -> set[int]:
    """Líneas con código dentro de los bloques ```clean del capítulo."""
    out = set()
    dentro = False
    texto = (LEY / archivo).read_text(encoding="utf-8").split("\n")
    for n, linea in enumerate(texto, 1):
        if re.match(r"^\s*```clean\s*$", linea):
            dentro = True
            continue
        if dentro and re.match(r"^\s*```\s*$", linea):
            dentro = False
            continue
        if dentro and linea.strip() and not linea.strip().startswith("//"):
            out.add(n)
    return out


_REGLA = re.compile(r"^\s*[?!]?([a-z_][a-z0-9_]*)\s*:")
_ALIAS = re.compile(r"->\s*([a-z_][a-z0-9_]*)")


def unidades_por_companion() -> dict[str, set[str]]:
    """Las reglas y los alias que define cada companion del lenguaje: el
    nombre de una regla donde se escribe `nombre:`, el de un alias donde
    se escribe `-> alias`."""
    out = {}
    for ruta in componer.companions_del_lenguaje():
        nombres = set()
        for linea in componer.bloques(ruta).split("\n"):
            codigo = re.sub(r"\s//(\s.*)?$", "", linea)
            if codigo.lstrip().startswith("//"):
                continue
            m = _REGLA.match(codigo)
            if m:
                nombres.add(m.group(1))
            nombres |= set(_ALIAS.findall(codigo))
        out[ruta.name.removesuffix(".lark.md")] = nombres
    return out


def imprimir(casos) -> None:
    for r in casos:
        print(f"\n  {r['id']}  [{r['origen']}]")
        for linea in r["fuente"].split("\n"):
            print(f"      | {linea}")
        for p in r["problemas"]:
            print(f"      -> {p}")


def main() -> int:
    parser = sintaxis.cargar()
    resultados = []
    ejercidas: set[str] = set()

    for caso in CASOS:
        fuente = codigo_de(caso)
        inicio = caso.get("inicio", sintaxis.INICIOS[0])
        v = sintaxis.decidir(parser, fuente, inicio)
        r = dict(id=caso["id"], unidad=caso["unidad"],
                 origen=caso["origen"], esperado=caso["sintaxis"],
                 obtenido="valida" if v.valido else "invalida",
                 codigo=v.codigo, detalle=v.detalle, linea=v.linea,
                 fuente=fuente, problemas=[],
                 decidido=caso.get("decidido"))
        if r["esperado"] != r["obtenido"]:
            r["problemas"].append(
                f"se esperaba {r['esperado']} y salió {r['obtenido']}"
                + (f" ({v.codigo}: {v.detalle})" if v.codigo else ""))
        esperado = caso.get("codigo_sintaxis")
        if esperado and not v.valido and esperado != v.codigo:
            r["problemas"].append(
                f"se esperaba el código {esperado} y salió {v.codigo}"
                f" ({v.detalle})")
        if v.valido:
            ejercidas |= sintaxis.unidades_ejercidas(v.arbol)
        # El puente a Lean lee secuencias de sentencias; un archivo entero
        # solo se juzga por su sintaxis.
        if v.valido and inicio == sintaxis.INICIOS[0]:
            try:
                ast = arbol.programa(v.arbol)
                r["ast"] = ast
            except arbol.FueraDelPuente as e:
                r["problemas"].append(f"el puente no traduce: {e}")
                ast = None
            agrupa = caso.get("agrupa_como")
            if agrupa and ast is not None:
                v2 = sintaxis.decidir(parser, agrupa)
                if not v2.valido:
                    r["problemas"].append(
                        f"la forma agrupada no parsea: {agrupa}")
                else:
                    a1 = arbol.sin_parentesis(ast)
                    a2 = arbol.sin_parentesis(arbol.programa(v2.arbol))
                    if a1 != a2:
                        r["problemas"].append(
                            f"no agrupa como {agrupa}")
        resultados.append(r)

    todas = sintaxis.unidades_de_la_gramatica(parser)
    por_companion = unidades_por_companion()
    # Solo las unidades de los companions que el corpus cubre deben estar
    # todas ejercidas; las de los demás se informan, sin fallar.
    exigidas = set()
    for nombre in GRAMATICAS:
        exigidas |= por_companion[nombre] & todas
    sin_ejercer = sorted(exigidas - ejercidas)
    pendientes = {n: sorted((u & todas) - ejercidas)
                  for n, u in por_companion.items() if n not in GRAMATICAS}

    sin_cubrir = {}
    for cap in CAPITULOS:
        cubiertas = set()
        for caso in CASOS:
            if caso.get("archivo") == cap:
                a, b = caso["lineas"]
                cubiertas |= set(range(a, b + 1))
        sin_cubrir[cap] = sorted(lineas_de_codigo(cap) - cubiertas)

    malos = [r for r in resultados if r["problemas"]]
    abiertos = [r for r in malos if not r["decidido"]]
    decididos = [r for r in malos if r["decidido"]]
    print(f"casos: {len(resultados)}   conformes: "
          f"{len(resultados) - len(malos)}   abiertos: {len(abiertos)}   "
          f"decididos sin aterrizar: {len(decididos)}")
    if abiertos:
        print("\nABIERTOS")
        imprimir(abiertos)
    if decididos:
        print("\nDECIDIDOS, PENDIENTES DE ATERRIZAR EN LA LEY")
        for r in decididos:
            print(f"   {r['decidido']}  {r['id']}: {r['problemas'][0]}")
    print(f"\nunidades de la gramática: {len(todas)}   "
          f"de los companions cubiertos: {len(exigidas)}   "
          f"sin ejercer: {len(sin_ejercer)}")
    for u in sin_ejercer:
        print(f"      {u}")
    print(f"companions aún sin cubrir por el corpus: {len(pendientes)}   "
          f"unidades sin ejercer en ellos: "
          f"{sum(len(v) for v in pendientes.values())} (no cuentan)")
    for cap, faltan in sin_cubrir.items():
        print(f"\nlíneas de código de {cap.split('/')[-1]} sin caso: "
              f"{len(faltan)}")
        for n in faltan:
            print(f"      {n}")

    (AQUI / "etapa_a.json").write_text(
        json.dumps(resultados, ensure_ascii=False, indent=1, default=list),
        encoding="utf-8")
    return 1 if abiertos or sin_ejercer or any(sin_cubrir.values()) else 0


if __name__ == "__main__":
    sys.exit(main())
