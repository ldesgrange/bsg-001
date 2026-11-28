//! Asking a device how it is set up to record.

use crate::application::OperationError;
use crate::application::ports::DeviceTransport;
use crate::application::query::query;
use crate::domain::logging::LoggingState;
use crate::domain::protocol::Opcode;

pub async fn logging_state(transport: &mut impl DeviceTransport) -> Result<LoggingState, OperationError> {
    query(transport, Opcode::QueryLoggingState, LoggingState::decode, OperationError::InvalidLoggingState).await
}
