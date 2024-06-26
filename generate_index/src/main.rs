use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use clap::Parser;
use csv::{Writer, WriterBuilder};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(long, default_value = "index.tsv")]
    output: String,

    #[arg(long)]
    file: String,
}

fn iter_lines<W: std::io::Write>(mut reader: BufReader<File>, mut wtr: Writer<W>) {
    let mut position: usize = 0;
    let mut count: usize = 0;

    // write headers
    wtr.write_record(&[
        "Read",
        "CellBarcode",
        "FlankEditDist",
        "BarcodeEditDist",
        "UMI",
        "Position",
    ])
    .unwrap();

    let mut result = String::new();
    while let Ok(bsize) = reader.read_line(&mut result) {
        if bsize == 0 {
            // EOF has been reached
            break;
        }

        if count % 4 == 0 {
            // extract barcode, UMI, and the read ID
            // format: @TCTGGCTCATTCTCCG_GCAGCGAAGCCC#32b5d571-ad88-4ac7-bc46-f2ff03de65aa_+1of1
            let i = result.find('_').unwrap();
            let j = result.find('#').unwrap();
            let k = result.rfind('_').unwrap();

            let bc = &result[1..i];
            let umi = &result[(i + 1)..j];
            let id = &result[(j + 1)..k];

            //println!("{}, {}, {}, {}", bc, umi, id, actual_pos);
            wtr.write_record(&[id, bc, "?", "?", umi, &position.to_string()])
                .unwrap();
        }
        count += 1;
        position += bsize;

        // reset string
        result.clear();
    }
    wtr.flush().unwrap();
}

pub fn construct_index(infile: &str, outfile: &str) {
    let f = File::open(infile).expect("File could not be opened");
    let reader = BufReader::new(f);

    let wtr = WriterBuilder::new()
        .delimiter(b'\t')
        .from_path(outfile)
        .unwrap();

    iter_lines(reader, wtr);
}

fn main() {
    let args = Args::parse();

    construct_index(&args.file, &args.output);

    eprintln!("Completed index generation to {}", &args.output);
}
