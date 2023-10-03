use proj::align;
use proj::options::{Cli, Commands};
use proj::pairings::PairingCollection;
use proj::reader::fastq;

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
            for x in seqs.duplicates() {
                count += 1;
                total += x.1.len();
            }
            println!(
                "Number of duplicates: {} with total number of duplicate reads: {}",
                count, total
            );
        }
    }
}
