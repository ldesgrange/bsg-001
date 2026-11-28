//! Executable examples from `specs/`.
//!
//! The examples run against fake Bluetooth devices by default.
//! With `BSG001_E2E_REAL=1` they run against a real BSG-001 nearby, using the real Bluetooth adapters.
//! An example that only a fake can reproduce is then reported as ignored, with the reason.
//! An example that renames the device puts the original name back afterwards.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use bsg_001::adapters::bluetooth::{BtleplugDeviceScanner, LiveSightings, Report, device_info_from_advertisement};
use bsg_001::adapters::cli::{Cli, Command, NoProgress, escape_name, format_pulses, run, write_result};
use bsg_001::adapters::device_bluetooth::BluetoothConnector;
use bsg_001::adapters::system_clock::SystemClock;
use bsg_001::application::logging::logging_state;
use bsg_001::application::read::read;
use bsg_001::application::records::records;
use bsg_001::application::run::{start, stop};
use bsg_001::application::set_name::set_name;
use bsg_001::domain::logging::{Download, LoggingDuration, LoggingState, Run};
use bsg_001::domain::protocol::{Opcode, command_frame, decode_number, decode_text, encode_text, reply_content};
use bsg_001::domain::reading::{BatteryLevel, DoseRate, Reading};
use bsg_001::{Clock, DeviceConnector, DeviceInfo, DeviceLabel, DeviceScanner, OperationError, discover_devices};
use clap::{CommandFactory, Parser};
#[path = "support/fakes.rs"]
mod fakes;
#[path = "support/report.rs"]
mod report;

use fakes::{FakeConnector, FakeDevice, FakeScanner, FixedClock};
use libtest_mimic::{Arguments, Completion, Failed, Trial};
use tokio::runtime::Runtime;
use varar::{HandlerError, IntoSensor, Registry, Steps};
use varar_core::reference::{OathWorkspace, build_workspace};
use varar_runner::{example_names, find_oaths, plan_oath};

const PROMPT: &str = "$ ";
const COMMAND: &str = "bsg-001";
/// The time in UTC unless an example says otherwise.
const CLOCK: &str = "2026-01-02 03:04:05";
const ADDRESS: &str = "AA:BB:CC:DD:EE:FF";
const NAME_PREFIX: &str = "BLE_Gamma:";

fn hardware() -> bool {
    std::env::var("BSG001_E2E_REAL").is_ok_and(|value| value == "1")
}

type Row = BTreeMap<String, String>;

thread_local! {
    /// Why the example running on this thread was skipped, when it was.
    static SKIPPED: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The expressions of the steps that run the command, which make an example end-to-end.
    static E2E_STEPS: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

/// Register a sensor that runs the command, so that the examples using it are reported as end-to-end.
#[track_caller]
fn e2e_sensor<Args>(steps: &mut Steps<World>, expression: &str, handler: impl IntoSensor<World, Args>) {
    E2E_STEPS.with_borrow_mut(|e2e| e2e.insert(expression.to_owned()));
    steps.sensor(expression, handler);
}

/// The fake Bluetooth surroundings an example describes.
#[derive(Clone, Default)]
struct World {
    nearby: Vec<DeviceInfo>,
    device: Option<FakeDevice>,
    /// The address the example gives its device, for which the real device stands in on hardware.
    address: Option<String>,
    /// The address, name and signal strength under which the example lists its device.
    listed: Option<[String; 3]>,
    /// Whether the example shows values a device reports, for which those of the real device stand in on hardware.
    shows_device_values: bool,
    /// Why a real device cannot reproduce the example, when it cannot.
    needs_fake: Option<String>,
    /// The time in UTC, when the example says so.
    clock: Option<String>,
    /// The time in UTC once the connection to the device is made, when the example says that connecting takes time.
    connected_at: Option<String>,
    /// The last terminal session of the example, shared so that a later step can look at it.
    last_session: Rc<RefCell<Option<Session>>>,
}

impl World {
    /// Mark an example that a real device cannot reproduce.
    fn needing_fake(mut self, what: &str) -> Self {
        if hardware() {
            self.needs_fake = Some(format!("{what} can only be reproduced by a fake device"));
        }
        self
    }
}

/// What running the tool produced.
#[derive(Clone)]
struct Session {
    stdout: String,
    stderr: String,
    success: bool,
}

/// The outcome of asking for the tool to be run: a session, or the reason it was not run on hardware.
enum Attempt {
    Ran(Session),
    NotRun(String),
}

/// Stands in for standard output once the program reading it has stopped.
struct ClosedPipe;

impl std::io::Write for ClosedPipe {
    fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    Runtime::new().expect("the test runtime starts").block_on(future)
}

/// Read a command line such as `BSG_001_ADDRESS=AA:BB:CC:DD:EE:FF bsg-001 read`, as a shell would.
fn parse(command_line: &[&str]) -> Result<Cli, HandlerError> {
    let variables = command_line.iter().take_while(|word| **word != COMMAND).count();
    let (environment, arguments) = command_line.split_at(variables);
    if arguments.is_empty() {
        return Err(HandlerError::new(format!("The session must run {COMMAND}")));
    }
    let variable = |name: &str| {
        environment
            .iter()
            .find_map(|word| word.split_once('=').filter(|(variable, _)| *variable == name).map(|(_, value)| value.to_owned()))
    };
    let cli = Cli::try_parse_from(arguments.iter().copied()).map_err(|error| HandlerError::new(error.to_string()))?;
    Ok(cli.with_environment(variable))
}

fn session(cli: Cli, scanner: &impl DeviceScanner, connector: &impl DeviceConnector, clock: &impl Clock) -> Session {
    let mut stderr = Vec::new();
    let outcome = block_on(run(cli, scanner, connector, clock, &mut stderr, &mut NoProgress));
    Session {
        stdout: outcome.stdout,
        stderr: String::from_utf8(stderr).expect("standard error is UTF-8"),
        success: outcome.success,
    }
}

/// The address and advertised name of the real BSG-001 the examples run against.
///
/// Only a successful scan is remembered, so one missed scan does not fail every later example.
fn real_device() -> Result<&'static (String, String), HandlerError> {
    static DEVICE: OnceLock<(String, String)> = OnceLock::new();
    if let Some(device) = DEVICE.get() {
        return Ok(device);
    }
    let devices = block_on(discover_devices(&BtleplugDeviceScanner::new(), Duration::from_secs(20))).map_err(|error| HandlerError::new(error.to_string()))?;
    let device = devices.first().ok_or_else(|| HandlerError::new("No BSG-001 was discovered nearby."))?;
    Ok(DEVICE.get_or_init(|| (device.address.to_string(), device.name.as_ref().map(ToString::to_string).unwrap_or_default())))
}

/// Run the tool against the real device, which stands in for the device of the example.
fn run_on_hardware(world: &World, arguments: &[&str]) -> Result<Attempt, HandlerError> {
    let (address, advertised) = real_device()?;
    // The real address stands in for the address of the example, in an argument or in an environment variable.
    let arguments: Vec<String> = arguments
        .iter()
        .map(|argument| {
            world
                .address
                .as_deref()
                .map_or_else(|| (*argument).to_owned(), |example| argument.replace(example, address))
        })
        .collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let renames = matches!(arguments.as_slice(), [COMMAND, "set-name", "--address", _, name] if DeviceLabel::new(*name).is_ok());
    if !renames {
        return Ok(Attempt::Ran(session(parse(&arguments)?, &BtleplugDeviceScanner::new(), &BluetoothConnector, &SystemClock)));
    }
    // The advertised label is the name the device had when it was switched on, which is what is put back.
    let original = advertised.strip_prefix(NAME_PREFIX).unwrap_or_default();
    if DeviceLabel::new(original).is_err() {
        return Err(HandlerError::new(
            "The advertised label of the real device is not a valid name, so it could not be put back.",
        ));
    }
    let ran = session(parse(&arguments)?, &BtleplugDeviceScanner::new(), &BluetoothConnector, &SystemClock);
    let restored = session(
        parse(&[COMMAND, "set-name", "--address", address, original])?,
        &BtleplugDeviceScanner::new(),
        &BluetoothConnector,
        &SystemClock,
    );
    assert!(restored.success, "THE REAL DEVICE WAS LEFT RENAMED: putting its name back failed with {}", restored.stderr);
    Ok(Attempt::Ran(ran))
}

/// Run the tool against the Bluetooth surroundings of the example.
fn run_tool(world: &World, arguments: &[&str]) -> Result<Attempt, HandlerError> {
    if let Some(reason) = &world.needs_fake {
        return Ok(Attempt::NotRun(reason.clone()));
    }
    if hardware() {
        return run_on_hardware(world, arguments);
    }
    let scanner = FakeScanner::seeing(world.nearby.clone());
    let clock = FixedClock::showing(world.clock.as_deref().unwrap_or(CLOCK));
    let connector = FakeConnector {
        device: world.device.clone(),
        connected_at: world.connected_at.as_deref().map(|time| (clock.clone(), FixedClock::showing(time).time())),
    };
    Ok(Attempt::Ran(session(parse(arguments)?, &scanner, &connector, &clock)))
}

/// Stop an example that a real device cannot reproduce, so that it is reported as ignored.
fn skip(reason: String) -> HandlerError {
    let error = HandlerError::new(format!("Skipped: {reason}"));
    SKIPPED.set(Some(reason));
    error
}

/// Show the real device under the address, name and signal strength the example gives its device.
fn as_in_the_example(world: &World, output: &str) -> Result<String, HandlerError> {
    let Some([address, name, rssi]) = world.listed.as_ref().filter(|_| hardware()) else {
        return Ok(output.to_owned());
    };
    let (real_address, real_name) = real_device()?;
    Ok(output
        .replace(real_address, address)
        .replace(&escape_name(real_name), name)
        .split_inclusive('\n')
        .map(|line| match line.trim_end_matches('\n').rsplit_once('\t') {
            Some((start, _signal)) => format!("{start}\t{rssi} dBm\n"),
            None => line.to_owned(),
        })
        .collect())
}

/// Turn Unicode control pictures such as ␊ into the characters they depict.
fn from_control_pictures(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '\u{2400}'..='\u{2420}' => char::from_u32(character as u32 - 0x2400).expect("a control character"),
            '\u{2421}' => '\u{7f}',
            other => other,
        })
        .collect()
}

/// The status a session shows, when it shows one.
fn status_shown(session: &str) -> Option<&str> {
    session.lines().find_map(|line| line.strip_prefix("Status: "))
}

/// What a session looks like once the values a device reports are set aside: every number, and every word that stands for one.
fn shape(session: &str) -> String {
    let words = session
        .replace("Run started: unknown", "Run started: 0-0-0 0:0:0")
        .replace("Logging duration: off", "Logging duration: 0 days");
    let mut shape = String::new();
    for character in words.chars() {
        match (character.is_ascii_digit(), shape.ends_with('#')) {
            (true, true) => {}
            (true, false) => shape.push('#'),
            (false, _) => shape.push(character),
        }
    }
    let shape = shape
        .replace("# day\n", "# days\n")
        .replace("(# pulse)", "(# pulses)")
        .replace("# d #:#:#\t", "#-#-# #:#:#\t");
    // A device holds any number of records, so the lines of a run of records count as one.
    let mut lines: Vec<&str> = shape.lines().collect();
    lines.dedup();
    lines.join("\n")
}

fn is_ordinary_name(name: &str) -> bool {
    name.strip_prefix(NAME_PREFIX).is_some_and(|label| DeviceLabel::new(label).is_ok())
}

/// Dose rates as a list, or `none` when there are none.
fn listed(records: &[DoseRate]) -> String {
    if records.is_empty() {
        return "none".to_owned();
    }
    records.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
}

/// The dose rates of a list such as `0.05, 0.10`, or of `none`.
fn dose_rates(list: &str) -> Vec<DoseRate> {
    if list == "none" {
        return Vec::new();
    }
    let hundredths = |text: &str| text.replace('.', "").parse().expect("a dose rate with two decimal places");
    list.split(", ").map(|text| DoseRate::from_hundredths(hundredths(text))).collect()
}

/// The frames written to a device, in order.
fn frames(device: &FakeDevice) -> String {
    device.writes().iter().map(|frame| to_hex(frame)).collect::<Vec<_>>().join(", ")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(" ")
}

fn from_hex(text: &str) -> Vec<u8> {
    text.split_whitespace().map(|byte| u8::from_str_radix(byte, 16).expect("a hexadecimal byte")).collect()
}

fn row_with(column: &str, value: impl Into<String>) -> Row {
    BTreeMap::from([(column.to_owned(), value.into())])
}

fn tool_steps(steps: &mut Steps<World>) {
    steps.stimulus("These Bluetooth devices are advertising nearby:", |mut world: World, table: Vec<Vec<String>>| {
        if hardware() {
            // A real device can stand in for the example only when a lone, ordinary BSG-001 is advertising.
            return Ok(match &table[1..] {
                [row] if is_ordinary_name(&row[1]) && row[2].parse::<i16>().is_ok() => World {
                    address: Some(row[0].clone()),
                    listed: Some([row[0].clone(), row[1].clone(), row[2].clone()]),
                    ..world
                },
                _ => world.needing_fake("These surroundings"),
            });
        }
        world.nearby = table
            .iter()
            .skip(1)
            .filter_map(|row| {
                let name = (!row[1].is_empty()).then(|| from_control_pictures(&row[1]));
                let rssi = (!row[2].is_empty()).then(|| row[2].parse().expect("the oath RSSI is a number"));
                // As the real scanner does, leave out a device whose address cannot select it.
                device_info_from_advertisement(&row[0], name, rssi).ok()
            })
            .collect();
        Ok(world)
    });
    steps.stimulus("No Bluetooth device is advertising nearby.", |mut world: World| {
        world.nearby = Vec::new();
        Ok(world.needing_fake("Empty surroundings"))
    });
    steps.stimulus("A device at address {string} stores the name it is sent.", |mut world: World, address: String| {
        world.device = Some(FakeDevice::storing(&address));
        world.address = Some(address);
        Ok(world)
    });
    steps.stimulus(
        "A device at address {string} keeps the name {string} whatever it is sent.",
        |mut world: World, address: String, name: String| {
            world.device = Some(FakeDevice::keeping(&address, &name));
            world.address = Some(address);
            Ok(world.needing_fake("A device that keeps its name"))
        },
    );

    steps.stimulus(
        "A device at address {string} reports this reading:",
        |mut world: World, address: String, table: Vec<Vec<String>>| {
            let device = FakeDevice::storing(&address).reporting_reading(&table[1]);
            world.device = Some(device);
            world.address = Some(address);
            world.shows_device_values = true;
            Ok(world)
        },
    );
    steps.stimulus(
        "A device at address {string} reports this logging state:",
        |mut world: World, address: String, table: Vec<Vec<String>>| {
            let device = FakeDevice::storing(&address).reporting_logging_state(&table[1]);
            world.device = Some(device);
            world.address = Some(address);
            world.shows_device_values = true;
            Ok(world)
        },
    );
    steps.stimulus(
        "A device at address {string} answers the logging state query with a reply that is too short.",
        |mut world: World, address: String| {
            let device = FakeDevice::storing(&address).answering(Opcode::QueryLoggingState, vec![0x21, 0x0b, 0x0a, 0x1a]);
            world.device = Some(device);
            world.address = Some(address);
            Ok(world.needing_fake("A reply that is too short"))
        },
    );
    steps.stimulus(
        "A device at address {string} answers the reading query with a reply that is too short.",
        |mut world: World, address: String| {
            let device = FakeDevice::storing(&address).answering(Opcode::QueryReading, vec![0xa0, 0x0b, 0x01, 0x00]);
            world.device = Some(device);
            world.address = Some(address);
            Ok(world.needing_fake("A reply that is too short"))
        },
    );

    steps.stimulus("A device at address {string} is stopped.", |mut world: World, address: String| {
        world.device = Some(FakeDevice::stopped(&address));
        world.address = Some(address);
        Ok(world.needing_fake("A run that is started or stopped"))
    });
    steps.stimulus("A device at address {string} is measuring.", |mut world: World, address: String| {
        world.device = Some(FakeDevice::storing(&address));
        world.address = Some(address);
        Ok(world.needing_fake("A run that is started or stopped"))
    });
    steps.stimulus("The device does not start or stop when told to.", |mut world: World| {
        world.device = world.device.map(FakeDevice::ignoring_start_and_stop);
        Ok(world)
    });
    steps.stimulus("It is {string} in UTC.", |mut world: World, time: String| {
        world.clock = Some(time);
        Ok(world)
    });
    steps.stimulus("The connection to the device is made at {string} in UTC.", |mut world: World, time: String| {
        world.connected_at = Some(time);
        Ok(world.needing_fake("A connection that takes a known time"))
    });
    steps.stimulus("The device holds these records:", |mut world: World, table: Vec<Vec<String>>| {
        let dose_rates: Vec<String> = table.iter().skip(1).map(|row| row[0].clone()).collect();
        world.device = world.device.map(|device| device.holding_records(&dose_rates));
        Ok(world)
    });
    steps.stimulus(
        "While the tool downloads the records for the first time, the device writes a record of {string}.",
        |mut world: World, dose_rate: String| {
            world.device = world.device.map(|device| device.writing_a_record_during_the_download(&dose_rate));
            Ok(world.needing_fake("A record written during the download"))
        },
    );
    steps.stimulus("The device sends records that never match its checksum.", |mut world: World| {
        world.device = world.device.map(FakeDevice::sending_wrong_records);
        Ok(world.needing_fake("Records that do not match the checksum"))
    });
    steps.stimulus("While the tool downloads the records for the first time, the last packet is lost.", |mut world: World| {
        world.device = world.device.map(|device| device.losing_the_last_packet_of(1));
        Ok(world.needing_fake("A packet lost during the download"))
    });
    steps.stimulus("The last packet of every download is lost.", |mut world: World| {
        world.device = world.device.map(|device| device.losing_the_last_packet_of(usize::MAX));
        Ok(world.needing_fake("A packet lost during the download"))
    });

    e2e_sensor(steps, "Here is the terminal session:", |world: World, expected: String| {
        let command = expected.lines().next().unwrap_or_default();
        let arguments = command
            .strip_prefix(PROMPT)
            .ok_or_else(|| HandlerError::new(format!("The session must start with {PROMPT}")))?;
        let session = match run_tool(&world, &arguments.split_whitespace().collect::<Vec<_>>())? {
            Attempt::Ran(session) => session,
            Attempt::NotRun(reason) => return Err(skip(reason)),
        };
        world.last_session.replace(Some(session.clone()));
        let failed = session.stderr.lines().next_back().is_some_and(|line| line.starts_with("Error:"));
        assert_eq!(
            !session.success, failed,
            "a command fails exactly when it ends with an Error: line, but it printed {:?}",
            session.stderr
        );
        assert!(!failed || session.stdout.is_empty(), "a failed command prints no result");
        let shown = format!("{command}\n{}", as_in_the_example(&world, &format!("{}{}", session.stderr, session.stdout))?);
        if hardware() && world.shows_device_values {
            // A real device is either measuring or stopped, and only the examples for the state it is in can be reproduced.
            if let (Some(real), Some(example)) = (status_shown(&shown), status_shown(&expected))
                && real != example
            {
                return Err(skip(format!("The real device is {real}, so it cannot show an example of a device that is {example}")));
            }
            // A real device reports its own values, so only the shape of what is shown is compared.
            if shape(&shown) == shape(&expected) {
                return Ok(expected);
            }
        }
        Ok(shown)
    });
    steps.sensor("Nothing is written to the device.", |world: World| {
        if hardware() {
            return Ok(()); // What a real device receives cannot be observed.
        }
        let device = world.device.expect("the example describes a device");
        assert!(device.writes().is_empty(), "the device received {}", frames(&device));
        Ok(())
    });
    steps.sensor("Nothing is printed on standard output.", |world: World| {
        let session = world.last_session.borrow().clone().expect("the example shows a terminal session first");
        assert_eq!(session.stdout, "", "standard output is empty");
        Ok(())
    });
    steps.sensor("A result written to a program that has stopped reading is dropped without an error.", |_world: World| {
        write_result(&mut ClosedPipe, "a result\n").map_err(|error| HandlerError::new(error.to_string()))
    });
    steps.sensor("Nothing but queries is written to the device.", |world: World| {
        if hardware() {
            return Ok(()); // What a real device receives cannot be observed.
        }
        let device = world.device.expect("the example describes a device");
        assert!(device.writes().iter().all(|frame| frame.len() == 1), "the device received {}", frames(&device));
        Ok(())
    });
    steps.sensor("For each number of days, a run records at the record interval shown.", |_world: World, row: Row| {
        let days = row["days"].parse().map_err(|_| HandlerError::new("A number of days is a number."))?;
        let interval = LoggingDuration::of_run(days).map_or_else(|| "refused".to_owned(), |duration| format!("{} minutes", duration.record_interval_minutes()));
        Ok(row_with("record interval", interval))
    });
    steps.sensor("Running `bsg-001 --help` lists the commands {string}.", |_world: World, _expected: String| {
        let help = Cli::try_parse_from(["bsg-001", "--help"]).err().expect("asking for help stops the command").to_string();
        let mut commands: Vec<_> = Cli::command()
            .get_subcommands()
            .map(|command| command.get_name().to_owned())
            .filter(|name| name != "help" && help.contains(name.as_str()))
            .collect();
        commands.sort();
        Ok(commands.join(", "))
    });

    steps.sensor("For each battery sensor value, the tool shows the percentage in this table.", |_world: World, row: Row| {
        let sensor = row["battery sensor"].parse().map_err(|_| HandlerError::new("A battery sensor value is a number."))?;
        Ok(row_with("percentage", BatteryLevel::from_sensor(sensor).to_string()))
    });
    e2e_sensor(
        steps,
        "The tool checks each name and gives its verdict before writing anything to the device.",
        |_world: World, row: Row| {
            let device = FakeDevice::storing(ADDRESS);
            let world = World {
                device: Some(device.clone()),
                address: Some(ADDRESS.to_owned()),
                ..World::default()
            };
            let session = match run_tool(&world, &[COMMAND, "set-name", "--address", ADDRESS, &row["name"]])? {
                Attempt::Ran(session) => session,
                Attempt::NotRun(reason) => return Err(skip(reason)),
            };
            let verdict = if session.success {
                "valid"
            } else {
                assert!(device.writes().is_empty(), "an invalid name was written to the device");
                "invalid"
            };
            Ok(row_with("verdict", verdict))
        },
    );
    e2e_sensor(
        steps,
        "When the device keeps the stored name shown, renaming it to ABCD fails with the error shown.",
        |_world: World, row: Row| {
            let world = World {
                device: Some(FakeDevice::keeping(ADDRESS, &from_control_pictures(&row["stored"]))),
                ..World::default()
            }
            .needing_fake("A device that keeps its name");
            let session = match run_tool(&world, &[COMMAND, "set-name", "--address", ADDRESS, "ABCD"])? {
                Attempt::Ran(session) => session,
                Attempt::NotRun(reason) => return Err(skip(reason)),
            };
            assert!(!session.success, "the rename succeeded");
            assert!(session.stdout.is_empty(), "a failed command printed {}", session.stdout);
            let error = session.stderr.lines().next_back().unwrap_or_default();
            Ok(row_with("error", error.strip_prefix("Error: ").unwrap_or(error)))
        },
    );
    steps.sensor(
        "Whether a device is seen depends on whether it was remembered and on what is reported during the scan.",
        |_world: World, row: Row| {
            let device = "a device";
            let remembered = if row["remembered"] == "yes" { vec![device] } else { Vec::new() };
            let mut sightings = LiveSightings::remembering(remembered);
            let mut seen = false;
            for reported in row["reported during the scan"].split(", ") {
                seen |= match reported {
                    "found" => sightings.report(&device, Report::Found),
                    "advertisement" => sightings.report(&device, Report::Advertisement),
                    "nothing" => false,
                    other => return Err(HandlerError::new(format!("A scan does not report {other}."))),
                };
            }
            Ok(row_with("seen", if seen { "yes" } else { "no" }))
        },
    );
    steps.sensor("Without `--timeout`, the scan lasts {string} seconds.", |_world: World, _expected: String| {
        let cli = Cli::try_parse_from([COMMAND, "discover"]).map_err(|error| HandlerError::new(error.to_string()))?;
        let Command::Discover { scan_duration } = cli.command else {
            return Err(HandlerError::new("discover was not read as the discover command."));
        };
        Ok(scan_duration.as_secs_f64().to_string())
    });
    steps.sensor("The tool checks each timeout and gives its verdict before scanning.", |_world: World, row: Row| {
        let accepted = Cli::try_parse_from(["bsg-001", "discover", &format!("--timeout={}", row["timeout"])]).is_ok();
        Ok(row_with("verdict", if accepted { "valid" } else { "invalid" }))
    });
}

/// An example that failed because the operation did.
fn failed(error: OperationError) -> HandlerError {
    HandlerError::new(error.to_string())
}

/// The opcode written in hexadecimal, such as `31`.
fn opcode(hex: &str) -> Opcode {
    u8::from_str_radix(hex, 16).ok().and_then(Opcode::from_byte).expect("the opcode of an operation")
}

fn protocol_steps(steps: &mut Steps<World>) {
    steps.sensor("The tool builds the frame for an opcode and a payload as shown.", |_world: World, row: Row| {
        let opcode = opcode(&row["opcode"]);
        Ok(row_with("frame", to_hex(&command_frame(opcode, &from_hex(&row["payload"])))))
    });
    steps.sensor("The tool takes the content of the reply to a query as shown.", |_world: World, row: Row| {
        let query = opcode(&row["query"]);
        let content = reply_content(query, &from_hex(&row["reply"])).map_or_else(|_| "rejected".to_owned(), to_hex);
        Ok(row_with("content", content))
    });
    steps.sensor("The tool turns text into bytes as shown.", |_world: World, row: Row| {
        let bytes = encode_text(&row["text"]).map_err(|error| HandlerError::new(error.to_string()))?;
        Ok(row_with("bytes", to_hex(&bytes)))
    });
    steps.sensor("The tool reads text from the start of some content as shown.", |_world: World, row: Row| {
        Ok(row_with("text", decode_text(&from_hex(&row["content"])).unwrap_or_else(|_| "rejected".to_owned())))
    });
    steps.sensor("The tool reads the number held in two bytes as shown.", |_world: World, row: Row| {
        let number = decode_number(&from_hex(&row["bytes"])).map_err(|error| HandlerError::new(error.to_string()))?;
        Ok(row_with("number", number.to_string()))
    });
    steps.sensor(
        "From each reply to the reading query, the tool reads the status, measuring time, average dose rate, last-minute dose rate¹ and battery sensor value shown.",
        |_world: World, row: Row| {
            let reply = from_hex(&row["reply"]);
            let reading = reply_content(Opcode::QueryReading, &reply)
                .and_then(Reading::decode)
                .map_err(|error| HandlerError::new(error.to_string()))?;
            Ok(BTreeMap::from([
                ("status".to_owned(), reading.status.to_string()),
                ("measuring time".to_owned(), reading.measuring_time.to_string()),
                ("average dose rate".to_owned(), reading.average.to_string()),
                (
                    "last-minute dose rate¹".to_owned(),
                    format!("{} {}", reading.one_minute(), format_pulses(reading.last_minute)),
                ),
                ("battery sensor".to_owned(), reading.battery_sensor.to_string()),
            ]))
        },
    );
    steps.sensor(
        "From each reply to the logging state query, the tool reads the logging duration, run started, record interval, records and checksum shown.",
        |_world: World, row: Row| {
            let reply = from_hex(&row["reply"]);
            let state = reply_content(Opcode::QueryLoggingState, &reply)
                .and_then(LoggingState::decode)
                .map_err(|error| HandlerError::new(error.to_string()))?;
            Ok(BTreeMap::from([
                ("logging duration".to_owned(), state.duration.to_string()),
                ("run started".to_owned(), state.started()),
                ("record interval".to_owned(), format!("{} minutes", state.interval_minutes)),
                ("records".to_owned(), state.records.to_string()),
                ("checksum".to_owned(), format!("{:02x}", state.checksum)),
            ]))
        },
    );
    steps.sensor(
        "Given the number of records, the tool reads from the packets the records shown.",
        |_world: World, row: Row| {
            let number = row["number"].parse().map_err(|_| HandlerError::new("A number of records is a number."))?;
            let mut download = Download::expecting(number);
            for packet in row["packets"].split(',') {
                download.receive(&from_hex(packet));
            }
            Ok(row_with("records", download.records().map_or_else(|_| "rejected".to_owned(), |records| listed(&records))))
        },
    );
    steps.sensor("The tool works out the checksum of the records shown.", |_world: World, row: Row| {
        let records = dose_rates(&row["records"]);
        let mut download = Download::expecting(u16::try_from(records.len()).expect("a few records"));
        for record in records {
            download.receive(&u16::try_from(record.hundredths()).expect("a dose rate that fits in two bytes").to_le_bytes());
        }
        Ok(row_with("checksum", format!("{:02x}", download.checksum())))
    });
    steps.sensor(
        "While it downloads the number of records shown, the tool reports the progress shown.",
        |_world: World, row: Row| {
            let number: usize = row["records"].parse().map_err(|_| HandlerError::new("A number of records is a number."))?;
            let mut device = FakeDevice::storing(ADDRESS).holding_records(&vec!["0.05".to_owned(); number]);
            let mut steps = Vec::new();
            block_on(records(&mut device, &mut |done, total| steps.push(format!("{done} of {total}")))).map_err(failed)?;
            Ok(row_with("progress", if steps.is_empty() { "none".to_owned() } else { steps.join(", ") }))
        },
    );
    steps.sensor("Of the records the device holds, the tool keeps those shown.", |_world: World, row: Row| {
        let state = block_on(logging_state(&mut FakeDevice::storing(ADDRESS))).map_err(failed)?;
        let kept: Vec<DoseRate> = Run::new(state, dose_rates(&row["device holds"])).records().map(|record| record.dose_rate).collect();
        Ok(row_with("tool keeps", listed(&kept)))
    });
    steps.sensor(
        "Given when the run started and its record interval, each record was recorded at the time shown.",
        |_world: World, row: Row| {
            let state = [row["run started"].clone(), row["record interval"].clone()];
            let device = FakeDevice::storing(ADDRESS).reporting_logging_state(&["10 days".to_owned(), state[0].clone(), state[1].clone(), "0".to_owned()]);
            let state = block_on(logging_state(&mut device.clone())).map_err(failed)?;
            let record = row["record"].parse().map_err(|_| HandlerError::new("A record is a number."))?;
            Ok(row_with("recorded at", state.recorded_at(record).to_string()))
        },
    );
    steps.sensor("The tool sends each time as the payload shown.", |_world: World, row: Row| {
        Ok(row_with("payload", to_hex(&FixedClock::showing(&row["time"]).time().bytes())))
    });
    steps.sensor("Each operation of the tool writes the frames shown, in order.", |_world: World, row: Row| {
        let mut device = FakeDevice::storing(ADDRESS);
        let arguments: Vec<&str> = row["operation"].split_whitespace().collect();
        match arguments.as_slice() {
            ["set-name", name] => {
                let label = DeviceLabel::new(*name).map_err(|error| HandlerError::new(error.to_string()))?;
                block_on(set_name(&mut device, &label)).map(drop).map_err(failed)?;
            }
            ["logging"] => block_on(logging_state(&mut device)).map(drop).map_err(failed)?,
            ["read"] => block_on(read(&mut device)).map(drop).map_err(failed)?,
            ["start", "--days", days] => {
                let duration = days
                    .parse()
                    .ok()
                    .and_then(LoggingDuration::of_run)
                    .ok_or_else(|| HandlerError::new("A run lasts from 1 to 31 days."))?;
                device = FakeDevice::stopped(ADDRESS);
                block_on(start(&mut device, duration, &FixedClock::showing(CLOCK), &mut |_| {})).map(drop).map_err(failed)?;
            }
            ["start", "--no-logging"] => {
                device = FakeDevice::stopped(ADDRESS);
                block_on(start(&mut device, LoggingDuration::OFF, &FixedClock::showing(CLOCK), &mut |_| {}))
                    .map(drop)
                    .map_err(failed)?;
            }
            ["stop"] => block_on(stop(&mut device)).map_err(failed)?,
            ["records"] => block_on(records(&mut device, &mut |_, _| {})).map(drop).map_err(failed)?,
            _ => return Err(HandlerError::new(format!("No binding for the operation {}.", row["operation"]))),
        };
        Ok(row_with("frames", frames(&device)))
    });
}

fn build_registry() -> Registry {
    let mut steps = Steps::new();
    tool_steps(&mut steps);
    protocol_steps(&mut steps);
    steps.into_registry()
}

fn context(_oath_file: &str) -> Rc<dyn Any> {
    Rc::new(World::default())
}

/// Every oath, by its path from the repository root, with its source, and the workspace that resolves the references between them.
fn load_oaths(root: &Path) -> (Vec<(String, String)>, OathWorkspace) {
    let config = varar_config::read_config(root).unwrap_or_else(|error| panic!("{error}"));
    let oaths: Vec<(String, String)> = find_oaths(&config, root)
        .iter()
        .map(|path| {
            let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            (relative, std::fs::read_to_string(path).expect("the oath can be read"))
        })
        .collect();
    let documents: Vec<_> = oaths.iter().map(|(path, source)| varar_core::parse::parse(path, source)).collect();
    let workspace = build_workspace(&documents);
    (oaths, workspace)
}

/// Render the oaths, with the results of the last run, as a site in `out`.
fn specs_report(root: &Path, out: &Path) {
    let (oaths, workspace) = load_oaths(root);
    let registry = build_registry();
    let e2e = E2E_STEPS.take();
    report::render_site(root, out, &oaths, &registry, &workspace, |step| e2e.contains(&step.expression))
        .unwrap_or_else(|error| panic!("the report could not be written to {}: {error}", out.display()));
    println!("The report is in {}", out.join("index.html").display());
}

/// One trial per example, like `varar_cargotest`, except that an example can skip itself while it runs.
///
/// Drift is left alone: the run against the fakes reconciles `varar.lock.json`.
fn hardware_trials(root: &Path) -> Vec<Trial> {
    let (oaths, workspace) = load_oaths(root);
    let workspace = Arc::new(workspace);
    let registry = build_registry();
    let mut trials = Vec::new();
    for (path, source) in oaths {
        let names = example_names(&plan_oath(&path, &source, &registry, &workspace));
        for (index, name) in names.into_iter().enumerate() {
            let (path, source, workspace) = (path.clone(), source.clone(), Arc::clone(&workspace));
            trials.push(Trial::ignorable_test(format!("{path}::{name}"), move || {
                SKIPPED.set(None);
                let outcome = varar_cargotest::run_one(&path, &source, &path, build_registry, context, index, &workspace);
                match (SKIPPED.take(), outcome) {
                    (Some(reason), _) => Ok(Completion::ignored_with(reason)),
                    (None, Ok(())) => Ok(Completion::Completed),
                    (None, Err(failure)) => Err(Failed::from(failure)),
                }
            }));
        }
    }
    trials
}

fn main() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(out) = std::env::var_os("BSG001_SPECS_REPORT") {
        return specs_report(repository_root, Path::new(&out));
    }
    if hardware() {
        libtest_mimic::run(&Arguments::from_args(), hardware_trials(repository_root)).exit();
    }
    varar_cargotest::run(repository_root, build_registry, context);
}
