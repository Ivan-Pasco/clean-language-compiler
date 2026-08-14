//! Pass [6] — Block Handler Expansion (Platform 14 §14.4.2, ADR-0004).
//! Executes library `compiletime` handlers in a sandboxed wasmtime
//! sub-instance (epoch interruption, `StoreLimits`, no wall clock, seeded
//! randomness). Milestone 1 compiles programs without library blocks, so
//! this pass is a typed pass-through until M5.
