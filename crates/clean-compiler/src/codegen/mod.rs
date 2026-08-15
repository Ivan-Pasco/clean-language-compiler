//! Passes [9] and [10] — World Import Check and Codegen + Assembly
//! (Platform 14 §14.4.2).
//!
//! - `world`: parses `target_world.wit` and resolves the selected world
//!   (Milestone 1 step 3).
//! - `component`: emits the core module and wraps it as a Component Model
//!   component with the world's WIT embedded (steps 4 and 8).
//! - `world_check`: pass [9], `COM012` on any host call site absent from the
//!   target world (step 7).
//! - `abi`: Canonical ABI lift/lower (step 6).

pub mod component;
pub mod core;
pub mod world;
pub mod world_check;
