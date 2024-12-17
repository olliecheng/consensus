# nailpolish index

This command is used to create an index file from a demultiplexed `.fastq`.
An index is required to run the other _nailpolish_ commands.
The index command supports reads in multiple formats.

## Usage

```shell
$ nailpolish index --help
Create an index file from a demultiplexed .fast2q

Usage: nailpolish index [OPTIONS] <FILE> [PRESET]

Arguments:
  <FILE>    the input .fastq file
  [PRESET]  [default: bc-umi] [possible values: bc-umi, umi-tools, illumina]

Options:
      --index <INDEX>                  the output index file [default: index.tsv]
      --clusters <CLUSTERS>            whether to use a file containing pre-clustered reads, with every line in one of two formats: READ_ID;BARCODE     or, READ_ID;BARCODE;UMI
      --barcode-regex <BARCODE_REGEX>  barcode regex format type, for custom header styles. This will override the preset given. For example: ^@([ATCG]{16})_([ATCG]{12}) for the BC-UMI preset
      --skip-unmatched                 skip, instead of error, on reads which are not accounted for: - if a cluster file is passed, any reads which are not in any cluster - if a barcode regex or preset is used (default), any reads which do not match the regex
  -h, --help                           Print help (see more with '--help')
```

## Presets

Three presets are bundled with _nailpolish_ for common barcode formats.
These are useful when the header of each read contains information about the barcode.

- `bc-umi`: read headers look like this: `@ATCGATCGATCG_ATCGATCGATCGATCG` in the `@BC_UMI` format.
  This is the default barcoding format produced by
  the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex) (Cheng et al., 2024).
- `umi-tools`: read headers look like this: `@HISEQ:87:00000000T_ATCGATCGATCG` where `ATCGATCGATCG` is the UMI sequence.
  This is the default UMI header format expected from the [umi-tools](https://umi-tools.readthedocs.io/en/latest/)
  collection of UMI management tools.
- `illumina`: read headers look like this: `@SIM:1:FCX:1:2106:15337:1063:ATCGATCGATCG 1:N:0:ATCACG` where `ATCGATCGATCG`
  is the UMI sequence.
  This is the default UMI header format produced by tools such as `bcl2fastq`.

## Barcode regex

For reads where barcodes and UMIs are contained in the header, in an esoteric format, a custom regular expression
can be provided through the `--barcode-regex <BARCODE_REGEX>` parameter. As examples, here are the regular expressions
for the presets above:

- `bc-umi`: `--barcode-regex "^([ATCG]{16})_([ATCG]{12})"`
- `umi-tools`: `--barcode-regex "_([ATCG]+)$"`
- `illumina`: `--barcode-regex ":([ATCG]+)$"`

Regular expressions are parsed by the excellent `regex` library for Rust.
This library is performant and has guarantees on worst-case time complexity;
however, the scope of supported regular expression features is more limited.
For complex queries, it is recommended that you consult the [crate documentation](https://docs.rs/regex/latest/regex/)
and test your regular expression using [regex101](https://regex101.com/), ensuring that you set the 'Flavor' to 'Rust'.

By default, _nailpolish_ expects that every read in the input `.fastq` **must** be able to be matched to the provided
regular expression.
In the event where this is not the case, _nailpolish_ will error. To ignore this error and silently skip over any
unmatched reads, the `--skip-unmatched` flag should be passed.

## Cluster file

_nailpolish_ can alternatively extract UMIs from a separately provided delimiter-separated file, if this information is
not in the read headers.
The file must be **semicolon-delimited** (`;`). Rows must be in the format `READ_ID;BARCODE` or `READ_ID;BARCODE;UMI`.
Note that **no header line** should be present in the file.

By default, _nailpolish_ expects that every read in the input `.fastq` **must** have a corresponding entry in the
cluster file.
In the event where this is not the case, _nailpolish_ will error. To ignore this error and silently skip over any
unmatched reads, the `--skip-unmatched` flag should be passed.