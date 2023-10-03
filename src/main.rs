use clap::{Args, Parser, Subcommand};
use proj::align;
use proj::pairings::PairingCollection;
use proj::reader::fastq;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(default_value_t = String::from("tests/samples/small.fastq"))]
    file: String,
}

#[derive(Subcommand)]
enum Commands {
    Count(CountArgs),
    Consensus(ConsensusArgs),
}

#[derive(Args)]
struct CountArgs {}

#[derive(Args)]
struct ConsensusArgs {}

fn main() {
    let cli = Cli::parse();

    let file_path = cli.file;

    let mut r = fastq::FastQReader::new(file_path);

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
