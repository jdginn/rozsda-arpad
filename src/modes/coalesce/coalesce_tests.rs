use crate::modes::coalesce::{Coalescible, OrderedCoalescingBuffer};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Msg {
    key: Option<u32>,
    value: i32,
}

impl Coalescible for Msg {
    type Key = u32;

    fn key(&self) -> Option<Self::Key> {
        self.key
    }

    fn merge_from(&mut self, newer: Self) {
        // last-write-wins
        self.value = newer.value;
    }
}

#[test]
fn coalesces_same_key_last_write_wins() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: Some(1),
        value: 10,
    });
    b.push(Msg {
        key: Some(1),
        value: 99,
    });

    let out: Vec<_> = b.drain().collect();
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0],
        Msg {
            key: Some(1),
            value: 99
        }
    );
}

#[test]
fn preserves_first_seen_key_order() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: Some(2),
        value: 20,
    });
    b.push(Msg {
        key: Some(1),
        value: 10,
    });
    b.push(Msg {
        key: Some(2),
        value: 21,
    }); // merge into first slot
    b.push(Msg {
        key: Some(3),
        value: 30,
    });

    let out: Vec<_> = b.drain().collect();
    assert_eq!(
        out,
        vec![
            Msg {
                key: Some(2),
                value: 21
            },
            Msg {
                key: Some(1),
                value: 10
            },
            Msg {
                key: Some(3),
                value: 30
            },
        ]
    );
}

#[test]
fn none_key_never_coalesces() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: None,
        value: 1,
    });
    b.push(Msg {
        key: None,
        value: 2,
    });

    let out: Vec<_> = b.drain().collect();
    assert_eq!(
        out,
        vec![
            Msg {
                key: None,
                value: 1
            },
            Msg {
                key: None,
                value: 2
            },
        ]
    );
}

#[test]
fn mixed_some_and_none_behavior() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: Some(1),
        value: 10,
    });
    b.push(Msg {
        key: None,
        value: 100,
    });
    b.push(Msg {
        key: Some(1),
        value: 11,
    }); // coalesce
    b.push(Msg {
        key: None,
        value: 101,
    });

    let out: Vec<_> = b.drain().collect();
    assert_eq!(
        out,
        vec![
            Msg {
                key: Some(1),
                value: 11
            },
            Msg {
                key: None,
                value: 100
            },
            Msg {
                key: None,
                value: 101
            },
        ]
    );
}

#[test]
fn clear_resets_items_and_index() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: Some(1),
        value: 1,
    });
    b.push(Msg {
        key: Some(1),
        value: 2,
    });
    assert_eq!(b.len(), 1);

    b.clear();
    assert!(b.is_empty());

    // ensure index is reset too
    b.push(Msg {
        key: Some(1),
        value: 3,
    });
    let out: Vec<_> = b.drain().collect();
    assert_eq!(
        out,
        vec![Msg {
            key: Some(1),
            value: 3
        }]
    );
}

#[test]
fn drain_clears_key_index_for_reuse() {
    let mut b = OrderedCoalescingBuffer::<Msg>::new();

    b.push(Msg {
        key: Some(1),
        value: 1,
    });
    let _: Vec<_> = b.drain().collect();

    // If index wasn't cleared, this could misbehave.
    b.push(Msg {
        key: Some(1),
        value: 2,
    });
    b.push(Msg {
        key: Some(1),
        value: 3,
    });

    let out: Vec<_> = b.drain().collect();
    assert_eq!(
        out,
        vec![Msg {
            key: Some(1),
            value: 3
        }]
    );
}
