//! Starting and stopping a run.

use crate::application::logging::logging_state;
use crate::application::ports::{Clock, DeviceTransport};
use crate::application::read::read;
use crate::domain::logging::{LoggingDuration, LoggingState};
use crate::domain::protocol::command_frame;

const SET_CLOCK: u8 = 0x20;
const START_STOP: u8 = 0x10;
const START: u8 = 1;
const STOP: u8 = 0;

/// Start a run of the given duration, and give the logging state the device then reports.
///
/// The device's clock is set first, because the device stamps the start of the run with it.
/// The time is read from `clock` just before it is sent, so that reaching the device does not make the stamp early.
/// `warn` is told, before the run starts, about the records that starting erases.
pub async fn start(transport: &mut impl DeviceTransport, duration: LoggingDuration, clock: &impl Clock, warn: &mut impl FnMut(String)) -> Result<LoggingState, String> {
    if read(transport).await?.measuring {
        return Err("The device is already measuring.".to_owned());
    }
    match logging_state(transport).await?.records {
        0 => {}
        1 => warn("starting a run erases the 1 record the device holds.".to_owned()),
        records => warn(format!("starting a run erases the {records} records the device holds.")),
    }
    let now = clock.now().ok_or("The clock of the computer shows a time the device cannot hold.")?;
    transport.write(&command_frame(SET_CLOCK, &now.bytes())).await?;
    transport.write(&command_frame(START_STOP, &[START, duration.days])).await?;
    let measuring = read(transport).await?.measuring;
    let state = logging_state(transport).await?;
    if measuring && state.duration == duration {
        Ok(state)
    } else {
        Err("The device did not start.".to_owned())
    }
}

/// Stop the run in progress.
pub async fn stop(transport: &mut impl DeviceTransport) -> Result<(), String> {
    if !read(transport).await?.measuring {
        return Err("The device is already stopped.".to_owned());
    }
    // The logging duration the device already has is sent back, so that stopping leaves it as it is.
    let duration = logging_state(transport).await?.duration;
    transport.write(&command_frame(START_STOP, &[STOP, duration.days])).await?;
    if read(transport).await?.measuring {
        Err("The device did not stop.".to_owned())
    } else {
        Ok(())
    }
}
