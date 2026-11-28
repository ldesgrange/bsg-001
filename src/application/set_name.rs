//! Device naming command and response handling.

use crate::application::ports::DeviceTransport;
use crate::application::query::query;
use crate::domain::DeviceLabel;
use crate::domain::protocol::{Opcode, command_frame, decode_text, encode_text};

/// Give the device a new name, and the name it then reports it has stored.
pub async fn set_name(transport: &mut impl DeviceTransport, label: &DeviceLabel) -> Result<DeviceLabel, String> {
    let text = encode_text(label.as_str()).map_err(|error| error.to_string())?;
    transport.write(&command_frame(Opcode::SetName, &text)).await?;
    let stored = |content: &[u8]| decode_text(content).ok().and_then(|text| DeviceLabel::new(text).ok()).ok_or(());
    let confirmed = query(transport, Opcode::QueryName, stored, "Invalid name query reply.").await?;
    if confirmed != *label {
        return Err("The device did not confirm the requested name.".to_owned());
    }
    Ok(confirmed)
}
