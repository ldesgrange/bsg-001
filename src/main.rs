//! Command-line entry point for BSG-001 tools.

use std::io::IsTerminal;
use std::process::ExitCode;

use bsg_001::adapters::bluetooth::BtleplugDeviceScanner;
use bsg_001::adapters::cli::{Cli, NoProgress, TerminalProgress, run, write_result};
use bsg_001::adapters::device_bluetooth::BluetoothConnector;
use bsg_001::adapters::system_clock::SystemClock;
use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse().with_environment(|variable| std::env::var(variable).ok());
    let (scanner, mut stderr) = (BtleplugDeviceScanner::new(), std::io::stderr());
    // A progress bar is for a person watching a terminal, not for a file or another program.
    let outcome = if stderr.is_terminal() {
        run(cli, &scanner, &BluetoothConnector, &SystemClock, &mut stderr, &mut TerminalProgress::default()).await
    } else {
        run(cli, &scanner, &BluetoothConnector, &SystemClock, &mut stderr, &mut NoProgress).await
    };
    if let Err(error) = write_result(&mut std::io::stdout().lock(), &outcome.stdout) {
        eprintln!("Error: {error}");
        return ExitCode::FAILURE;
    }
    if outcome.success { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
