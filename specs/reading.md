# Reading the current measurement

A BSG-001 can be asked at any time for what it is measuring right now.
The tool connects to the device, asks once, prints the answer, and disconnects.

## Reading a device

First [discover the device](discovery.md), then pass its address to `bsg-001 read` with `--address`.
The address can also be set once for all commands, as explained in [choosing the device](command.md#choosing-the-device).

A device at address "AA:BB:CC:DD:EE:FF" reports this reading:

| status | measuring time | average dose rate | one-minute dose rate | battery sensor |
| --- | --- | --- | --- | --- |
| measuring | 0 d 00:12:18 | 0.05 | 0.50 (1 pulse) | 198 |

Here is the terminal session:

```console
$ bsg-001 read --address AA:BB:CC:DD:EE:FF
Status: measuring
Measuring time: 0 d 00:12:18
Average dose rate: 0.05 µSv/h
One-minute dose rate: 0.50 µSv/h (1 pulse)
Battery: 40%
```

## What each line means

- **Status** is `measuring` or `stopped`.
- **Measuring time** is the device's own count of how long it has been measuring, in days, hours, minutes and seconds.
  Starting a run restarts its hours, minutes and seconds, but not its days, which the device clears only when it loses power.
  So the days can include whole days of earlier runs: one day into a run that follows a ten-day run, the device reports 11 days.
  [`bsg-001 logging`](logging.md) shows when the run really started.
- **Average dose rate** is the average over the last ten completed minutes, and is the value the vendor application displays.
  It covers ten minutes whatever the record interval of the run.
- **One-minute dose rate** is the rate over the last completed minute, followed by the number of pulses the sensor counted in that minute.
  It reacts faster than the average and fluctuates more.
- **Battery** is the charge level, as a percentage.

Dose rates are in microsieverts per hour and have two decimal places.
Quiet indoor background usually gives an average between 0.00 and 0.05 µSv/h.

The device counts pulses minute by minute, so both dose rates change only once a minute, and reading more often shows nothing new.
One pulse in a minute amounts to 0.50 µSv/h, and one pulse in ten minutes to 0.05 µSv/h.
At background a minute holds no pulse or one, so the one-minute dose rate jumps between 0.00 and 0.50 µSv/h and only the average is meaningful.
The device does not send the one-minute dose rate as such.
It sends the last minute's pulses spread over ten minutes, which is a tenth of the real rate: 0.05 µSv/h for one pulse.
The tool works the pulses back out of that value and shows the rate they amount to over one minute.

The device reports a raw battery sensor value, not a percentage.
The tool turns it into one of seven levels, the same way the vendor application does: 100%, 80%, 60%, 40%, 20%, 10% or 1%.
The levels suit rechargeable cells, and a device powered over USB without cells shows 1%.

Light reaching the sensor and knocks on the case register as radiation, so a sudden high reading is not always real.
A spike leaves the one-minute dose rate after a minute, but stays in the average dose rate for ten minutes.

## Edge cases

<details>
<summary>A device that is not measuring</summary>

A device stops by itself when the run reaches the duration it was started with.
A stopped device still answers, but it is not measuring, so the tool shows no dose rate.
The dose rates it reports are those it held when it stopped, and they do not change until it measures again.
Showing them would pass old values off as current ones.

The tool shows how long the last run lasted instead of a measuring time.
Its days are the device's count too, so they can include whole days of earlier runs.

A device at address "AA:BB:CC:DD:EE:FF" reports this reading:

| status | measuring time | average dose rate | one-minute dose rate | battery sensor |
| --- | --- | --- | --- | --- |
| stopped | 12 d 03:04:05 | 13.90 | 0.00 (0 pulses) | 211 |

Here is the terminal session:

```console
$ bsg-001 read --address AA:BB:CC:DD:EE:FF
Status: stopped
Last run: 12 d 03:04:05
Battery: 100%
```

</details>

<details>
<summary>How the battery percentage is worked out</summary>

Each level covers a range of battery sensor values, and the lowest level is 1%, never 0%.

For each battery sensor value, the tool shows the percentage in this table.

| battery sensor | percentage |
| --- | --- |
| 300 | 100% |
| 211 | 100% |
| 210 | 80% |
| 205 | 80% |
| 204 | 60% |
| 200 | 60% |
| 199 | 40% |
| 196 | 40% |
| 195 | 20% |
| 194 | 20% |
| 193 | 10% |
| 191 | 10% |
| 190 | 1% |
| 99 | 1% |
| 0 | 1% |

</details>

<details>
<summary>A reply the tool cannot read</summary>

The tool reports an invalid reply instead of printing values it cannot trust.

A device at address "AA:BB:CC:DD:EE:FF" answers the reading query with a reply that is too short.
Here is the terminal session:

```console
$ bsg-001 read --address AA:BB:CC:DD:EE:FF
Error: Invalid reading reply.
```

</details>
