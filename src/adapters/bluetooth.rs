//! Btleplug implementation of the Bluetooth discovery port.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_util::StreamExt;

use crate::application::{DeviceScanner, PortError};
use crate::domain::{DeviceAddress, DeviceInfo, DeviceName, DeviceValidationError, Rssi};

/// Scans with the host's first Bluetooth adapter.
pub struct BtleplugDeviceScanner;

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

pub(crate) async fn first_adapter() -> Result<Adapter, PortError> {
    let manager = Manager::new().await?;
    Ok(manager.adapters().await?.into_iter().next().ok_or(NoBluetoothAdapter)?)
}

/// What the Bluetooth stack reports about a peripheral during a scan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    /// The stack lists the peripheral, which on Linux it also does for one it remembers from an earlier scan.
    Found,
    /// An advertisement of the peripheral arrived.
    Advertisement,
}

/// Tells the peripherals that advertise during a scan from those the Bluetooth stack only remembers.
pub struct LiveSightings<Id> {
    remembered: HashSet<Id>,
    seen: HashSet<Id>,
}

impl<Id: Clone + Eq + Hash> LiveSightings<Id> {
    /// Start a scan, given the peripherals the stack knew before it.
    pub fn remembering(remembered: impl IntoIterator<Item = Id>) -> Self {
        Self {
            remembered: remembered.into_iter().collect(),
            seen: HashSet::new(),
        }
    }

    /// Take a report, and say whether it is the first to show that the peripheral is advertising.
    ///
    /// A remembered peripheral may be switched off, so only an advertisement shows that it is there.
    pub fn report(&mut self, id: &Id, report: Report) -> bool {
        let advertising = report == Report::Advertisement || !self.remembered.contains(id);
        advertising && self.seen.insert(id.clone())
    }
}

/// A peripheral that advertised during a scan, with the properties it had while the scan was running.
pub(crate) struct Sighting {
    pub peripheral: Peripheral,
    pub properties: Option<PeripheralProperties>,
}

/// Scan for up to `duration` and return only the peripherals that advertised during the scan.
/// The scan ends early once `stop_when` accepts a peripheral.
pub(crate) async fn scan(adapter: &Adapter, duration: Duration, mut stop_when: impl FnMut(&Peripheral) -> bool) -> Result<Vec<Sighting>, PortError> {
    // On Linux the events begin with every peripheral the stack remembers, whether or not it is still there.
    let remembered = adapter.peripherals().await?.iter().map(Peripheral::id).collect::<Vec<_>>();
    let mut sightings = LiveSightings::remembering(remembered);
    let mut events = adapter.events().await?;
    adapter.start_scan(ScanFilter::default()).await?;

    let mut peripherals = Vec::new();
    let collected = async {
        let deadline = tokio::time::sleep(duration);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                () = &mut deadline => break,
                event = events.next() => match event {
                    Some(event) => {
                        let (id, report) = match event {
                            CentralEvent::DeviceDiscovered(id) => (id, Report::Found),
                            CentralEvent::DeviceUpdated(id)
                            | CentralEvent::RssiUpdate { id, .. }
                            | CentralEvent::ManufacturerDataAdvertisement { id, .. }
                            | CentralEvent::ServiceDataAdvertisement { id, .. } => (id, Report::Advertisement),
                            // The advertised services are reported for remembered peripherals too, so they show nothing.
                            _ => continue,
                        };
                        if !sightings.report(&id, report) {
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
                    None => break,
                },
            }
        }
        Ok::<_, PortError>(())
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
    async fn discover(&self, scan_duration: Duration) -> Result<Vec<DeviceInfo>, PortError> {
        let adapter = first_adapter().await?;
        let mut devices = Vec::new();
        for sighting in scan(&adapter, scan_duration, |_| false).await? {
            // A device without properties left range after it advertised; it is left out.
            // A device without an address that can select it is left out too: macOS gives every device an address of zeros.
            if let Some(properties) = sighting.properties
                && let Ok(device) = device_info_from_advertisement(properties.address.to_string(), properties.local_name, properties.rssi)
            {
                devices.push(device);
            }
        }
        Ok(devices)
    }
}
