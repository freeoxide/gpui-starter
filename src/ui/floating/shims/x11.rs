use std::ffi::c_void;
use std::ptr::NonNull;

use x11rb::connection::Connection as _;
use x11rb::protocol::shape::ConnectionExt as _;
use x11rb::protocol::shape::{SK, SO};
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::{
    Atom, ClientMessageEvent, ClipOrdering, EventMask, Window as X11Window,
};
use x11rb::xcb_ffi::XCBConnection;

use super::LOG;

#[cfg(test)]
#[path = "x11.test.rs"]
mod x11_test;

const STATE_ATOM_NAMES: [&str; 5] = [
    "_NET_WM_STATE",
    "_NET_WM_STATE_ABOVE",
    "_NET_WM_STATE_BELOW",
    "_NET_WM_STATE_SKIP_TASKBAR",
    "_NET_WM_STATE_STICKY",
];

struct StateAtoms {
    net_wm_state: Atom,
    above: Atom,
    below: Atom,
    skip_taskbar: Atom,
    sticky: Atom,
}

const STATE_ADD: u32 = 1;
const STATE_REMOVE: u32 = 0;

fn state_messages(
    atoms: &StateAtoms,
    action: u32,
    target: X11Window,
    states: &[Atom],
) -> Vec<ClientMessageEvent> {
    states
        .chunks(2)
        .map(|pair| {
            let second = pair.get(1).copied().unwrap_or(0);
            ClientMessageEvent::new(
                32,
                target,
                atoms.net_wm_state,
                [action, pair[0], second, 1, 0],
            )
        })
        .collect()
}

fn wrap_connection(ptr: NonNull<c_void>) -> Option<XCBConnection> {
    // SAFETY: ptr is the live xcb_connection_t owned by gpui's X11 client
    // (handed out through HasDisplayHandle); should_drop = false keeps the
    // wrapper from disconnecting it, and the wrapper is dropped before the
    // shim call returns.
    let conn = unsafe { XCBConnection::from_raw_xcb_connection(ptr.as_ptr(), false) }.ok()?;
    Some(conn)
}

fn intern_state_atoms(conn: &XCBConnection) -> Option<StateAtoms> {
    let cookies = STATE_ATOM_NAMES.map(|name| conn.intern_atom(true, name.as_bytes()));
    let atoms: Option<Vec<Atom>> = cookies
        .into_iter()
        .map(|cookie| cookie.ok()?.reply().ok().map(|reply| reply.atom))
        .collect();
    let [net_wm_state, above, below, skip_taskbar, sticky] = atoms?.try_into().ok()?;
    let atoms = StateAtoms {
        net_wm_state,
        above,
        below,
        skip_taskbar,
        sticky,
    };
    if [
        atoms.net_wm_state,
        atoms.above,
        atoms.below,
        atoms.skip_taskbar,
        atoms.sticky,
    ]
    .contains(&0)
    {
        tracing::debug!(
            target: LOG,
            "EWMH state atoms missing; the window manager does not speak EWMH"
        );
        return None;
    }
    Some(atoms)
}

fn send_state_messages(conn: &XCBConnection, root: X11Window, msgs: &[ClientMessageEvent]) -> bool {
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    for msg in msgs {
        if let Err(err) = conn.send_event(false, root, mask, *msg) {
            tracing::debug!(target: LOG, error = %err, "sending _NET_WM_STATE message failed");
            return false;
        }
    }
    conn.flush().is_ok()
}

pub(crate) fn set_click_through(
    window_id: u32,
    connection: NonNull<c_void>,
    _screen: i32,
    enabled: bool,
) -> bool {
    let Some(conn) = wrap_connection(connection) else {
        return false;
    };
    let request = if enabled {
        conn.shape_rectangles(
            SO::SET,
            SK::INPUT,
            ClipOrdering::UNSORTED,
            window_id,
            0,
            0,
            &[],
        )
    } else {
        conn.shape_combine(SO::SET, SK::INPUT, SK::BOUNDING, window_id, 0, 0, window_id)
    };
    match request.and_then(|_| conn.flush()) {
        Ok(()) => true,
        Err(err) => {
            tracing::debug!(target: LOG, error = %err, "XShape input region request failed");
            false
        }
    }
}

pub(crate) fn set_always_on_top(
    window_id: u32,
    connection: NonNull<c_void>,
    screen: i32,
    enabled: bool,
) -> bool {
    let Some(conn) = wrap_connection(connection) else {
        return false;
    };
    let Some(atoms) = intern_state_atoms(&conn) else {
        return false;
    };
    let Some(root) = conn.setup().roots.get(screen as usize) else {
        tracing::debug!(target: LOG, "screen {} has no X11 root window", screen);
        return false;
    };
    let action = if enabled { STATE_ADD } else { STATE_REMOVE };
    let messages = state_messages(
        &atoms,
        action,
        window_id,
        &[atoms.above, atoms.skip_taskbar, atoms.sticky],
    );
    send_state_messages(&conn, root.root, &messages)
}
