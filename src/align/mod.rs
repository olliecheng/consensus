use crate::seq;

use std::env;
use std::fs;
use std::process::{Command, Stdio};

// use petgraph::dot::{Config, Dot};

pub fn msa(id: &seq::Identifier, sequences: &Vec<seq::RecordData>) {
    let temp_file_path = match env::var("TEMP_FASTQ") {
        Ok(v) => v,
        Err(_) => String::from("/tmp/temporary_fastq_for_spoa.fastq"),
    };

    let spoa_path = match env::var("SPOA_PATH") {
        Ok(v) => v,
        Err(_) => String::from("spoa"),
    };

    // first, output a FASTQ file to /tmp
    let mut temp_fq = String::new();

    for (i, rec) in sequences.iter().enumerate() {
        println!(">{}:{}_{}\n{}", id.bc, id.umi, i + 1, rec.seq);
        temp_fq = format!(
            "{}@{}\n{}\n+\n{}\n",
            temp_fq,
            i,
            rec.seq,
            String::from_utf8(rec.qual.clone()).unwrap()
        );
    }

    // write to /tmp
    fs::write(&temp_file_path, temp_fq).expect("Unable to write file");

    // run SPOA through the command line
    let child = Command::new(spoa_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["-l", "2", &temp_file_path])
        // -l 0: local
        // -l 1: global
        // -l 2: semi-global
        .spawn()
        .expect("Spawning should not fail");

    let result = child.wait_with_output().expect("Command didn't start");

    if result.status.success() {
        println!(
            ">{}:{}_Consensus\n{}",
            id.bc,
            id.umi,
            String::from_utf8(result.stdout)
                .unwrap()
                .lines()
                .skip(1) // skip the first line, this is the FASTA header
                .next() // get line #2 as a String
                .expect("Fastq should be well formed with 2 lines")
        );
    } else {
        println!(
            "Fail - error\nstdout:\n{}\n\nstderr:\n{}\n",
            String::from_utf8(result.stdout).unwrap(),
            String::from_utf8(result.stderr).unwrap()
        )
    }
}
