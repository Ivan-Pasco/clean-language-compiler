//! Module for managing the string pool during code generation.

use std::collections::HashMap;

/// A pool for deduplicating strings used in the generated code.
pub(crate) struct StringPool {
    strings: Vec<String>,
    string_to_index: HashMap<String, usize>,
}

impl StringPool {
    /// Creates a new, empty string pool.
    pub(crate) fn new() -> Self {
        Self {
            strings: Vec::new(),
            string_to_index: HashMap::new(),
        }
    }

    /// Adds a string to the pool if it doesn't exist, returning its index.
    /// If the string already exists, returns the index of the existing entry.
    pub(crate) fn add_string(&mut self, s: &str) -> usize {
        if let Some(&index) = self.string_to_index.get(s) {
            return index;
        }
        
        let index = self.strings.len();
        self.strings.push(s.to_string());
        self.string_to_index.insert(s.to_string(), index);
        index
    }

    /// Gets a string slice from the pool by its index.
    pub(crate) fn get_string(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(|s| s.as_str())
    }
} 