// disable unused code warnings for dev builds
// #![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

extern crate env_logger;
#[macro_use]
extern crate log;
use std::{
    fs::File,
    io::{prelude::*, stdout, BufWriter},
    path::Path,
};

use anyhow::Result;
use clap::Parser;

mod call;
mod duplicates;
mod generate_index;
mod cli;
mod preset;
mod file;
mod summary;
mod io;
mod group;


use cli::{Cli, Commands};

/// Creates a `BufWriter` for the given output option. This allows for an output file to be passed
/// or otherwise will default to using standard output.
///
/// If `output` is `Some`, it creates a file at the specified path and returns a `BufWriter` for it.
/// If `output` is `None`, it returns a `BufWriter` for the standard output.
///
/// # Arguments
///
/// * `output` - An `Option` containing the path to the output file as a `String`.
///
/// # Returns
///
/// A `Result` containing a `BufWriter` that implements `Write`.
fn get_writer(output: &Option<String>) -> Result<impl Write> {
    // get output as a BufWriter - equal to stdout if None
    let writer = BufWriter::new(match output {
        Some(ref x) => {
            let file = File::create(Path::new(x))?;
            Box::new(file) as Box<dyn Write + Send>
        }
        None => Box::new(stdout()) as Box<dyn Write + Send>,
    });
    Ok(writer)
}

fn try_main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    let cli = Cli::parse();

    println!("nailpolish v{}", cli::VERSION);

    match &cli.command {
        Commands::Summary { index, output } => {
            summary::summarize(index, output)?;
        }
        Commands::Index {
            file,
            index,
            preset,
            barcode_regex,
            clusters,
            skip_unmatched
        } => {
            let barcode_regex = match barcode_regex {
                Some(v) => {
                    info!("Using specified barcode format: {v}");
                    v.clone()
                }
                None => {
                    let regex = preset::get_barcode_regex(preset);
                    info!("Using preset barcode format {regex}");
                    regex
                }
            };

            generate_index::construct_index(file, index, &barcode_regex, *skip_unmatched, clusters)?;
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
            let (duplicates, _statistics, _) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            info!("Iterating through individual duplicates");

            let mut writer = get_writer(output)?;

            call::consensus(
                input,
                &mut writer,
                duplicates,
                *threads,
                *duplicates_only,
                *report_original_reads,
            )?;

            info!("Completed successfully.")
        }
        Commands::Group {
            index,
            input,
            output
        } => {
            let (duplicates, _, _) =
                duplicates::get_duplicates(index).expect("Could not parse index.");

            let mut writer = get_writer(output)?;

            group::group(input, &mut writer, duplicates)?;

            info!("Completed successfully.")
        }
    };
    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        error!("{}", err);

        // report any errors that are produced
        err.chain()
            .skip(1)
            .for_each(|cause| error!("  because: {}", cause));
    }
}
