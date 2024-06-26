use bio::io::fastq;
use clap::{Parser, Subcommand};
use serde_json;

mod duplicates;

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// the index file
    #[arg(long)]
    index: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a summary of duplicate statistics from an index file
    Summary {},

    /// Generate a consensus-called 'cleaned up' file
    Generate {
        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the output .fastq
        #[arg(long)]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Summary {}) => {
            let (_, statistics) =
                duplicates::get_duplicates(&cli.index).expect("Could not parse index.");

            println!("{}", serde_json::to_string_pretty(&statistics).unwrap());
        }
        Some(Commands::Generate { input, output }) => {
            let (duplicates, statistics) =
                duplicates::get_duplicates(&cli.index).expect("Could not parse index.");

            for (key, val) in duplicates.iter() {
                // println!("key: {key:?}, val: {val:?}");
            }
        }
        None => {}
    }
}
