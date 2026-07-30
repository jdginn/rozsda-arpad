use coalescible_derive::CoalescibleEnum;

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

#[derive(Clone, CoalescibleEnum)]
enum LedState {
    Off,
    #[nocoalesce]
    Flash,
    On,
}

fn main() {
    assert_eq!(LedState::Off.key(), Some(()));
    assert_eq!(LedState::Flash.key(), None);
}
