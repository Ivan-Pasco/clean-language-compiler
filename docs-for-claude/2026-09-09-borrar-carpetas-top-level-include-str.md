---
title: "Borrar carpetas top-level: los tests las incluyen por ruta"
captured: 2026-09-09
applies-to:
  - "*"
topics:
  - repo-layout
  - tests
  - ci
status: active
---

# Borrar carpetas top-level: los tests las incluyen por ruta

_Capturado 2026-09-09 mientras se sacaban la documentación y `contracts/` del repo._

**Regla.** Antes de borrar o mover una carpeta de primer nivel, grepear `include_str!`, `include_bytes!` y `CARGO_MANIFEST_DIR` en `crates/` y `tests/`. Los tests de integración incluyen archivos por ruta relativa (`../../../<carpeta>/...`), así que lo que se rompe es la **compilación** de los tests, no un test en runtime.

**Por qué el código no lo dice.** Nada avisa al hacer `git rm`: la carpeta parece "solo docs" y el fallo aparece recién en `cargo test --workspace`, o sea en CI después del push.

**Síntoma.** `error: couldn't read crates/<crate>/tests/../../../<carpeta>/<archivo>: No such file or directory` al compilar un crate de tests; CI en rojo en `main` tras un commit de limpieza.

**Caso que lo originó.** `vendored_wit.rs` comparaba `contracts/host.wit` byte a byte contra `tests/fixtures/wit/host.wit`. Al borrar `contracts/` hubo que quitar esa aserción; queda solo el pin sha256 del fixture.
