use bsg_001::domain::logging::{ClockTime, Download, LoggingDuration, LoggingState};
use bsg_001::domain::protocol::ProtocolError;

/// A logging state of a ten-day run started at this clock time, recording every `interval` minutes.
fn state(started: [u8; 6], interval: u8) -> LoggingState {
    LoggingState::decode(&[&[10][..], &started, &[interval, 3, 0, 0x42]].concat()).unwrap()
}

#[test]
fn holds_the_years_from_2000_to_2255() {
    assert!(ClockTime::new(1999, 12, 31, 23, 59, 59).is_none());
    assert_eq!(ClockTime::new(2000, 1, 1, 0, 0, 0).unwrap().bytes(), [0, 1, 1, 0, 0, 0]);
    assert_eq!(ClockTime::new(2255, 12, 31, 23, 59, 59).unwrap().bytes(), [255, 12, 31, 23, 59, 59]);
    assert!(ClockTime::new(2256, 1, 1, 0, 0, 0).is_none());
}

#[test]
fn rejects_a_time_that_is_not_real() {
    for [month, day, hour, minute, second] in [
        [0, 1, 0, 0, 0],
        [13, 1, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 32, 0, 0, 0],
        [1, 1, 24, 0, 0],
        [1, 1, 0, 60, 0],
        [1, 1, 0, 0, 60],
    ] {
        assert!(ClockTime::new(2024, month, day, hour, minute, second).is_none());
    }
}

#[test]
fn rejects_a_day_its_month_does_not_have() {
    assert!(ClockTime::new(2024, 2, 29, 0, 0, 0).is_some());
    assert!(ClockTime::new(2023, 2, 29, 0, 0, 0).is_none());
    assert!(ClockTime::new(2024, 4, 31, 0, 0, 0).is_none());
    assert!(ClockTime::new(2100, 2, 29, 0, 0, 0).is_none());
    assert!(ClockTime::new(2000, 2, 29, 0, 0, 0).is_some());
}

#[test]
fn a_run_records_for_1_to_31_days() {
    assert!(LoggingDuration::of_run(0).is_none());
    assert_eq!(LoggingDuration::of_run(1).unwrap().days(), 1);
    assert_eq!(LoggingDuration::of_run(31).unwrap().days(), 31);
    assert!(LoggingDuration::of_run(32).is_none());
    assert_eq!(LoggingDuration::OFF.days(), 0);
}

#[test]
fn rejects_a_logging_duration_no_run_has() {
    assert_eq!(LoggingState::decode(&[32, 24, 1, 1, 0, 0, 0, 10, 3, 0, 0]), Err(ProtocolError::UnknownLoggingDuration));
    assert_eq!(LoggingState::decode(&[0, 24, 1, 1, 0, 0, 0, 10, 3, 0, 0]).unwrap().duration, LoggingDuration::OFF);
}

#[test]
fn reads_the_time_from_seconds_since_1970() {
    assert!(ClockTime::from_unix_seconds(0).is_none());
    assert_eq!(ClockTime::from_unix_seconds(946_684_800).unwrap().iso(), "2000-01-01T00:00:00");
    assert_eq!(ClockTime::from_unix_seconds(951_782_400 + 3661).unwrap().to_string(), "2000-02-29 01:01:01");
    assert_eq!(ClockTime::from_unix_seconds(1_735_689_599).unwrap().iso(), "2024-12-31T23:59:59");
}

#[test]
fn a_record_is_written_at_the_end_of_each_interval_across_days_months_and_years() {
    for (started, interval, written) in [
        ([24, 12, 31, 23, 50, 7], 10, "2025-01-01 00:00:07"),
        ([24, 2, 28, 23, 0, 0], 60, "2024-02-29 00:00:00"),
        ([23, 2, 28, 23, 0, 0], 60, "2023-03-01 00:00:00"),
        ([24, 4, 30, 23, 30, 0], 60, "2024-05-01 00:30:00"),
    ] {
        assert_eq!(state(started, interval).recorded_at(1).to_string(), written);
    }
}

#[test]
fn without_a_clock_a_record_is_dated_from_the_start_of_the_run() {
    assert_eq!(state([0; 6], 60).recorded_at(25).to_string(), "1 d 01:00:00");
}

#[test]
fn no_interval_stands_for_ten_minutes() {
    assert_eq!(state([24, 1, 1, 0, 0, 0], 0).interval_minutes, 10);
}

#[test]
fn rejects_a_truncated_logging_state() {
    assert_eq!(LoggingState::decode(&[10, 24, 1, 1, 0, 0, 0, 10, 3, 0]), Err(ProtocolError::TruncatedLoggingState));
}

#[test]
fn a_download_joins_records_split_across_packets_and_ignores_the_padding() {
    let mut download = Download::expecting(2);
    download.receive(&[0x10]);
    assert_eq!(download.received(), 0);
    download.receive(&[0x00, 0x20, 0x01, 0xff, 0xff]);
    assert!(download.is_complete());
    assert_eq!(download.checksum(), 0x31);
    let records: Vec<String> = download.records().unwrap().iter().map(ToString::to_string).collect();
    assert_eq!(records, ["0.16", "2.88"]);
}

#[test]
fn a_download_missing_records_is_truncated() {
    let mut download = Download::expecting(2);
    download.receive(&[1, 0, 2]);
    assert_eq!(download.records(), Err(ProtocolError::TruncatedRecords));
}
