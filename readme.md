# 💅 nailpolish

When demultiplexing data, duplicates are produced which usually contain many similarities,
but also contain conflicting information at certain points.
This project contains tools which can quickly index, manipulate, and consensus call
 these duplicates.

<div align="center">
 <a href="#example">Example</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#usage">Usage</a> &nbsp;&nbsp; | &nbsp;&nbsp; <a href="#installation">Installation</a>
</div>

## Example
Say I have a demultiplexed `sample.fastq` file of the following form—for instance, one generated using the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex):
```
@BC1_UMI1
sequence...
+
quality...
```
I first create an _index_ file using
```sh
$ duplicate-tools generate_index --file sample.fastq --output index.tsv
```
I can view summary statistics about duplicate rates using:
```sh
$ duplicate-tools summary --index index.tsv
```
and I can also transparently remove duplicate reads using:
```sh
$ duplicate-tools call \
  --index index.tsv \
  --input sample.fastq \
  --output sample_called.fastq \
  --threads 4
```
which will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the process.

I can also choose to pass along groups to the `spoa` program, which should produce similar
results since `duplicate-tools` uses native bindings to `spoa` for consensus calling:
```sh
  # needed since spoa doesn't support standard input
$ mkfifo /tmp/myfifo.fastq
$ duplicate-tools group --index $IDX --input $I --output sample-called.fastq \
	"tee /tmp/myfifo.fastq | spoa /tmp/myfifo.fastq -r 0"
```
Of course, this method isn't recommended, as it is slower than using native bindings, and
offers less functionality (such as the lack of a `--duplicates-only=false` option). However,
especially for programs which make use of pipes, this can be a good approach to allow
external consensus calling functionality.

## Usage
### Help

```
tools for consensus calling reads with duplicate barcode and UMI matches

Usage: duplicate-tools generate-index [OPTIONS] --file <FILE>
       duplicate-tools summary --index <INDEX>
       duplicate-tools call [OPTIONS] --index <INDEX> --input <INPUT>
       duplicate-tools group [OPTIONS] --index <INDEX> --input <INPUT> [COMMAND]...
       duplicate-tools help [COMMAND]...

Options:
  -h, --help     Print help
  -V, --version  Print version

duplicate-tools generate-index:
Create an index file from a demultiplexed .fastq, if one doesn't already exist
      --file <FILE>    the input .fastq file
      --index <INDEX>  the output index file [default: index.tsv]
  -h, --help           Print help

duplicate-tools summary:
Generate a summary of duplicate statistics from an index file
      --index <INDEX>  the index file
  -h, --help           Print help

duplicate-tools call:
Generate a consensus-called 'cleaned up' file
      --index <INDEX>          the index file
      --input <INPUT>          the input .fastq
      --output <OUTPUT>        the output .fasta; note that quality values are not preserved
  -t, --threads <THREADS>      the number of threads to use [default: 4]
  -d, --duplicates-only        only show the duplicated reads, not the single ones
  -r, --report-original-reads  for each duplicate group of reads, report the original reads along with the consensus
  -h, --help                   Print help

duplicate-tools group:
'Group' duplicate reads, and pass to downstream applications
      --index <INDEX>      the index file
      --input <INPUT>      the input .fastq
      --output <OUTPUT>    the output location, or default to stdout
      --shell <SHELL>      the shell used to run the given command [default: bash]
  -t, --threads <THREADS>  the number of threads to use. this will not guard against race conditions in any downstream applications used. this will effectively set the number of individual processes to launch [default: 1]
  -h, --help               Print help
  [COMMAND]...         the command to run. any groups will be passed as .fastq standard input [default: cat]
```


<details>
<summary>Example of <code>--duplicates-only</code> and <code>--report-original-reads</code></summary>
Suppose I have a demultiplexed read file of the following format (so that <code>seq2</code> and <code>seq3</code> are duplicates):
<pre>
@BCUMI_1
seq1
@BCUMI_2
seq2
@BCUMI_2
seq3
</pre>
Then, the effects of the following flags are:
<pre>
(default):
  >BCUMI_1_SIN
  seq1
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>

<pre>
--duplicates-only:
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>

<pre>
--report-original-reads
  >BCUMI_1_SIN
  seq1
  >BCUMI_2_DUP_1_of_2
  seq2
  >BCUMI_2_DUP_2_of_2
  seq3
  >BCUMI_2_CON_2
  seq2_and_3_consensus
</pre>
</details>


## Installation
You will need a modern version of Rust installed on your machine, as well as the Cargo package manager. That's it - all package installations will be done automatically at the build stage.

### Install to PATH
```sh
$ cargo install --git https://github.com/olliecheng/duplicate-tools.git

# or, from the local path
$ cargo install --path .
```

#### Note to HPC users on older systems
You will need a reasonably modern version of `gcc` and `cmake` installed, and the `CARGO_NET_GIT_FETCH_WITH_CLI` flag enabled. For instance:
```
$ module load gcc/latest cmake/latest
$ CARGO_NET_GIT_FETCH_WITH_CLI="true" cargo install --git https://github.com/olliecheng/duplicate-tools.git
```


### Build
```sh
$ git clone https://github.com/olliecheng/duplicate-tools.git
$ cargo build --release
```
The binary can be found at `/target/release/duplicate-tools`.
