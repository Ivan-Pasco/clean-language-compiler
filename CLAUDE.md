# CLAUDE.md

## Reglas

1. **No usar nunca registros en memoria** (el directorio de auto-memoria de Claude). Si hay que recordar algo, escribirlo aquí, en CLAUDE.md.
2. **La spec vive en `specs/` y el contrato compiler↔hosts en `contracts/`** — copias vendoreadas que son la autoridad de este repo (procedencia y registro de cambios en `specs/SOURCE.md` y `contracts/SOURCE.md`). No leer los checkouts hermanos (`../clean-language-foundation`, etc.); una ambigüedad de spec se resuelve editando `specs/` deliberadamente y anotándolo en su SOURCE.md. Un cambio a `contracts/` exige propagarlo a los repos de hosts a mano.
