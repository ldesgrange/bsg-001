# Discovering a BSG-001

Turn on the device and run `bsg-001 discover` to find it nearby.
Discovery only listens for advertisements: it does not pair, bond, authenticate, encrypt, or connect.

## Finding a device

A BSG-001 is recognised by an advertised name beginning with `BLE_Gamma`.
Other Bluetooth devices in range are left out.

These Bluetooth devices are advertising nearby:

| address | name | RSSI |
| --- | --- | --- |
| AA:BB:CC:DD:EE:01 | BLE_Gamma:1234 | -55 |

Here is the terminal session:

```console
$ bsg-001 discover
AA:BB:CC:DD:EE:01	BLE_Gamma:1234	-55 dBm
```

Each line shows the address, the advertised name, and the signal strength in dBm, separated by tab characters.
The address is what the other commands take to select the device.
Each device has its own address, which never changes, so it can be kept and used again.

## When no device is found

No Bluetooth device is advertising nearby.
Here is the terminal session:

```console
$ bsg-001 discover
No BSG-001 devices found.
```

Nothing is printed on standard output.

A BSG-001 that is connected to another client stops advertising, so it cannot be found until that client disconnects.

## Scanning for longer

The scan lasts 5 seconds unless `--timeout` gives another duration in seconds.

These Bluetooth devices are advertising nearby:

| address | name | RSSI |
| --- | --- | --- |
| AA:BB:CC:DD:EE:01 | BLE_Gamma:1234 | -80 |

Here is the terminal session:

```console
$ bsg-001 discover --timeout 30
AA:BB:CC:DD:EE:01	BLE_Gamma:1234	-80 dBm
```

## Edge cases

<details>
<summary>Which names are recognised</summary>

Only the `BLE_Gamma` prefix counts: the label after it is set by the user and can be anything.
A device without an advertised name is left out.

These Bluetooth devices are advertising nearby:

| address | name | RSSI |
| --- | --- | --- |
| AA:BB:CC:DD:EE:01 | BLE_Gamma | -50 |
| AA:BB:CC:DD:EE:02 | BLE_Gamma:1234 | -51 |
| AA:BB:CC:DD:EE:03 | BLE_GammaX | -52 |
| AA:BB:CC:DD:EE:04 | Headphones | -53 |
| AA:BB:CC:DD:EE:07 | DfuTarg | -56 |
| AA:BB:CC:DD:EE:05 | BSG-001 | -54 |
| AA:BB:CC:DD:EE:06 | | -55 |

Here is the terminal session:

```console
$ bsg-001 discover
AA:BB:CC:DD:EE:01	BLE_Gamma	-50 dBm
AA:BB:CC:DD:EE:02	BLE_Gamma:1234	-51 dBm
AA:BB:CC:DD:EE:03	BLE_GammaX	-52 dBm
```

In firmware-update mode a BSG-001 advertises as `DfuTarg`.
That is Nordic's generic bootloader name, so it cannot tell a BSG-001 from other Nordic devices and is not listed.

</details>

<details>
<summary>Unreliable advertisements</summary>

Advertised names and signal strengths come from any device in range, so they may be invalid.
An invalid name or signal strength is treated as missing, and the scan carries on.
A blank name cannot identify a BSG-001, and an out-of-range or absent signal strength is shown as unavailable.

In these tables, control characters in a name are written as Unicode control pictures, such as ␊ for a newline, ␡ for delete, and ␠ for a space.

These Bluetooth devices are advertising nearby:

| address | name | RSSI |
| --- | --- | --- |
| AA:BB:CC:DD:EE:01 | BLE_Gamma:1234 | 127 |
| AA:BB:CC:DD:EE:02 | BLE_Gamma:5678 | |
| AA:BB:CC:DD:EE:03 | ␠ | -60 |

Here is the terminal session:

```console
$ bsg-001 discover
AA:BB:CC:DD:EE:01	BLE_Gamma:1234	RSSI unavailable
AA:BB:CC:DD:EE:02	BLE_Gamma:5678	RSSI unavailable
```

</details>

<details>
<summary>Names containing control characters</summary>

The label after `BLE_Gamma:` can be malformed and contain control characters.
They are shown as backslash escapes, so each device stays on one line.
A backslash becomes `\\`, a newline `\n`, a carriage return `\r`, and a tab `\t`.
Any other control character, such as delete, becomes `\x` followed by two lowercase hexadecimal digits.

These Bluetooth devices are advertising nearby:

| address | name | RSSI |
| --- | --- | --- |
| AA:BB:CC:DD:EE:01 | BLE_Gamma:␊A␡B | -55 |

Here is the terminal session:

```console
$ bsg-001 discover
AA:BB:CC:DD:EE:01	BLE_Gamma:\nA\x7fB	-55 dBm
```

</details>

<details>
<summary>Scan durations</summary>

The scan duration is a finite number of seconds, at least 0.1.

The tool checks each timeout and gives its verdict before scanning.

| timeout | verdict |
| --- | --- |
| 0.1 | valid |
| 5 | valid |
| 0.05 | invalid |
| 0 | invalid |
| -1 | invalid |
| inf | invalid |
| nan | invalid |

</details>

## Good to know

- The normal advertised name is `BLE_Gamma:` followed by a four-character label that [the user can set](set-name.md).
- The Manufacturer Name String characteristic (`0x2A29`) may identify the manufacturer as `FTLAB`.
  It cannot be used for discovery because reading it requires a connection.
