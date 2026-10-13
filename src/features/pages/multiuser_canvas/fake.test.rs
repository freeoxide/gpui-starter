use std::collections::HashMap;

use collab_protocol::{WireOp, WireShape};

use super::{DEMO_HEIGHT, DEMO_WIDTH, FakePeerDriver, PEER_COUNT, PeerEvent};

fn drained(driver: &mut FakePeerDriver, ticks: u64) -> Vec<PeerEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        events.extend(driver.advance());
    }
    events
}

#[test]
fn same_seed_produces_identical_event_streams() {
    let mut first = FakePeerDriver::new(0x5EED_CAFE);
    let mut second = FakePeerDriver::new(0x5EED_CAFE);
    assert_eq!(drained(&mut first, 500), drained(&mut second, 500));
}

#[test]
fn op_versions_are_strictly_increasing_per_origin() {
    let mut driver = FakePeerDriver::new(0xABCDEF);
    let mut last: HashMap<u64, u64> = HashMap::new();
    for event in drained(&mut driver, 500) {
        if let PeerEvent::Op(WireOp::Add { version, .. })
        | PeerEvent::Op(WireOp::Update { version, .. })
        | PeerEvent::Op(WireOp::Delete { version, .. }) = event
        {
            let previous = last.insert(version.origin, version.lamport);
            assert!(
                previous.is_none_or(|lamport| lamport < version.lamport),
                "origin {} went backwards: {previous:?} -> {}",
                version.origin,
                version.lamport
            );
        }
    }
}

#[test]
fn cursors_stay_inside_the_demo_area() {
    let mut driver = FakePeerDriver::new(0x1234);
    for event in drained(&mut driver, 300) {
        if let PeerEvent::Cursor(msg) = event {
            assert!(
                ((0.)..=DEMO_WIDTH).contains(&msg.doc[0]),
                "x out of range: {:?}",
                msg.doc
            );
            assert!(
                ((0.)..=DEMO_HEIGHT).contains(&msg.doc[1]),
                "y out of range: {:?}",
                msg.doc
            );
        }
    }
}

#[test]
fn two_peers_emit_all_three_op_kinds_within_two_hundred_ticks() {
    let mut driver = FakePeerDriver::new(5182);
    assert_eq!(driver.peer_list().len(), PEER_COUNT);
    let (mut adds, mut updates, mut deletes) = (0, 0, 0);
    for event in drained(&mut driver, 200) {
        match event {
            PeerEvent::Op(WireOp::Add { .. }) => adds += 1,
            PeerEvent::Op(WireOp::Update { .. }) => updates += 1,
            PeerEvent::Op(WireOp::Delete { .. }) => deletes += 1,
            PeerEvent::Cursor(_) => {}
        }
    }
    assert!(adds > 0, "no adds emitted");
    assert!(updates > 0, "no updates emitted");
    assert!(deletes > 0, "no deletes emitted");
}

#[test]
fn emitted_paths_carry_ten_to_sixteen_points() {
    let mut driver = FakePeerDriver::new(5182);
    for event in drained(&mut driver, 200) {
        if let PeerEvent::Op(WireOp::Add {
            shape: WireShape::Path { points, .. },
            ..
        }) = event
        {
            assert!((10..=16).contains(&points.len()));
        }
    }
}
