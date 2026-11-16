//! Device naming command and response handling.

use crate::application::ports::DeviceTransport;
use crate::domain::protocol::{command_frame, decode_text, encode_text, reply_content};

const SET_NAME: u8 = 0x30;
const QUERY_NAME: u8 = 0x31;

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.len() != 4 || !name.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err("The name must contain exactly four ASCII letters or digits.".to_owned());
    }
    Ok(())
}

pub async fn set_name(transport: &mut impl DeviceTransport, name: &str) -> Result<String, String> {
    const INVALID_REPLY: &str = "Invalid name query reply.";
    validate_name(name)?;
    let text = encode_text(name).map_err(|error| error.to_string())?;
    transport.write(&command_frame(SET_NAME, &text)).await?;
    transport.write(&command_frame(QUERY_NAME, &[])).await?;
    let reply = transport.receive(QUERY_NAME).await?;
    let confirmed = reply_content(QUERY_NAME, &reply).and_then(decode_text).map_err(|_| INVALID_REPLY.to_owned())?;
    validate_name(&confirmed).map_err(|_| INVALID_REPLY.to_owned())?;
    if confirmed != name {
        return Err("The device did not confirm the requested name.".to_owned());
    }
    Ok(confirmed)
}
