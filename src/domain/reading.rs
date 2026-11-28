//! What a BSG-001 reports when asked for its current reading.

use std::fmt::{self, Display, Formatter};

use crate::domain::protocol::{ProtocolError, decode_number};

/// How long the device has been measuring.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeasuringTime {
    days: u8,
    hours: u8,
    minutes: u8,
    seconds: u8,
}

impl Display for MeasuringTime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let [days, hours, minutes, seconds] = [self.days, self.hours, self.minutes, self.seconds].map(u32::from);
        write_elapsed(formatter, days, hours, minutes, seconds)
    }
}

/// Write a time that has elapsed as days, then hours, minutes and seconds, such as `1 d 02:03:04`.
pub(crate) fn write_elapsed(formatter: &mut Formatter<'_>, days: u32, hours: u32, minutes: u32, seconds: u32) -> fmt::Result {
    write!(formatter, "{days} d {hours:02}:{minutes:02}:{seconds:02}")
}

/// A dose rate, in hundredths of a microsievert per hour.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DoseRate {
    hundredths: u32,
}

impl DoseRate {
    pub fn from_hundredths(hundredths: u32) -> Self {
        Self { hundredths }
    }

    pub fn hundredths(self) -> u32 {
        self.hundredths
    }
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

/// Whether the device is measuring.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasuringStatus {
    Measuring,
    Stopped,
}

impl Display for MeasuringStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Measuring => "measuring",
            Self::Stopped => "stopped",
        })
    }
}

/// The pulses the sensor counted in a minute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pulses(u16);

impl Pulses {
    /// The pulses in a last-minute value, rounded to the nearest: the device sends five units per pulse, and now and then one unit less.
    pub fn from_units(units: u16) -> Self {
        Self(units / 5 + u16::from(units % 5 >= 3))
    }

    pub fn count(self) -> u16 {
        self.0
    }

    /// The dose rate these pulses amount to over a minute, where one pulse amounts to 0.50 µSv/h.
    pub fn dose_rate(self) -> DoseRate {
        DoseRate::from_hundredths(u32::from(self.0) * 50)
    }
}

impl Display for Pulses {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.0, if self.0 == 1 { "pulse" } else { "pulses" })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reading {
    pub status: MeasuringStatus,
    pub measuring_time: MeasuringTime,
    pub average: DoseRate,
    /// The pulses the sensor counted in the last completed minute.
    pub last_minute: Pulses,
    /// The raw value of the battery sensor, not a percentage.
    pub battery_sensor: u16,
}

impl Reading {
    /// Read a reading from the start of the content of a reply, ignoring what follows.
    pub fn decode(content: &[u8]) -> Result<Self, ProtocolError> {
        let [status, days, hours, minutes, seconds, numbers @ ..] = content else {
            return Err(ProtocolError::TruncatedReading);
        };
        let number = |offset: usize| numbers.get(offset..).and_then(|rest| decode_number(rest).ok()).ok_or(ProtocolError::TruncatedReading);
        Ok(Self {
            status: match status {
                0 => MeasuringStatus::Stopped,
                1 => MeasuringStatus::Measuring,
                _ => return Err(ProtocolError::UnknownStatus),
            },
            measuring_time: MeasuringTime {
                days: *days,
                hours: *hours,
                minutes: *minutes,
                seconds: *seconds,
            },
            average: DoseRate::from_hundredths(u32::from(number(0)?)),
            last_minute: Pulses::from_units(number(2)?),
            battery_sensor: number(4)?,
        })
    }

    pub fn is_measuring(&self) -> bool {
        self.status == MeasuringStatus::Measuring
    }

    /// The dose rate over the last completed minute.
    pub fn one_minute(&self) -> DoseRate {
        self.last_minute.dose_rate()
    }

    pub fn battery_level(&self) -> BatteryLevel {
        BatteryLevel::from_sensor(self.battery_sensor)
    }
}
