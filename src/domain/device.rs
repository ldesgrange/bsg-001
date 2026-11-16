//! Bluetooth device values used by the application layer.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceValidationError {
    EmptyAddress,
    EmptyName,
    RssiOutOfRange,
}

impl Display for DeviceValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyAddress => "A Bluetooth device address cannot be empty.",
            Self::EmptyName => "An advertised device name cannot be empty.",
            Self::RssiOutOfRange => "A Bluetooth RSSI must be between -127 and 20 dBm.",
        };
        formatter.write_str(message)
    }
}

impl Error for DeviceValidationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceAddress(String);

impl DeviceAddress {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DeviceValidationError::EmptyAddress);
        }
        Ok(Self(value))
    }
}

impl Display for DeviceAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceName(String);

impl DeviceName {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DeviceValidationError::EmptyName);
        }
        Ok(Self(value))
    }

    pub fn identifies_bsg_001(&self) -> bool {
        self.0.starts_with("BLE_Gamma")
    }
}

impl Display for DeviceName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rssi(i16);

impl Rssi {
    pub fn new(value: i16) -> Result<Self, DeviceValidationError> {
        if !(-127..=20).contains(&value) {
            return Err(DeviceValidationError::RssiOutOfRange);
        }
        Ok(Self(value))
    }
}

impl Display for Rssi {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInfo {
    pub address: DeviceAddress,
    pub name: Option<DeviceName>,
    pub rssi: Option<Rssi>,
}

impl DeviceInfo {
    pub fn new(address: DeviceAddress, name: Option<DeviceName>, rssi: Option<Rssi>) -> Self {
        Self { address, name, rssi }
    }

    pub fn is_bsg_001(&self) -> bool {
        self.name.as_ref().is_some_and(DeviceName::identifies_bsg_001)
    }
}
