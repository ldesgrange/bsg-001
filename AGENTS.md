# BSG-001 agent guide

## Markdown

- Put each complete sentence on its own line in Markdown files.
- Do not hard-wrap a sentence before it is complete.
- Keep sentences concise enough to remain one readable line.

## How we work

We use Varar for behaviour-driven development.
When you implement a feature or fix a bug, you must:

1. Write or update a `*.md` oath before touching production code. 
   The oath describes the behaviour in plain English with concrete examples.
2. Write or update the matching step definitions in `tests/oaths.rs`.
3. Run the Varar suite and read the failures.
4. Implement the production code in small steps, running the suite after each step.
5. When the suite is green and you believe the feature is complete, stop and summarise what you changed.
   Do not refactor unrelated code.

When several examples share a world state, write it once under its own heading and link to it — but keep the chain shallow: one level, two at the outside.
If setup repeats because it is one idea spelled out in several sentences, write the missing step instead of sharing the sentences.

The oath is the contract.
If you cannot satisfy the oath, surface the disagreement instead of changing the oath to match your implementation.

## Workflow

- The tool is written in Rust, and the crate is at the repository root.
  `mise.toml` provisions the Rust toolchain.
- Use `mise run <task>` for repository operations.
  Run `mise install` after cloning; it also registers the pre-commit and pre-push hooks.
  Run the applicable quality tasks before completing a change.
- Run focused tests with `cargo test --locked --test cli` (replace the test name as needed).
- Keep `Cargo.lock` synchronized through `cargo`, and build with `--locked`.
- Treat `specs/` as the source of truth for both documentation and executable behavior.
  Run `mise run specs-e2e` after changing an oath or its bindings.
  Keep inferred and hypothetical protocol findings as prose until they can be verified by an executable oath.
  Varar oaths run against fake Bluetooth devices by default.
  `mise run specs-e2e` runs every oath against the fakes and needs no hardware, so it can run in CI.
  `mise run specs-e2e-real` is an explicit hardware opt-in: it runs the oaths a real BSG-001 nearby can reproduce, using the real Bluetooth adapters.
  Bindings must skip the examples a real device cannot reproduce, and an example that changes a real device must put its state back afterwards.

## Supply chain

- Do not override the three-day release cooldown of mise or Cargo, or install/update dependencies published within the previous three days unless the user explicitly authorizes it.
- Keep `mise.lock` and `Cargo.lock` committed and current when their respective tools or dependencies change.
  If adding a third-party pre-commit plugin, pin it to an immutable revision.
- Commit `varar.lock.json` with intentional oath changes.
  Never accept drift with `VARAR_UPDATE=1` without reviewing the behavioral change.

## Architecture and testing

- Preserve the enforced dependency direction: `adapters -> application -> domain`.
  Domain and application code must not use btleplug, clap, or other infrastructure.
- Keep Bluetooth access behind application ports.
  Unit tests must use fakes/mocks and must not require Bluetooth hardware; mark hardware tests `#[ignore]` so they run only on explicit opt-in.
- Identify BSG-001 devices from an advertised name beginning with `BLE_Gamma`.
  Each device has a random address that never changes, so the address selects a device once it has been discovered.
- Develop behavior test-first.
  Use value objects for domain concepts that need validation or behavior, but do not introduce wrappers without a purpose.

## Protocol research

- Record UUIDs, packet formats, and protocol behavior in `specs/protocol.md` with the stated evidence level.
  Decompiled Android code is research input only; do not copy it into this repository.
