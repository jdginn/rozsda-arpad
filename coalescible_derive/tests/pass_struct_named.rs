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
struct Msg {
    id: u32,
    #[data]
    value: u32,
}

fn main() {
    let mut a = Msg { id: 1, value: 10 };
    let b = Msg { id: 1, value: 99 };
    let k = a.key();
    assert!(k.is_some());
    a.merge_from(b);
    assert_eq!(a.value, 99);
}
