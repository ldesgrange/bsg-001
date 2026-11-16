//! Asking a device how it is set up to record.

use crate::application::ports::DeviceTransport;
use crate::domain::logging::LoggingState;
use crate::domain::protocol::{command_frame, reply_content};

const QUERY_LOGGING_STATE: u8 = 0x21;

pub async fn logging_state(transport: &mut impl DeviceTransport) -> Result<LoggingState, String> {
    transport.write(&command_frame(QUERY_LOGGING_STATE, &[])).await?;
    let reply = transport.receive(QUERY_LOGGING_STATE).await?;
    reply_content(QUERY_LOGGING_STATE, &reply)
        .and_then(LoggingState::decode)
        .map_err(|_| "Invalid logging state reply.".to_owned())
}
