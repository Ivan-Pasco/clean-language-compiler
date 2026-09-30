"""Verificación de la ley formal (FS-01 a FS-05), de un golpe.

1. La gramática del lenguaje, compuesta desde la ley, construye LALR(1)
   con el lexer contextual y sin un solo conflicto.
2. Cada gramática de librería, compuesta igual, construye sin conflicto,
   y lee los ejemplos de su capítulo que casos_librerias.py nombra.
3. Etapa A: los casos de sintaxis contra la gramática.
4. Etapa B: las marcas de semántica contra Lean 4 (con --semantica).

Sale 0 solo si todo está conforme.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import casos_librerias
import componer
import sintaxis

AQUI = Path(__file__).parent


def gramaticas() -> list[str]:
    fallos = []
    try:
        sintaxis.cargar()
        print("lenguaje: construye sin conflictos "
              f"({len(componer.companions_del_lenguaje())} companions)")
    except Exception as e:  # noqa: BLE001 — el informe es el fallo
        fallos.append(f"lenguaje: {e}")
    for ruta in componer.companions_de_librerias():
        try:
            parser = sintaxis.cargar(texto=componer.libreria(ruta))
        except Exception as e:  # noqa: BLE001
            fallos.append(f"{ruta.name}: {e}")
            continue
        casos = [c for c in casos_librerias.CASOS
                 if c["libreria"] == ruta.name.removesuffix(".lark.md")]
        malos = []
        for c in casos:
            try:
                v = sintaxis.decidir(parser, casos_librerias.codigo_de(c),
                                     c["inicio"])
            except LookupError as e:
                malos.append(str(e))
                continue
            obtenido = "valida" if v.valido else "invalida"
            if obtenido != c["sintaxis"]:
                malos.append(f"{c['id']}: se esperaba {c['sintaxis']} y salió "
                             f"{obtenido} ({v.codigo} en {v.linea}:{v.columna}"
                             f" — {v.detalle})")
        print(f"{ruta.name}: construye sin conflictos; casos: {len(casos)}"
              f"   conformes: {len(casos) - len(malos)}")
        fallos += [f"{ruta.name}: {m}" for m in malos]
    return fallos


def main() -> int:
    fallos = gramaticas()
    for f in fallos:
        print("FALLO", f)
    etapas = ["etapa_a.py"] + (["etapa_b.py"] if "--semantica" in sys.argv else [])
    codigo = 1 if fallos else 0
    for etapa in etapas:
        r = subprocess.run([sys.executable, str(AQUI / etapa)])
        codigo = codigo or r.returncode
    return codigo


if __name__ == "__main__":
    sys.exit(main())
