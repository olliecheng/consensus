use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const INFO_STRING: &str = "
💅 nailpolish version ";
const AFTER_STRING: &str = "
   ──────────────────────────────────
   tools for consensus calling barcode and UMI duplicates
   https://github.com/olliecheng/nailpolish";

// colouring of the help
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().bold())
    .usage(AnsiColor::BrightMagenta.on_default().bold())
    .literal(AnsiColor::BrightMagenta.on_default())
    .placeholder(AnsiColor::White.on_default());

#[derive(Parser)]
#[command(
    version = VERSION,
    about = format!("{}{}{}", INFO_STRING, VERSION, AFTER_STRING),
    arg_required_else_help = true,
    flatten_help = true,
    styles = STYLES
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create an index file from a demultiplexed .fast2q
    #[command(arg_required_else_help = true)]
    Index {
        /// the input .fastq file
        file: String,

        #[arg(value_enum, conflicts_with = "barcode_regex", default_value = "bc-umi")]
        preset: crate::preset::PresetBarcodeFormats,

        /// the output index file
        #[arg(long, default_value = "index.tsv")]
        index: String,

        /// whether to use a file containing pre-clustered reads, with every line in one of two
        /// formats:
        ///   READ_ID;BARCODE     or,
        ///   READ_ID;BARCODE;UMI
        #[arg(long)]
        clusters: Option<String>,

        /// barcode regex format type, for custom header styles.
        /// This will override the preset given. For example:
        ///     ^@([ATCG]{16})_([ATCG]{12})
        /// for the BC-UMI preset.
        #[arg(long)]
        barcode_regex: Option<String>,

        /// skip, instead of error, on reads which are not accounted for:
        /// - if a cluster file is passed, any reads which are not in any cluster
        /// - if a barcode regex or preset is used (default), any reads which do not match the regex
        #[arg(long)]
        skip_unmatched: bool,
    },

    /// Generate a summary of duplicate statistics from an index file
    #[command(arg_required_else_help = true)]
    Summary {
        /// the index file
        #[arg(long)]
        index: String,

        /// output file
        #[arg(long, default_value = "summary.html")]
        output: String,
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
        threads: usize,

        /// only show the duplicated reads, not the single ones
        #[arg(short, long, action)]
        duplicates_only: bool,

        /// for each duplicate group of reads, report the original reads along with the consensus
        #[arg(short, long, action)]
        report_original_reads: bool,
    },

    /// Tag each read by its UMI group, and write to a .fastq file
    #[command(arg_required_else_help = true)]
    Group {
        /// the index file
        #[arg(long)]
        index: String,

        #[arg(long)]
        input: String,

        #[arg(long)]
        output: Option<String>,
    },
}
