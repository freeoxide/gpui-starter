use std::collections::HashMap;

use zbus::names::OwnedInterfaceName;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Str};

use super::*;

type InterfaceProps = HashMap<String, OwnedValue>;

fn props(entries: &[(&str, OwnedValue)]) -> InterfaceProps {
    let mut properties = InterfaceProps::new();
    for (name, value) in entries {
        properties.insert((*name).to_string(), value.clone());
    }
    properties
}

fn interface_entry(
    name: &str,
    properties: InterfaceProps,
) -> HashMap<OwnedInterfaceName, InterfaceProps> {
    let interface = OwnedInterfaceName::try_from(name).expect("valid interface name");
    HashMap::from([(interface, properties)])
}

fn object_path(path: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(path).expect("valid object path")
}

fn device_props(paired: bool, address: &str, alias: &str) -> InterfaceProps {
    props(&[
        ("Paired", OwnedValue::from(paired)),
        ("Connected", OwnedValue::from(false)),
        ("Address", OwnedValue::from(Str::from(address))),
        ("Alias", OwnedValue::from(Str::from(alias))),
    ])
}

// Shaped like a bluez GetManagedObjects reply: every interface's full
// property set, adapter and devices alike, on one response.
fn bluez_objects(adapter_props: Option<InterfaceProps>) -> ManagedObjects {
    let mut adapter = props(&[("Address", OwnedValue::from(Str::from("00:11:22:33:44:55")))]);
    if let Some(extra) = adapter_props {
        adapter.extend(extra);
    }
    let mut objects = HashMap::new();
    objects.insert(
        object_path("/org/bluez/hci0"),
        interface_entry(ADAPTER_INTERFACE, adapter),
    );
    let headset = device_props(true, "AA:BB:CC:DD:EE:FF", "Zephyr Headphones");
    objects.insert(
        object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
        interface_entry(DEVICE_INTERFACE, headset),
    );
    let speaker = device_props(true, "11:22:33:44:55:66", "Aurora Speaker");
    objects.insert(
        object_path("/org/bluez/hci0/dev_11_22_33_44_55_66"),
        interface_entry(DEVICE_INTERFACE, speaker),
    );
    let stranger = device_props(false, "77:88:99:AA:BB:CC", "Solo Buds");
    objects.insert(
        object_path("/org/bluez/hci0/dev_77_88_99_AA_BB_CC"),
        interface_entry(DEVICE_INTERFACE, stranger),
    );
    objects
}

#[test]
fn find_adapter_picks_the_adapter1_object_not_a_device() {
    let objects = bluez_objects(None);
    assert_eq!(find_adapter(&objects), Some(object_path("/org/bluez/hci0")));
    assert_eq!(find_adapter(&HashMap::new()), None);
}

#[test]
fn adapter_powered_reads_the_property_from_the_managed_objects_dict() {
    for powered in [true, false] {
        let adapter_props = props(&[("Powered", OwnedValue::from(powered))]);
        let objects = bluez_objects(Some(adapter_props));
        let adapter = find_adapter(&objects).expect("adapter object present");
        assert_eq!(adapter_powered(&objects, &adapter), Ok(powered));
    }
}

#[test]
fn adapter_powered_errors_when_the_property_is_missing() {
    let objects = bluez_objects(None);
    let adapter = find_adapter(&objects).expect("adapter object present");
    assert!(adapter_powered(&objects, &adapter).is_err());
}

#[test]
fn adapter_powered_errors_for_a_path_outside_the_response() {
    let objects = bluez_objects(None);
    assert!(adapter_powered(&objects, &object_path("/org/bluez/hci9")).is_err());
}

#[test]
fn paired_devices_filters_unpaired_and_sorts_by_alias() {
    let objects = bluez_objects(None);
    let devices = paired_devices(&objects);
    let aliases: Vec<String> = devices.iter().map(|device| device.alias.clone()).collect();
    assert_eq!(aliases, ["Aurora Speaker", "Zephyr Headphones"]);
    assert_eq!(devices[0].address, "11:22:33:44:55:66");
    assert!(!devices[0].connected);
}
