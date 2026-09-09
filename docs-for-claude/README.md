# docs-for-claude

Notas que **Claude puede y debe leer** en futuras sesiones. Se alimenta con `/X40-capture`; `INDEX.md` dice qué nota carga cada skill.

## El test de una línea

> Capturar **solo lo que el código NO puede decir**: el *por qué*, el conocimiento negativo (qué se intentó y falló), invariantes no-locales y restricciones de sistemas externos.
> Si un ingeniero competente puede reconstruirlo leyendo el código, **no va acá**.

Cada nota se recarga cada vez que corre su skill: el umbral no es "¿es cierto?" sino **"¿su valor supera su costo recurrente de tokens?"**.

## Qué NO va acá

- Narrativa, historia, log de sesiones → `docs-for-humans/`.
- Trabajo pendiente / deuda → `backlog.md`.
- Dónde vive X / qué llama a Y → grep.
- Reglas inviolables del proyecto → `CLAUDE.md`.
- La spec del lenguaje y la arquitectura → repo hermano `../clean-language-foundation`.
