use crate::duplicates::DuplicateMap;
use crate::duplicates::RecordIdentifier;
use bio::io::fastq;
use bio::io::fastq::{FastqRead, Reader};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::io::{Seek, SeekFrom};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rayon::{self, max_num_threads};

use spoa::{self, AlignmentEngine};
use std::ffi::{CStr, CString};

struct DuplicateRecord {
    id: RecordIdentifier,
    records: Vec<fastq::Record>,
}

pub fn consensus<R: Write + Send>(
    input: &str,
    writer: &Arc<Mutex<R>>,
    duplicates: DuplicateMap,
    threads: u8,
) -> Result<(), Box<dyn Error>> {
    // set number of threads that Rayon uses
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .unwrap();

    let mut file = File::open(input)?;
    //let file = Arc::new(Mutex::new(file));

    duplicates
        .into_iter()
        // first, read from the file (sequential)
        .map(|(id, positions)| {
            let mut rec = DuplicateRecord {
                id,
                records: Vec::new(),
            };
            for pos in positions.iter() {
                let mut record = fastq::Record::new();
                file.seek(SeekFrom::Start(*pos as u64))
                    .expect("Reading from file should not fail!");

                let mut reader = fastq::Reader::new(&mut file);
                reader.read(&mut record).unwrap();
                rec.records.push(record);
            }
            rec
        })
        // convert this sequential iterator into a parallel one for consensus calling
        .par_bridge()
        .for_each(|recs| {
            let mut alignment_engine =
                spoa::AlignmentEngine::new(spoa::AlignmentType::kOV, 5, -4, -8, -6, -10, -4);
            let mut poa_graph = spoa::Graph::new();

            for record in recs.records.iter() {
                let seq = record.seq();
                let qual = record.qual();

                let align = alignment_engine.align_from_bytes(seq, &poa_graph);
                poa_graph.add_alignment_from_bytes(&align, seq, &qual);
            }

            let consensus = poa_graph.consensus();
            let consensus = consensus
                .to_str()
                .expect("spoa module should produce valid utf-8");

            let writer = Arc::clone(writer);
            let mut writer = writer.lock().unwrap();

            writeln!(
                writer,
                ">{0}_{1}_CON_{2}\n{3}",
                recs.id.bc,
                recs.id.umi,
                recs.records.len(),
                consensus
            )
            .unwrap();
        });

    return Ok(());
}
