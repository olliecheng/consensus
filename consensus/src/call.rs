use crate::duplicates::DuplicateMap;
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
    let file = Arc::new(Mutex::new(file));

    duplicates.par_iter().for_each(|(id, positions)| {
        if positions.len() == 1 {
            // TODO: do something here later
            return;
        }

        let file = Arc::clone(&file);
        let mut file_binding = file.lock().unwrap();
        let mut file = file_binding.deref_mut();

        // these are the SPOA default values with semi-global alignment
        let mut alignment_engine =
            spoa::AlignmentEngine::new(spoa::AlignmentType::kOV, 5, -4, -8, -6, -10, -4);

        // Construct graph
        let mut poa_graph = spoa::Graph::new();

        for pos in positions.iter() {
            let mut record = fastq::Record::new();
            file.seek(SeekFrom::Start(*pos as u64))
                .expect("Reading from file should not fail");

            let mut reader = fastq::Reader::new(&mut file);
            reader.read(&mut record).unwrap();

            // seq and qual should both be null terminated before casting to CStr
            let seq_null_term = [record.seq(), b"\0"].concat();
            let qual_null_term = [record.qual(), b"\0"].concat();

            // add record to graph
            let seq =
                CStr::from_bytes_with_nul(&seq_null_term).expect("Casting to cstr should not fail");
            let qual = CStr::from_bytes_with_nul(&qual_null_term)
                .expect("Casting to cstr should not fail");

            let align = alignment_engine.align(seq, &poa_graph);
            poa_graph.add_alignment(&align, seq, &qual);

            // TODO: if asked, write each read as well
        }

        // free the lock
        drop(file);

        let cons = poa_graph.consensus();
        let consensus_seq = cons
            .to_str()
            .expect("spoa module should produce valid utf-8");

        let writer = Arc::clone(writer);
        let mut writer = writer.lock().unwrap();

        writeln!(
            writer,
            ">{0}_{1}_CON_{2}\n{3}",
            id.bc,
            id.umi,
            positions.len(),
            consensus_seq
        );
    });

    return Ok(());
}
