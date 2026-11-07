use super::*;

fn atoms() -> StateAtoms {
    StateAtoms {
        net_wm_state: 10,
        above: 11,
        skip_taskbar: 13,
        sticky: 14,
    }
}

#[test]
fn state_atom_names_are_byte_exact() {
    assert_eq!(
        STATE_ATOM_NAMES,
        [
            "_NET_WM_STATE",
            "_NET_WM_STATE_ABOVE",
            "_NET_WM_STATE_SKIP_TASKBAR",
            "_NET_WM_STATE_STICKY",
        ]
    );
}

#[test]
fn always_on_top_states_pins_the_full_bundle() {
    let atoms = atoms();
    assert_eq!(
        always_on_top_states(&atoms),
        [atoms.above, atoms.skip_taskbar, atoms.sticky],
        "always-on-top is ABOVE plus SKIP_TASKBAR plus STICKY, not ABOVE alone"
    );
}

#[test]
fn state_action_maps_enabled_onto_the_ewmh_actions() {
    assert_eq!(state_action(true), STATE_ADD);
    assert_eq!(state_action(false), STATE_REMOVE);
}

#[test]
fn the_production_bundle_chunks_into_two_messages_with_the_enabled_action() {
    let atoms = atoms();
    let msgs = state_messages(
        &atoms,
        state_action(true),
        4242,
        &always_on_top_states(&atoms),
    );
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].format, 32);
    assert_eq!(msgs[0].window, 4242);
    assert_eq!(msgs[0].type_, atoms.net_wm_state);
    assert_eq!(
        msgs[0].data.as_data32(),
        [STATE_ADD, atoms.above, atoms.skip_taskbar, 1, 0]
    );
    assert_eq!(msgs[1].data.as_data32(), [STATE_ADD, atoms.sticky, 0, 1, 0]);
}

#[test]
fn state_messages_yields_one_message_for_a_single_state() {
    let atoms = atoms();
    let msgs = state_messages(&atoms, STATE_REMOVE, 7, &[atoms.above]);
    assert_eq!(msgs.len(), 1);
    assert_eq!(
        msgs[0].data.as_data32(),
        [STATE_REMOVE, atoms.above, 0, 1, 0]
    );
}

#[test]
fn state_messages_yields_nothing_for_no_states() {
    assert!(state_messages(&atoms(), STATE_ADD, 7, &[]).is_empty());
}
