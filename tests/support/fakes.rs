//! The fake Bluetooth surroundings shared by every test: one scanner, one device and one connector.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use bsg_001::domain::logging::ClockTime;
use bsg_001::domain::protocol::Opcode;
use bsg_001::{Clock, DeviceAddress, DeviceConnector, DeviceInfo, DeviceScanner, DeviceTransport, PortError};

const REPLY_LENGTH: usize = 20;
const SET_NAME: u8 = Opcode::SetName.byte();
const QUERY_RECORDS: u8 = Opcode::QueryRecords.byte();
const START_STOP: u8 = Opcode::StartStop.byte();
const SET_CLOCK: u8 = Opcode::SetClock.byte();

/// Stands in for the clock of the computer: it shows the same time until it is told to show another.
///
/// Clones share the time, so one clone can move the clock that another is read from.
#[derive(Clone)]
pub struct FixedClock(Rc<Cell<ClockTime>>);

impl FixedClock {
    /// A clock showing a time such as `2026-01-02 03:04:05`.
    pub fn showing(time: &str) -> Self {
        let parts: Vec<u16> = time.split(['-', ' ', ':']).map(|part| part.parse().expect("a time such as 2026-01-02 03:04:05")).collect();
        let [year, month, day, hour, minute, second] = parts[..] else {
            panic!("a time such as 2026-01-02 03:04:05");
        };
        let byte = |part: u16| u8::try_from(part).expect("a small number");
        let time = ClockTime::new(year, byte(month), byte(day), byte(hour), byte(minute), byte(second)).expect("a time the device can hold");
        Self(Rc::new(Cell::new(time)))
    }

    /// The time the clock shows.
    pub fn time(&self) -> ClockTime {
        self.0.get()
    }

    /// Show another time from now on.
    pub fn show(&self, time: ClockTime) {
        self.0.set(time);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Option<ClockTime> {
        Some(self.time())
    }
}

/// Stands in for a Bluetooth scan: it sees the devices it was given, and records how it was asked.
pub struct FakeScanner {
    devices: Vec<DeviceInfo>,
    pub calls: Cell<usize>,
    pub scan_duration: Cell<Option<Duration>>,
}

impl FakeScanner {
    pub fn seeing(devices: Vec<DeviceInfo>) -> Self {
        Self {
            devices,
            calls: Cell::new(0),
            scan_duration: Cell::new(None),
        }
    }
}

impl DeviceScanner for FakeScanner {
    async fn discover(&self, scan_duration: Duration) -> Result<Vec<DeviceInfo>, PortError> {
        self.calls.set(self.calls.get() + 1);
        self.scan_duration.set(Some(scan_duration));
        Ok(self.devices.clone())
    }
}

/// A device that answers the queries of the tool, recording every frame written to it.
///
/// Clones share the recorded frames, so what one clone was sent can be read from another.
#[derive(Clone)]
pub struct FakeDevice {
    pub address: String,
    stored: Vec<u8>,
    keeps_name: bool,
    /// The whole reply to each query that is not answered from the stored name.
    replies: HashMap<Opcode, Vec<u8>>,
    /// The dose rate of each record held, in hundredths of a microsievert per hour.
    records: Vec<u16>,
    /// The dose rate of a record that the device writes when it is first asked for its records.
    writes_during_download: Option<u16>,
    sends_wrong_records: bool,
    /// How many of the next downloads lose their last packet.
    lossy_downloads: usize,
    ignores_start_and_stop: bool,
    /// The time the device's clock was last set to, which stamps the start of the next run.
    clock: Option<[u8; 6]>,
    /// The packets of the records download that were sent and not yet received.
    packets: VecDeque<Vec<u8>>,
    writes: Rc<RefCell<Vec<Vec<u8>>>>,
    disconnected: Rc<Cell<bool>>,
}

impl FakeDevice {
    /// A measuring device that stores the name it is sent, and holds one record.
    pub fn storing(address: &str) -> Self {
        let device = Self {
            address: address.to_owned(),
            stored: b"WXYZ".to_vec(),
            keeps_name: false,
            replies: HashMap::from([
                (Opcode::QueryReading, reply(Opcode::QueryReading, &[1, 0, 0, 12, 18, 5, 0, 5, 0, 198, 0])),
                (Opcode::QueryLoggingState, reply(Opcode::QueryLoggingState, &[10, 26, 1, 2, 3, 4, 5, 10, 0, 0, 0])),
            ]),
            records: Vec::new(),
            writes_during_download: None,
            sends_wrong_records: false,
            lossy_downloads: 0,
            ignores_start_and_stop: false,
            clock: None,
            packets: VecDeque::new(),
            writes: Rc::default(),
            disconnected: Rc::default(),
        };
        device.holding(vec![5])
    }

    /// A stopped device that holds no record.
    pub fn stopped(address: &str) -> Self {
        let mut device = Self::storing(address).holding(Vec::new());
        device.set_measuring(false);
        device
    }

    /// Stay measuring or stopped whatever the device is told.
    pub fn ignoring_start_and_stop(mut self) -> Self {
        self.ignores_start_and_stop = true;
        self
    }

    /// Put the status in the reading, unless its reply was made too short.
    fn set_measuring(&mut self, measuring: bool) {
        if let Some(status) = self.replies.get_mut(&Opcode::QueryReading).and_then(|reply| reply.get_mut(2)) {
            *status = u8::from(measuring);
        }
    }

    /// Start a run of this many days as the device does: it erases its records, chooses the record interval,
    /// and stamps the start with its clock, which shows no real date until it has been set.
    fn start_run(&mut self, days: u8) {
        self.set_measuring(true);
        self.records.clear();
        if let Some(reply) = self.replies.get_mut(&Opcode::QueryLoggingState).filter(|reply| reply.len() >= 13) {
            reply[2] = days;
            reply[3..9].copy_from_slice(&self.clock.unwrap_or_default());
            reply[9] = if days > 10 { 60 } else { 10 };
        }
        self.report_records();
    }

    /// Stop as the device does: it keeps its records, and the logging duration it was sent.
    fn stop_run(&mut self, days: u8) {
        self.set_measuring(false);
        if let Some(duration) = self.replies.get_mut(&Opcode::QueryLoggingState).and_then(|reply| reply.get_mut(2)) {
            *duration = days;
        }
    }

    /// A device that keeps the given name whatever it is sent.
    pub fn keeping(address: &str, name: &str) -> Self {
        Self {
            stored: name.as_bytes().to_vec(),
            keeps_name: true,
            ..Self::storing(address)
        }
    }

    /// Answer a query with exactly these bytes, well-formed or not.
    pub fn answering(mut self, query: Opcode, reply: Vec<u8>) -> Self {
        self.replies.insert(query, reply);
        self
    }

    /// Report a reading, given as the tool shows it:
    /// a status, a measuring time such as `0 d 00:12:18`, an average dose rate such as `0.05`,
    /// a one-minute dose rate such as `0.50 (1 pulse)` and a battery sensor value.
    pub fn reporting_reading(self, reading: &[String]) -> Self {
        let [status, time, average, one_minute, battery] = reading else {
            panic!("a reading has a status, a measuring time, two dose rates and a battery sensor value");
        };
        let (days, clock) = time.split_once(" d ").expect("a measuring time such as 0 d 00:12:18");
        let mut content = vec![u8::from(status == "measuring"), days.parse().expect("a number of days")];
        content.extend(clock.split(':').map(|part| part.parse::<u8>().expect("a two-digit number")));
        content.extend(hundredths(average).to_le_bytes());
        let pulses = one_minute
            .split_once('(')
            .and_then(|(_, pulses)| pulses.split(' ').next())
            .and_then(|pulses| pulses.parse::<u16>().ok());
        // The device sends the pulses of the last minute as five units each.
        content.extend((pulses.expect("a one-minute dose rate such as 0.50 (1 pulse)") * 5).to_le_bytes());
        content.extend(battery.parse::<u16>().expect("a battery value").to_le_bytes());
        self.answering(Opcode::QueryReading, reply(Opcode::QueryReading, &content))
    }

    /// Report a logging state, given as the tool shows it:
    /// a logging duration such as `10 days` or `off`, a start such as `2026-01-02 03:04:05` or `unknown`,
    /// a record interval such as `10 minutes` and a number of records.
    /// The device then holds that many records of 0.00, until it is given others to hold.
    pub fn reporting_logging_state(self, state: &[String]) -> Self {
        let [duration, started, interval, records] = state else {
            panic!("a logging state has a logging duration, a start, a record interval and a number of records");
        };
        let leading_number = |text: &str| text.split(' ').next().and_then(|number| number.parse::<u8>().ok());
        let mut content = vec![leading_number(duration).unwrap_or(0)];
        if started == "unknown" {
            content.extend([0; 6]);
        } else {
            let mut parts = started.split(['-', ' ', ':']).map(|part| part.parse::<u16>().expect("a start such as 2026-01-02 03:04:05"));
            content.push(u8::try_from(parts.next().expect("a year") - 2000).expect("a year of this century"));
            content.extend(parts.map(|part| u8::try_from(part).expect("a small number")));
        }
        content.push(leading_number(interval).expect("a record interval in minutes"));
        content.extend([0; 3]);
        let records = records.parse::<usize>().expect("a number of records");
        self.answering(Opcode::QueryLoggingState, reply(Opcode::QueryLoggingState, &content))
            .holding(vec![0; records])
    }

    /// Hold these records, each given as a dose rate such as `0.05`, oldest first.
    /// The logging state reports their number and their checksum.
    pub fn holding_records(self, dose_rates: &[String]) -> Self {
        self.holding(dose_rates.iter().map(|dose_rate| hundredths(dose_rate)).collect())
    }

    /// Write one more record, given as a dose rate, the first time the records are asked for.
    pub fn writing_a_record_during_the_download(mut self, dose_rate: &str) -> Self {
        self.writes_during_download = Some(hundredths(dose_rate));
        self
    }

    /// Send records that never match the checksum of the logging state.
    pub fn sending_wrong_records(mut self) -> Self {
        self.sends_wrong_records = true;
        self
    }

    /// Lose the last packet of this many downloads, starting with the next one.
    pub fn losing_the_last_packet_of(mut self, downloads: usize) -> Self {
        self.lossy_downloads = downloads;
        self
    }

    fn holding(mut self, records: Vec<u16>) -> Self {
        self.records = records;
        self.report_records();
        self
    }

    /// Put the number of records held and their checksum in the logging state, unless its reply was made too short.
    fn report_records(&mut self) {
        let number = u16::try_from(self.records.len()).expect("a number of records that fits in two bytes");
        let checksum = self.record_bytes().iter().fold(0, |sum: u8, byte| sum.wrapping_add(*byte));
        if let Some([.., low, high, sum]) = self.replies.get_mut(&Opcode::QueryLoggingState).and_then(|reply| reply.get_mut(..13)) {
            [*low, *high] = number.to_le_bytes();
            *sum = checksum;
        }
    }

    fn record_bytes(&self) -> Vec<u8> {
        self.records.iter().flat_map(|record| record.to_le_bytes()).collect()
    }

    /// Every frame written to the device so far, in order.
    /// Whether the tool disconnected from the device.
    pub fn was_disconnected(&self) -> bool {
        self.disconnected.get()
    }

    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.writes.borrow().clone()
    }
}

impl DeviceTransport for FakeDevice {
    async fn write(&mut self, message: &[u8]) -> Result<(), PortError> {
        self.writes.borrow_mut().push(message.to_vec());
        if message.first() == Some(&SET_NAME) && !self.keeps_name {
            self.stored = message[3..3 + usize::from(message[2])].to_vec();
        }
        if let [SET_CLOCK, 6, time @ ..] = message {
            self.clock = time.try_into().ok();
        }
        if let [START_STOP, 2, run, days] = message
            && !self.ignores_start_and_stop
        {
            if *run == 1 { self.start_run(*days) } else { self.stop_run(*days) }
        }
        if message == [QUERY_RECORDS] {
            if let Some(record) = self.writes_during_download.take() {
                self.records.push(record);
                self.report_records();
            }
            // The device sends whole packets of 20 bytes, padding the last one with zeros.
            let mut bytes = self.record_bytes();
            bytes.resize(bytes.len().next_multiple_of(REPLY_LENGTH), 0);
            if self.sends_wrong_records {
                bytes[0] = bytes[0].wrapping_add(1);
            }
            self.packets.extend(bytes.chunks(REPLY_LENGTH).map(<[u8]>::to_vec));
            if self.lossy_downloads > 0 {
                self.lossy_downloads -= 1;
                self.packets.pop_back();
            }
        }
        Ok(())
    }

    async fn receive(&mut self, query: Opcode) -> Result<Vec<u8>, PortError> {
        if let Some(reply) = self.replies.get(&query) {
            return Ok(reply.clone());
        }
        assert_eq!(query, Opcode::QueryName, "the fake device does not answer {query:?}");
        let mut content = vec![u8::try_from(self.stored.len()).expect("a short name")];
        content.extend_from_slice(&self.stored);
        Ok(reply(Opcode::QueryName, &content))
    }

    async fn receive_records_packet(&mut self, more_expected: bool) -> Result<Option<Vec<u8>>, PortError> {
        match self.packets.pop_front() {
            None if more_expected => Err("Timed out waiting for the records.".into()),
            packet => Ok(packet),
        }
    }

    async fn disconnect(&mut self) -> Result<(), PortError> {
        self.disconnected.set(true);
        Ok(())
    }
}

/// A dose rate such as `0.05`, in hundredths of a microsievert per hour.
fn hundredths(dose_rate: &str) -> u16 {
    dose_rate.replace('.', "").parse().expect("a dose rate with two decimal places")
}

/// A reply as the device sends it: the opcode, its usual length byte, the content, and padding.
fn reply(opcode: Opcode, content: &[u8]) -> Vec<u8> {
    let mut reply = vec![opcode.byte(), 0x0b];
    reply.extend_from_slice(content);
    reply.resize(REPLY_LENGTH, 0);
    reply
}

/// Stands in for the Bluetooth adapter: only the address of its device connects.
pub struct FakeConnector {
    pub device: Option<FakeDevice>,
    /// The clock of the computer and the time it shows once the connection is made, when connecting takes time.
    pub connected_at: Option<(FixedClock, ClockTime)>,
}

impl DeviceConnector for FakeConnector {
    type Transport = FakeDevice;

    async fn connect(&self, address: &DeviceAddress) -> Result<FakeDevice, PortError> {
        if let Some((clock, time)) = &self.connected_at {
            clock.show(*time);
        }
        match &self.device {
            Some(device) if device.address == address.to_string() => Ok(device.clone()),
            _ => Err(format!("Device {address} was not found.").into()),
        }
    }
}
