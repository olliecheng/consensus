use std::{
    fs::File,
    io::{prelude::*, stdout, BufWriter},
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json;

extern crate env_logger;
#[macro_use]
extern crate log;

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

    /// Generate a summary of duplicate statistics from an index file
    #[command(arg_required_else_help = true)]
    Summary {
        /// the index file
        #[arg(long)]
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

        /// the output .fasta; note that quality values are not preserved
        #[arg(long)]
        output: Option<String>,

        /// the number of threads to use
        #[arg(short, long, default_value_t = 4)]
        threads: u8,

        /// only show the duplicated reads, not the single ones
        #[arg(short, long, action)]
        duplicates_only: bool,

        /// for each duplicate group of reads, report the original reads along with the consensus
        #[arg(short, long, action)]
        report_original_reads: bool,
    },

    /// 'Group' duplicate reads, and pass to downstream applications.
    #[command(arg_required_else_help = true)]
    Group {
        /// the index file
        #[arg(long)]
        index: String,

        /// the input .fastq
        #[arg(long)]
        input: String,

        /// the output location, or default to stdout
        #[arg(long)]
        output: Option<String>,

        /// the shell used to run the given command
        #[arg(long, default_value = "bash")]
        shell: String,

        /// the number of threads to use. this will not guard against race conditions in any
        /// downstream applications used. this will effectively set the number of individual
        /// processes to launch
        #[arg(short, long, default_value_t = 1)]
        threads: u8,

        /// the command to run. any groups will be passed as .fastq standard input.
        #[arg(trailing_var_arg = true, default_value = "cat")]
        command: Vec<String>,
    },
}

fn get_writer(output: &Option<String>) -> Result<Arc<Mutex<impl Write>>, std::io::Error> {
    // get output as a BufWriter - equal to stdout if None
    let writer = BufWriter::new(match output {
        Some(ref x) => {
            let file = File::create(&Path::new(x))?;
            Box::new(file) as Box<dyn Write + Send>
        }
        None => Box::new(stdout()) as Box<dyn Write + Send>,
    });

    Ok(Arc::new(Mutex::new(writer)))
}

fn try_main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Summary { index } => {
            info!("Summarising index at {index}");
            let (_, statistics) = duplicates::get_duplicates(index)?;

            println!(
                "{}",
                serde_json::to_string_pretty(&statistics).expect("Should be serialisable")
            );
        }
        Commands::GenerateIndex { file, index } => {
            generate_index::construct_index(file, index);
            info!("Completed index generation to {index}");
        }
        Commands::Call {
            index,
            input,
            output,
            threads,
            duplicates_only,
            report_original_reads,
        } => {
            info!("Collecting duplicates... {}", duplicates_only);
            let (duplicates, _statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            info!("Iterating through individual duplicates");

            let writer = get_writer(output).unwrap();

            call::consensus(
                &input,
                &writer,
                duplicates,
                *threads,
                *duplicates_only,
                *report_original_reads,
            )?;
        }
        Commands::Group {
            index,
            input,
            output,
            threads,
            shell,
            command,
        } => {
            let command_str = command.join(" ");
            info!(
                "Executing `{}` for every group using {}",
                command_str, shell
            );
            info!(
                "Multithreading is {}",
                if *threads != 1 { "enabled" } else { "disabled" }
            );

            info!("Collecting duplicates...");
            let (duplicates, _statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            info!("Iterating through individual duplicates");

            let writer = get_writer(output).unwrap();

            call::custom_command(&input, &writer, duplicates, *threads, &shell, &command_str)?;
        }
    };
    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        error!("{}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| error!("  because: {}", cause));
    }
}
