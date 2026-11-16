# Setting the device name

You can give a BSG-001 a four-character name made from ASCII letters and digits.
The name is the label that follows `BLE_Gamma:` in the name the device advertises.

## Renaming a device

First [discover the device](discovery.md), then pass its address with `--address`, and the new name, to `bsg-001 set-name`.
The address can also be set once for all commands, as explained in [choosing the device](command.md#choosing-the-device).

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ bsg-001 set-name --address AA:BB:CC:DD:EE:FF ABCD
Warning: renaming during logging may reset the measuring-time counter.
Confirmed name: ABCD. Power-cycle the device to update its advertised name.
```

The tool sends the new name, asks the device which name it has stored, and reports success only when the two match.
The name persists across power cycles, but the advertised name changes only after the device is power-cycled.
Avoid renaming during a logging run, because it may reset the measuring-time counter.

## Choosing a name

A name has exactly four characters, each an ASCII letter or digit.
Letters and digits can be mixed, and upper and lower case are kept as typed.
An invalid name is refused before anything is sent to the device.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ bsg-001 set-name --address AA:BB:CC:DD:EE:FF bad!
Error: The name must contain exactly four ASCII letters or digits.
```

Nothing is written to the device.

## When the device does not confirm

The rename fails when the device reports a stored name other than the one requested.

A device at address "AA:BB:CC:DD:EE:FF" keeps the name "WXYZ" whatever it is sent.
Here is the terminal session:

```console
$ bsg-001 set-name --address AA:BB:CC:DD:EE:FF ABCD
Warning: renaming during logging may reset the measuring-time counter.
Error: The device did not confirm the requested name.
```

## Edge cases

<details>
<summary>More valid and invalid names</summary>

Any length other than four is refused, and so are punctuation, spaces, and accented letters.

The tool checks each name and gives its verdict before writing anything to the device.

| name | verdict |
| --- | --- |
| ABCD | valid |
| A1b2 | valid |
| 0000 | valid |
| ABC | invalid |
| ABCDE | invalid |
| bad! | invalid |
| A CD | invalid |
| ÀBCD | invalid |

</details>

<details>
<summary>A stored name that is not a valid name</summary>

The device can report a stored name that the tool would never write, for example after its memory was corrupted.
The tool then reports an invalid reply instead of a name it cannot trust.
In this table, control characters are written as Unicode control pictures, such as ␊ for a newline.

When the device keeps the stored name shown, renaming it to ABCD fails with the error shown.

| stored | error |
| --- | --- |
| WXYZ | The device did not confirm the requested name. |
| ABCDE | Invalid name query reply. |
| AB | Invalid name query reply. |
| AB␊D | Invalid name query reply. |

</details>

## Good to know

- The vendor application only supports four ASCII letters or digits, and other name lengths have not been tested.
- Writing the standard Bluetooth Device Name characteristic does not persist the name.
- The frames exchanged with the device are described in [the protocol page](protocol.md).
