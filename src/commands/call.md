# nailpolish call

Consensus call duplicated reads
The reads must first have been [indexed](./generate-index.md).

## Usage

```shell
$ nailpolish call --help
Generate a consensus-called 'cleaned up' file

Usage: nailpolish call [OPTIONS] --index <INDEX> --input <INPUT>

Options:
      --index <INDEX>          the index file
      --input <INPUT>          the input .fastq
      --output <OUTPUT>        the output .fasta; note that quality values are not preserved
  -t, --threads <THREADS>      the number of threads to use [default: 4]
  -d, --duplicates-only        only show the duplicated reads, not the single ones
  -r, --report-original-reads  for each duplicate group of reads, report the original reads along with the consensus
  -h, --help                   Print help
```

## Output format

TODO

## Options

- `--duplicates-only`: By default, _nailpolish_ will consensus call all duplicated reads and also produces each
  non-duplicated read intact. With this flag, only the duplicated reads will be produced, and all non-duplicated reads
  will be skipped.
- `--report-original-reads`: By default, when reads are consensus called, only the consensus read is produced.
  With this flag, each of the original reads will be produced as well.