//! Reading what a device is measuring right now.

use crate::application::ports::DeviceTransport;
use crate::domain::protocol::{command_frame, reply_content};
use crate::domain::reading::Reading;

const QUERY_READING: u8 = 0xa0;

pub async fn read(transport: &mut impl DeviceTransport) -> Result<Reading, String> {
    transport.write(&command_frame(QUERY_READING, &[])).await?;
    let reply = transport.receive(QUERY_READING).await?;
    reply_content(QUERY_READING, &reply)
        .and_then(Reading::decode)
        .map_err(|_| "Invalid reading reply.".to_owned())
}
