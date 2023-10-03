use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

const BINARY: &str = "proj";
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn file_doesnt_exist() -> TestResult {
    let mut cmd = Command::cargo_bin(BINARY)?;

    cmd.arg("consensus").arg("file_which_does_not_exist.fastq");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}

#[test]
fn small_file_output() -> TestResult {
    let output = Command::cargo_bin(BINARY)?
        .arg("tests/samples/small.fastq")
        .output()
        .expect("Failed to run process");
    let stdout = String::from_utf8(output.stdout).unwrap();

    // We don't care about the order of the output, so sort first
    let mut stdout: Vec<&str> = stdout.lines().collect();
    stdout.sort_unstable();
    let stdout_sorted = stdout.join("\n");

    let intended_out = String::from_utf8(std::fs::read("tests/samples/small.fastq.out")?)
        .unwrap()
        .trim_end()
        .to_string();

    assert!(
        stdout_sorted == intended_out,
        "Tests do not match. Got output:\n{}",
        stdout_sorted
    );

    Ok(())
}
