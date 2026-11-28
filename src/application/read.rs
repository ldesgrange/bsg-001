//! Reading what a device is measuring right now.

use crate::application::OperationError;
use crate::application::ports::DeviceTransport;
use crate::application::query::query;
use crate::domain::protocol::Opcode;
use crate::domain::reading::Reading;

pub async fn read(transport: &mut impl DeviceTransport) -> Result<Reading, OperationError> {
    query(transport, Opcode::QueryReading, Reading::decode, OperationError::InvalidReading).await
}
