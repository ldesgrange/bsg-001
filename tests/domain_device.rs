use bsg_001::{DeviceAddress, DeviceInfo, DeviceName, Rssi};

#[test]
fn rejects_empty_device_address() {
    assert_eq!(DeviceAddress::new("  ").unwrap_err().to_string(), "A Bluetooth device address cannot be empty.");
}

#[test]
fn rejects_empty_advertised_name() {
    assert_eq!(DeviceName::new("\n").unwrap_err().to_string(), "An advertised device name cannot be empty.");
}

#[test]
fn rejects_rssi_outside_the_bluetooth_range() {
    assert_eq!(Rssi::new(-128).unwrap_err().to_string(), "A Bluetooth RSSI must be between -127 and 20 dBm.");
    assert_eq!(Rssi::new(21).unwrap_err().to_string(), "A Bluetooth RSSI must be between -127 and 20 dBm.");
}

#[test]
fn identifies_devices_from_the_advertised_name_prefix() {
    let address = DeviceAddress::new("randomized-address").unwrap();

    for (name, expected) in [
        (Some("BLE_Gamma"), true),
        (Some("BLE_Gamma:1234"), true),
        (Some("BLE_GammaX"), true),
        (Some("DfuTarg"), false),
        (Some("BSG-001"), false),
        (None, false),
    ] {
        let device = DeviceInfo::new(address.clone(), name.map(|value| DeviceName::new(value).unwrap()), Some(Rssi::new(-55).unwrap()));

        assert_eq!(device.is_bsg_001(), expected);
    }
}
