# Downloading the records

A BSG-001 that is logging keeps a record of the dose rate at regular intervals.
`bsg-001 records` downloads the records the device holds and prints them, oldest first, leaving out the first one.

## Downloading from a device

First [discover the device](discovery.md), then pass its address to `bsg-001 records` with `--address`.
The address can also be set once for all commands, as explained in [choosing the device](command.md#choosing-the-device).

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 4 |

The device holds these records:

| dose rate |
| --- |
| 4.35 |
| 0.05 |
| 0.00 |
| 0.10 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
2026-01-02 03:24:05 UTC	0.05 µSv/h
2026-01-02 03:34:05 UTC	0.00 µSv/h
2026-01-02 03:44:05 UTC	0.10 µSv/h
```

## What each line means

Each line is one record: the time it was recorded, a tab, and its dose rate in microsieverts per hour.

- **The time** is on the device's clock, which the tool sets to UTC when it [starts a run](run.md).
  The device does not store a time with each record, so the tool works it out from the start of the run and the record interval.
  The first record is written one interval after the run started, and each following record one interval later.
- **The dose rate** is the average over the interval that ends at that time.

The first record of a run is left out, which is why the first line comes two intervals after the start of the run.
The sensor keeps counting while the device is stopped, and the first record holds everything it counted since the previous run ended.
In the example above it would have shown 4.35 µSv/h for what was ordinary background.

The start of the run and the record interval are those shown by [`bsg-001 logging`](logging.md).

## Saving the records as CSV

With `--format csv` the records are printed as comma-separated values, ready for a spreadsheet or a script.
The first line names the columns.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 4 |

The device holds these records:

| dose rate |
| --- |
| 4.35 |
| 0.05 |
| 0.00 |
| 0.10 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF --format csv
recorded_at,minutes_from_start,dose_rate_usv_h
2026-01-02T03:24:05Z,20,0.05
2026-01-02T03:34:05Z,30,0.00
2026-01-02T03:44:05Z,40,0.10
```

- **recorded_at** is the time the record was written, as a date, a `T`, a time, and a `Z` that stands for UTC.
- **minutes_from_start** is how many minutes after the start of the run the record was written.
- **dose_rate_usv_h** is the dose rate in microsieverts per hour.

The format can also be chosen with the environment variable `BSG_001_FORMAT`, and `--format` wins over the variable.
Without either, the format is `text`, the one shown at the top of this page.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 2 |

The device holds these records:

| dose rate |
| --- |
| 4.35 |
| 0.05 |

Here is the terminal session:

```console
$ BSG_001_FORMAT=csv bsg-001 records --address AA:BB:CC:DD:EE:FF
recorded_at,minutes_from_start,dose_rate_usv_h
2026-01-02T03:24:05Z,20,0.05
```

## Following a long download

A full device holds 1440 records, which take a while to arrive.
The tool shows a progress bar while it downloads, and removes it once it is done.
The bar is drawn on standard error, and only when standard error is a terminal.
It is never part of the result: with `bsg-001 records > records.csv` the bar still shows in the terminal, and the file holds only the records.

The device sends its records ten at a time, so the progress moves in steps of ten records.

While it downloads the number of records shown, the tool reports the progress shown.

| records | progress |
| --- | --- |
| 25 | 10 of 25, 20 of 25, 25 of 25 |
| 10 | 10 of 10 |
| 3 | 3 of 3 |
| 0 | none |

## Good to know

- At background a ten-minute record holds no pulse or a few, so single records say little and only averages over many of them are meaningful.
- The device holds at most 1440 records, which is ten days at ten minutes or sixty days at sixty minutes.
- The records are kept in memory that is erased when the device loses power, so download them before switching it off.
- Light reaching the sensor and knocks on the case are recorded as radiation, as explained in [reading the current measurement](reading.md).

## Edge cases

<details>
<summary>A device that holds no record</summary>

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 0 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
No records.
```

Nothing is printed on standard output.

</details>

<details>
<summary>A device that holds only its first record</summary>

The first record is left out, so there is nothing to show until the second one is written.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 1 |

The device holds these records:

| dose rate |
| --- |
| 4.35 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
No records.
```

Nothing is printed on standard output.

Of the records the device holds, the tool keeps those shown.

| device holds | tool keeps |
| --- | --- |
| 4.35, 0.05, 0.00, 0.10 | 0.05, 0.00, 0.10 |
| 0.00, 0.05 | 0.05 |
| 4.35 | none |
| none | none |

</details>

<details>
<summary>CSV for a device that holds no record, or whose clock was not set</summary>

Without records, the CSV has only its first line.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 0 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF --format csv
recorded_at,minutes_from_start,dose_rate_usv_h
```

Without a start time, the `recorded_at` column is left empty.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 1 day | unknown | 10 minutes | 3 |

The device holds these records:

| dose rate |
| --- |
| 2.10 |
| 0.05 |
| 0.15 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF --format csv
recorded_at,minutes_from_start,dose_rate_usv_h
,20,0.05
,30,0.15
```

</details>

<details>
<summary>A format that does not exist</summary>

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ BSG_001_FORMAT=xml bsg-001 records --address AA:BB:CC:DD:EE:FF
Error: The format must be text or csv, not xml.
```

</details>

<details>
<summary>A device whose clock was not set</summary>

Without a start time the tool cannot tell when a record was written.
It shows how long after the start of the run instead, in days, hours, minutes and seconds.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 1 day | unknown | 10 minutes | 3 |

The device holds these records:

| dose rate |
| --- |
| 2.10 |
| 0.05 |
| 0.15 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
0 d 00:20:00	0.05 µSv/h
0 d 00:30:00	0.15 µSv/h
```

</details>

<details>
<summary>Hourly records, and runs that cross midnight or the end of a month</summary>

A run longer than ten days records every sixty minutes.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 31 days | 2026-12-31 20:30:00 | 60 minutes | 4 |

The device holds these records:

| dose rate |
| --- |
| 0.95 |
| 0.03 |
| 0.07 |
| 0.00 |

Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
2026-12-31 22:30:00 UTC	0.03 µSv/h
2026-12-31 23:30:00 UTC	0.07 µSv/h
2027-01-01 00:30:00 UTC	0.00 µSv/h
```

Given when the run started and its record interval, each record was recorded at the time shown.
Record 1 is the first record, the one the tool leaves out.

| run started | record interval | record | recorded at |
| --- | --- | --- | --- |
| 2026-01-02 03:04:05 | 10 minutes | 1 | 2026-01-02 03:14:05 |
| 2026-01-02 03:04:05 | 10 minutes | 1440 | 2026-01-12 03:04:05 |
| 2026-02-28 23:55:00 | 10 minutes | 1 | 2026-03-01 00:05:00 |
| 2028-02-28 23:55:00 | 10 minutes | 1 | 2028-02-29 00:05:00 |
| 2026-12-01 00:00:00 | 60 minutes | 1440 | 2027-01-30 00:00:00 |
| unknown | 10 minutes | 1 | 0 d 00:10:00 |
| unknown | 60 minutes | 1440 | 60 d 00:00:00 |

</details>

<details>
<summary>A record written during the download</summary>

The tool first asks how many records the device holds, then downloads them.
A device that writes a record between the two steps sends one record more than it announced.
The tool keeps the records that were announced and leaves the new one for the next download.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 3 |

The device holds these records:

| dose rate |
| --- |
| 4.35 |
| 0.05 |
| 0.10 |

While the tool downloads the records for the first time, the device writes a record of "0.20".
Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
2026-01-02 03:24:05 UTC	0.05 µSv/h
2026-01-02 03:34:05 UTC	0.10 µSv/h
```

</details>

<details>
<summary>Records that do not match the checksum</summary>

The device tells a checksum of its records, and the tool checks what it downloaded against it.
When they differ, for example because part of the download was lost, the tool asks again and downloads again.
It tries three times, then reports the failure instead of printing records it cannot trust.

A device at address "AA:BB:CC:DD:EE:FF" reports this logging state:

| logging duration | run started | record interval | records |
| --- | --- | --- | --- |
| 10 days | 2026-01-02 03:04:05 | 10 minutes | 2 |

The device holds these records:

| dose rate |
| --- |
| 0.05 |
| 0.10 |

The device sends records that never match its checksum.
Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
Error: The records do not match the checksum of the device.
```

</details>

<details>
<summary>A reply the tool cannot read</summary>

A device at address "AA:BB:CC:DD:EE:FF" answers the logging state query with a reply that is too short.
Here is the terminal session:

```console
$ bsg-001 records --address AA:BB:CC:DD:EE:FF
Error: Invalid logging state reply.
```

</details>
