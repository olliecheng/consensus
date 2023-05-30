use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proj::{pairings::PairingCollection, reader::fastq};

pub fn read_small_file(c: &mut Criterion) {
    c.bench_function("small file", |b| {
        b.iter(|| {
            let mut r = fastq::FastQReader::new("tests/samples/small.fastq".to_string());

            let mut seqs = PairingCollection::from_reader_fastq(&mut r);

            for x in seqs.duplicates() {
                black_box(x);
            }
        })
    });
}

criterion_group!(benches, read_small_file);
criterion_main!(benches);
