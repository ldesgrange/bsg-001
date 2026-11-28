//! Device naming command and response handling.

use crate::application::ports::DeviceTransport;
use crate::domain::DeviceLabel;
use crate::domain::protocol::{command_frame, decode_text, encode_text, reply_content};

const SET_NAME: u8 = 0x30;
const QUERY_NAME: u8 = 0x31;

/// Give the device a new name, and the name it then reports it has stored.
pub async fn set_name(transport: &mut impl DeviceTransport, label: &DeviceLabel) -> Result<DeviceLabel, String> {
    const INVALID_REPLY: &str = "Invalid name query reply.";
    let text = encode_text(label.as_str()).map_err(|error| error.to_string())?;
    transport.write(&command_frame(SET_NAME, &text)).await?;
    transport.write(&command_frame(QUERY_NAME, &[])).await?;
    let reply = transport.receive(QUERY_NAME).await?;
    let confirmed = reply_content(QUERY_NAME, &reply)
        .and_then(decode_text)
        .ok()
        .and_then(|text| DeviceLabel::new(text).ok())
        .ok_or(INVALID_REPLY)?;
    if confirmed != *label {
        return Err("The device did not confirm the requested name.".to_owned());
    }
    Ok(confirmed)
}
