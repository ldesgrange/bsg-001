//! Connecting to a device for one operation.

use crate::application::OperationError;
use crate::application::ports::{DeviceConnector, DeviceTransport};
use crate::domain::DeviceAddress;

/// Connect to the device, run an operation over the connection, and disconnect whatever the outcome.
///
/// When both the operation and the disconnection fail, the operation's own error is the one given.
pub async fn on_device<C: DeviceConnector, T>(
    connector: &C,
    address: &DeviceAddress,
    operation: impl AsyncFnOnce(&mut C::Transport) -> Result<T, OperationError>,
) -> Result<T, OperationError> {
    let mut device = connector.connect(address).await?;
    let result = operation(&mut device).await;
    let disconnected = device.disconnect().await;
    let value = result?;
    disconnected?;
    Ok(value)
}
