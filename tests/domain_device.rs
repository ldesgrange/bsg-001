use bsg_001::{DeviceAddress, DeviceInfo, DeviceLabel, DeviceName, DeviceValidationError, Rssi};

#[test]
fn rejects_empty_device_address() {
    assert_eq!(DeviceAddress::new("  ").unwrap_err().to_string(), "A Bluetooth device address cannot be empty.");
}

#[test]
fn rejects_an_address_made_only_of_zeros_whatever_its_separators() {
    for zeros in ["00:00:00:00:00:00", "00-00-00-00-00-00", "000000000000", "0"] {
        let error = DeviceAddress::new(zeros).unwrap_err();
        assert_eq!(error, DeviceValidationError::ZeroAddress(zeros.to_owned()));
        assert_eq!(error.to_string(), format!("{zeros} is not the address of a device."));
    }
    for address in ["AA:BB:CC:DD:EE:FF", "00:00:00:00:00:01", "randomized-address"] {
        assert!(DeviceAddress::new(address).is_ok(), "{address} must be accepted");
    }
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

#[test]
fn a_label_is_exactly_four_ascii_letters_or_digits() {
    for label in ["ABCD", "ab12", "0000"] {
        assert_eq!(DeviceLabel::new(label).unwrap().to_string(), label);
    }
    for label in ["", "ABC", "ABCDE", "A-B1", " A12", "é123", "AB\nD"] {
        assert_eq!(DeviceLabel::new(label), Err(DeviceValidationError::InvalidLabel), "{label:?} must be refused");
    }
    assert_eq!(
        DeviceValidationError::InvalidLabel.to_string(),
        "The name must contain exactly four ASCII letters or digits."
    );
}

#[test]
fn an_address_is_the_same_whatever_the_case_of_its_letters() {
    let typed = DeviceAddress::new("aa:bb:cc:dd:ee:ff").unwrap();
    assert_eq!(typed, DeviceAddress::new("AA:BB:CC:DD:EE:FF").unwrap());
    assert_ne!(typed, DeviceAddress::new("AA:BB:CC:DD:EE:00").unwrap());
    assert_eq!(typed.to_string(), "aa:bb:cc:dd:ee:ff");
}
