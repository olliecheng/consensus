# duplicate-tools

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
$ consensus generate_index --file sample.fastq --output index.tsv
```
I can view summary statistics about duplicate rates using:
```sh
$ consensus summary --index index.tsv
```
and I can also transparently remove duplicate reads using:
```sh
$ consensus call \
  --index index.tsv \
  --input sample.fastq \
  --output sample_called.fastq \
  --threads 4
```
which will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the process.

## Usage
### Help

```
$ consensus --help

tools for consensus calling reads with duplicate barcode and UMI matches

Usage: consensus generate-index [OPTIONS] --file <FILE>
       consensus summary --index <INDEX>
       consensus call [OPTIONS] --index <INDEX> --input <INPUT>
       consensus help [COMMAND]...

Options:
  -h, --help     Print help
  -V, --version  Print version

consensus generate-index:
Create an index file from a demultiplexed .fastq, if one doesn't already exist
      --file <FILE>    the input .fastq file
      --index <INDEX>  the output index file [default: index.tsv]
  -h, --help           Print help

consensus summary:
Generate a summary of duplicate statistics from an index file
      --index <INDEX>  the index file
  -h, --help           Print help

consensus call:
Generate a consensus-called 'cleaned up' file
      --index <INDEX>          the index file
      --input <INPUT>          the input .fastq
      --output <OUTPUT>        the output .fasta; note that quality values are not preserved
  -t, --threads <THREADS>      the number of threads to use [default: 4]
  -d, --duplicates-only        only show the duplicated reads, not the single ones
  -r, --report-original-reads  for each duplicate group of reads, report the original reads along with the consensus
  -h, --help                   Print help
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
$ cargo install --git https://github.com/olliecheng/consensus.git

# or, from the local path
$ cargo install --path .
```

#### Note to HPC users on older systems
You will need a reasonably modern version of `gcc` and `cmake` installed, and the `CARGO_NET_GIT_FETCH_WITH_CLI` flag enabled. For instance:
```
$ module load gcc/latest cmake/latest
$ CARGO_NET_GIT_FETCH_WITH_CLI="true" cargo install --git https://github.com/olliecheng/consensus.git
```


### Build
```sh
$ git clone https://github.com/olliecheng/consensus.git
$ cargo build --release
```
The binary can be found at `/target/release/consensus`.
