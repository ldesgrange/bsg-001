//! Device naming command and response handling.

use crate::application::OperationError;
use crate::application::ports::DeviceTransport;
use crate::application::query::query;
use crate::domain::DeviceLabel;
use crate::domain::protocol::{Opcode, command_frame, decode_text, encode_text};

/// Give the device a new name, and the name it then reports it has stored.
///
/// `warn` is told, before the name is sent, what renaming may do to a run in progress.
pub async fn set_name(transport: &mut impl DeviceTransport, label: &DeviceLabel, warn: &mut impl FnMut(String)) -> Result<DeviceLabel, OperationError> {
    warn("renaming during logging may reset the measuring-time counter.".to_owned());
    let text = encode_text(label.as_str()).expect("a label is four ASCII characters");
    transport.write(&command_frame(Opcode::SetName, &text)).await?;
    let stored = |content: &[u8]| decode_text(content).ok().and_then(|text| DeviceLabel::new(text).ok()).ok_or(());
    let confirmed = query(transport, Opcode::QueryName, stored, OperationError::InvalidName).await?;
    if confirmed != *label {
        return Err(OperationError::NameNotConfirmed);
    }
    Ok(confirmed)
}
