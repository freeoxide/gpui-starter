use super::*;

fn rect_shape() -> WireShape {
    WireShape::Rect {
        bounds: [10., 20., 30., 40.],
        style: WireStyle {
            fill: Some(WireColor([210., 0.6, 0.5, 1.])),
            stroke: Some(WireStroke {
                width: 2.,
                color: WireColor([0., 0., 0., 1.]),
                dash: Some(vec![4., 2.]),
            }),
        },
    }
}

fn path_shape() -> WireShape {
    WireShape::Path {
        points: vec![[0., 0.], [10., 5.], [20., 0.]],
        stroke: WireStroke {
            width: 3.,
            color: WireColor([120., 0.7, 0.6, 1.]),
            dash: None,
        },
    }
}

fn key(origin: ClientId, seq: u32) -> ShapeKey {
    ShapeKey { origin, seq }
}

fn version(lamport: u64, origin: ClientId) -> Version {
    Version { lamport, origin }
}

#[test]
fn wire_ops_round_trip_through_json() {
    let ops = vec![
        WireOp::Add {
            key: key(7, 1),
            version: version(3, 7),
            shape: rect_shape(),
        },
        WireOp::Update {
            key: key(7, 1),
            version: version(4, 7),
            shape: path_shape(),
        },
        WireOp::Delete {
            key: key(7, 1),
            version: version(5, 7),
        },
    ];
    for op in ops {
        let json = serde_json::to_value(&op).unwrap();
        let back: WireOp = serde_json::from_value(json).unwrap();
        assert_eq!(back, op);
    }
}

#[test]
fn snapshot_round_trips_through_json() {
    let msg = SnapshotMsg {
        peers: vec![Peer {
            id: 9,
            name: "Ada".into(),
            color: WireColor([0., 0.5, 0.5, 1.]),
        }],
        shapes: vec![ShapeEntry {
            key: key(9, 1),
            version: version(2, 9),
            shape: rect_shape(),
        }],
    };
    let json = serde_json::to_value(&msg).unwrap();
    let back: SnapshotMsg = serde_json::from_value(json).unwrap();
    assert_eq!(back, msg);
}

#[test]
fn add_op_discriminant_is_snake_case_kind_tag() {
    let op = WireOp::Add {
        key: key(1, 1),
        version: version(1, 1),
        shape: rect_shape(),
    };
    let json = serde_json::to_value(&op).unwrap();
    assert_eq!(json["kind"], serde_json::json!("add"));
    let back: WireOp =
        serde_json::from_value(serde_json::json!({"kind": "add", "key": {"origin": 1, "seq": 1}, "version": {"lamport": 1, "origin": 1}, "shape": serde_json::to_value(rect_shape()).unwrap()})).unwrap();
    assert_eq!(back, op);
}

#[test]
fn stale_update_is_rejected_after_add() {
    let mut board = Board::new();
    let k = key(1, 1);
    assert!(board.integrate(k, version(2, 1), Change::Put(rect_shape())));
    assert!(!board.integrate(k, version(1, 1), Change::Put(path_shape())));
    assert_eq!(board.shape(&k).unwrap().shape, rect_shape());
}

#[test]
fn lamport_ties_are_broken_by_origin_in_both_directions() {
    let mut board = Board::new();
    let k = key(1, 1);
    assert!(board.integrate(k, version(5, 2), Change::Put(rect_shape())));
    assert!(!board.integrate(k, version(5, 1), Change::Put(path_shape())));
    assert!(board.integrate(k, version(5, 3), Change::Put(path_shape())));
    assert_eq!(board.shape(&k).unwrap().shape, path_shape());
}

#[test]
fn tombstone_blocks_lower_add_and_higher_add_resurrects() {
    let mut board = Board::new();
    let k = key(1, 1);
    assert!(board.integrate(k, version(2, 1), Change::Put(rect_shape())));
    assert!(board.integrate(k, version(3, 1), Change::Remove));
    assert!(board.shape(&k).is_none());
    assert!(!board.integrate(k, version(2, 1), Change::Put(rect_shape())));
    assert!(board.shape(&k).is_none());
    assert!(board.integrate(k, version(4, 1), Change::Put(path_shape())));
    assert_eq!(board.shape(&k).unwrap().shape, path_shape());
}

#[test]
fn next_version_and_observe_keep_lamports_monotonic() {
    let mut board = Board::new();
    let first = board.next_version(1);
    assert_eq!(first, version(1, 1));
    board.observe(version(9, 2));
    let second = board.next_version(1);
    assert!(second.lamport > 9);
    assert!(second > first);
    board.observe(version(3, 2));
    assert!(board.next_version(1).lamport > second.lamport);
}

#[test]
fn replace_all_swaps_state_and_raises_the_clock() {
    let mut board = Board::new();
    board.integrate(key(1, 1), version(4, 1), Change::Put(rect_shape()));
    board.integrate(key(1, 2), version(9, 1), Change::Remove);
    board.observe(version(9, 1));

    board.replace_all(vec![ShapeEntry {
        key: key(2, 1),
        version: version(6, 2),
        shape: path_shape(),
    }]);

    assert!(board.shape(&key(1, 1)).is_none());
    assert!(board.shape(&key(2, 1)).is_some());
    assert_eq!(board.lamport(), 10);
    assert!(board.next_version(1).lamport > 10);

    board.replace_all(vec![ShapeEntry {
        key: key(2, 2),
        version: version(50, 2),
        shape: path_shape(),
    }]);
    assert!(board.next_version(1).lamport > 50);
}

#[test]
fn clear_drops_state_and_keeps_the_clock_counting() {
    let mut board = Board::new();
    board.integrate(key(1, 1), version(4, 1), Change::Put(rect_shape()));
    board.observe(version(4, 1));
    board.clear();
    assert!(board.shapes().next().is_none());
    let after = board.next_version(1);
    assert!(after.lamport > 4);
}
