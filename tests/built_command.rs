use std::process::Command;

#[test]
fn built_bsg_001_command_displays_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_bsg-001"))
        .arg("--help")
        .output()
        .expect("the built bsg-001 executable starts");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
    assert!(stdout.contains("discover"));
    assert!(stdout.contains("set-name"));
}
