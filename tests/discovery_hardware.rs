use std::time::Duration;

use bsg_001::adapters::bluetooth::BtleplugDeviceScanner;
use bsg_001::application::discover_devices;
use tokio::runtime::Runtime;

#[test]
#[ignore = "requires explicit hardware opt-in"]
fn discovers_a_bsg_001_with_real_bluetooth() {
    let devices = Runtime::new()
        .expect("the test runtime starts")
        .block_on(discover_devices(&BtleplugDeviceScanner, Duration::from_secs(20)))
        .expect("the Bluetooth scan succeeds");
    assert!(!devices.is_empty(), "No BSG-001 was discovered.");
}
