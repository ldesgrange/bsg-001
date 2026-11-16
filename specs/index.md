# BSG-001 device manual

The FTLAB BSG-001 Wireless Smart Geiger is a battery or USB powered device that measures radiation and communicates over Bluetooth Low Energy.
Despite its name, its sensor is based on photodiodes rather than a Geiger–Müller tube.
Its readings have limitations: light and movement can produce misleading results, and the reported dose is calibrated for a particular radiation spectrum.

This manual explains the device and how to use the BSG-001 command-line tool.
Its examples are executable specifications: each terminal session and each table of cases is run against the tool.
Every page starts with the main cases, and its edge cases are folded away under headings you can expand.

1. [The command-line tool](command.md) introduces the command and what it prints.
2. [Discovering a BSG-001](discovery.md) explains how to find a nearby device.
3. [Reading the current measurement](reading.md) explains how to read what a device is measuring.
4. [Checking the logging state](logging.md) explains how to see how a device records its measurements.
5. [Downloading the records](records.md) explains how to get the measurements a device recorded.
6. [Starting and stopping a run](run.md) explains how to make a device measure and record.
7. [Setting the device name](set-name.md) explains how to rename a device.
8. [The Bluetooth protocol](protocol.md) describes the frames exchanged with the device.

The examples run against fake Bluetooth devices by default.
With an explicit opt-in, the examples that a real device can reproduce run against a BSG-001 nearby instead.
Examples that rename the device put the original name back afterwards.
