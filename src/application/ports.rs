//! Ports implemented by infrastructure adapters.

use std::error::Error;
use std::future::Future;
use std::time::Duration;

use crate::domain::DeviceInfo;
use crate::domain::logging::ClockTime;

pub type ScannerError = Box<dyn Error + Send + Sync>;

pub trait DeviceScanner {
    fn discover(&self, scan_duration: Duration) -> impl Future<Output = Result<Vec<DeviceInfo>, ScannerError>>;
}

/// A connection to one device, over which frames are exchanged.
pub trait DeviceTransport {
    fn write(&mut self, message: &[u8]) -> impl Future<Output = Result<(), String>>;
    /// Wait for the reply to the query with this opcode.
    fn receive(&mut self, opcode: u8) -> impl Future<Output = Result<Vec<u8>, String>>;
    /// Wait for the next packet of a records download.
    ///
    /// While more records are expected, a packet that does not come is an error.
    /// Once they have all arrived, the wait is short and ends with nothing when the device has fallen silent.
    fn receive_records_packet(&mut self, more_expected: bool) -> impl Future<Output = Result<Option<Vec<u8>>, String>>;
    fn disconnect(&mut self) -> impl Future<Output = Result<(), String>>;
}

/// The clock of the computer the tool runs on.
pub trait Clock {
    /// The date and time in UTC, unless the device's clock cannot hold it.
    fn now(&self) -> Option<ClockTime>;
}

pub trait DeviceConnector {
    type Transport: DeviceTransport;

    fn connect(&self, address: &str) -> impl Future<Output = Result<Self::Transport, String>>;
}
