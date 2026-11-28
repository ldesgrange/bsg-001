#[path = "support/fakes.rs"]
mod fakes;

use bsg_001::application::on_device;
use bsg_001::application::read::read;
use bsg_001::{DeviceAddress, OperationError};
use fakes::{FakeConnector, FakeDevice};

const ADDRESS: &str = "AA:BB:CC:DD:EE:FF";

fn connector(device: &FakeDevice) -> FakeConnector {
    FakeConnector {
        device: Some(device.clone()),
        connected_at: None,
    }
}

#[tokio::test]
async fn runs_the_operation_then_disconnects() {
    let device = FakeDevice::storing(ADDRESS);
    let reading = on_device(&connector(&device), &DeviceAddress::new(ADDRESS).unwrap(), async |transport| read(transport).await).await;
    assert!(reading.unwrap().is_measuring());
    assert!(device.was_disconnected());
}

#[tokio::test]
async fn disconnects_when_the_operation_fails_and_gives_its_error() {
    let device = FakeDevice::storing(ADDRESS);
    let result: Result<(), _> = on_device(&connector(&device), &DeviceAddress::new(ADDRESS).unwrap(), async |_| Err(OperationError::DidNotStop)).await;
    assert!(matches!(result, Err(OperationError::DidNotStop)));
    assert!(device.was_disconnected());
}

#[tokio::test]
async fn fails_when_the_device_cannot_be_reached() {
    let device = FakeDevice::storing(ADDRESS);
    let result = on_device(&connector(&device), &DeviceAddress::new("11:22:33:44:55:66").unwrap(), async |transport| {
        read(transport).await
    })
    .await;
    assert_eq!(result.unwrap_err().to_string(), "Device 11:22:33:44:55:66 was not found.");
    assert!(!device.was_disconnected());
}
