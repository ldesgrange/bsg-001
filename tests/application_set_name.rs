#[path = "support/fakes.rs"]
mod fakes;

use bsg_001::application::set_name::set_name;
use fakes::FakeDevice;

const ADDRESS: &str = "AA:BB:CC:DD:EE:FF";

#[tokio::test]
async fn rejects_invalid_names_before_writes() {
    for name in ["", "ABC", "ABCDE", "A-B1", " A12", "é123"] {
        let mut device = FakeDevice::storing(ADDRESS);
        assert!(set_name(&mut device, name).await.is_err());
        assert!(device.writes().is_empty());
    }
}

#[tokio::test]
async fn rejects_malformed_and_mismatched_replies() {
    for reply in [vec![], vec![0x31], b"\x31\x00\x04ABC".to_vec(), b"\x31\x00\x05ABCDE".to_vec(), b"\x31\x00\x04WXYZ".to_vec()] {
        let mut device = FakeDevice::storing(ADDRESS).answering(0x31, reply);
        assert!(set_name(&mut device, "ABCD").await.is_err());
    }
}
