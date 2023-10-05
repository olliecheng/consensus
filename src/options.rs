use clap::{Args, Parser, Subcommand};

#[derive(Parser, Clone)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(global=true, default_value_t = String::from("tests/samples/small.fastq"))]
    pub file: String,

    #[arg(global = true, default_value_t = 16, long)]
    pub bc: usize,

    #[arg(global = true, default_value_t = 12, long)]
    pub umi: usize,

    #[arg(global = true, default_value_t = false, short, long)]
    pub gzip: bool,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    Count(CountArgs),
    Consensus(ConsensusArgs),
}

#[derive(Args, Clone)]
pub struct CountArgs {}

#[derive(Args, Clone)]
pub struct ConsensusArgs {}

pub struct Options {
    pub bc: usize,
    pub umi: usize,
}

pub fn default_cli(file_path: String) -> Cli {
    Cli::parse_from(vec!["testing-bin".into(), "consensus".into(), file_path])
}
