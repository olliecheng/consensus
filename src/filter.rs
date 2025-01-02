use crate::cli::ArgInterval;
use crate::io::Record;

pub struct FilterOpts {
    pub len: ArgInterval,
    pub quality: ArgInterval,
}

pub fn filter(read: &Record, opts: &FilterOpts) -> bool {
    opts.len.contains(read.len() as f64) && opts.quality.contains(read.phred_quality_avg())
}
