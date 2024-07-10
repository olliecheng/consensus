use std::{
    fs::File,
    io::{stdout, BufWriter, Write},
    path::Path,
    sync::{Arc, Mutex},
};

use clap::{Parser, Subcommand};
use serde_json;

mod call;
mod duplicates;
mod generate_index;

#[derive(Parser)]
#[command(
    version = "0.1.0",
    about = "tools for consensus calling reads with duplicate barcode and UMI matches",
    arg_required_else_help = true,
    flatten_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a summary of duplicate statistics from an index file
    #[command(arg_required_else_help = true)]
    Summary {
        /// the index file
        #[arg(long)]
        index: String,
    },

    /// Create an index file from a demultiplexed .fastq, if one doesn't already exist
    #[command(arg_required_else_help = true)]
    GenerateIndex {
        /// the input .fastq file
        #[arg(long)]
        file: String,

        /// the output index file
        #[arg(long, default_value = "index.tsv")]
        index: String,
    },

    /// Generate a consensus-called 'cleaned up' file
    #[command(arg_required_else_help = true)]
    Call {
        /// the index file
        #[arg(long)]
        index: String,

        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the number of threads to use
        #[arg(long, default_value_t = 4)]
        threads: u8,

        /// the output .fastq
        #[arg(long, default_value = "called_reads.fastq")]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Summary { index } => {
            let (_, statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");

            println!("{}", serde_json::to_string_pretty(&statistics).unwrap());
        }
        Commands::Call {
            index,
            input,
            output,
            threads,
        } => {
            eprintln!("Collecting duplicates...");
            let (duplicates, _statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            eprintln!("Iterating through individual duplicates");

            // get output as a BufWriter - equal to stdout if None
            let writer = BufWriter::new(match output {
                Some(ref x) => {
                    let file = match File::create(&Path::new(x)) {
                        Ok(r) => r,
                        Err(_) => {
                            eprintln!("Could not open file {x}");
                            return;
                        }
                    };
                    Box::new(file) as Box<dyn Write + Send>
                }
                None => Box::new(stdout()) as Box<dyn Write + Send>,
            });
            let writer = Arc::new(Mutex::new(writer));

            call::consensus(&input, &writer, duplicates, *threads).unwrap();
        }
        Commands::GenerateIndex { file, index } => {
            generate_index::construct_index(file, index);
            eprintln!("Completed index generation to {index}");
        }
    }
}
