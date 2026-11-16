//! Btleplug implementation of the Bluetooth discovery port.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_util::StreamExt;

use crate::application::{DeviceScanner, ScannerError};
use crate::domain::{DeviceAddress, DeviceInfo, DeviceName, DeviceValidationError, Rssi};

#[derive(Default)]
pub struct BtleplugDeviceScanner;

impl BtleplugDeviceScanner {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug)]
struct NoBluetoothAdapter;

impl Display for NoBluetoothAdapter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("No Bluetooth adapter is available.")
    }
}

impl Error for NoBluetoothAdapter {}

/// Build discovery details, treating an invalid name or RSSI as missing.
pub fn device_info_from_advertisement(address: impl Into<String>, local_name: Option<String>, rssi: Option<i16>) -> Result<DeviceInfo, DeviceValidationError> {
    Ok(DeviceInfo::new(
        DeviceAddress::new(address)?,
        local_name.and_then(|name| DeviceName::new(name).ok()),
        rssi.and_then(|value| Rssi::new(value).ok()),
    ))
}

pub(crate) async fn first_adapter() -> Result<Adapter, ScannerError> {
    let manager = Manager::new().await?;
    Ok(manager.adapters().await?.into_iter().next().ok_or(NoBluetoothAdapter)?)
}

/// A peripheral that advertised during a scan, with the properties it had while the scan was running.
pub(crate) struct Sighting {
    pub peripheral: Peripheral,
    pub properties: Option<PeripheralProperties>,
}

/// Scan for up to `duration` and return only the peripherals that advertised during the scan.
/// The scan ends early once `stop_when` accepts a peripheral.
pub(crate) async fn scan(adapter: &Adapter, duration: Duration, mut stop_when: impl FnMut(&Peripheral) -> bool) -> Result<Vec<Sighting>, ScannerError> {
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;

    let mut seen = HashSet::new();
    let mut peripherals = Vec::new();
    let collected = async {
        let deadline = tokio::time::sleep(duration);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                () = &mut deadline => break,
                event = events.next() => match event {
                    Some(CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id)) => {
                        if !seen.insert(id.clone()) {
                            continue;
                        }
                        // A device can leave range between the event and this lookup; it is then left out.
                        let Ok(peripheral) = adapter.peripheral(&id).await else {
                            continue;
                        };
                        let stop = stop_when(&peripheral);
                        peripherals.push(peripheral);
                        if stop {
                            break;
                        }
                    }
                    Some(_) => {}
                    None => break,
                },
            }
        }
        Ok::<_, ScannerError>(())
    }
    .await;
    // Properties are read before the scan stops, because the signal strength is forgotten once it has.
    let mut sightings = Vec::new();
    for peripheral in peripherals {
        // A device that left range since it advertised no longer has properties.
        let properties = peripheral.properties().await.ok().flatten();
        sightings.push(Sighting { peripheral, properties });
    }
    let stop_result = adapter.stop_scan().await;

    collected?;
    stop_result?;
    Ok(sightings)
}

impl DeviceScanner for BtleplugDeviceScanner {
    async fn discover(&self, scan_duration: Duration) -> Result<Vec<DeviceInfo>, ScannerError> {
        let adapter = first_adapter().await?;
        let mut devices = Vec::new();
        for sighting in scan(&adapter, scan_duration, |_| false).await? {
            // A device without properties left range after it advertised; it is left out.
            if let Some(properties) = sighting.properties {
                devices.push(device_info_from_advertisement(properties.address.to_string(), properties.local_name, properties.rssi)?);
            }
        }
        Ok(devices)
    }
}
