use crate::duplicates::DuplicateMap;
use bio::io::fastq;
use bio::io::fastq::{FastqRead, Reader};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::io::{Seek, SeekFrom};

use spoa::{self, AlignmentEngine};
use std::ffi::{CStr, CString};

pub fn consensus<R: Write>(
    input: &str,
    mut writer: &mut R,
    duplicates: DuplicateMap,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(input)?;

    // these are the SPOA default values with semi-global alignment
    let mut alignment_engine =
        spoa::AlignmentEngine::new(spoa::AlignmentType::kOV, 5, -4, -8, -6, -10, -4);

    for (id, positions) in duplicates.iter() {
        if positions.len() == 1 {
            // TODO: do something here later
            continue;
        }

        // Construct graph
        let mut poa_graph = spoa::Graph::new();

        for pos in positions.iter() {
            let mut record = fastq::Record::new();
            file.seek(SeekFrom::Start(*pos as u64))?;

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
        let cons = poa_graph.consensus();
        let consensus_seq = cons
            .to_str()
            .expect("spoa module should produce valid utf-8");

        write!(
            writer,
            ">{0}_{1}_CON_{2}\n{3}",
            id.bc,
            id.umi,
            positions.len(),
            consensus_seq
        );
    }

    return Ok(());
}
