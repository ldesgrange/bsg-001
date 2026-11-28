#[path = "support/fakes.rs"]
mod fakes;

use bsg_001::DeviceLabel;
use bsg_001::application::OperationError;
use bsg_001::application::set_name::set_name;
use bsg_001::domain::protocol::Opcode;
use fakes::FakeDevice;

const ADDRESS: &str = "AA:BB:CC:DD:EE:FF";

#[tokio::test]
async fn confirms_the_name_the_device_stored() {
    let mut device = FakeDevice::storing(ADDRESS);
    let label = DeviceLabel::new("ABCD").unwrap();
    assert_eq!(set_name(&mut device, &label).await.unwrap(), label);
}

#[tokio::test]
async fn rejects_malformed_replies() {
    for reply in [vec![], vec![0x31], b"\x31\x00\x04ABC".to_vec(), b"\x31\x00\x05ABCDE".to_vec()] {
        let mut device = FakeDevice::storing(ADDRESS).answering(Opcode::QueryName, reply);
        let error = set_name(&mut device, &DeviceLabel::new("ABCD").unwrap()).await.unwrap_err();
        assert!(matches!(error, OperationError::InvalidName), "{error:?}");
        assert_eq!(error.to_string(), "Invalid name query reply.");
    }
}

#[tokio::test]
async fn rejects_a_name_other_than_the_one_requested() {
    let mut device = FakeDevice::storing(ADDRESS).answering(Opcode::QueryName, b"\x31\x00\x04WXYZ".to_vec());
    let error = set_name(&mut device, &DeviceLabel::new("ABCD").unwrap()).await.unwrap_err();
    assert!(matches!(error, OperationError::NameNotConfirmed), "{error:?}");
    assert_eq!(error.to_string(), "The device did not confirm the requested name.");
}
