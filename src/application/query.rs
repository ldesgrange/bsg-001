//! Asking a device a query and reading its reply.

use crate::application::ports::DeviceTransport;
use crate::domain::protocol::{Opcode, command_frame, reply_content};

/// Send a query and decode the content of its reply, or say that the reply is `invalid`.
pub(crate) async fn query<T, E>(transport: &mut impl DeviceTransport, opcode: Opcode, decode: impl FnOnce(&[u8]) -> Result<T, E>, invalid: &str) -> Result<T, String> {
    transport.write(&command_frame(opcode, &[])).await?;
    let reply = transport.receive(opcode).await?;
    let content = reply_content(opcode, &reply).map_err(|_| invalid.to_owned())?;
    decode(content).map_err(|_| invalid.to_owned())
}
