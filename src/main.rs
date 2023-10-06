use proj::align;
use proj::options::{Cli, Commands};
use proj::pairings::PairingCollection;
use proj::reader::fastq;

use itertools::Itertools;
use std::collections::HashMap;

use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let mut r = fastq::FastQReader::new(cli.clone());
    let mut seqs = PairingCollection::from_reader_fastq(&mut r);

    match &cli.command {
        Commands::Consensus(_) => {
            for x in seqs.duplicates() {
                align::msa(x.0, x.1);
            }
        }

        Commands::Count(_) => {
            let mut total = 0;
            let mut count = 0;

            let mut map = HashMap::new();

            for x in seqs.duplicates() {
                count += 1;

                let duplicates = x.1.len();
                total += duplicates;

                let e = map.entry(duplicates).or_insert(0);
                *e += 1;
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
            for i in map.keys().sorted() {
                println!("{}: {}", i, map.get(i).unwrap());
            }
        }
    }
}
