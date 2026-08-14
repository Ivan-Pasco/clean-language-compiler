//! Pass [8] — MIR Lowering + Optimize (Platform 14 §14.4.2). Linear,
//! SSA-shaped IR for direct WASM emission. Optimization profiles are
//! semantics-preserving by contract. Lands in Milestone 1 step 5 (no
//! optimization until the conformance suite exists to prove preservation).
