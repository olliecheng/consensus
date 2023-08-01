use clap::Parser;
use proj::align;
use proj::pairings::PairingCollection;
use proj::reader::fastq;

#[derive(Parser, Debug)]
struct Args {
    #[arg(default_value_t = String::from("tests/samples/small.fastq"))]
    file: String,
}

fn main() {
    let args = Args::parse();

    let file_path = args.file;

    let mut r = fastq::FastQReader::new(file_path);

    let mut seqs = PairingCollection::from_reader_fastq(&mut r);

    for x in seqs.duplicates() {
        align::msa(x.0, x.1);
    }
}
