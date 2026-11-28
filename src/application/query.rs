//! Asking a device a query and reading its reply.

use crate::application::OperationError;
use crate::application::ports::DeviceTransport;
use crate::domain::protocol::{Opcode, command_frame, reply_content};

/// Send a query and decode the content of its reply, or fail with `invalid` when it cannot be read.
pub(crate) async fn query<T, E>(
    transport: &mut impl DeviceTransport,
    opcode: Opcode,
    decode: impl FnOnce(&[u8]) -> Result<T, E>,
    invalid: OperationError,
) -> Result<T, OperationError> {
    transport.write(&command_frame(opcode, &[])).await?;
    let reply = transport.receive(opcode).await?;
    reply_content(opcode, &reply).ok().and_then(|content| decode(content).ok()).ok_or(invalid)
}
