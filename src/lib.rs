//! Domain and application behavior for BSG-001 discovery.

pub mod adapters;
pub mod application;
pub mod domain;

pub use application::{Clock, DeviceConnector, DeviceScanner, DeviceTransport, DiscoveryError, ScannerError, discover_devices};
pub use domain::{DeviceAddress, DeviceInfo, DeviceName, DeviceValidationError, Rssi};
