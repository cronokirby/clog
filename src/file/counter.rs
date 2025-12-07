use std::{collections::HashMap, hash::Hash};

/// A data structure assigning sequential counters to keys.
///
/// For example, assign sequential values to particular footnote identifiers.
#[derive(Default)]
pub struct Sequential<K> {
    next: u64,
    values: HashMap<K, u64>,
}

impl<K: Eq + Hash> Sequential<K> {
    /// Get the value for a given key.
    ///
    /// This needs mutable access in order to adjust the counter value, if we haven't
    /// seen this key before.
    pub fn value(&mut self, k: K) -> u64 {
        let entry = self.values.entry(k).or_insert_with(|| {
            let to_insert = self.next;
            self.next += 1;
            to_insert
        });
        *entry
    }
}
