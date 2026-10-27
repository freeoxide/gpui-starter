use super::SharedTransport;

#[test]
fn volume_roundtrips_bit_exact() {
    let transport = SharedTransport::new(0.8);
    for volume in [0.0, 0.5, 1.0] {
        transport.set_volume(volume);
        assert_eq!(transport.volume().to_bits(), volume.to_bits());
    }
}

#[test]
fn cursor_stores_and_loads() {
    let transport = SharedTransport::new(1.);
    assert_eq!(transport.cursor_frames(), 0);
    transport.set_cursor(44_100);
    assert_eq!(transport.cursor_frames(), 44_100);
}

#[test]
fn eof_is_latched_once_then_cleared() {
    let transport = SharedTransport::new(1.);
    assert!(!transport.take_eof());
    transport.set_eof();
    assert!(transport.take_eof());
    assert!(!transport.take_eof());
}

#[test]
fn playing_toggles() {
    let transport = SharedTransport::new(1.);
    assert!(!transport.is_playing());
    transport.set_playing(true);
    assert!(transport.is_playing());
    transport.set_playing(false);
    assert!(!transport.is_playing());
}
