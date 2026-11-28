#[path = "support/fakes.rs"]
mod fakes;

use std::time::Duration;

use bsg_001::application::discover_devices;
use bsg_001::domain::{DeviceAddress, DeviceInfo, DeviceName, Rssi};
use fakes::FakeScanner;
use tokio::runtime::Runtime;

fn device(name: Option<&str>) -> DeviceInfo {
    DeviceInfo::new(
        DeviceAddress::new("randomized-address").unwrap(),
        name.map(|value| DeviceName::new(value).unwrap()),
        Some(Rssi::new(-55).unwrap()),
    )
}

#[test]
fn returns_only_devices_identified_as_bsg_001() {
    let scanner = FakeScanner::seeing(vec![device(Some("BLE_Gamma:1234")), device(Some("Other device")), device(None)]);

    let result = Runtime::new().unwrap().block_on(discover_devices(&scanner, Duration::from_millis(100))).unwrap();

    assert_eq!(result, vec![device(Some("BLE_Gamma:1234"))]);
    assert_eq!(scanner.scan_duration.get(), Some(Duration::from_millis(100)));
}

#[test]
fn rejects_a_zero_scan_duration() {
    let scanner = FakeScanner::seeing(vec![]);

    assert_eq!(
        Runtime::new().unwrap().block_on(discover_devices(&scanner, Duration::ZERO)).unwrap_err().to_string(),
        "Scan timeout must be greater than zero."
    );
    assert_eq!(scanner.calls.get(), 0);
}
