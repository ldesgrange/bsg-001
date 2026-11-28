//! Ports implemented by infrastructure adapters.

use std::error::Error;
use std::future::Future;
use std::time::Duration;

use crate::domain::logging::ClockTime;
use crate::domain::protocol::Opcode;
use crate::domain::{DeviceAddress, DeviceInfo};

/// What went wrong in an adapter, as the adapter describes it.
pub type PortError = Box<dyn Error + Send + Sync>;

pub trait DeviceScanner {
    fn discover(&self, scan_duration: Duration) -> impl Future<Output = Result<Vec<DeviceInfo>, PortError>>;
}

/// A connection to one device, over which frames are exchanged.
pub trait DeviceTransport {
    fn write(&mut self, message: &[u8]) -> impl Future<Output = Result<(), PortError>>;
    /// Wait for the reply to this query.
    fn receive(&mut self, query: Opcode) -> impl Future<Output = Result<Vec<u8>, PortError>>;
    /// Wait for the next packet of a records download.
    ///
    /// While more records are expected, a packet that does not come is an error.
    /// Once they have all arrived, the wait is short and ends with nothing when the device has fallen silent.
    fn receive_records_packet(&mut self, more_expected: bool) -> impl Future<Output = Result<Option<Vec<u8>>, PortError>>;
    fn disconnect(&mut self) -> impl Future<Output = Result<(), PortError>>;
}

/// The clock of the computer the tool runs on.
pub trait Clock {
    /// The date and time in UTC, unless the device's clock cannot hold it.
    fn now(&self) -> Option<ClockTime>;
}

pub trait DeviceConnector {
    type Transport: DeviceTransport;

    fn connect(&self, address: &DeviceAddress) -> impl Future<Output = Result<Self::Transport, PortError>>;
}
