//! Pass [6] — Block Handler Expansion (Platform 14 §14.4.2, ADR-0004).
//! Executes library `compiletime` handlers in a sandboxed wasmtime
//! sub-instance (epoch interruption, memory cap, no host imports) and
//! splices the returned IR into the program.

pub mod sandbox;
