//! The clock of the computer, read in UTC.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::Clock;
use crate::domain::logging::ClockTime;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Option<ClockTime> {
        let since_1970 = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        ClockTime::from_unix_seconds(since_1970.as_secs())
    }
}
