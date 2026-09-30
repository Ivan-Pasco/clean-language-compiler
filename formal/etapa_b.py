"""Etapa B del piloto: la semántica Lean contra los ejemplos de la ley."""
from __future__ import annotations

import json
import math
import struct
import subprocess
import sys
from collections import Counter
from pathlib import Path

import arbol
import sintaxis
from corpus import CASOS
from etapa_a import codigo_de

AQUI = Path(__file__).parent
LEAN = AQUI / "lean"


def generar_main(programas: list[tuple[str, str]]) -> str:
    out = ["import Clean", "open Clean", ""]
    for i, (_, termino) in enumerate(programas):
        out.append(f"def caso{i} : List Stmt := {termino}")
    out.append("")
    out.append("def casos : List (String × List Stmt) := [")
    filas = [f'  ("{id}", caso{i})' for i, (id, _) in enumerate(programas)]
    out.append(",\n".join(filas))
    out.append("]")
    out.append("")
    out.append("def main : IO Unit := do")
    out.append("  for (id, p) in casos do")
    out.append('    IO.println s!"{id}\\t{textoDe (correr p)}"')
    out.append("")
    return "\n".join(out)


def decimal_de_bits(bits: str) -> float:
    return struct.unpack(">d", int(bits).to_bytes(8, "big"))[0]


def leer_salida(texto: str) -> dict:
    """Cinco campos por línea: id, clase, tipo o código, valor o
    mensaje, y lo impreso (hex); ver Programa.lean `textoDe`."""
    out = {}
    for linea in texto.split("\n"):
        if not linea.strip():
            continue
        campos = linea.split("\t")
        id, clase = campos[0], campos[1]
        impreso = bytes.fromhex(campos[4]).decode("utf-8") \
            if len(campos) > 4 else ""
        if clase == "valor":
            tipo, crudo = campos[2], campos[3] if len(campos) > 3 else ""
            if tipo == "string":
                v = bytes.fromhex(crudo).decode("utf-8")
            elif tipo == "number":
                v = decimal_de_bits(crudo)
            else:
                v = crudo
            out[id] = dict(clase="valor", tipo=tipo, valor=v,
                           impreso=impreso)
        elif clase == "sin-valor":
            out[id] = dict(clase="sin-valor", impreso=impreso)
        else:
            mensaje = bytes.fromhex(campos[3]).decode("utf-8") \
                if len(campos) > 3 else ""
            out[id] = dict(clase=clase, codigo=campos[2], mensaje=mensaje,
                           impreso=impreso)
    return out


def mismo_valor(esperado: dict, obtenido: dict) -> bool:
    if esperado["tipo"] != obtenido["tipo"]:
        return False
    if esperado["tipo"] == "number":
        e = float(esperado["valor"])
        o = obtenido["valor"]
        if math.isnan(e):
            return math.isnan(o)
        return e == o
    return esperado["valor"] == obtenido["valor"]


def describir(o: dict) -> str:
    c = o["clase"]
    if c == "valor":
        return f"{o['tipo']} {o['valor']!r}"
    if c in ("estatico", "fallo"):
        return f"{c} {o['codigo']} «{o['mensaje']}»"
    if c == "sin-valor":
        return "sin valor"
    return f"{c}: {o['mensaje']}"


def comparar_impreso(caso: dict, o: dict) -> str | None:
    """STD-04: lo que el programa escribió, tal cual, si el caso lo marca."""
    if "imprime" not in caso:
        return None
    if o["clase"] in ("sin-especificar", "fuera-de-alcance"):
        return f"se esperaba una salida impresa y salió {describir(o)}"
    if o.get("impreso", "") != caso["imprime"]:
        return (f"se esperaba imprimir {caso['imprime']!r} y se imprimió "
                f"{o.get('impreso', '')!r}")
    return None


def comparar(esperado: dict, o: dict) -> str | None:
    c = esperado["clase"]
    if c == "pregunta":
        if o["clase"] != "sin-especificar":
            return (f"es una pregunta ({esperado['que']}) y la semántica "
                    f"respondió: {describir(o)}")
        return None
    if c == "valor":
        if o["clase"] != "valor" or not mismo_valor(esperado, o):
            return (f"se esperaba {esperado['tipo']} "
                    f"{esperado['valor']!r} y salió {describir(o)}")
        return None
    if c == "sin-valor":
        if o["clase"] != "sin-valor":
            return f"se esperaba sin valor y salió {describir(o)}"
        return None
    if c in ("estatico", "fallo"):
        if o["clase"] != c:
            return f"se esperaba {c} y salió {describir(o)}"
        if esperado["codigo"] and esperado["codigo"] != o["codigo"]:
            return (f"se esperaba {esperado['codigo']} y salió "
                    f"{o['codigo']}")
        if esperado.get("mensaje") and esperado["mensaje"] != o["mensaje"]:
            return (f"se esperaba el mensaje «{esperado['mensaje']}» y "
                    f"salió «{o['mensaje']}»")
        return None
    return f"clase esperada desconocida: {c}"


def main() -> int:
    parser = sintaxis.cargar()
    programas = []
    casos = {}
    sin_sintaxis = []
    for caso in CASOS:
        # El modelo de Lean lee secuencias de sentencias; un archivo
        # entero solo tiene etapa A.
        if caso.get("inicio", sintaxis.INICIOS[0]) != sintaxis.INICIOS[0]:
            continue
        v = sintaxis.decidir(parser, codigo_de(caso))
        if not v.valido:
            if caso.get("semantica") or "imprime" in caso:
                sin_sintaxis.append(
                    f"{caso['id']} ({caso.get('decidido') or 'abierto'})")
            continue
        ast = arbol.programa(v.arbol)
        programas.append((caso["id"], arbol.programa_lean(ast)))
        casos[caso["id"]] = caso

    (LEAN / "Main.lean").write_text(generar_main(programas),
                                    encoding="utf-8")
    c = subprocess.run(["lake", "build", "corpus"], cwd=LEAN,
                       capture_output=True, text=True)
    if c.returncode != 0:
        print("LEAN NO COMPILA")
        print("\n".join(l for l in (c.stdout + c.stderr).split("\n")
                        if not l.startswith("trace:"))[-3000:])
        return 1
    r = subprocess.run([str(LEAN / ".lake/build/bin/corpus")],
                       capture_output=True, text=True)
    salida = leer_salida(r.stdout)

    clases = Counter(o["clase"] for o in salida.values())
    con_marca = [i for i, c in casos.items()
                 if c.get("semantica") or "imprime" in c]
    malos = []
    for id in con_marca:
        problema = None
        if casos[id].get("semantica"):
            problema = comparar(casos[id]["semantica"], salida[id])
        problema = problema or comparar_impreso(casos[id], salida[id])
        if problema:
            malos.append((id, problema))

    print(f"programas evaluados en Lean: {len(salida)}")
    for k in ("valor", "sin-valor", "estatico", "fallo",
              "sin-especificar", "fuera-de-alcance"):
        print(f"   {k:18} {clases.get(k, 0)}")
    print(f"\ncasos con marca semántica: {len(con_marca)}   conformes: "
          f"{len(con_marca) - len(malos)}   con problema: {len(malos)}")
    for id, problema in malos:
        print(f"\n  {id}  [{casos[id]['origen']}]")
        for linea in codigo_de(casos[id]).split("\n"):
            print(f"      | {linea}")
        print(f"      -> {problema}")
    if sin_sintaxis:
        print(f"\ncon marca semántica pero sin pasar la sintaxis: "
              f"{', '.join(sin_sintaxis)}")

    huecos = sorted({o["mensaje"] for o in salida.values()
                     if o["clase"] == "sin-especificar"})
    print(f"\nhuecos de la ley alcanzados: {len(huecos)}")
    for h in huecos:
        ids = [i for i, o in salida.items()
               if o["clase"] == "sin-especificar" and o["mensaje"] == h]
        print(f"   - {h}  [{', '.join(ids)}]")

    (AQUI / "etapa_b.json").write_text(
        json.dumps(salida, ensure_ascii=False, indent=1, default=str),
        encoding="utf-8")
    return 1 if malos else 0


if __name__ == "__main__":
    sys.exit(main())
