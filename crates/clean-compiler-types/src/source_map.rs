//! `source-map.json` — maps WASM offsets back to `sources[].path` and byte
//! ranges (Platform 14 §14.1.2). Present when `optimization` is `debug` or
//! `release`, absent for `size`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMap {
    pub mappings: Vec<Mapping>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub wasm_offset: u64,
    /// Project-relative POSIX path from the request document.
    pub path: String,
    pub byte_start: u64,
    pub byte_end: u64,
}
