use assert_cmd::Command;
use assert_fs::prelude::*;

const SAMPLE_FASTQ: &str = "tests/data/scmixology2_sample.fastq";

#[test]
fn index() {
    let temp = assert_fs::NamedTempFile::new("_index.tsv").unwrap();

    let mut command = Command::cargo_bin("nailpolish").unwrap();

    let a = command
        .args(&["index", SAMPLE_FASTQ, "--index", temp.path().to_str().unwrap()])
        .assert()
        .success();

    // lazy way of checking that these files are the same
    // EXCEPT for the header, which contains unique date and runtime information
    let cmp_cmd = format!(
        "diff <(tail -n+2 tests/correct/index.tsv) <(tail -n+2 {})",
        temp.path().to_str().unwrap()
    );

    let a = Command::new("bash")
        .arg("-c")
        .arg(&cmp_cmd)
        .unwrap();

    temp.close().unwrap();
}

#[test]
fn summary() {
    let temp = assert_fs::NamedTempFile::new("_summary.html").unwrap();
}