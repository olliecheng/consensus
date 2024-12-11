use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::path::Path;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";

#[test]
fn index() {
    let temp = assert_fs::NamedTempFile::new("_index.tsv").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&["index", SAMPLE_FASTQ, "--index", temp.path().to_str().unwrap()])
        .assert()
        .success();

    // lazy way of checking that these files are the same
    // EXCEPT for the header, which contains unique date and runtime information
    let cmp_cmd = format!(
        "diff <(tail -n+2 tests/correct/index.tsv) <(tail -n+2 {})",
        temp.path().to_str().unwrap()
    );

    let _ = Command::new("bash")
        .arg("-c")
        .arg(&cmp_cmd)
        .unwrap();

    temp.close().unwrap();
}

#[test]
fn summary() {
    let temp = assert_fs::NamedTempFile::new("_summary.html").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&["summary", "--index", "tests/correct/index.tsv", "--output", temp.path().to_str().unwrap()])
        .assert()
        .success();

    temp.assert(predicate::path::exists());
}

#[test]
fn consensus_1t() {
    let temp = assert_fs::NamedTempFile::new("consensus.fasta").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&[
            "call",
            "--index", "tests/correct/index.tsv",
            "--input", SAMPLE_FASTQ,
            "--output", temp.path().to_str().unwrap(),
            "--threads", "1"
        ])
        .assert()
        .success();

    let pred_file = predicate::path::eq_file(Path::new("tests/correct/consensus.fastq"));

    pred_file.eval(temp.path());
    temp.close().unwrap();
}

#[test]
fn consensus_4t() {
    let temp = assert_fs::NamedTempFile::new("consensus_4t.fasta").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let _ = command
        .args(&[
            "call",
            "--index", "tests/correct/index.tsv",
            "--input", SAMPLE_FASTQ,
            "--output", temp.path().to_str().unwrap(),
            "--threads", "4"
        ])
        .assert()
        .success();

    let pred_file = predicate::path::eq_file(Path::new("tests/correct/consensus.fastq"));

    pred_file.eval(temp.path());
    temp.close().unwrap();
}