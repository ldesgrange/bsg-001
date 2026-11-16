use bsg_001::adapters::bluetooth::{LiveSightings, Report, device_info_from_advertisement};

#[test]
fn maps_advertisement_properties_to_device_information() {
    let device = device_info_from_advertisement("randomized-address", Some("BLE_Gamma:1234".to_owned()), Some(-55)).unwrap();

    assert_eq!(device.address.to_string(), "randomized-address");
    assert_eq!(device.name.unwrap().to_string(), "BLE_Gamma:1234");
    assert_eq!(device.rssi.unwrap().to_string(), "-55");
}

#[test]
fn preserves_missing_advertisement_name_and_rssi() {
    let device = device_info_from_advertisement("randomized-address", None, None).unwrap();

    assert!(device.name.is_none());
    assert!(device.rssi.is_none());
}

#[test]
fn treats_an_invalid_name_and_rssi_as_missing() {
    let device = device_info_from_advertisement("randomized-address", Some(" ".to_owned()), Some(127)).unwrap();

    assert_eq!(device.address.to_string(), "randomized-address");
    assert!(device.name.is_none());
    assert!(device.rssi.is_none());
}

#[test]
fn sees_a_remembered_device_only_once_it_advertises_and_each_device_once() {
    let mut sightings = LiveSightings::remembering(["remembered"]);

    assert!(!sightings.report(&"remembered", Report::Found));
    assert!(sightings.report(&"remembered", Report::Advertisement));
    assert!(!sightings.report(&"remembered", Report::Advertisement));
    assert!(sightings.report(&"new", Report::Found));
    assert!(!sightings.report(&"new", Report::Advertisement));
}
