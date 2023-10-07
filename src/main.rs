use proj::align;
use proj::options::{Cli, Commands};
use proj::pairings::PairingCollection;
use proj::reader::fastq;
use proj::stats::Stats;

use itertools::Itertools;
use std::collections::HashMap;

use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let mut r = fastq::FastQReader::new(cli.clone());
    let mut seqs = PairingCollection::from_reader_fastq(&mut r);

    match &cli.command {
        Commands::Consensus(_) => {
            let mut n_err = 0;

            for x in seqs.duplicates() {
                let s = Stats::from(&x);
                if s.pc_t > 50.0 || s.pc_a > 50.0 {
                    // these reads all have an abnormally high PC_T and/or PC_A count
                    // we should probably ignore them
                    eprintln!("Error due to %T or %A:\n{}", x);
                    n_err += 1;
                }

                align::msa(&x);
            }
            eprintln!(
                "\n\nCompleted, with {} errors due to %A or %T counts.",
                n_err
            );
        }

        Commands::Stats(_) => {
            let mut total = 0;
            let mut count = 0;

            let mut map = HashMap::new();

            for x in seqs.all() {
                count += 1;

                let duplicates = x.reads.len();
                if duplicates > 1 {
                    total += duplicates;
                }

                let e = map
                    .entry(duplicates)
                    .or_insert_with(|| Stats::default(duplicates.to_string()));

                e.add_pairing(&x);
            }

            println!(
                "Number of duplicates: {} with total number of duplicate reads: {}",
                count, total
            );

            println!(
                "Total number of reads including duplicates: {}",
                seqs.total_reads
            );

            println!("\nBreakdown:");
            println!("{}", Stats::display_header());
            for i in map.keys().sorted() {
                // println!("{}: {}", i, map.get(i).unwrap());
                println!("{}", map.get(i).unwrap());
            }
        }
    }
}
