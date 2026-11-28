//! Bluetooth device values used by the application layer.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceValidationError {
    EmptyAddress,
    /// An address made only of zeros, which belongs to no device.
    ZeroAddress(String),
    EmptyName,
    InvalidLabel,
    RssiOutOfRange,
}

impl Display for DeviceValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroAddress(address) => return write!(formatter, "{address} is not the address of a device."),
            Self::EmptyAddress => "A Bluetooth device address cannot be empty.",
            Self::EmptyName => "An advertised device name cannot be empty.",
            Self::InvalidLabel => "The name must contain exactly four ASCII letters or digits.",
            Self::RssiOutOfRange => "A Bluetooth RSSI must be between -127 and 20 dBm.",
        };
        formatter.write_str(message)
    }
}

impl Error for DeviceValidationError {}

/// What selects one device: its Bluetooth address, written as the user typed it.
///
/// Two addresses that differ only in the case of their letters are the same address.
#[derive(Clone, Debug)]
pub struct DeviceAddress(String);

impl PartialEq for DeviceAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for DeviceAddress {}

impl DeviceAddress {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DeviceValidationError::EmptyAddress);
        }
        // Whatever the separators, an address whose digits are all zeros selects no device.
        let mut digits = value.chars().filter(char::is_ascii_alphanumeric).peekable();
        if digits.peek().is_some() && digits.all(|digit| digit == '0') {
            return Err(DeviceValidationError::ZeroAddress(value));
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

/// The name a device is given: the label that follows `BLE_Gamma:` in the name it advertises.
///
/// It has exactly four characters, each an ASCII letter or digit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceLabel(String);

impl DeviceLabel {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceValidationError> {
        let value = value.into();
        if value.len() != 4 || !value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(DeviceValidationError::InvalidLabel);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DeviceLabel {
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
