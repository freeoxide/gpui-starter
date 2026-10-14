use super::*;

fn atoms() -> StateAtoms {
    StateAtoms {
        net_wm_state: 10,
        above: 11,
        below: 12,
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
            "_NET_WM_STATE_BELOW",
            "_NET_WM_STATE_SKIP_TASKBAR",
            "_NET_WM_STATE_STICKY",
        ]
    );
}

#[test]
fn state_messages_chunks_three_states_into_two_messages() {
    let atoms = atoms();
    let target = 4242;
    let msgs = state_messages(
        &atoms,
        STATE_ADD,
        target,
        &[atoms.above, atoms.skip_taskbar, atoms.sticky],
    );
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0].format, 32);
    assert_eq!(msgs[0].window, target);
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

#[test]
fn state_messages_carries_the_below_atom_when_requested() {
    let atoms = atoms();
    let msgs = state_messages(&atoms, STATE_ADD, 7, &[atoms.below]);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].data.as_data32(), [STATE_ADD, atoms.below, 0, 1, 0]);
}
