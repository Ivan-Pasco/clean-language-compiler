//! Pass [5] — Type Check (Platform 14 §14.4.2), Milestone 1 slice: direct
//! checking with the world-typed boundary projections of ADR-0002.
//! Bidirectional inference with `ena` union-find arrives in M4.

mod check;
pub mod tir;
pub mod types;

pub use check::check;
