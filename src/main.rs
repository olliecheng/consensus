mod pairings;
mod reader;
mod seq;

use crate::reader::{FastQReader, Reader};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    file: String,
}

fn main() {
    let args = Args::parse();

    let file_path = args.file;

    let mut r = FastQReader::new(file_path);
    let mut seqs = r.read();
    for x in seqs.duplicates() {
        println!("BC:{},UMI:{}", x.0.bc, x.0.umi);
    }
}
