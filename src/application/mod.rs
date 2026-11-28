//! Application use cases and infrastructure ports.

mod discover;
mod error;
pub mod logging;
mod ports;
mod query;
pub mod read;
pub mod records;
pub mod run;

pub use discover::{DiscoveryError, discover_devices};
pub use error::OperationError;
pub use ports::{Clock, DeviceConnector, DeviceScanner, DeviceTransport, PortError};
pub mod set_name;
