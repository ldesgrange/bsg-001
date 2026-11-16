//! Bluetooth transport for commands and measurement notifications.

use std::pin::Pin;
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Peripheral as _, ValueNotification, WriteType};
use btleplug::platform::{Adapter, Peripheral};
use futures_util::{Stream, StreamExt};

use crate::adapters::bluetooth::{first_adapter, scan};
use crate::application::{DeviceConnector, DeviceTransport};

const SERVICE: &str = "00001523-1212-efde-1523-785feabcd123";
const COMMAND: &str = "00001524-1212-efde-1523-785feabcd123";
const MEASUREMENT: &str = "00001525-1212-efde-1523-785feabcd123";
const LOG: &str = "00001526-1212-efde-1523-785feabcd123";
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
/// How long the log characteristic stays silent once a download is over.
const DOWNLOAD_SILENCE: Duration = Duration::from_millis(300);
const CONNECT_SCAN: Duration = Duration::from_secs(15);

pub struct BluetoothTransport {
    peripheral: Peripheral,
    command: Characteristic,
    measurement: Characteristic,
    log: Characteristic,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
}

impl DeviceTransport for BluetoothTransport {
    async fn write(&mut self, message: &[u8]) -> Result<(), String> {
        self.peripheral
            .write(&self.command, message, WriteType::WithResponse)
            .await
            .map_err(|error| error.to_string())
    }

    async fn receive(&mut self, opcode: u8) -> Result<Vec<u8>, String> {
        receive_matching(&mut self.notifications, &self.measurement.uuid.to_string(), opcode, REPLY_TIMEOUT).await
    }

    async fn receive_records_packet(&mut self, more_expected: bool) -> Result<Option<Vec<u8>>, String> {
        let wait = if more_expected { REPLY_TIMEOUT } else { DOWNLOAD_SILENCE };
        match receive_packet(&mut self.notifications, &self.log.uuid.to_string(), wait).await {
            Err(error) if more_expected => Err(error),
            packet => Ok(packet.ok()),
        }
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        self.peripheral.disconnect().await.map_err(|error| error.to_string())
    }
}

async fn receive_matching(notifications: &mut (impl Stream<Item = ValueNotification> + Unpin), characteristic: &str, opcode: u8, timeout: Duration) -> Result<Vec<u8>, String> {
    tokio::time::timeout(timeout, async {
        while let Some(notification) = notifications.next().await {
            if notification.uuid.to_string() == characteristic && notification.value.first() == Some(&opcode) {
                return Ok(notification.value);
            }
        }
        Err("Notification stream closed.".to_owned())
    })
    .await
    .map_err(|_| "Timed out waiting for a device reply.".to_owned())?
}

/// Wait for the next packet of one characteristic, skipping those of the others.
async fn receive_packet(notifications: &mut (impl Stream<Item = ValueNotification> + Unpin), characteristic: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    tokio::time::timeout(timeout, async {
        while let Some(notification) = notifications.next().await {
            if notification.uuid.to_string() == characteristic {
                return Ok(notification.value);
            }
        }
        Err("Notification stream closed.".to_owned())
    })
    .await
    .map_err(|_| "Timed out waiting for the records.".to_owned())?
}

fn has_address(device: &Peripheral, address: &str) -> bool {
    device.address().to_string().eq_ignore_ascii_case(address)
}

async fn find_known(adapter: &Adapter, address: &str) -> Result<Option<Peripheral>, String> {
    let devices = adapter.peripherals().await.map_err(|error| error.to_string())?;
    Ok(devices.into_iter().find(|device| has_address(device, address)))
}

async fn connect_peripheral(address: &str) -> Result<Peripheral, String> {
    let adapter = first_adapter().await.map_err(|error| error.to_string())?;
    if let Some(device) = find_known(&adapter, address).await?
        && device.connect().await.is_ok()
    {
        return Ok(device);
    }
    // The scan stops as soon as the device is seen; the allowance covers a device that advertises slowly.
    let device = scan(&adapter, CONNECT_SCAN, |device| has_address(device, address))
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|sighting| sighting.peripheral)
        .find(|device| has_address(device, address))
        .ok_or_else(|| format!("Device {address} was not found."))?;
    device.connect().await.map_err(|error| error.to_string())?;
    Ok(device)
}

/// Find the characteristics of the device and subscribe to its replies and its records.
async fn open(peripheral: &Peripheral) -> Result<BluetoothTransport, String> {
    peripheral.discover_services().await.map_err(|error| error.to_string())?;
    let service = peripheral
        .services()
        .into_iter()
        .find(|service| service.uuid.to_string() == SERVICE)
        .ok_or("Device command service not found.")?;
    let characteristic = |uuid: &str, missing: &str| {
        service
            .characteristics
            .iter()
            .find(|characteristic| characteristic.uuid.to_string() == uuid)
            .cloned()
            .ok_or_else(|| missing.to_owned())
    };
    let command = characteristic(COMMAND, "Command characteristic not found.")?;
    let measurement = characteristic(MEASUREMENT, "Measurement characteristic not found.")?;
    let log = characteristic(LOG, "Log characteristic not found.")?;
    let notifications = peripheral.notifications().await.map_err(|error| error.to_string())?;
    peripheral.subscribe(&measurement).await.map_err(|error| error.to_string())?;
    peripheral.subscribe(&log).await.map_err(|error| error.to_string())?;
    Ok(BluetoothTransport {
        peripheral: peripheral.clone(),
        command,
        measurement,
        log,
        notifications,
    })
}

/// Connects to devices over the host's Bluetooth adapter.
pub struct BluetoothConnector;

impl DeviceConnector for BluetoothConnector {
    type Transport = BluetoothTransport;

    async fn connect(&self, address: &str) -> Result<BluetoothTransport, String> {
        let peripheral = connect_peripheral(address).await?;
        match open(&peripheral).await {
            Ok(transport) => Ok(transport),
            Err(error) => {
                let _ = peripheral.disconnect().await;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[tokio::test]
    async fn matches_on_characteristic_and_opcode_even_without_a_service_uuid() {
        let uuid = "00001525-1212-efde-1523-785feabcd123".parse().unwrap();
        let other_uuid = "00001526-1212-efde-1523-785feabcd123".parse().unwrap();
        let service_uuid = SERVICE.parse().unwrap();
        let mut notifications = stream::iter(vec![
            ValueNotification {
                service_uuid,
                uuid,
                value: vec![0x21],
            },
            ValueNotification {
                service_uuid,
                uuid: other_uuid,
                value: vec![0x31],
            },
            ValueNotification {
                service_uuid: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
                uuid,
                value: b"\x31\x00\x04ABCD".to_vec(),
            },
        ]);
        assert_eq!(
            receive_matching(&mut notifications, MEASUREMENT, 0x31, Duration::from_millis(100)).await.unwrap(),
            b"\x31\x00\x04ABCD"
        );
    }

    #[tokio::test]
    async fn takes_the_next_packet_of_the_log_characteristic() {
        let service_uuid = SERVICE.parse().unwrap();
        let packet = |uuid: &str, value: &[u8]| ValueNotification {
            service_uuid,
            uuid: uuid.parse().unwrap(),
            value: value.to_vec(),
        };
        let mut notifications = stream::iter(vec![packet(MEASUREMENT, &[0xa0]), packet(LOG, &[1, 2]), packet(LOG, &[3, 4])]);
        assert_eq!(receive_packet(&mut notifications, LOG, Duration::from_millis(100)).await.unwrap(), [1, 2]);
        assert_eq!(receive_packet(&mut notifications, LOG, Duration::from_millis(100)).await.unwrap(), [3, 4]);
        assert!(receive_packet(&mut notifications, LOG, Duration::from_millis(100)).await.is_err());
    }

    #[tokio::test]
    async fn times_out_when_no_packet_arrives() {
        let mut notifications = stream::pending();
        assert!(receive_packet(&mut notifications, LOG, Duration::from_millis(10)).await.is_err());
    }

    #[tokio::test]
    async fn times_out_even_when_no_notifications_arrive() {
        let mut notifications = stream::pending();
        assert!(receive_matching(&mut notifications, MEASUREMENT, 0x31, Duration::from_millis(10)).await.is_err());
    }
}
