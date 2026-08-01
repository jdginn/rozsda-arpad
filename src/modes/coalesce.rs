use std::collections::HashMap;
use std::hash::Hash;

/// A message that can be coalesced by "address key".
/// - `Key` = address fields
/// - `merge_from` = last-write-wins for data fields
pub trait Coalescible: Clone + std::fmt::Debug {
    type Key: Eq + Hash + Clone;

    fn key(&self) -> Option<Self::Key>;

    /// Merge newer message into existing buffered one.
    /// For most state messages this is last-write-wins.
    fn merge_from(&mut self, newer: Self);
}

/// Stable-order coalescing buffer:
/// - preserves first-seen key order
/// - keeps latest value per key
#[derive(Debug, Default)]
pub struct OrderedCoalescingBuffer<T: Coalescible> {
    items: Vec<T>,
    index_by_key: HashMap<T::Key, usize>,
}

impl<T: Coalescible> OrderedCoalescingBuffer<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            index_by_key: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.index_by_key.clear();
    }

    /// Insert a message or merge into existing key slot.
    pub fn push(&mut self, msg: T) {
        match msg.key() {
            Some(key) => {
                if let Some(&idx) = self.index_by_key.get(&key) {
                    println!("Coalescing message {:?}", msg);
                    self.items[idx].merge_from(msg);
                } else {
                    let idx = self.items.len();
                    self.items.push(msg);
                    self.index_by_key.insert(key, idx);
                }
            }
            None => {
                self.items.push(msg);
            }
        }
    }

    /// Drain in preserved order of first-seen keys.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        let drained = self.items.drain(..);
        self.index_by_key.clear();
        drained
    }
}

#[cfg(test)]
mod coalesce_derive_tests;
#[cfg(test)]
mod coalesce_tests;
