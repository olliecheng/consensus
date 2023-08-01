use super::bytes;
use crate::seq::{self, Record};
use std::fs::File;
use std::io::{BufReader, Read};

#[allow(dead_code)]
pub struct FastQReader {
    filename: String,
    options: crate::options::Options,
}

impl FastQReader {
    pub fn new(filename: String) -> Self {
        Self {
            filename,
            options: crate::options::Options { bc: 16, umi: 12 },
        }
    }
}

impl super::Reader<Record> for FastQReader {
    fn read(&mut self) -> Box<dyn Iterator<Item = Record>> {
        Self::read_file(&self.filename)
    }

    fn read_file(filename: &str) -> Box<dyn Iterator<Item = Record>> {
        let file = File::open(std::path::Path::new(filename)).expect("Could not open file");
        Self::read_from_reader(Box::new(file))
    }

    fn read_from_reader(reader: Box<dyn Read>) -> Box<dyn Iterator<Item = Record>> {
        let options = crate::options::Options { bc: 16, umi: 12 };
        Box::new(FastQReadIterator::new(BufReader::new(reader), options))
    }
}

pub struct FastQReadIterator {
    bytes: bytes::ByteReader,
    options: crate::options::Options,
    reads: usize,
    eof: bool,
    avg_seq_len: f32,
}

impl FastQReadIterator {
    pub fn new(reader: bytes::GenericBufReader, options: crate::options::Options) -> Self {
        Self {
            bytes: bytes::ByteReader::new(reader),
            options,
            eof: false,
            avg_seq_len: 0.0,
            reads: 0,
        }
    }

    fn read_to_seq(&mut self, seq: &mut seq::Seq) -> usize {
        self.bytes.apply_on_slice_until_byte(b'\n', |chunk| {
            let size = chunk.len();
            if size == 16 {
                // https://users.rust-lang.org/t/unsafe-conversion-from-slice-to-array-reference/88910/2
                unsafe { seq.push_u32_chunk_of_n(&*chunk.as_ptr().cast(), 16) }
            } else {
                let mut new_arr = [0u8; 16];
                chunk.iter().enumerate().for_each(|(i, x)| unsafe {
                    *new_arr.get_unchecked_mut(i) = *x;
                });
                seq.push_u32_chunk_of_n(&new_arr, size)
            }
        })
    }

    fn read_n_to_seq(&mut self, n: usize, seq: &mut seq::Seq) {
        for _ in 0..n {
            let v = self.bytes.next_byte().expect("Found EOF, not allowed");
            seq.push(v)
        }
    }
}

impl Iterator for FastQReadIterator {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        match self.bytes.next_byte() {
            Some(b'\n') | None => {
                // end of file
                self.eof = true;
                return None;
            }
            Some(b'@') => (),
            _ => panic!("Wrong character: not @ starting read {}", self.reads),
        }

        // read barcode
        let mut bc = seq::Seq::with_capacity(self.options.bc);
        self.read_n_to_seq(self.options.bc, &mut bc);

        // next character must be _
        assert!(
            self.bytes.next_byte().expect("Expected _, not EOF") == b'_',
            "{} Next character after bc must be _",
            self.reads
        );

        // next: UMI
        let mut umi = seq::Seq::with_capacity(self.options.umi);
        self.read_n_to_seq(self.options.umi, &mut umi);

        // read metadata
        let mut metadata = Vec::new();
        let (_, eof) = self.bytes.read_line_trim_newline(&mut metadata);
        if eof {
            panic!("Metadata should not contain EOF");
        }

        // line 2: fastq sequence
        let mut seq = seq::Seq::with_capacity({
            if self.avg_seq_len > 10.0 {
                self.avg_seq_len as usize
            } else {
                10
            }
        });
        let length = self.read_to_seq(&mut seq);

        // line 3: expect a +
        match self.bytes.next_byte() {
            Some(b'+') => (),
            _ => panic!("Wrong character: not + starting read {}", self.reads),
        }
        self.bytes.seek_until_byte(b'\n');

        // line 4: read quality scores - for now, add to a string
        let mut qual = Vec::with_capacity(length);

        let (_, eof) = self.bytes.read_line_trim_newline(&mut qual);
        if eof {
            self.eof = true;
        }

        self.reads += 1;
        // calculate new exponentially weighted moving average (EMA)
        let alpha = 0.6;
        self.avg_seq_len = (alpha * length as f32) + (1.0 - alpha) * (self.avg_seq_len as f32);

        Some(Record {
            metadata,
            seq,
            id: seq::Identifier { bc, umi },
            qual,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::fastq;
    use crate::reader::Reader;
    use crate::seq::{Identifier, Seq};
    use pretty_assertions::assert_eq;
    use std::hint::black_box;

    fn read_file(file_path: String) -> Vec<fastq::Record> {
        let mut r = fastq::FastQReader::new(file_path);

        r.read().collect()
    }

    fn read_string(v: String) -> Vec<fastq::Record> {
        let f = Box::new(std::io::Cursor::new(v));
        let r = fastq::FastQReader::read_from_reader(f);

        r.collect()
    }

    #[test]
    fn small_file() {
        read_file("tests/samples/small.fastq".to_string());
    }

    #[test]
    #[should_panic]
    fn file_does_not_exist() {
        read_file("tests/samples/small1.fastq".to_string())
            .iter()
            .for_each(|x| {
                black_box(x);
            });
    }

    #[test]
    fn single_record_typical() {
        let rec = "@TCTGGCTCATTCTCCG_ACTGGTTGGTCT#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2
TTTGCTTAGCAATCTGCAGATCAAAATCTCCCTTTACCACTGGCATATTCAATAACTGGGCATTCTCTGCTTCCACAGCAGGTAAACTTCTGTCTTTTATTTGAGTGACCTCTTCAAGTTTCATAATCTCACTGGTCAAGCTAGAAATTTTAGCATCCAAATTTTGCTTTGTCCACAGCCTTGCTGGTTAGGCTGTGAAGACTCTCCTCTGCCCATTTTATATAACTTCATGCTTAAATTATTTCTTTGAGTGGATTAACTGATGTTGAGCACAAATGTATGCCAACCCAGTTCTATTTAGCCATCTCTAGTCGTCTCTCCTCAAGGATTTCTTGATTATCACAAATAGATGGTGTCTGTATATCTAAAAGTTGAAAATTGTCTTCCATTTAGAAAAATCTCAACTACTTCATGTATACCCTGAAAGAACTGTTTTTTGGTATACAAAAAAGTTAATGCTGCTGTGCTTTGCTCTTCCTGACTTAGGTATTTTTCAAGGAAAATTGCGATAAAAATACCAGTGGATTTGTCCCTTGACCTAAATTAGAATGTCTGAAGAACATCATCAATTGTGTAACTTCATCAGTAAAAGCCTGAAGTTCATTACTGATCTTAGTGATCATTGCATTTAGAATTCCTTGACTCTGCTACAGCTTTAGTGGCTTCTTCTTTCTTCGCATTTAACCTCAGAATTTTGTGGCTAGTTACTGAAGCCATCAATTGACATTTCTACATTCGCTGAATTTTTAGGTTCTTTAATTTCAGTAGAGTTTGAACCTCATCCTCTAATTTCTCCAGCTCTTTATCATCCAGTCTAGGTGTCTTCAAATCAGAAGTTTTACGCGTTCGAGCTTCATCCAATGCCGCCCCTTTCTAGAATAGGCTTGCCTGAATTTTTTTCTAGAAATTACTAAAGGCTTCCAATTCTCTTTTCAGACAACACGTTCTGTTCATTAGTTCCACAAAACCACTGCAGAAACGATTCATCTTCAACGCCCTCAAACAA
+TCTGGCTCATTCTCCG_ACTGGTTGGTCT#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2
'982+***,HHKHGGJFJDBBBCHIM{FICA85651?DFEHLGIIJKLV[NIFEDDDDFGHCJG<;9F11112445>?@?FCF:6=.81../?>?==:28<<<AEDEGEEF:998578AAFDA@@GJGLMKJNGGIGFFD?A86666311.+,,,,,--31,.64-0///,,00-,-('(,*'''()=DDGHH{GGLILJNFEEGEGDFEGEFGDD<;2-,(&$%%'(((*,000,+*(*(%%%%*+,(**)(''&&&()-.59=@C>>>>=GB?0+.-+*))'&%$$%&(()&&%%&&$&(*,,-0*++++31122:>;BACCCJHEHJ{HHIIIIHLIGHHFGGIJEEDGF{KGINHNHIIIGIEDHKE@><<<@HLMIGEEEA80032(')))*.)(()++8;;761.-/110342221ACEXHIHEECACA959,**)'&&&))/487553351...,0++,@EMIJGGEDA?>@EGGMLMEEED@DIGB9223?";
        let test_record = read_string(rec.to_string());

        let actual_record = fastq::Record {
            id: Identifier {
                bc: Seq::from_string("TCTGGCTCATTCTCCG"),
                umi: Seq::from_string("ACTGGTTGGTCT")
            },
            metadata: "#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2".bytes().collect(),
            seq: Seq::from_string("TTTGCTTAGCAATCTGCAGATCAAAATCTCCCTTTACCACTGGCATATTCAATAACTGGGCATTCTCTGCTTCCACAGCAGGTAAACTTCTGTCTTTTATTTGAGTGACCTCTTCAAGTTTCATAATCTCACTGGTCAAGCTAGAAATTTTAGCATCCAAATTTTGCTTTGTCCACAGCCTTGCTGGTTAGGCTGTGAAGACTCTCCTCTGCCCATTTTATATAACTTCATGCTTAAATTATTTCTTTGAGTGGATTAACTGATGTTGAGCACAAATGTATGCCAACCCAGTTCTATTTAGCCATCTCTAGTCGTCTCTCCTCAAGGATTTCTTGATTATCACAAATAGATGGTGTCTGTATATCTAAAAGTTGAAAATTGTCTTCCATTTAGAAAAATCTCAACTACTTCATGTATACCCTGAAAGAACTGTTTTTTGGTATACAAAAAAGTTAATGCTGCTGTGCTTTGCTCTTCCTGACTTAGGTATTTTTCAAGGAAAATTGCGATAAAAATACCAGTGGATTTGTCCCTTGACCTAAATTAGAATGTCTGAAGAACATCATCAATTGTGTAACTTCATCAGTAAAAGCCTGAAGTTCATTACTGATCTTAGTGATCATTGCATTTAGAATTCCTTGACTCTGCTACAGCTTTAGTGGCTTCTTCTTTCTTCGCATTTAACCTCAGAATTTTGTGGCTAGTTACTGAAGCCATCAATTGACATTTCTACATTCGCTGAATTTTTAGGTTCTTTAATTTCAGTAGAGTTTGAACCTCATCCTCTAATTTCTCCAGCTCTTTATCATCCAGTCTAGGTGTCTTCAAATCAGAAGTTTTACGCGTTCGAGCTTCATCCAATGCCGCCCCTTTCTAGAATAGGCTTGCCTGAATTTTTTTCTAGAAATTACTAAAGGCTTCCAATTCTCTTTTCAGACAACACGTTCTGTTCATTAGTTCCACAAAACCACTGCAGAAACGATTCATCTTCAACGCCCTCAAACAA"),
            qual: "'982+***,HHKHGGJFJDBBBCHIM{FICA85651?DFEHLGIIJKLV[NIFEDDDDFGHCJG<;9F11112445>?@?FCF:6=.81../?>?==:28<<<AEDEGEEF:998578AAFDA@@GJGLMKJNGGIGFFD?A86666311.+,,,,,--31,.64-0///,,00-,-('(,*'''()=DDGHH{GGLILJNFEEGEGDFEGEFGDD<;2-,(&$%%'(((*,000,+*(*(%%%%*+,(**)(''&&&()-.59=@C>>>>=GB?0+.-+*))'&%$$%&(()&&%%&&$&(*,,-0*++++31122:>;BACCCJHEHJ{HHIIIIHLIGHHFGGIJEEDGF{KGINHNHIIIGIEDHKE@><<<@HLMIGEEEA80032(')))*.)(()++8;;761.-/110342221ACEXHIHEECACA959,**)'&&&))/487553351...,0++,@EMIJGGEDA?>@EGGMLMEEED@DIGB9223?".bytes().collect()
        };

        assert!(test_record.len() == 1);
        assert_eq!(test_record[0], actual_record);
    }

    #[test]
    fn single_record_trailing_newline() {
        let rec = "@TCTGGCTCATTCTCCG_ACTGGTTGGTCT#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2
TTTGCTTAGCAATCTGCAGATCAAAATCTCCCTTTACCACTGGCATATTCAATAACTGGGCATTCTCTGCTTCCACAGCAGGTAAACTTCTGTCTTTTATTTGAGTGACCTCTTCAAGTTTCATAATCTCACTGGTCAAGCTAGAAATTTTAGCATCCAAATTTTGCTTTGTCCACAGCCTTGCTGGTTAGGCTGTGAAGACTCTCCTCTGCCCATTTTATATAACTTCATGCTTAAATTATTTCTTTGAGTGGATTAACTGATGTTGAGCACAAATGTATGCCAACCCAGTTCTATTTAGCCATCTCTAGTCGTCTCTCCTCAAGGATTTCTTGATTATCACAAATAGATGGTGTCTGTATATCTAAAAGTTGAAAATTGTCTTCCATTTAGAAAAATCTCAACTACTTCATGTATACCCTGAAAGAACTGTTTTTTGGTATACAAAAAAGTTAATGCTGCTGTGCTTTGCTCTTCCTGACTTAGGTATTTTTCAAGGAAAATTGCGATAAAAATACCAGTGGATTTGTCCCTTGACCTAAATTAGAATGTCTGAAGAACATCATCAATTGTGTAACTTCATCAGTAAAAGCCTGAAGTTCATTACTGATCTTAGTGATCATTGCATTTAGAATTCCTTGACTCTGCTACAGCTTTAGTGGCTTCTTCTTTCTTCGCATTTAACCTCAGAATTTTGTGGCTAGTTACTGAAGCCATCAATTGACATTTCTACATTCGCTGAATTTTTAGGTTCTTTAATTTCAGTAGAGTTTGAACCTCATCCTCTAATTTCTCCAGCTCTTTATCATCCAGTCTAGGTGTCTTCAAATCAGAAGTTTTACGCGTTCGAGCTTCATCCAATGCCGCCCCTTTCTAGAATAGGCTTGCCTGAATTTTTTTCTAGAAATTACTAAAGGCTTCCAATTCTCTTTTCAGACAACACGTTCTGTTCATTAGTTCCACAAAACCACTGCAGAAACGATTCATCTTCAACGCCCTCAAACAA
+TCTGGCTCATTCTCCG_ACTGGTTGGTCT#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2
'982+***,HHKHGGJFJDBBBCHIM{FICA85651?DFEHLGIIJKLV[NIFEDDDDFGHCJG<;9F11112445>?@?FCF:6=.81../?>?==:28<<<AEDEGEEF:998578AAFDA@@GJGLMKJNGGIGFFD?A86666311.+,,,,,--31,.64-0///,,00-,-('(,*'''()=DDGHH{GGLILJNFEEGEGDFEGEFGDD<;2-,(&$%%'(((*,000,+*(*(%%%%*+,(**)(''&&&()-.59=@C>>>>=GB?0+.-+*))'&%$$%&(()&&%%&&$&(*,,-0*++++31122:>;BACCCJHEHJ{HHIIIIHLIGHHFGGIJEEDGF{KGINHNHIIIGIEDHKE@><<<@HLMIGEEEA80032(')))*.)(()++8;;761.-/110342221ACEXHIHEECACA959,**)'&&&))/487553351...,0++,@EMIJGGEDA?>@EGGMLMEEED@DIGB9223?\n";
        let test_record = read_string(rec.to_string());
        let actual_record = fastq::Record {
            id: Identifier {
                bc: Seq::from_string("TCTGGCTCATTCTCCG"),
                umi: Seq::from_string("ACTGGTTGGTCT")
            },
            metadata: "#726ab78a-d517-40c9-a0de-dbf406419dba_+1of2".bytes().collect(),
            seq: Seq::from_string("TTTGCTTAGCAATCTGCAGATCAAAATCTCCCTTTACCACTGGCATATTCAATAACTGGGCATTCTCTGCTTCCACAGCAGGTAAACTTCTGTCTTTTATTTGAGTGACCTCTTCAAGTTTCATAATCTCACTGGTCAAGCTAGAAATTTTAGCATCCAAATTTTGCTTTGTCCACAGCCTTGCTGGTTAGGCTGTGAAGACTCTCCTCTGCCCATTTTATATAACTTCATGCTTAAATTATTTCTTTGAGTGGATTAACTGATGTTGAGCACAAATGTATGCCAACCCAGTTCTATTTAGCCATCTCTAGTCGTCTCTCCTCAAGGATTTCTTGATTATCACAAATAGATGGTGTCTGTATATCTAAAAGTTGAAAATTGTCTTCCATTTAGAAAAATCTCAACTACTTCATGTATACCCTGAAAGAACTGTTTTTTGGTATACAAAAAAGTTAATGCTGCTGTGCTTTGCTCTTCCTGACTTAGGTATTTTTCAAGGAAAATTGCGATAAAAATACCAGTGGATTTGTCCCTTGACCTAAATTAGAATGTCTGAAGAACATCATCAATTGTGTAACTTCATCAGTAAAAGCCTGAAGTTCATTACTGATCTTAGTGATCATTGCATTTAGAATTCCTTGACTCTGCTACAGCTTTAGTGGCTTCTTCTTTCTTCGCATTTAACCTCAGAATTTTGTGGCTAGTTACTGAAGCCATCAATTGACATTTCTACATTCGCTGAATTTTTAGGTTCTTTAATTTCAGTAGAGTTTGAACCTCATCCTCTAATTTCTCCAGCTCTTTATCATCCAGTCTAGGTGTCTTCAAATCAGAAGTTTTACGCGTTCGAGCTTCATCCAATGCCGCCCCTTTCTAGAATAGGCTTGCCTGAATTTTTTTCTAGAAATTACTAAAGGCTTCCAATTCTCTTTTCAGACAACACGTTCTGTTCATTAGTTCCACAAAACCACTGCAGAAACGATTCATCTTCAACGCCCTCAAACAA"),
            qual: "'982+***,HHKHGGJFJDBBBCHIM{FICA85651?DFEHLGIIJKLV[NIFEDDDDFGHCJG<;9F11112445>?@?FCF:6=.81../?>?==:28<<<AEDEGEEF:998578AAFDA@@GJGLMKJNGGIGFFD?A86666311.+,,,,,--31,.64-0///,,00-,-('(,*'''()=DDGHH{GGLILJNFEEGEGDFEGEFGDD<;2-,(&$%%'(((*,000,+*(*(%%%%*+,(**)(''&&&()-.59=@C>>>>=GB?0+.-+*))'&%$$%&(()&&%%&&$&(*,,-0*++++31122:>;BACCCJHEHJ{HHIIIIHLIGHHFGGIJEEDGF{KGINHNHIIIGIEDHKE@><<<@HLMIGEEEA80032(')))*.)(()++8;;761.-/110342221ACEXHIHEECACA959,**)'&&&))/487553351...,0++,@EMIJGGEDA?>@EGGMLMEEED@DIGB9223?".bytes().collect()
        };
        assert!(test_record.len() == 1);
        assert_eq!(test_record[0], actual_record);
    }
}
