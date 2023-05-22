use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

const BINARY: &str = "proj";
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn file_doesnt_exist() -> TestResult {
    let mut cmd = Command::cargo_bin(BINARY)?;

    cmd.arg("file_which_does_not_exist.fastq");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}

#[test]
fn small_file_output() -> TestResult {
    let mut cmd = Command::cargo_bin(BINARY)?;
    let intended_out = std::fs::read("tests/samples/small.fastq.out")?;
    cmd.arg("tests/samples/small.fastq");
    cmd.assert().success().stdout(predicate::eq(intended_out));

    Ok(())
}
