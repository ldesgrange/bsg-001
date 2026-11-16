//! Downloading the records a device holds.

use crate::application::logging::logging_state;
use crate::application::ports::DeviceTransport;
use crate::domain::logging::{Download, Run};
use crate::domain::protocol::command_frame;

const QUERY_RECORDS: u8 = 0xa1;
const ATTEMPTS: usize = 3;

/// Download the records and check them against the checksum of the device.
///
/// `progress` is told how many records have arrived, and how many were announced, after each packet.
/// The device can write a record between the two queries, so each attempt asks for the logging state again.
pub async fn records(transport: &mut impl DeviceTransport, progress: &mut impl FnMut(u16, u16)) -> Result<Run, String> {
    for _ in 0..ATTEMPTS {
        let state = logging_state(transport).await?;
        if state.records == 0 {
            return Ok(Run::new(state, Vec::new()));
        }
        transport.write(&command_frame(QUERY_RECORDS, &[])).await?;
        let mut download = Download::expecting(state.records);
        // Packets that follow the announced records are taken too, so they do not end up in the next attempt.
        while let Some(packet) = transport.receive_records_packet(!download.is_complete()).await? {
            if !download.is_complete() {
                download.receive(&packet);
                progress(download.received(), state.records);
            }
        }
        if download.checksum() == state.checksum
            && let Ok(dose_rates) = download.records()
        {
            return Ok(Run::new(state, dose_rates));
        }
    }
    Err("The records do not match the checksum of the device.".to_owned())
}
