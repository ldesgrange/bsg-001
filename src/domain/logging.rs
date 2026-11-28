//! How a BSG-001 is set up to record its measurements, and how much it has recorded.

use std::fmt::{self, Display, Formatter};

use crate::domain::protocol::{ProtocolError, decode_number};
use crate::domain::reading::{DoseRate, write_elapsed};

/// How many days a run records for; zero days means the device measures without recording.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoggingDuration {
    days: u8,
}

impl LoggingDuration {
    /// Measuring without recording.
    pub const OFF: Self = Self { days: 0 };

    /// The duration of a run that records, which lasts from 1 to 31 days.
    pub fn of_run(days: u8) -> Option<Self> {
        (1..=31).contains(&days).then_some(Self { days })
    }

    /// The duration of a run, or measuring without recording for zero days.
    fn of_days(days: u8) -> Option<Self> {
        if days == 0 { Some(Self::OFF) } else { Self::of_run(days) }
    }

    pub fn days(self) -> u8 {
        self.days
    }

    /// How often a run of this duration records: the device keeps room for the whole run.
    pub fn record_interval_minutes(self) -> u8 {
        if self.days > 10 { 60 } else { 10 }
    }
}

impl Display for LoggingDuration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.days {
            0 => formatter.write_str("off"),
            1 => formatter.write_str("1 day"),
            days => write!(formatter, "{days} days"),
        }
    }
}

/// A date and time on the device's clock, which holds a real date only once it has been set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl ClockTime {
    /// A date and time the device's clock can hold, which counts its years from 2000 in one byte.
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        let year_byte = u8::try_from(year.checked_sub(2000)?).ok()?;
        Self::decode([year_byte, month, day, hour, minute, second])
    }

    /// The date and time in UTC this many seconds after the start of 1970, unless the device's clock cannot hold it.
    pub fn from_unix_seconds(seconds: u64) -> Option<Self> {
        let (year, month, day) = civil_from_days(i64::try_from(seconds / 86_400).ok()?);
        let second_of_day = seconds % 86_400;
        let part = |value: u64| u8::try_from(value).ok();
        Self::new(year, month, day, part(second_of_day / 3600)?, part(second_of_day % 3600 / 60)?, part(second_of_day % 60)?)
    }

    /// The six bytes the device holds for this time: the year since 2000, then month, day, hour, minute and second.
    pub fn bytes(&self) -> [u8; 6] {
        let year = u8::try_from(self.year - 2000).expect("a year the device can hold");
        [year, self.month, self.day, self.hour, self.minute, self.second]
    }

    /// Read the six bytes of a clock time, or nothing when they do not hold a real date and time.
    fn decode(bytes: [u8; 6]) -> Option<Self> {
        let [year, month, day, hour, minute, second] = bytes;
        let year = 2000 + u16::from(year);
        let real = (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day) && hour < 24 && minute < 60 && second < 60;
        real.then_some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
}

impl ClockTime {
    /// The date, a `T`, and the time, as ISO 8601 writes them, without a time zone.
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", self.year, self.month, self.day, self.hour, self.minute, self.second)
    }

    /// The clock time this many minutes later.
    fn plus_minutes(self, minutes: u32) -> Self {
        let minute_of_day = u32::from(self.hour) * 60 + u32::from(self.minute) + minutes;
        let (year, month, day) = civil_from_days(days_from_civil(self.year, self.month, self.day) + i64::from(minute_of_day / 1440));
        Self {
            year,
            month,
            day,
            hour: u8::try_from(minute_of_day % 1440 / 60).expect("an hour of the day"),
            minute: u8::try_from(minute_of_day % 60).expect("a minute of the hour"),
            second: self.second,
        }
    }
}

/// How many days a month of the Gregorian calendar has.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The number of days from 1970-01-01 to a date of the Gregorian calendar.
fn days_from_civil(year: u16, month: u8, day: u8) -> i64 {
    // Years run from March, so the leap day is the last day of a year.
    let year = i64::from(year) - i64::from(month <= 2);
    let month_from_march = (i64::from(month) + 9) % 12;
    let (era, year_of_era) = (year.div_euclid(400), year.rem_euclid(400));
    let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(day) - 1;
    era * 146_097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year - 719_468
}

/// The date of the Gregorian calendar this many days after 1970-01-01.
fn civil_from_days(days: i64) -> (u16, u8, u8) {
    let days = days + 719_468;
    let (era, day_of_era) = (days.div_euclid(146_097), days.rem_euclid(146_097));
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = (month_from_march + 2) % 12 + 1;
    let year = era * 400 + year_of_era + i64::from(month <= 2);
    (
        u16::try_from(year).unwrap_or(u16::MAX),
        u8::try_from(month).expect("a month"),
        u8::try_from(day).expect("a day of the month"),
    )
}

impl Display for ClockTime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoggingState {
    pub duration: LoggingDuration,
    /// When the run began, unless the device's clock was not set.
    pub started: Option<ClockTime>,
    pub interval_minutes: u8,
    pub records: u16,
    /// The sum of every byte of the records, modulo 256.
    pub checksum: u8,
}

impl LoggingState {
    /// Read a logging state from the start of the content of a reply, ignoring what follows.
    pub fn decode(content: &[u8]) -> Result<Self, ProtocolError> {
        let [days, year, month, day, hour, minute, second, interval, records @ ..] = content else {
            return Err(ProtocolError::TruncatedLoggingState);
        };
        let [_, _, checksum, ..] = records else {
            return Err(ProtocolError::TruncatedLoggingState);
        };
        Ok(Self {
            duration: LoggingDuration::of_days(*days).ok_or(ProtocolError::UnknownLoggingDuration)?,
            started: ClockTime::decode([*year, *month, *day, *hour, *minute, *second]),
            // The device can report no interval at all, which stands for ten minutes.
            interval_minutes: if *interval == 0 { 10 } else { *interval },
            records: decode_number(records)?,
            checksum: *checksum,
        })
    }

    /// How long after the start of the run a record was written, counting the first record as record 1.
    pub fn minutes_from_start(&self, record: u16) -> u32 {
        u32::from(record) * u32::from(self.interval_minutes)
    }

    /// When a record was written, counting the first record as record 1.
    ///
    /// The device writes a record at the end of each interval, so record 1 is one interval after the start.
    pub fn recorded_at(&self, record: u16) -> RecordTime {
        let minutes = self.minutes_from_start(record);
        match self.started {
            Some(started) => RecordTime::Clock(started.plus_minutes(minutes)),
            None => RecordTime::SinceStart { minutes },
        }
    }
}

/// When a record was written: on the device's clock, or after the start of the run when the clock was not set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordTime {
    Clock(ClockTime),
    SinceStart { minutes: u32 },
}

impl Display for RecordTime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clock(time) => time.fmt(formatter),
            Self::SinceStart { minutes } => write_elapsed(formatter, minutes / 1440, minutes % 1440 / 60, minutes % 60, 0),
        }
    }
}

/// The records of a download as they arrive, packet after packet.
///
/// Each packet is read and added to the checksum when it arrives, so the packets themselves are not kept.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Download {
    /// The number of records the logging state announced.
    announced: u16,
    dose_rates: Vec<DoseRate>,
    checksum: u8,
    /// The first byte of a record whose second byte is in the next packet.
    half_record: Option<u8>,
}

impl Download {
    pub fn expecting(announced: u16) -> Self {
        Self {
            announced,
            dose_rates: Vec::with_capacity(usize::from(announced)),
            checksum: 0,
            half_record: None,
        }
    }

    /// Take the records of the next packet, ignoring the padding and anything else that follows the announced records.
    pub fn receive(&mut self, packet: &[u8]) {
        for byte in packet {
            if self.is_complete() {
                return;
            }
            self.checksum = self.checksum.wrapping_add(*byte);
            match self.half_record.take() {
                None => self.half_record = Some(*byte),
                Some(low) => self.dose_rates.push(DoseRate::from_hundredths(u32::from(u16::from_le_bytes([low, *byte])))),
            }
        }
    }

    /// How many records have arrived so far.
    pub fn received(&self) -> u16 {
        u16::try_from(self.dose_rates.len()).expect("no more records than were announced")
    }

    pub fn is_complete(&self) -> bool {
        self.received() == self.announced
    }

    /// The sum of every byte of the records that have arrived, modulo 256, as the device works out its checksum.
    pub fn checksum(&self) -> u8 {
        self.checksum
    }

    /// The records, oldest first, once all the announced records have arrived.
    pub fn records(self) -> Result<Vec<DoseRate>, ProtocolError> {
        if self.is_complete() {
            Ok(self.dose_rates)
        } else {
            Err(ProtocolError::TruncatedRecords)
        }
    }
}

/// A record worth showing: when it was written, and the dose rate it holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Record {
    pub recorded_at: RecordTime,
    /// How long after the start of the run the record was written.
    pub minutes_from_start: u32,
    pub dose_rate: DoseRate,
}

/// The records downloaded from a device, with the logging state that tells when each was written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Run {
    state: LoggingState,
    /// Every record the device holds, oldest first.
    downloaded: Vec<DoseRate>,
}

impl Run {
    pub fn new(state: LoggingState, downloaded: Vec<DoseRate>) -> Self {
        Self { state, downloaded }
    }

    /// The records of the run, oldest first, without the first one.
    ///
    /// The sensor keeps counting while the device is stopped, and the first record holds everything it counted
    /// since the previous run ended, so it says nothing about the interval it stands for.
    pub fn records(&self) -> impl Iterator<Item = Record> + '_ {
        (1..).zip(&self.downloaded).skip(1).map(|(number, dose_rate)| Record {
            recorded_at: self.state.recorded_at(number),
            minutes_from_start: self.state.minutes_from_start(number),
            dose_rate: *dose_rate,
        })
    }
}
