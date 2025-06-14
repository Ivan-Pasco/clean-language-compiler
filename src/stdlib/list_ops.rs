use std::collections::HashSet;
use crate::ast::{Value, ListBehavior};

/// List implementation with property-based behavior
#[derive(Debug, Clone)]
pub struct List {
    data: Vec<Value>,
    behavior: ListBehavior,
    unique_set: Option<HashSet<String>>, // For unique behavior tracking
}

impl List {
    pub fn new() -> Self {
        List {
            data: Vec::new(),
            behavior: ListBehavior::Default,
            unique_set: None,
        }
    }

    pub fn with_behavior(behavior: ListBehavior) -> Self {
        let unique_set = if matches!(behavior, 
            ListBehavior::Unique | 
            ListBehavior::LineUnique | 
            ListBehavior::PileUnique | 
            ListBehavior::LineUniquePile
        ) {
            Some(HashSet::new())
        } else {
            None
        };

        List {
            data: Vec::new(),
            behavior,
            unique_set,
        }
    }

    /// Set the behavior type of the list
    pub fn set_behavior(&mut self, behavior: ListBehavior) {
        // Initialize unique_set if needed
        if matches!(behavior, 
            ListBehavior::Unique | 
            ListBehavior::LineUnique | 
            ListBehavior::PileUnique | 
            ListBehavior::LineUniquePile
        ) && self.unique_set.is_none() {
            let mut unique_set = HashSet::new();
            // Add existing items to the set
            for item in &self.data {
                unique_set.insert(format!("{}", item));
            }
            self.unique_set = Some(unique_set);
        } else if !matches!(behavior, 
            ListBehavior::Unique | 
            ListBehavior::LineUnique | 
            ListBehavior::PileUnique | 
            ListBehavior::LineUniquePile
        ) {
            self.unique_set = None;
        }
        
        self.behavior = behavior;
    }

    /// Add an item to the list based on behavior
    pub fn add(&mut self, item: Value) -> bool {
        // Check uniqueness if required
        if let Some(ref mut unique_set) = self.unique_set {
            let item_str = format!("{}", item);
            if unique_set.contains(&item_str) {
                return false; // Item already exists, don't add
            }
            unique_set.insert(item_str);
        }

        match self.behavior {
            ListBehavior::Default | ListBehavior::Unique => {
                // Add to end (standard behavior)
                self.data.push(item);
            },
            ListBehavior::Line | ListBehavior::LineUnique => {
                // Queue: add to back
                self.data.push(item);
            },
            ListBehavior::Pile | ListBehavior::PileUnique => {
                // Stack: add to top (end for Vec)
                self.data.push(item);
            },
            ListBehavior::LinePile | ListBehavior::LineUniquePile => {
                // Combined: add to end (default behavior)
                self.data.push(item);
            },
        }
        true
    }

    /// Remove an item from the list based on behavior
    pub fn remove(&mut self) -> Option<Value> {
        if self.data.is_empty() {
            return None;
        }

        let removed_item = match self.behavior {
            ListBehavior::Default | ListBehavior::Unique => {
                // Remove from end (standard behavior)
                self.data.pop()
            },
            ListBehavior::Line | ListBehavior::LineUnique => {
                // Queue: remove from front (FIFO)
                if !self.data.is_empty() {
                    Some(self.data.remove(0))
                } else {
                    None
                }
            },
            ListBehavior::Pile | ListBehavior::PileUnique => {
                // Stack: remove from top (LIFO)
                self.data.pop()
            },
            ListBehavior::LinePile | ListBehavior::LineUniquePile => {
                // Combined: remove from front (queue-like)
                if !self.data.is_empty() {
                    Some(self.data.remove(0))
                } else {
                    None
                }
            },
        };

        // Update unique_set if item was removed
        if let (Some(ref item), Some(ref mut unique_set)) = (&removed_item, &mut self.unique_set) {
            let item_str = format!("{}", item);
            unique_set.remove(&item_str);
        }

        removed_item
    }

    /// Remove a specific item (for unique behavior)
    pub fn remove_item(&mut self, item: &Value) -> bool {
        let item_str = format!("{}", item);
        
        if let Some(pos) = self.data.iter().position(|x| format!("{}", x) == item_str) {
            self.data.remove(pos);
            
            // Update unique_set
            if let Some(ref mut unique_set) = self.unique_set {
                unique_set.remove(&item_str);
            }
            true
        } else {
            false
        }
    }

    /// Peek at the next item without removing it
    pub fn peek(&self) -> Option<&Value> {
        if self.data.is_empty() {
            return None;
        }

        match self.behavior {
            ListBehavior::Default | ListBehavior::Unique => {
                // Peek at last item
                self.data.last()
            },
            ListBehavior::Line | ListBehavior::LineUnique => {
                // Queue: peek at front
                self.data.first()
            },
            ListBehavior::Pile | ListBehavior::PileUnique => {
                // Stack: peek at top
                self.data.last()
            },
            ListBehavior::LinePile | ListBehavior::LineUniquePile => {
                // Combined: peek at front
                self.data.first()
            },
        }
    }

    /// Check if the list contains an item
    pub fn contains(&self, item: &Value) -> bool {
        if let Some(ref unique_set) = self.unique_set {
            let item_str = format!("{}", item);
            unique_set.contains(&item_str)
        } else {
            let item_str = format!("{}", item);
            self.data.iter().any(|x| format!("{}", x) == item_str)
        }
    }

    /// Get the size of the list
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get an item by index (standard list operation)
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    /// Set an item by index (standard list operation)
    pub fn set(&mut self, index: usize, item: Value) -> bool {
        if index < self.data.len() {
            // Check uniqueness if required
            if let Some(ref mut unique_set) = self.unique_set {
                let old_item_str = format!("{}", &self.data[index]);
                let new_item_str = format!("{}", item);
                
                if old_item_str != new_item_str && unique_set.contains(&new_item_str) {
                    return false; // New item already exists elsewhere
                }
                
                unique_set.remove(&old_item_str);
                unique_set.insert(new_item_str);
            }
            
            self.data[index] = item;
            true
        } else {
            false
        }
    }

    /// Get the current behavior
    pub fn get_behavior(&self) -> &ListBehavior {
        &self.behavior
    }

    /// Convert to Vec for iteration
    pub fn to_vec(&self) -> Vec<Value> {
        self.data.clone()
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.data.clear();
        if let Some(ref mut unique_set) = self.unique_set {
            unique_set.clear();
        }
    }
}

/// Parse behavior string to ListBehavior enum
pub fn parse_behavior(behavior_str: &str) -> Result<ListBehavior, String> {
    match behavior_str.to_lowercase().as_str() {
        "line" => Ok(ListBehavior::Line),
        "pile" => Ok(ListBehavior::Pile),
        "unique" => Ok(ListBehavior::Unique),
        "default" => Ok(ListBehavior::Default),
        _ => Err(format!("Unknown list behavior: {}", behavior_str))
    }
}

/// Combine multiple behaviors
pub fn combine_behaviors(behaviors: Vec<ListBehavior>) -> ListBehavior {
    let mut has_line = false;
    let mut has_pile = false;
    let mut has_unique = false;

    for behavior in behaviors {
        match behavior {
            ListBehavior::Line | ListBehavior::LineUnique | ListBehavior::LinePile | ListBehavior::LineUniquePile => {
                has_line = true;
            },
            ListBehavior::Pile | ListBehavior::PileUnique => {
                has_pile = true;
            },
            ListBehavior::Unique | ListBehavior::LineUnique | ListBehavior::PileUnique | ListBehavior::LineUniquePile => {
                has_unique = true;
            },
            _ => {}
        }
    }

    match (has_line, has_pile, has_unique) {
        (true, true, true) => ListBehavior::LineUniquePile,
        (true, false, true) => ListBehavior::LineUnique,
        (false, true, true) => ListBehavior::PileUnique,
        (true, true, false) => ListBehavior::LinePile,
        (true, false, false) => ListBehavior::Line,
        (false, true, false) => ListBehavior::Pile,
        (false, false, true) => ListBehavior::Unique,
        (false, false, false) => ListBehavior::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    #[test]
    fn test_default_behavior() {
        let mut list = List::new();
        assert!(list.add(Value::Integer(1)));
        assert!(list.add(Value::Integer(2)));
        assert_eq!(list.size(), 2);
        assert_eq!(list.remove(), Some(Value::Integer(2))); // Remove from end
    }

    #[test]
    fn test_line_behavior() {
        let mut list = List::with_behavior(ListBehavior::Line);
        assert!(list.add(Value::Integer(1)));
        assert!(list.add(Value::Integer(2)));
        assert_eq!(list.peek(), Some(&Value::Integer(1))); // Peek at front
        assert_eq!(list.remove(), Some(Value::Integer(1))); // Remove from front (FIFO)
        assert_eq!(list.remove(), Some(Value::Integer(2)));
    }

    #[test]
    fn test_pile_behavior() {
        let mut list = List::with_behavior(ListBehavior::Pile);
        assert!(list.add(Value::Integer(1)));
        assert!(list.add(Value::Integer(2)));
        assert_eq!(list.peek(), Some(&Value::Integer(2))); // Peek at top
        assert_eq!(list.remove(), Some(Value::Integer(2))); // Remove from top (LIFO)
        assert_eq!(list.remove(), Some(Value::Integer(1)));
    }

    #[test]
    fn test_unique_behavior() {
        let mut list = List::with_behavior(ListBehavior::Unique);
        assert!(list.add(Value::Integer(1)));
        assert!(list.add(Value::Integer(2)));
        assert!(!list.add(Value::Integer(1))); // Duplicate should be rejected
        assert_eq!(list.size(), 2);
        assert!(list.contains(&Value::Integer(1)));
        assert!(list.remove_item(&Value::Integer(1)));
        assert!(!list.contains(&Value::Integer(1)));
    }

    #[test]
    fn test_behavior_change() {
        let mut list = List::new();
        list.add(Value::Integer(1));
        list.add(Value::Integer(2));
        
        list.set_behavior(ListBehavior::Line);
        assert_eq!(list.peek(), Some(&Value::Integer(1))); // Now peeks at front
        
        list.set_behavior(ListBehavior::Pile);
        assert_eq!(list.peek(), Some(&Value::Integer(2))); // Now peeks at top
    }
} 