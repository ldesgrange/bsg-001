//! Command-line presentation helpers for BSG-001 operations.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Duration;

use clap::{Parser, Subcommand};

use crate::application::logging::logging_state;
use crate::application::read::read;
use crate::application::records::records;
use crate::application::run::{start, stop};
use crate::application::set_name::set_name;
use crate::application::{Clock, DeviceConnector, DeviceScanner, DeviceTransport, DiscoveryError, OperationError, PortError, discover_devices};
use crate::domain::logging::{ClockTime, LoggingDuration, LoggingState, RecordTime, Run};
use crate::domain::reading::{Pulses, Reading};
use crate::domain::{DeviceAddress, DeviceInfo, DeviceLabel, DeviceValidationError};

#[derive(Parser)]
#[command(name = "bsg-001", about = "Tools for the FTLab Wireless Smart Geiger BSG-001.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List nearby Bluetooth devices without connecting to them.
    Discover {
        #[arg(long = "timeout", default_value = "20.0", value_parser = parse_scan_duration)]
        scan_duration: Duration,
    },
    /// Show how the device at the given address is set up to record, and how many records it holds.
    Logging {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
    },
    /// Show what the device at the given address is measuring right now.
    Read {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
    },
    /// Download the records held by the device at the given address, leaving out the first one.
    Records {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
        /// How to print the records: text or csv; taken from the BSG_001_FORMAT environment variable when not given, else text.
        #[arg(long)]
        format: Option<String>,
    },
    /// Permanently rename the device at the given address.
    SetName {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
        name: String,
    },
    /// Start a run on the device, which erases the records of the previous run.
    Start {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
        /// How long the run records for, from 1 to 31 days.
        #[arg(long, default_value_t = 10, conflicts_with = "no_logging")]
        days: u8,
        /// Measure until stopped, without recording.
        #[arg(long)]
        no_logging: bool,
    },
    /// Stop the run in progress on the device.
    Stop {
        /// The address of the device; taken from the BSG_001_ADDRESS environment variable when not given.
        #[arg(long)]
        address: Option<String>,
    },
}

pub const ADDRESS_VARIABLE: &str = "BSG_001_ADDRESS";
pub const FORMAT_VARIABLE: &str = "BSG_001_FORMAT";

impl Cli {
    /// Take from the environment what the command line left out.
    #[must_use]
    pub fn with_environment(mut self, variable: impl Fn(&str) -> Option<String>) -> Self {
        // A variable that is set but empty counts as not set.
        let variable = |name: &str| variable(name).filter(|value| !value.is_empty());
        match &mut self.command {
            Command::Discover { .. } => {}
            Command::Logging { address } | Command::Read { address } | Command::SetName { address, .. } | Command::Start { address, .. } | Command::Stop { address } => {
                *address = address.take().or_else(|| variable(ADDRESS_VARIABLE));
            }
            Command::Records { address, format } => {
                *address = address.take().or_else(|| variable(ADDRESS_VARIABLE));
                *format = format.take().or_else(|| variable(FORMAT_VARIABLE));
            }
        }
        self
    }
}

/// How the records are printed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordsFormat {
    Text,
    Csv,
}

impl RecordsFormat {
    fn chosen(format: Option<&str>) -> Result<Self, String> {
        match format {
            None | Some("text") => Ok(Self::Text),
            Some("csv") => Ok(Self::Csv),
            Some(other) => Err(format!("The format must be text or csv, not {other}.")),
        }
    }
}

fn given(address: Option<String>) -> Result<DeviceAddress, String> {
    let address = address.ok_or_else(|| format!("No device address was given: pass --address or set {ADDRESS_VARIABLE}."))?;
    DeviceAddress::new(address).map_err(|error| refusal(&error, cfg!(target_os = "macos")))
}

/// Why an address was refused, as the tool says it.
///
/// macOS hides the address of a device, so there zeros are what a user is most likely to have been shown.
pub fn refusal(error: &DeviceValidationError, on_macos: bool) -> String {
    match error {
        DeviceValidationError::ZeroAddress(_) if on_macos => format!("{error} On macOS the tool cannot tell devices apart yet."),
        _ => error.to_string(),
    }
}

/// Where a long operation shows how far it has got.
pub trait Progress {
    fn show(&mut self, done: u16, total: u16);
    /// Remove what was shown, so that what is printed next starts on a clean line.
    fn clear(&mut self);
}

/// Shows nothing, for when standard error is not a terminal.
pub struct NoProgress;

impl Progress for NoProgress {
    fn show(&mut self, _done: u16, _total: u16) {}

    fn clear(&mut self) {}
}

/// Draws a progress bar on the current line of standard error.
#[derive(Default)]
pub struct TerminalProgress {
    shown: bool,
}

impl Progress for TerminalProgress {
    fn show(&mut self, done: u16, total: u16) {
        self.shown = true;
        let mut stderr = std::io::stderr();
        let _ = write!(stderr, "\r{}", format_progress(done, total));
        let _ = stderr.flush();
    }

    fn clear(&mut self) {
        if std::mem::take(&mut self.shown) {
            let mut stderr = std::io::stderr();
            // Return to the start of the line and erase it.
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        }
    }
}

const PROGRESS_WIDTH: usize = 30;

/// A progress bar such as `[###############               ] 720 of 1440 records`.
pub fn format_progress(done: u16, total: u16) -> String {
    let filled = usize::from(done) * PROGRESS_WIDTH / usize::from(total.max(1));
    format!("[{:<PROGRESS_WIDTH$}] {done} of {total} records", "#".repeat(filled.min(PROGRESS_WIDTH)))
}

/// What a command has to say: a result, for standard output, or a notice that there is none, for standard error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Printed {
    Result(String),
    Notice(String),
}

/// What a command printed to standard output, and whether it succeeded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: String,
    pub success: bool,
}

/// Write a result to standard output.
///
/// A program that stopped reading early, as `head` does, has closed the pipe: what is left is dropped, which is not an error.
pub fn write_result(stdout: &mut impl std::io::Write, result: &str) -> std::io::Result<()> {
    match stdout.write_all(result.as_bytes()).and_then(|()| stdout.flush()) {
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        written => written,
    }
}

/// Run a parsed command against the supplied ports.
///
/// Warnings and errors are written to `stderr` as they arise, so a warning is visible before the device is contacted.
pub async fn run(
    cli: Cli,
    scanner: &impl DeviceScanner,
    connector: &impl DeviceConnector,
    clock: &impl Clock,
    stderr: &mut impl std::io::Write,
    progress: &mut impl Progress,
) -> Outcome {
    let result = match cli.command {
        Command::Discover { scan_duration } => render_discovery(scanner, scan_duration).await.map_err(|error| error.to_string()),
        Command::Logging { address } => match given(address) {
            Err(error) => Err(error),
            Ok(address) => with_device(connector, &address, async |device| logging_state(device).await)
                .await
                .map(|state| Printed::Result(format_logging_state(&state))),
        },
        Command::Read { address } => match given(address) {
            Err(error) => Err(error),
            Ok(address) => with_device(connector, &address, async |device| read(device).await)
                .await
                .map(|reading| Printed::Result(format_reading(&reading))),
        },
        Command::Records { address, format } => match (given(address), RecordsFormat::chosen(format.as_deref())) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(address), Ok(format)) => with_device(connector, &address, async |device| {
                let result = records(device, &mut |done, total| progress.show(done, total)).await;
                progress.clear();
                result
            })
            .await
            .map(|run| format_records(&run, format)),
        },
        Command::SetName { address, name } => match (given(address), DeviceLabel::new(name).map_err(|error| error.to_string())) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(address), Ok(label)) => {
                let _ = writeln!(stderr, "Warning: renaming during logging may reset the measuring-time counter.");
                with_device(connector, &address, async |device| set_name(device, &label).await)
                    .await
                    .map(|confirmed| Printed::Result(format!("Confirmed name: {confirmed}. Power-cycle the device to update its advertised name.")))
            }
        },
        Command::Start { address, days, no_logging } => {
            let duration = if no_logging { Some(LoggingDuration::OFF) } else { LoggingDuration::of_run(days) };
            match (given(address), duration.ok_or_else(|| "A run lasts from 1 to 31 days.".to_owned())) {
                (Err(error), _) | (_, Err(error)) => Err(error),
                (Ok(address), Ok(duration)) => {
                    let mut warn = |warning: String| {
                        let _ = writeln!(stderr, "Warning: {warning}");
                    };
                    with_device(connector, &address, async |device| start(device, duration, clock, &mut warn).await)
                        .await
                        .map(|state| Printed::Result(format_started(&state)))
                }
            }
        }
        Command::Stop { address } => match given(address) {
            Err(error) => Err(error),
            Ok(address) => with_device(connector, &address, async |device| stop(device).await)
                .await
                .map(|()| Printed::Result("Stopped measuring.".to_owned())),
        },
    };
    match result {
        Ok(Printed::Result(output)) => Outcome {
            stdout: format!("{output}\n"),
            success: true,
        },
        Ok(Printed::Notice(notice)) => {
            let _ = writeln!(stderr, "{notice}");
            Outcome {
                stdout: String::new(),
                success: true,
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "Error: {}", error.trim_end());
            Outcome {
                stdout: String::new(),
                success: false,
            }
        }
    }
}

/// Give the outcome of an operation once the device is disconnected, preferring the operation's own error.
fn after_disconnecting<T>(result: Result<T, OperationError>, disconnected: Result<(), PortError>) -> Result<T, OperationError> {
    let value = result?;
    disconnected?;
    Ok(value)
}

/// Connect to the device, run an operation over the connection, and disconnect whatever the outcome.
async fn with_device<C: DeviceConnector, T>(
    connector: &C,
    address: &DeviceAddress,
    operation: impl AsyncFnOnce(&mut C::Transport) -> Result<T, OperationError>,
) -> Result<T, String> {
    let mut device = connector.connect(address).await.map_err(|error| error.to_string())?;
    let result = operation(&mut device).await;
    after_disconnecting(result, device.disconnect().await).map_err(|error| error.to_string())
}

/// What a run that has just started is doing, from the logging state the device reports.
pub fn format_started(state: &LoggingState) -> String {
    if state.duration == LoggingDuration::OFF {
        return "Started measuring without recording.".to_owned();
    }
    format!(
        "Started a run of {} at {}, recording every {} minutes.",
        state.duration,
        utc(state.started),
        state.interval_minutes
    )
}

/// A time on the device's clock, which the tool sets to UTC, or `unknown` when the clock was not set.
fn utc(time: Option<ClockTime>) -> String {
    time.map_or_else(|| "unknown".to_owned(), |time| format!("{time} UTC"))
}

pub fn format_logging_state(state: &LoggingState) -> String {
    format!(
        "Logging duration: {}\nRun started: {}\nRecord interval: {} minutes\nRecords: {}",
        state.duration,
        utc(state.started),
        state.interval_minutes,
        state.records
    )
}

/// The records of a run, in the format asked for.
pub fn format_records(run: &Run, format: RecordsFormat) -> Printed {
    match format {
        RecordsFormat::Text => {
            let lines: Vec<String> = run
                .records()
                .map(|record| match record.recorded_at {
                    RecordTime::Clock(time) => format!("{time} UTC\t{} µSv/h", record.dose_rate),
                    since_start @ RecordTime::SinceStart { .. } => format!("{since_start}\t{} µSv/h", record.dose_rate),
                })
                .collect();
            if lines.is_empty() {
                Printed::Notice("No records.".to_owned())
            } else {
                Printed::Result(lines.join("\n"))
            }
        }
        RecordsFormat::Csv => {
            let lines = run.records().map(|record| {
                let recorded_at = match record.recorded_at {
                    RecordTime::Clock(time) => format!("{}Z", time.iso()),
                    RecordTime::SinceStart { .. } => String::new(),
                };
                format!("{recorded_at},{},{}", record.minutes_from_start, record.dose_rate)
            });
            let header = "recorded_at,minutes_from_start,dose_rate_usv_h".to_owned();
            Printed::Result(std::iter::once(header).chain(lines).collect::<Vec<_>>().join("\n"))
        }
    }
}

pub fn format_reading(reading: &Reading) -> String {
    // A stopped device reports the dose rates it held when it stopped; they are not shown, so they cannot pass for current ones.
    if !reading.is_measuring() {
        return format!("Status: {}\nLast run: {}\nBattery: {}", reading.status, reading.measuring_time, reading.battery_level());
    }
    format!(
        "Status: {}\nMeasuring time: {}\nAverage dose rate: {} µSv/h\nOne-minute dose rate: {} µSv/h {}\nBattery: {}",
        reading.status,
        reading.measuring_time,
        reading.average,
        reading.one_minute(),
        format_pulses(reading.last_minute),
        reading.battery_level()
    )
}

/// A number of pulses in brackets, such as `(1 pulse)`.
pub fn format_pulses(pulses: Pulses) -> String {
    format!("({pulses})")
}

/// Show control characters as backslash escapes so a name stays on one line.
pub fn escape_name(name: &str) -> String {
    let mut escaped = String::with_capacity(name.len());
    for character in name.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(escaped, "\\x{:02x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

pub fn format_device(device: &DeviceInfo) -> String {
    let name = device.name.as_ref().map_or_else(|| "<unnamed>".to_owned(), |name| escape_name(&name.to_string()));
    let rssi = device.rssi.map(|value| format!("{value} dBm")).unwrap_or_else(|| "RSSI unavailable".to_owned());
    format!("{}\t{name}\t{rssi}", device.address)
}

pub async fn render_discovery(scanner: &impl DeviceScanner, scan_duration: Duration) -> Result<Printed, DiscoveryError> {
    let devices = discover_devices(scanner, scan_duration).await?;
    if devices.is_empty() {
        return Ok(Printed::Notice("No BSG-001 devices found.".to_owned()));
    }
    Ok(Printed::Result(devices.iter().map(format_device).collect::<Vec<_>>().join("\n")))
}

fn parse_scan_duration(value: &str) -> Result<Duration, String> {
    let seconds = value.parse::<f64>().map_err(|_| "timeout must be a number of seconds".to_owned())?;
    if !seconds.is_finite() || seconds < 0.1 {
        return Err("timeout must be at least 0.1 seconds".to_owned());
    }
    Duration::try_from_secs_f64(seconds).map_err(|_| "timeout is too large".to_owned())
}
