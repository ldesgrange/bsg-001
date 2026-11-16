#[path = "support/fakes.rs"]
mod fakes;

use std::time::Duration;

use bsg_001::adapters::cli::{Cli, Command, NoProgress, Printed, format_device, format_progress, refusal, render_discovery, run, write_result};
use bsg_001::{DeviceAddress, DeviceInfo, DeviceName, Rssi};
use clap::Parser;
use fakes::{FakeConnector, FakeDevice, FakeScanner, FixedClock};
use tokio::runtime::Runtime;

fn device(name: Option<&str>, rssi: Option<i16>) -> DeviceInfo {
    DeviceInfo::new(
        DeviceAddress::new("randomized-address").unwrap(),
        name.map(|value| DeviceName::new(value).unwrap()),
        rssi.map(|value| Rssi::new(value).unwrap()),
    )
}

#[test]
fn formats_missing_advertisement_data_with_fallbacks() {
    assert_eq!(format_device(&device(None, None)), "randomized-address\t<unnamed>\tRSSI unavailable");
}

#[test]
fn renders_discovered_devices_as_tab_separated_lines() {
    let scanner = FakeScanner::seeing(vec![device(Some("BLE_Gamma:1234"), Some(-55))]);

    assert_eq!(
        Runtime::new().unwrap().block_on(render_discovery(&scanner, Duration::from_secs(2))).unwrap(),
        Printed::Result("randomized-address\tBLE_Gamma:1234\t-55 dBm".to_owned())
    );
}

#[test]
fn reports_when_no_bsg_001_devices_are_found() {
    let scanner = FakeScanner::seeing(vec![device(Some("Other device"), Some(-55))]);

    assert_eq!(
        Runtime::new().unwrap().block_on(render_discovery(&scanner, Duration::from_secs(2))).unwrap(),
        Printed::Notice("No BSG-001 devices found.".to_owned())
    );
}

#[test]
fn defaults_the_discover_timeout_to_twenty_seconds() {
    let cli = Cli::try_parse_from(["bsg-001", "discover"]).unwrap();

    assert!(matches!(
        cli.command,
        Command::Discover {
            scan_duration
        } if scan_duration == Duration::from_secs(20)
    ));
}

#[test]
fn rejects_discover_timeouts_below_one_tenth_of_a_second() {
    let result = Cli::try_parse_from(["bsg-001", "discover", "--timeout", "0.09"]);

    assert!(result.is_err());
    let error = result.err().unwrap();
    assert!(error.to_string().contains("at least 0.1 seconds"));
}

#[test]
fn rejects_non_finite_and_oversized_discover_timeouts_without_panicking() {
    for timeout in ["nan", "inf", "1e20"] {
        assert!(Cli::try_parse_from(["bsg-001", "discover", "--timeout", timeout]).is_err(), "{timeout} must be rejected");
    }
}

#[test]
fn escapes_control_characters_in_the_listed_name() {
    assert_eq!(
        format_device(&device(Some("BLE_Gamma:\\\n\r\t\u{0}\u{7f}\u{85}é"), Some(-55))),
        "randomized-address\tBLE_Gamma:\\\\\\n\\r\\t\\x00\\x7f\\x85é\t-55 dBm"
    );
}

#[test]
fn parses_set_name_address_and_name() {
    let cli = Cli::try_parse_from(["bsg-001", "set-name", "--address", "AA:BB:CC:DD:EE:FF", "ABCD"]).unwrap();
    assert!(matches!(cli.command, Command::SetName { address, name } if address.as_deref() == Some("AA:BB:CC:DD:EE:FF") && name == "ABCD"));
}

#[test]
fn rejects_invalid_name_before_reaching_the_device() {
    let cli = Cli::try_parse_from(["bsg-001", "set-name", "--address", "AA:BB:CC:DD:EE:FF", "bad!"]).unwrap();
    let device = FakeDevice::storing("AA:BB:CC:DD:EE:FF");
    let connector = FakeConnector {
        device: Some(device.clone()),
        connected_at: None,
    };
    let mut stderr = Vec::new();
    let outcome = Runtime::new().unwrap().block_on(run(
        cli,
        &FakeScanner::seeing(Vec::new()),
        &connector,
        &FixedClock::showing("2026-01-02 03:04:05"),
        &mut stderr,
        &mut NoProgress,
    ));

    assert!(device.writes().is_empty(), "an invalid name must not reach the device");

    assert!(!outcome.success);
    assert_eq!(outcome.stdout, "");
    assert_eq!(String::from_utf8(stderr).unwrap(), "Error: The name must contain exactly four ASCII letters or digits.\n");
}

/// Stands in for standard output when writing to it fails.
struct Failing(std::io::ErrorKind);

impl std::io::Write for Failing {
    fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
        Err(self.0.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn drops_a_result_the_reader_no_longer_wants_but_reports_other_write_failures() {
    assert!(write_result(&mut Failing(std::io::ErrorKind::BrokenPipe), "a result\n").is_ok());
    assert!(write_result(&mut Failing(std::io::ErrorKind::StorageFull), "a result\n").is_err());

    let mut written = Vec::new();
    write_result(&mut written, "a result\n").unwrap();
    assert_eq!(written, b"a result\n");
}

#[test]
fn says_on_macos_only_that_devices_cannot_be_told_apart() {
    let zeros = DeviceAddress::new("00:00:00:00:00:00").unwrap_err();

    assert_eq!(refusal(&zeros, false), "00:00:00:00:00:00 is not the address of a device.");
    assert_eq!(
        refusal(&zeros, true),
        "00:00:00:00:00:00 is not the address of a device. On macOS the tool cannot tell devices apart yet."
    );
    assert_eq!(refusal(&DeviceAddress::new(" ").unwrap_err(), true), "A Bluetooth device address cannot be empty.");
}

#[test]
fn draws_a_progress_bar_that_fills_with_the_records_downloaded() {
    assert_eq!(format_progress(0, 1440), format!("[{}] 0 of 1440 records", " ".repeat(30)));
    assert_eq!(format_progress(720, 1440), format!("[{}{}] 720 of 1440 records", "#".repeat(15), " ".repeat(15)));
    assert_eq!(format_progress(1440, 1440), format!("[{}] 1440 of 1440 records", "#".repeat(30)));
}
