//! Downloading the records a device holds.

use crate::application::OperationError;
use crate::application::logging::logging_state;
use crate::application::ports::DeviceTransport;
use crate::application::ports::PortError;
use crate::domain::logging::{Download, Run};
use crate::domain::protocol::{Opcode, command_frame};

const ATTEMPTS: usize = 3;

/// Download the records and check them against the checksum of the device.
///
/// `progress` is told how many records have arrived, and how many were announced, after each packet.
/// The device can write a record between the two queries, so each attempt asks for the logging state again.
/// An attempt fails when records are lost on the way or do not match the checksum, and the last failure is the one reported.
pub async fn records(transport: &mut impl DeviceTransport, progress: &mut impl FnMut(u16, u16)) -> Result<Run, OperationError> {
    let mut failure = OperationError::RecordsMismatch;
    for _ in 0..ATTEMPTS {
        let state = logging_state(transport).await?;
        if state.records == 0 {
            return Ok(Run::new(state, Vec::new()));
        }
        transport.write(&command_frame(Opcode::QueryRecords, &[])).await?;
        failure = match download(transport, state.records, progress).await {
            Err(lost) => OperationError::Device(lost),
            Ok(download) => {
                if download.checksum() == state.checksum
                    && let Ok(dose_rates) = download.records()
                {
                    return Ok(Run::new(state, dose_rates));
                }
                OperationError::RecordsMismatch
            }
        };
    }
    Err(failure)
}

/// Receive the records the device announced, or say why they did not all arrive.
async fn download(transport: &mut impl DeviceTransport, announced: u16, progress: &mut impl FnMut(u16, u16)) -> Result<Download, PortError> {
    let mut download = Download::expecting(announced);
    // Packets that follow the announced records are taken too, so they do not end up in the next attempt.
    while let Some(packet) = transport.receive_records_packet(!download.is_complete()).await? {
        if !download.is_complete() {
            download.receive(&packet);
            progress(download.received(), announced);
        }
    }
    Ok(download)
}
