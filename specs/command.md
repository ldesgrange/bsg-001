# The command-line tool

The tool is a command named `bsg-001`.
The build produces an executable with that name.

## Getting help

Asking for help does not contact a device.
Running `bsg-001 --help` lists the commands "discover, logging, read, records, set-name, start, stop".

Each command has its own page: [discover](discovery.md), [read](reading.md), [logging](logging.md), [records](records.md), [start and stop](run.md) and [set-name](set-name.md).

## Choosing the device

Every command except `discover` talks to one device, chosen by its address.
[Discovering a BSG-001](discovery.md) shows the address of each device nearby.
A device keeps the same address for good, so it only has to be looked up once.

The address is passed with `--address`.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ bsg-001 read --address AA:BB:CC:DD:EE:FF
Status: measuring
Measuring time: 0 d 00:12:18
Average dose rate: 0.05 µSv/h
One-minute dose rate: 0.50 µSv/h (1 pulse)
Battery: 40%
```

To avoid repeating it, the address can be put in the environment variable `BSG_001_ADDRESS`.
The sessions below set the variable on the command line; in practice it is set once, for example in the shell profile.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ BSG_001_ADDRESS=AA:BB:CC:DD:EE:FF bsg-001 read
Status: measuring
Measuring time: 0 d 00:12:18
Average dose rate: 0.05 µSv/h
One-minute dose rate: 0.50 µSv/h (1 pulse)
Battery: 40%
```

When both are given, `--address` wins over the variable.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ BSG_001_ADDRESS=AA:BB:CC:DD:EE:00 bsg-001 read --address AA:BB:CC:DD:EE:FF
Status: measuring
Measuring time: 0 d 00:12:18
Average dose rate: 0.05 µSv/h
One-minute dose rate: 0.50 µSv/h (1 pulse)
Battery: 40%
```

Without either, the command fails before looking for a device.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ bsg-001 read
Error: No device address was given: pass --address or set BSG_001_ADDRESS.
```

A variable that is set but empty counts as not set.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ BSG_001_ADDRESS= bsg-001 read
Error: No device address was given: pass --address or set BSG_001_ADDRESS.
```

An address made only of zeros is not the address of any device, whatever its separators.
The command refuses it before looking for a device.

A device at address "AA:BB:CC:DD:EE:FF" stores the name it is sent.
Here is the terminal session:

```console
$ bsg-001 set-name --address 00:00:00:00:00:00 ABCD
Error: 00:00:00:00:00:00 is not the address of a device.
```

Nothing is written to the device.

macOS does not tell the tool the address of a device, so [`bsg-001 discover`](discovery.md) lists no device there.
On macOS the error adds that the tool cannot tell devices apart yet.

## What the tool prints

Results are printed on standard output.
Warnings and errors are printed on standard error.
So is a message that says there is no result, such as `No records.`, so that a file or a program receiving the output gets results and nothing else.
A command that fails prints a line beginning with `Error:`, prints no result, and exits with a non-zero status.

The program receiving the output may stop reading before the end, as `head` does in `bsg-001 records | head -1`.
The tool then ends quietly.

A result written to a program that has stopped reading is dropped without an error.

The terminal sessions in this manual show the command after a `$` prompt, followed by both streams as a terminal would display them.
