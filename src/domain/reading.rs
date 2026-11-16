//! What a BSG-001 reports when asked for its current reading.

use std::fmt::{self, Display, Formatter};

use crate::domain::protocol::{ProtocolError, decode_number};

/// How long the device has been measuring.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeasuringTime {
    pub days: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

impl Display for MeasuringTime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} d {:02}:{:02}:{:02}", self.days, self.hours, self.minutes, self.seconds)
    }
}

/// A dose rate, in hundredths of a microsievert per hour.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoseRate {
    pub hundredths: u32,
}

impl Display for DoseRate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{:02}", self.hundredths / 100, self.hundredths % 100)
    }
}

/// A battery charge level: one of the seven percentages the vendor application shows.
///
/// It can only be built from a battery sensor value, so no other percentage exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatteryLevel {
    percentage: u8,
}

impl BatteryLevel {
    /// The level for a battery sensor value; the lowest level is 1%, never 0%.
    pub fn from_sensor(sensor: u16) -> Self {
        let percentage = match sensor {
            211.. => 100,
            205..=210 => 80,
            200..=204 => 60,
            196..=199 => 40,
            194..=195 => 20,
            191..=193 => 10,
            ..=190 => 1,
        };
        Self { percentage }
    }
}

impl Display for BatteryLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}%", self.percentage)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reading {
    pub measuring: bool,
    pub measuring_time: MeasuringTime,
    pub average: DoseRate,
    /// The pulses the sensor counted in the last completed minute.
    pub last_minute_pulses: u16,
    /// The raw value of the battery sensor, not a percentage.
    pub battery_sensor: u16,
}

/// The pulses in a last-minute value, rounded to the nearest: the device sends five units per pulse, and now and then one unit less.
fn pulses(units: u16) -> u16 {
    units / 5 + u16::from(units % 5 >= 3)
}

impl Reading {
    /// Read a reading from the start of the content of a reply, ignoring what follows.
    pub fn decode(content: &[u8]) -> Result<Self, ProtocolError> {
        let [status, days, hours, minutes, seconds, numbers @ ..] = content else {
            return Err(ProtocolError::TruncatedReading);
        };
        let number = |offset: usize| numbers.get(offset..).ok_or(ProtocolError::TruncatedReading).and_then(decode_number);
        Ok(Self {
            measuring: match status {
                0 => false,
                1 => true,
                _ => return Err(ProtocolError::UnknownStatus),
            },
            measuring_time: MeasuringTime {
                days: *days,
                hours: *hours,
                minutes: *minutes,
                seconds: *seconds,
            },
            average: DoseRate {
                hundredths: u32::from(number(0)?),
            },
            last_minute_pulses: pulses(number(2)?),
            battery_sensor: number(4)?,
        })
    }

    /// The dose rate over the last completed minute, where one pulse amounts to 0.50 µSv/h.
    pub fn one_minute(&self) -> DoseRate {
        DoseRate {
            hundredths: u32::from(self.last_minute_pulses) * 50,
        }
    }

    pub fn battery_level(&self) -> BatteryLevel {
        BatteryLevel::from_sensor(self.battery_sensor)
    }

    pub fn status(&self) -> &'static str {
        if self.measuring { "measuring" } else { "stopped" }
    }
}
