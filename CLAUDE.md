# CLAUDE.md

## Reglas

1. **No usar nunca registros en memoria** (el directorio de auto-memoria de Claude). Si hay que recordar algo, escribirlo aquí, en CLAUDE.md.
2. **Toda verificación es doble: specs y código.** Cuando el usuario pida verificar algo, comprobarlo tanto en `specs/` como en el código fuente, mostrar las discrepancias encontradas entre ambos, y dar recomendaciones para cerrar esas diferencias.
3. **La spec vive en `specs/` y el contrato compiler↔hosts en `contracts/`** — copias vendoreadas que son la autoridad de este repo (procedencia y registro de cambios en `specs/SOURCE.md` y `contracts/SOURCE.md`). No leer los checkouts hermanos (`../clean-language-foundation`, etc.); una ambigüedad de spec se resuelve editando `specs/` deliberadamente y anotándolo en su SOURCE.md. Un cambio a `contracts/` exige propagarlo a los repos de hosts a mano.
