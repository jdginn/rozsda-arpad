use coalescible_derive::{Coalescible, CoalescibleEnum};
use std::hash::Hash;

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
struct Single {
    id: u32,
    #[data]
    v: u32,
}

#[derive(Clone, CoalescibleEnum)]
enum Msg {
    A(Single),
    #[nocoalesce]
    Batch(Vec<Single>), // Vec<Single> does NOT implement Coalescible, should still compile
}

fn main() {
    let m = Msg::Batch(vec![]);
    assert_eq!(m.key(), None);
}
