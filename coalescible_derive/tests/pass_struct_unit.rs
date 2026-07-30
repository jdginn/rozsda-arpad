use coalescible_derive::Coalescible;

mod modes {
    pub mod coalesce {
        pub trait Coalescible: Clone {
            type Key: Eq + std::hash::Hash + Clone;
            fn key(&self) -> Option<Self::Key>;
            fn merge_from(&mut self, newer: Self);
        }
    }
}
use modes::coalesce::Coalescible as _;

#[derive(Clone, Coalescible)]
struct Tick;

fn main() {
    assert_eq!(Tick.key(), Some(()));
}
