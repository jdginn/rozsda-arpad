use crate::modes::coalesce::OrderedCoalescingBuffer;
use coalescible_derive::{Coalescible, CoalescibleEnum};

/// CASE: newtype enum variant marked #[nocoalesce]
#[derive(Clone, Debug, PartialEq, Eq, Coalescible)]
struct NonCoalescingEnvelope {
    // no #[data] fields needed for this test; variant-level nocoalesce will force key() = None at enum level
    id: u8,
}

/// CASE: struct with at least 2 fields, exactly one #[data]
#[derive(Clone, Debug, PartialEq, Eq, Coalescible)]
struct AddressedSingleData {
    channel: u8, // key field
    #[data]
    value: i32, // data field
}

/// CASE: struct with at least 2 key fields + at least 2 data fields
#[derive(Clone, Debug, PartialEq, Eq, Coalescible)]
struct MultiAddressMultiData {
    bus: u8,  // key
    slot: u8, // key
    #[data]
    text_a: i32, // data
    #[data]
    text_b: i32, // data
}

/// CASE: unit-variant enum payload (all data semantics -> Key = ())
#[derive(Clone, Debug, PartialEq, Eq, CoalescibleEnum)]
enum UnitPayloadState {
    Off,
    Blink,
}

/// Outer enum under test
#[derive(Clone, Debug, PartialEq, Eq, CoalescibleEnum)]
enum MixedEnvelope {
    #[nocoalesce]
    NonCoalescing(NonCoalescingEnvelope),

    SingleData(AddressedSingleData),

    MultiData(MultiAddressMultiData),

    UnitState(UnitPayloadState),
}

#[test]
fn derive_enum_last_write_wins_per_key() {
    let mut buf = OrderedCoalescingBuffer::<MixedEnvelope>::new();

    // same key -> should coalesce, last write wins
    buf.push(MixedEnvelope::SingleData(AddressedSingleData {
        channel: 1,
        value: 10,
    }));
    buf.push(MixedEnvelope::SingleData(AddressedSingleData {
        channel: 1,
        value: 99,
    }));

    let out: Vec<_> = buf.drain().collect();
    assert_eq!(out.len(), 1);

    match &out[0] {
        MixedEnvelope::SingleData(m) => {
            assert_eq!(m.channel, 1);
            assert_eq!(m.value, 99);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

#[test]
fn derive_enum_first_seen_key_order_is_preserved() {
    let mut buf = OrderedCoalescingBuffer::<MixedEnvelope>::new();

    // key order should be: (2), (1), then UnitState bucket
    buf.push(MixedEnvelope::SingleData(AddressedSingleData {
        channel: 2,
        value: 20,
    }));
    buf.push(MixedEnvelope::SingleData(AddressedSingleData {
        channel: 1,
        value: 10,
    }));
    // same key as first => coalesce in-place, do not move position
    buf.push(MixedEnvelope::SingleData(AddressedSingleData {
        channel: 2,
        value: 21,
    }));
    // Unit payload enum key = () => one bucket for UnitState variant
    buf.push(MixedEnvelope::UnitState(UnitPayloadState::Off));
    buf.push(MixedEnvelope::UnitState(UnitPayloadState::Blink)); // coalesces, stays at same position

    let out: Vec<_> = buf.drain().collect();
    assert_eq!(out.len(), 3);

    match &out[0] {
        MixedEnvelope::SingleData(m) => {
            assert_eq!(m.channel, 2);
            assert_eq!(m.value, 21);
        }
        other => panic!("unexpected first item: {other:?}"),
    }

    match &out[1] {
        MixedEnvelope::SingleData(m) => {
            assert_eq!(m.channel, 1);
            assert_eq!(m.value, 10);
        }
        other => panic!("unexpected second item: {other:?}"),
    }

    match &out[2] {
        MixedEnvelope::UnitState(s) => {
            assert_eq!(s, &UnitPayloadState::Blink);
        }
        other => panic!("unexpected third item: {other:?}"),
    }
}

#[test]
fn derive_enum_multikey_multidata_last_write_wins_only_for_same_key() {
    let mut buf = OrderedCoalescingBuffer::<MixedEnvelope>::new();

    // same key (bus=1, slot=7): should merge data
    buf.push(MixedEnvelope::MultiData(MultiAddressMultiData {
        bus: 1,
        slot: 7,
        text_a: 100,
        text_b: 200,
    }));
    buf.push(MixedEnvelope::MultiData(MultiAddressMultiData {
        bus: 1,
        slot: 7,
        text_a: 111,
        text_b: 222,
    }));

    // different key (bus=1, slot=8): separate item
    buf.push(MixedEnvelope::MultiData(MultiAddressMultiData {
        bus: 1,
        slot: 8,
        text_a: 300,
        text_b: 400,
    }));

    let out: Vec<_> = buf.drain().collect();
    assert_eq!(out.len(), 2);

    match &out[0] {
        MixedEnvelope::MultiData(m) => {
            assert_eq!((m.bus, m.slot), (1, 7));
            assert_eq!((m.text_a, m.text_b), (111, 222));
        }
        other => panic!("unexpected first item: {other:?}"),
    }

    match &out[1] {
        MixedEnvelope::MultiData(m) => {
            assert_eq!((m.bus, m.slot), (1, 8));
            assert_eq!((m.text_a, m.text_b), (300, 400));
        }
        other => panic!("unexpected second item: {other:?}"),
    }
}

#[test]
fn derive_enum_variant_nocoalesce_bypasses_coalescing() {
    let mut buf = OrderedCoalescingBuffer::<MixedEnvelope>::new();

    buf.push(MixedEnvelope::NonCoalescing(NonCoalescingEnvelope {
        id: 1,
    }));
    buf.push(MixedEnvelope::NonCoalescing(NonCoalescingEnvelope {
        id: 1,
    }));

    // #[nocoalesce] means key() == None at enum level, so both are retained
    let out: Vec<_> = buf.drain().collect();
    assert_eq!(out.len(), 2);
}
