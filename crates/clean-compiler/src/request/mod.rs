//! Pass [1] — Request Validation (Platform 14 §14.4.2).
//!
//! Deserializes and validates the request document, verifies every
//! `sources[].sha256`, applies `overrides`, and produces a
//! `ValidatedRequest`. Failure mode: `RQD###` diagnostics; the pipeline
//! stops immediately.

mod validate;

pub use validate::{from_json, validate, ValidatedRequest};
