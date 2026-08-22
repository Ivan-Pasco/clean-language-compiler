# contracts/ — El contrato compiler ↔ hosts

Este directorio es el domicilio del contrato binario del ecosistema: los
archivos WIT que declaran los huecos (imports) que el compiler emite en cada
guest `.wasm` y que los hosts rellenan en Rust. Decisión de gobernanza
(2026-08-21): el contrato vive en el repo del compiler — un compilador, N
hosts; los hosts consumen copias pinneadas de aquí.

## Contenido

| Archivo | Origen | Qué es |
|---|---|---|
| `host.wit` | `clean-server` @ `54ca10d1eb980afc146c195ab62d63e90c66b60f` (2026-08-12, árbol limpio) | `package clean:host@0.1.0` — el mundo `server`: routing, request/response, websocket, sse, y los envelopes `session/http-envelope` y `realtime/sockets` |

SHA-256 de la copia:
`c4aaba83494e63577cb798e1483ce6604c6e55660010c5d0ced3be0d2a6963de  host.wit`

## Estado y dirección

- Foundation `03 platform/wit/` nunca recibió los WIT (solo un README).
  El único contrato real del ecosistema era el `host.wit`
  en la raíz de clean-server (HCV-02).
- Esta copia está **pinneada** al commit de origen. Paso pendiente en el repo
  de clean-server: reapuntar su `host.wit` para que sea copia verificada de
  este directorio (test de deriva por SHA-256), invirtiendo la autoridad.
- El repo ya mantenía una copia de prueba en `tests/fixtures/wit/host.wit`
  (bytes pinneados por `vendored_wit.rs`); el test
  `contracts_host_wit_matches_fixture` obliga a que ambas copias sean
  byte-idénticas — un refresh es un cambio de tres piezas en el mismo commit
  (fixture, `contracts/host.wit`, `RECORDED_SHA256`).
- La prosa que explica y gobierna este contrato vive en foundation
  `03 platform/` (02 host bridge, 08 bridge versioning, 15/16/18
  component model). Un cambio aquí exige propagación deliberada a los hosts:
  ellos no lo ven solos.
- Los paquetes `wasi:*` (filesystem, http, clocks…) son estándar externo,
  consumidos por versión pinneada; no se vendorean aquí.

## Cambios locales desde la copia

Registrar aquí cada cambio al contrato (fecha, qué, por qué, y a qué hosts se
propagó). Vacío = fiel al origen.

(ninguno)
