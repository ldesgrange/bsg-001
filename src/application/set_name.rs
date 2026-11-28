//! Device naming command and response handling.

use crate::application::OperationError;
use crate::application::ports::DeviceTransport;
use crate::application::query::query;
use crate::domain::DeviceLabel;
use crate::domain::protocol::{Opcode, command_frame, decode_text, encode_text};

/// Give the device a new name, and the name it then reports it has stored.
pub async fn set_name(transport: &mut impl DeviceTransport, label: &DeviceLabel) -> Result<DeviceLabel, OperationError> {
    let text = encode_text(label.as_str()).expect("a label is four ASCII characters");
    transport.write(&command_frame(Opcode::SetName, &text)).await?;
    let stored = |content: &[u8]| decode_text(content).ok().and_then(|text| DeviceLabel::new(text).ok()).ok_or(());
    let confirmed = query(transport, Opcode::QueryName, stored, OperationError::InvalidName).await?;
    if confirmed != *label {
        return Err(OperationError::NameNotConfirmed);
    }
    Ok(confirmed)
}
