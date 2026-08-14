//! Pass [4] — Resolve (Platform 14 §14.4.2). Builds the module graph,
//! applies the folder-to-library mapping, binds every identifier. Unresolved
//! names become `Error` bindings so type checking can continue. Lands in
//! Milestone 1 step 5 (flat binding); the full module graph is M4.
