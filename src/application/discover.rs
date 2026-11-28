//! BSG-001 discovery use case.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use crate::application::ports::{DeviceScanner, PortError};
use crate::domain::DeviceInfo;

#[derive(Debug)]
pub enum DiscoveryError {
    InvalidScanDuration,
    Scan(PortError),
}

impl Display for DiscoveryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScanDuration => formatter.write_str("Scan timeout must be greater than zero."),
            Self::Scan(error) => error.fmt(formatter),
        }
    }
}

impl Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidScanDuration => None,
            Self::Scan(error) => Some(error.as_ref()),
        }
    }
}

pub async fn discover_devices(scanner: &impl DeviceScanner, scan_duration: Duration) -> Result<Vec<DeviceInfo>, DiscoveryError> {
    if scan_duration.is_zero() {
        return Err(DiscoveryError::InvalidScanDuration);
    }
    let devices = scanner.discover(scan_duration).await.map_err(DiscoveryError::Scan)?;
    Ok(devices.into_iter().filter(DeviceInfo::is_bsg_001).collect())
}
