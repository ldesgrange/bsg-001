//! Bluetooth transport for commands and measurement notifications.

use std::pin::Pin;
use std::time::Duration;

use btleplug::api::{Central, Characteristic, Peripheral as _, ValueNotification, WriteType};
use btleplug::platform::{Adapter, Peripheral};
use futures_util::{Stream, StreamExt};
use uuid::Uuid;

use crate::adapters::bluetooth::{first_adapter, scan};
use crate::application::{DeviceConnector, DeviceTransport, PortError};
use crate::domain::DeviceAddress;
use crate::domain::protocol::Opcode;

const SERVICE: Uuid = Uuid::from_u128(0x0000_1523_1212_efde_1523_785f_eabc_d123);
const COMMAND: Uuid = Uuid::from_u128(0x0000_1524_1212_efde_1523_785f_eabc_d123);
const MEASUREMENT: Uuid = Uuid::from_u128(0x0000_1525_1212_efde_1523_785f_eabc_d123);
const LOG: Uuid = Uuid::from_u128(0x0000_1526_1212_efde_1523_785f_eabc_d123);
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
    async fn write(&mut self, message: &[u8]) -> Result<(), PortError> {
        Ok(self.peripheral.write(&self.command, message, WriteType::WithResponse).await?)
    }

    async fn receive(&mut self, query: Opcode) -> Result<Vec<u8>, PortError> {
        receive_matching(&mut self.notifications, self.measurement.uuid, query.byte(), REPLY_TIMEOUT).await
    }

    async fn receive_records_packet(&mut self, more_expected: bool) -> Result<Option<Vec<u8>>, PortError> {
        let wait = if more_expected { REPLY_TIMEOUT } else { DOWNLOAD_SILENCE };
        match receive_packet(&mut self.notifications, self.log.uuid, wait).await {
            Err(error) if more_expected => Err(error),
            packet => Ok(packet.ok()),
        }
    }

    async fn disconnect(&mut self) -> Result<(), PortError> {
        Ok(self.peripheral.disconnect().await?)
    }
}

/// Wait for the reply to the query with this opcode, skipping every other notification.
async fn receive_matching(notifications: &mut (impl Stream<Item = ValueNotification> + Unpin), characteristic: Uuid, opcode: u8, timeout: Duration) -> Result<Vec<u8>, PortError> {
    let reply = |notification: &ValueNotification| notification.uuid == characteristic && notification.value.first() == Some(&opcode);
    receive_where(notifications, timeout, "Timed out waiting for a device reply.", reply).await
}

/// Wait for the next packet of one characteristic, skipping those of the others.
async fn receive_packet(notifications: &mut (impl Stream<Item = ValueNotification> + Unpin), characteristic: Uuid, timeout: Duration) -> Result<Vec<u8>, PortError> {
    let packet = |notification: &ValueNotification| notification.uuid == characteristic;
    receive_where(notifications, timeout, "Timed out waiting for the records.", packet).await
}

/// Wait for the next notification that is wanted, and give its value.
async fn receive_where(
    notifications: &mut (impl Stream<Item = ValueNotification> + Unpin),
    timeout: Duration,
    timed_out: &str,
    wanted: impl Fn(&ValueNotification) -> bool,
) -> Result<Vec<u8>, PortError> {
    tokio::time::timeout(timeout, async {
        while let Some(notification) = notifications.next().await {
            if wanted(&notification) {
                return Ok(notification.value);
            }
        }
        Err("Notification stream closed.".into())
    })
    .await
    .map_err(|_| PortError::from(timed_out))?
}

fn has_address(device: &Peripheral, address: &DeviceAddress) -> bool {
    DeviceAddress::new(device.address().to_string()).is_ok_and(|own| own == *address)
}

async fn find_known(adapter: &Adapter, address: &DeviceAddress) -> Result<Option<Peripheral>, PortError> {
    let devices = adapter.peripherals().await?;
    Ok(devices.into_iter().find(|device| has_address(device, address)))
}

async fn connect_peripheral(address: &DeviceAddress) -> Result<Peripheral, PortError> {
    let adapter = first_adapter().await?;
    if let Some(device) = find_known(&adapter, address).await?
        && device.connect().await.is_ok()
    {
        return Ok(device);
    }
    // The scan stops as soon as the device is seen; the allowance covers a device that advertises slowly.
    let device = scan(&adapter, CONNECT_SCAN, |device| has_address(device, address))
        .await?
        .into_iter()
        .map(|sighting| sighting.peripheral)
        .find(|device| has_address(device, address))
        .ok_or_else(|| format!("Device {address} was not found."))?;
    device.connect().await?;
    Ok(device)
}

/// Find the characteristics of the device and subscribe to its replies and its records.
async fn open(peripheral: &Peripheral) -> Result<BluetoothTransport, PortError> {
    peripheral.discover_services().await?;
    let service = peripheral
        .services()
        .into_iter()
        .find(|service| service.uuid == SERVICE)
        .ok_or("Device command service not found.")?;
    let characteristic = |uuid: Uuid, missing: &str| {
        service
            .characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == uuid)
            .cloned()
            .ok_or_else(|| PortError::from(missing))
    };
    let command = characteristic(COMMAND, "Command characteristic not found.")?;
    let measurement = characteristic(MEASUREMENT, "Measurement characteristic not found.")?;
    let log = characteristic(LOG, "Log characteristic not found.")?;
    let notifications = peripheral.notifications().await?;
    peripheral.subscribe(&measurement).await?;
    peripheral.subscribe(&log).await?;
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

    async fn connect(&self, address: &DeviceAddress) -> Result<BluetoothTransport, PortError> {
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
        let (service_uuid, uuid, other_uuid) = (SERVICE, MEASUREMENT, LOG);
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
                service_uuid: Uuid::nil(),
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
        let packet = |uuid: Uuid, value: &[u8]| ValueNotification {
            service_uuid: SERVICE,
            uuid,
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
