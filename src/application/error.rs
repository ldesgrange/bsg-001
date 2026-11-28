//! What can go wrong while operating a device.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::application::ports::PortError;

#[derive(Debug)]
pub enum OperationError {
    /// The device could not be reached, or stopped answering.
    Device(PortError),
    InvalidLoggingState,
    InvalidReading,
    InvalidName,
    NameNotConfirmed,
    AlreadyMeasuring,
    ClockOutOfRange,
    DidNotStart,
    AlreadyStopped,
    DidNotStop,
    RecordsMismatch,
}

impl From<PortError> for OperationError {
    fn from(error: PortError) -> Self {
        Self::Device(error)
    }
}

impl Display for OperationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Device(error) => return error.fmt(formatter),
            Self::InvalidLoggingState => "Invalid logging state reply.",
            Self::InvalidReading => "Invalid reading reply.",
            Self::InvalidName => "Invalid name query reply.",
            Self::NameNotConfirmed => "The device did not confirm the requested name.",
            Self::AlreadyMeasuring => "The device is already measuring.",
            Self::ClockOutOfRange => "The clock of the computer shows a time the device cannot hold.",
            Self::DidNotStart => "The device did not start.",
            Self::AlreadyStopped => "The device is already stopped.",
            Self::DidNotStop => "The device did not stop.",
            Self::RecordsMismatch => "The records do not match the checksum of the device.",
        };
        formatter.write_str(message)
    }
}

impl Error for OperationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Device(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}
