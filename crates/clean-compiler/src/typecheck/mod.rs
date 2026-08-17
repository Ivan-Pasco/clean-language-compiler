//! Pass [5] — Type Check (Platform 14 §14.4.2): bidirectional checking
//! with an `ena` inference context (M4) over the chapter-04 type surface,
//! plus the world-typed boundary projections of ADR-0002.

mod check;
pub mod infer;
pub mod tir;
pub mod types;

pub use check::check;
