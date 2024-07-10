use std::{
    fs::File,
    io::{stdout, BufWriter, Write},
    path::Path,
};

use bio::io::fastq;
use clap::{Parser, Subcommand};
use serde_json;

mod call;
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
        output: Option<String>,
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
            eprintln!("Collecting duplicates...");
            let (duplicates, _statistics) =
                duplicates::get_duplicates(&cli.index).expect("Could not parse index.");
            eprintln!("Iterating through individual duplicates");

            // get output as a BufWriter - equal to stdout if None
            let mut writer = BufWriter::new(match output {
                Some(ref x) => {
                    let file = match File::create(&Path::new(x)) {
                        Ok(r) => r,
                        Err(_) => {
                            eprintln!("Could not open file {x}");
                            return;
                        }
                    };
                    Box::new(file) as Box<dyn Write>
                }
                None => Box::new(stdout()) as Box<dyn Write>,
            });

            call::consensus(&input, &mut writer, duplicates).unwrap();
        }
        None => {}
    }
}
