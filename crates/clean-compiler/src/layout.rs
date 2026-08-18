//! Linear-memory layout vocabulary (Platform 03 — Memory Model, MMD-01),
//! shared by pass [8] (value lowering) and pass [10] (emission) so the two
//! can never disagree about an address.
//!
//! Layout (MMD-01 §3.1):
//!
//! ```text
//! [0            .. 1024)         null-pointer guard (never written)
//! [1024         .. HEAP_START)   static data; empty-string constant first
//! [HEAP_START   .. current_end)  bump-allocated heap
//! ```
//!
//! `HEAP_START` is fixed at 1 MiB regardless of how much static data the
//! compiler emitted; hosts and libraries read the `__heap_start` global,
//! never a hardcoded offset.

/// Bytes left unmapped at address 0 so a null dereference traps (MMD-01).
pub const NULL_GUARD_BYTES: u32 = 1024;

/// Where static data begins; the 4-byte length-prefixed `""` constant
/// occupies the first 4 bytes, so every empty string shares this address
/// (MMD-01, MMD-04).
pub const DATA_SECTION_START: u32 = NULL_GUARD_BYTES;

/// The shared empty-string constant's address (`= DATA_SECTION_START`).
pub const EMPTY_STRING_ADDR: u32 = DATA_SECTION_START;

/// Bump-allocator start, fixed at 1 MiB (MMD-01).
pub const HEAP_START: u32 = 1_048_576;

/// Per the WASM spec.
pub const WASM_PAGE_SIZE: u32 = 65_536;

/// All heap allocations are 8-byte aligned (MMD-01).
pub const ALIGNMENT: u32 = 8;

/// `list<T>` header layout (MMD §3.4.1): length, capacity, compiler-local
/// element-type tag, padding; elements start at +16 so 8-byte leaves stay
/// aligned. The tag is NOT part of the ABI — stable within one compilation
/// only.
pub const LIST_LEN_OFFSET: u32 = 0;
pub const LIST_CAP_OFFSET: u32 = 4;
pub const LIST_TAG_OFFSET: u32 = 8;
pub const LIST_ELEMS_OFFSET: u32 = 16;

/// Global indices in the emitted core module. `__heap_start` is immutable
/// and `__heap_ptr` is the mutable bump pointer; both are exported under
/// those names (MMD-01: "guest-visible").
pub const HEAP_START_GLOBAL: u32 = 0;
pub const HEAP_PTR_GLOBAL: u32 = 1;

/// A memory tier row (Platform 05 §5.1, TIER-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier {
    pub name: &'static str,
    pub initial_bytes: u32,
    pub max_bytes: u32,
}

/// The five tiers, verbatim from TIER-01. `embedded` is reserved (not
/// shipped in V2) and is additionally unusable while `HEAP_START` sits at
/// its 1 MiB maximum — DISCOVERIES-M6 records the conflict.
pub const TIERS: [Tier; 5] = [
    Tier {
        name: "embedded",
        initial_bytes: 256 * 1024,
        max_bytes: 1024 * 1024,
    },
    Tier {
        name: "minimal",
        initial_bytes: 512 * 1024,
        max_bytes: 8 * 1024 * 1024,
    },
    Tier {
        name: "standard",
        initial_bytes: 2 * 1024 * 1024,
        max_bytes: 32 * 1024 * 1024,
    },
    Tier {
        name: "heavy",
        initial_bytes: 4 * 1024 * 1024,
        max_bytes: 64 * 1024 * 1024,
    },
    Tier {
        name: "canvas",
        initial_bytes: 16 * 1024 * 1024,
        max_bytes: 64 * 1024 * 1024,
    },
];

pub fn tier(name: &str) -> Option<Tier> {
    TIERS.into_iter().find(|t| t.name == name)
}

/// TIER-01 inference when the request's `[memory].tier` is absent:
/// `wasm32-server`/`wasm32-browser` → `standard`, `wasm32-cli` →
/// `minimal`. `canvas` is never inferred. Targets the table does not name
/// default to `standard`.
pub fn infer_tier(target: &str) -> Tier {
    let name = match target {
        "wasm32-cli" => "minimal",
        _ => "standard",
    };
    tier(name).expect("inference names only table rows")
}

impl Tier {
    pub fn initial_pages(&self) -> u64 {
        self.initial_bytes.div_ceil(WASM_PAGE_SIZE) as u64
    }
    pub fn max_pages(&self) -> u64 {
        self.max_bytes.div_ceil(WASM_PAGE_SIZE) as u64
    }
}

/// The TIER-02 growth computation, kept host-side only as a test oracle for
/// the emitted allocator (the allocator emits this arithmetic inline).
#[cfg(test)]
pub fn grow_target(current_bytes: u32, needed_bytes: u32, tier_max: u32) -> u32 {
    let amortized = (current_bytes as u64) * 3 / 2;
    let floor = current_bytes as u64 + 4 * WASM_PAGE_SIZE as u64;
    let target = (needed_bytes as u64).max(amortized).max(floor);
    target.min(tier_max as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_mmd01() {
        assert_eq!(EMPTY_STRING_ADDR, 1024);
        assert_eq!(DATA_SECTION_START, 1024);
        assert_eq!(HEAP_START, 1_048_576);
        assert_eq!(HEAP_START % ALIGNMENT, 0);
    }

    #[test]
    fn tier_table_matches_tier01() {
        assert_eq!(tier("standard").unwrap().initial_pages(), 32);
        assert_eq!(tier("standard").unwrap().max_pages(), 512);
        assert_eq!(tier("minimal").unwrap().max_pages(), 128);
        assert!(tier("turbo").is_none());
    }

    #[test]
    fn growth_follows_tier02() {
        let max = tier("standard").unwrap().max_bytes;
        // 4-page floor dominates while current memory is small (< 512 KiB,
        // where 1.5× grows by less than 4 pages).
        assert_eq!(
            grow_target(256 * 1024, 256 * 1024 + 8, max),
            256 * 1024 + 4 * 65_536
        );
        // 1.5× amortization dominates once current memory is large.
        assert_eq!(
            grow_target(16 * 1024 * 1024, 16 * 1024 * 1024 + 8, max),
            24 * 1024 * 1024
        );
        // The tier clips.
        assert_eq!(grow_target(24 * 1024 * 1024, 33 * 1024 * 1024, max), max);
    }
}
