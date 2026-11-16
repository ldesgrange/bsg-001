# BSG-001

Command-line tools for the FTLab Wireless Smart Geiger BSG-001.

## Development

```bash
mise install
mise run check
```

`mise install` provisions the Rust toolchain and the other tools, downloads the crates, and registers the pre-commit and pre-push hooks.

The tool is written in Rust and executes the oaths in `specs/` with Varar.

The checks enforce domain and application boundaries by inspecting parsed source paths, not by separating the layers into compiler-enforced crates.
Macro-generated references, aliases resolved across items, and dependencies not listed in the architecture check may evade detection and require review.
Formatting uses a 180-column target, and dependency auditing runs at pre-push or with `mise run audit`.

### Dependency cooldown

Cargo's native three-day package cooldown is configured in `.cargo/config.toml`.
Cargo preserves versions already recorded in `Cargo.lock` and does not apply the cooldown to `cargo install`.

Discover nearby Bluetooth devices without connecting to them:

```bash
cargo run -- discover
```

Read what a device is measuring right now, using the address that discovery printed:

```bash
cargo run -- read AA:BB:CC:DD:EE:FF
```

Show how a device is set up to record its measurements, and how many records it holds:

```bash
cargo run -- logging AA:BB:CC:DD:EE:FF
```

## Device manual and executable specifications

Start with the [device manual](specs/index.md) for device behavior, protocol details, and tool usage.
Its terminal sessions and tables of cases are executable examples, run against fake Bluetooth devices by default.

`mise run specs-e2e` runs every oath in `specs/` against fake Bluetooth devices.
It needs no hardware.

To run the oaths against a real BSG-001 nearby, with the real Bluetooth adapters:

```bash
mise run specs-e2e-real
```

Examples that only a fake can reproduce are reported as ignored, each with the reason.
The rename examples do rename the device, and each is followed by a second rename that puts back the name the device advertised.

When an executable oath or its bindings change, refresh the lockfile explicitly:

```bash
VARAR_UPDATE=1 mise run specs-e2e
git diff -- varar.lock.json
mise run specs-e2e
```

Review the lockfile diff before committing it with the oath and binding changes.
Prose-only specification changes do not normally require a lockfile update.


BSG-001 devices advertise a name beginning with `BLE_Gamma`.
Each device has its own Bluetooth address, which never changes.

### Set a device name

Rename a device using its address and a four-character ASCII letter-or-digit name:

```bash
cargo run -- set-name --address AA:BB:CC:DD:EE:FF ABCD
```

The address can also be set once in the `BSG_001_ADDRESS` environment variable.
Bluetooth backends may scan for the supplied address if necessary to connect.
The command reads the name back to verify the write.
The advertisement continues to show the old name until the device is power-cycled.
Avoid renaming during logging because it may reset the measuring-time counter.
`mise run specs-e2e-real` checks the rename and its readback, but not the power cycle.
To verify that by hand, run the command, power-cycle the device, and scan for `BLE_Gamma:ABCD`.
