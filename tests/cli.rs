use assert_cmd::prelude::*;
use assert_fs::prelude::PathChild;
use predicates::prelude::*;
use std::fs::read_to_string;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn parse_time() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::cargo_bin("txt-timer")?
        .arg("--time-regex")
        .arg("(?P<time>[0-9: -]*\\.\\d{3})")
        .arg("--time-regex-format")
        .arg("%Y-%m-%d %H:%M:%S%.3f")
        .arg("-B")
        .arg("1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(
                "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n"
                    .as_bytes(),
            )
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert_eq!(String::from_utf8_lossy(&output.stdout),
               "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n\nMaximals:\nΔ1.0000 @1.0000\n2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n\n\nΔ0.0010 @1.0010\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n\n\nΔ0.0000 @0.0000\n2022-12-12 08:19:00.000 a\n\n\n\n");
    Ok(())
}

#[test]
fn parse_time_iso() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::cargo_bin("txt-timer")?
        .arg("--time-regex-iso")
        .arg("-B")
        .arg("1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(
                "2022-12-12T08:19:00.000Z a\n2022-12-12T08:19:01.000Z b\n2022-12-12T08:19:01.001Z c\n"
                    .as_bytes(),
            )
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert_eq!(String::from_utf8_lossy(&output.stdout),
               "2022-12-12T08:19:00.000Z a\n2022-12-12T08:19:01.000Z b\n2022-12-12T08:19:01.001Z c\n\nMaximals:\nΔ1.0000 @1.0000\n2022-12-12T08:19:00.000Z a\n2022-12-12T08:19:01.000Z b\n\n\nΔ0.0010 @1.0010\n2022-12-12T08:19:01.000Z b\n2022-12-12T08:19:01.001Z c\n\n\nΔ0.0000 @0.0000\n2022-12-12T08:19:00.000Z a\n\n\n\n");
    Ok(())
}

#[test]
fn parse_time_lines_before() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::cargo_bin("txt-timer")?
        .arg("--time-regex")
        .arg("(?P<time>[0-9: -]*\\.\\d{3})")
        .arg("--time-regex-format")
        .arg("%Y-%m-%d %H:%M:%S%.3f")
        .arg("-B")
        .arg("2")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(
                "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n"
                    .as_bytes(),
            )
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert_eq!(String::from_utf8_lossy(&output.stdout),
               "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n\nMaximals:\nΔ1.0000 @1.0000\n2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n\n\nΔ0.0010 @1.0010\n2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n\n\nΔ0.0000 @0.0000\n2022-12-12 08:19:00.000 a\n\n\n\n");
    Ok(())
}

#[test]
fn parse_time_write_to_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = assert_fs::TempDir::new()?;
    let maximals_file_path = temp_dir.child("maximals");

    let maximals_file_str = maximals_file_path
        .to_str()
        .expect("cannot convert string for tmp file");
    let mut child = Command::cargo_bin("txt-timer")?
        .arg("--time-regex")
        .arg("(?P<time>[0-9: -]*\\.\\d{3})")
        .arg("--time-regex-format")
        .arg("%Y-%m-%d %H:%M:%S%.3f")
        .arg("-B")
        .arg("1")
        .arg("--output-maximals")
        .arg(maximals_file_str)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    std::thread::spawn(move || {
        stdin
            .write_all(
                "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n"
                    .as_bytes(),
            )
            .expect("Failed to write to stdin");
    });

    let output = child.wait_with_output().expect("Failed to read stdout");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n"
    );

    assert_eq!(read_to_string(maximals_file_path).expect("output was not created"),
               "Δ1.0000 @1.0000\n2022-12-12 08:19:00.000 a\n2022-12-12 08:19:01.000 b\n\n\nΔ0.0010 @1.0010\n2022-12-12 08:19:01.000 b\n2022-12-12 08:19:01.001 c\n\n\nΔ0.0000 @0.0000\n2022-12-12 08:19:00.000 a\n\n\n");
    Ok(())
}

#[test]
fn bad_regex() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("txt-timer")?
        .arg("--time-regex")
        .arg("(?P<ime>[0-9: -]*\\.\\d{3})")
        .arg("--time-regex-format")
        .arg("%Y-%m-%d %H:%M:%S%.3f")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "regex must have a `(?P<time>exp)` capturing group",
        ));

    Ok(())
}

#[test]
fn bad_regex_combination() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("txt-timer")?
        .arg("--time-regex-iso")
        .arg("--time-regex")
        .arg("(?P<time>[0-9: -]*\\.\\d{3})")
        .arg("--time-regex-format")
        .arg("%Y-%m-%d %H:%M:%S%.3f")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "time regex and format must be either both present or absent",
        ));

    Ok(())
}
