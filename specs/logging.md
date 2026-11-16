# Checking the logging state

A BSG-001 can record its measurements at regular intervals for a number of days.
`bsg-001 logging` shows how the device is set up to record, and how many records it holds.

## Checking a device

First [discover the device](discovery.md), then pass its address to `bsg-001 logging` with `--address`.
The address can also be set once for all commands, as explained in [choosing the device](command.md#choosing-the-device).

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 144 |

Here is the terminal session:

```console
$ bsg-001 logging --address AA:BB:CC:DD:EE:FF
Logging duration: 10 days
Run started: 2026-01-02 03:04:05 UTC
Record interval: 10 minutes
Records: 144
```

## What each line means

- **Logging duration** is how many days a run records for, from 1 to 31 days, or `off` when the device measures without recording.
  The device stops by itself when a run reaches this duration.
- **Run started** is the date and time on the device's clock when the run began.
  The tool sets that clock to UTC when it [starts a run](run.md), and labels the time accordingly.
  A run started with the vendor application is stamped with the phone's local time instead, whatever the label says.
- **Record interval** is the time between two records.
  The device chooses it from the logging duration: 10 minutes up to 10 days, and 60 minutes beyond.
- **Records** is how many records the device holds.

The device reports a start time and a record interval even when logging is off, so only the logging duration tells whether it records.

The device keeps all of this in memory that is erased when it loses power.
After a power cycle the records are gone, the logging duration is back to 1 day, and the clock is no longer set until [a run is started](run.md).

## Edge cases

<details>
<summary>A device whose clock was not set</summary>

The device forgets the time whenever it loses power, and the tool sets it again each time it [starts a run](run.md).
Until then the start time is not a real date, and the tool shows `unknown`.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 1 day | unknown | 10 minutes | 0 |

Here is the terminal session:

```console
$ bsg-001 logging --address AA:BB:CC:DD:EE:FF
Logging duration: 1 day
Run started: unknown
Record interval: 10 minutes
Records: 0
```

</details>

<details>
<summary>A device that measures without recording</summary>

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| off | 2026-01-02 03:04:05 | 10 minutes | 0 |

Here is the terminal session:

```console
$ bsg-001 logging --address AA:BB:CC:DD:EE:FF
Logging duration: off
Run started: 2026-01-02 03:04:05 UTC
Record interval: 10 minutes
Records: 0
```

</details>

<details>
<summary>A long run</summary>

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 31 days | 2026-12-31 23:59:59 | 60 minutes | 744 |

Here is the terminal session:

```console
$ bsg-001 logging --address AA:BB:CC:DD:EE:FF
Logging duration: 31 days
Run started: 2026-12-31 23:59:59 UTC
Record interval: 60 minutes
Records: 744
```

</details>

<details>
<summary>A reply the tool cannot read</summary>

The tool reports an invalid reply instead of printing values it cannot trust.

A device at address "AA:BB:CC:DD:EE:FF" answers the logging state query with a reply that is too short.
Here is the terminal session:

```console
$ bsg-001 logging --address AA:BB:CC:DD:EE:FF
Error: Invalid logging state reply.
```

</details>
