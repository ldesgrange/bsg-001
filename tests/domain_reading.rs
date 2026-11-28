use bsg_001::domain::protocol::ProtocolError;
use bsg_001::domain::reading::{BatteryLevel, DoseRate, MeasuringStatus, Pulses, Reading};

/// The content of a reading reply: status, measuring time, then the average, the last-minute units and the battery sensor.
fn content(status: u8, units: u16, battery: u16) -> Vec<u8> {
    [&[status, 1, 2, 3, 4][..], &1234_u16.to_le_bytes(), &units.to_le_bytes(), &battery.to_le_bytes()].concat()
}

#[test]
fn decodes_a_reading() {
    let reading = Reading::decode(&content(1, 10, 205)).unwrap();
    assert_eq!(reading.status, MeasuringStatus::Measuring);
    assert!(reading.is_measuring());
    assert_eq!(reading.measuring_time.to_string(), "1 d 02:03:04");
    assert_eq!(reading.average.to_string(), "12.34");
    assert_eq!(reading.one_minute().to_string(), "1.00");
    assert_eq!(reading.battery_level().to_string(), "80%");
}

#[test]
fn rounds_the_last_minute_units_to_the_nearest_pulse() {
    for (units, pulses) in [(0, 0), (2, 0), (3, 1), (5, 1), (7, 1), (8, 2), (9, 2), (10, 2)] {
        assert_eq!(Reading::decode(&content(1, units, 205)).unwrap().last_minute, Pulses::from_units(units), "{units} units");
        assert_eq!(Pulses::from_units(units).count(), pulses, "{units} units");
    }
}

#[test]
fn a_stopped_device_is_not_measuring() {
    let reading = Reading::decode(&content(0, 10, 205)).unwrap();
    assert_eq!(reading.status.to_string(), "stopped");
    assert!(!reading.is_measuring());
    assert_eq!(MeasuringStatus::Measuring.to_string(), "measuring");
}

#[test]
fn a_pulse_in_the_last_minute_amounts_to_half_a_microsievert_per_hour() {
    assert_eq!(Pulses::from_units(5).to_string(), "1 pulse");
    assert_eq!(Pulses::from_units(10).to_string(), "2 pulses");
    assert_eq!(Pulses::from_units(0).to_string(), "0 pulses");
    assert_eq!(Pulses::from_units(15).dose_rate().to_string(), "1.50");
}

#[test]
fn rejects_an_unknown_status_and_a_truncated_reading() {
    assert_eq!(Reading::decode(&content(2, 10, 205)), Err(ProtocolError::UnknownStatus));
    assert_eq!(Reading::decode(&content(1, 10, 205)[..10]), Err(ProtocolError::TruncatedReading));
    assert_eq!(Reading::decode(&[1, 0, 0]), Err(ProtocolError::TruncatedReading));
}

#[test]
fn maps_the_battery_sensor_to_the_levels_the_vendor_shows() {
    for (sensor, level) in [
        (u16::MAX, "100%"),
        (211, "100%"),
        (210, "80%"),
        (205, "80%"),
        (204, "60%"),
        (200, "60%"),
        (199, "40%"),
        (196, "40%"),
        (195, "20%"),
        (194, "20%"),
        (193, "10%"),
        (191, "10%"),
        (190, "1%"),
        (0, "1%"),
    ] {
        assert_eq!(BatteryLevel::from_sensor(sensor).to_string(), level, "sensor {sensor}");
    }
}

#[test]
fn a_dose_rate_is_shown_in_microsieverts_per_hour_with_two_decimals() {
    assert_eq!(DoseRate::from_hundredths(5).to_string(), "0.05");
    assert_eq!(DoseRate::from_hundredths(1234).to_string(), "12.34");
    assert_eq!(DoseRate::from_hundredths(1234).hundredths(), 1234);
}
