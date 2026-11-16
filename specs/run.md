# Starting and stopping a run

A BSG-001 measures only during a run.
`bsg-001 start` begins a run and `bsg-001 stop` ends it.

## Starting a run

First [discover the device](discovery.md), then pass its address to `bsg-001 start` with `--address`.
The address can also be set once for all commands, as explained in [choosing the device](command.md#choosing-the-device).

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
It is "2026-01-02 03:04:05" in UTC.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF
Started a run of 10 days at 2026-01-02 03:04:05 UTC, recording every 10 minutes.
```

A run lasts 10 days unless `--days` says otherwise, from 1 to 31 days.
The device stops by itself when the run reaches that duration.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
It is "2026-01-02 03:04:05" in UTC.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF --days 31
Started a run of 31 days at 2026-01-02 03:04:05 UTC, recording every 60 minutes.
```

The duration decides how often the device records: every 10 minutes up to 10 days, and every 60 minutes beyond.
Either way the device has room for the whole run.
The duration cannot be changed during a run: stop the device and start it again.

After starting, the tool asks the device for its status and its [logging state](logging.md), and reports success only when the device is measuring for the duration asked.

## The start of the run

The device stamps the start of a run with its own clock, and [the records](records.md) are timed from that stamp.
The device forgets the time whenever it loses power, so the tool sets the device's clock just before starting.
The time in the message is the stamp the device reports once it has started.
The tool reads the time of the computer only once the device is reached, so the time it takes to connect does not make the stamp early.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
It is "2026-01-02 03:04:05" in UTC.
The connection to the device is made at "2026-01-02 03:04:20" in UTC.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF
Started a run of 10 days at 2026-01-02 03:04:20 UTC, recording every 10 minutes.
```

There is no separate command to set the clock, because the clock serves no other purpose.

The tool sets the clock to UTC, and every time it shows is in UTC.
The device's clock has no time zone, and a local time would make the records jump by an hour whenever the clocks change during a run.

> **Warning:** the vendor application sets the device's clock to the local time of the phone.
> The tool shows the times of a run started with the vendor application as they are, with a UTC label they do not deserve.

## Stopping a run

A device at address "AA:BB:CC:DD:EE:FF" is measuring.
Here is the terminal session:

```console
$ bsg-001 stop --address AA:BB:CC:DD:EE:FF
Stopped measuring.
```

The device keeps the records of the run after it is stopped, until the next run starts or the device loses power.

## Starting erases the previous records

A device holds the records of one run only.
Starting a run erases those of the previous one, so [download them](records.md) first.
The tool warns when there are records to lose.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
The device holds these records:

| dose rate |
| --- |
| 4.35 |
| 0.05 |
| 0.10 |

It is "2026-01-02 03:04:05" in UTC.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF
Warning: starting a run erases the 3 records the device holds.
Started a run of 10 days at 2026-01-02 03:04:05 UTC, recording every 10 minutes.
```

## Measuring without recording

With `--no-logging` the device measures until it is stopped and keeps no record.
[Reading the current measurement](reading.md) works as usual.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
It is "2026-01-02 03:04:05" in UTC.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF --no-logging
Started measuring without recording.
```

## Good to know

- The first record of a run holds everything the sensor counted while the device was stopped, which is why [the records](records.md) leave it out.
- The measuring time shown by [reading the current measurement](reading.md) restarts with each run, except its days, which the device clears only when it loses power.

## Edge cases

<details>
<summary>A device that is already in the state asked for</summary>

Starting a device that is measuring would erase the records of the run in progress, so the tool refuses.

A device at address "AA:BB:CC:DD:EE:FF" is measuring.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF
Error: The device is already measuring.
```

Nothing but queries is written to the device.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
Here is the terminal session:

```console
$ bsg-001 stop --address AA:BB:CC:DD:EE:FF
Error: The device is already stopped.
```

Nothing but queries is written to the device.

</details>

<details>
<summary>Durations a run can have</summary>

A duration outside 1 to 31 days is refused before anything is sent to the device.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF --days 32
Error: A run lasts from 1 to 31 days.
```

Nothing is written to the device.

For each number of days, a run records at the record interval shown.

| days | record interval |
| --- | --- |
| 1 | 10 minutes |
| 10 | 10 minutes |
| 11 | 60 minutes |
| 31 | 60 minutes |
| 0 | refused |
| 32 | refused |

</details>

<details>
<summary>A device that does not do as it is told</summary>

The tool checks the device after telling it to start or stop, and reports a failure when its state did not change.

A device at address "AA:BB:CC:DD:EE:FF" is stopped.
The device does not start or stop when told to.
Here is the terminal session:

```console
$ bsg-001 start --address AA:BB:CC:DD:EE:FF
Error: The device did not start.
```

A device at address "AA:BB:CC:DD:EE:FF" is measuring.
The device does not start or stop when told to.
Here is the terminal session:

```console
$ bsg-001 stop --address AA:BB:CC:DD:EE:FF
Error: The device did not stop.
```

</details>
