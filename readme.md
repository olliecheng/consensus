# Consensus

When demultiplexing data, duplicates are produced which usually contain many similarities, but also contain conflicting information at certain points. The aim of this project is to create a tool which can quickly consensus call these duplicates.


## Example
Say I have a demultiplexed `sample.fastq` file of the following form—for instance, one generated using the [Flexiplex demultiplexer](https://github.com/DavidsonGroup/flexiplex)):
```
@BC_UMI#furtherheaderinformation
sequence...
+
quality...
```
I first create an _index_ file using
```sh
$ consensus generate_index --file sample.fastq --output index.tsv
```
I can then get summary statistics about duplicate rates using:
```sh
$ consensus summary --index index.tsv
```
and I can also transparently remove duplicate reads using:
```sh
$ consensus generate \
	--index index.tsv \
	--input sample.fastq \
	--output sample_called.fastq \
	--threads 4
```
which will output all non-duplicated and consensus called reads, removing all the original duplicated reads in the process.

## Usage
### Help



## Installation
You will need a modern version of Rust installed on your machine, as well as the Cargo package manager. That's it - all package installations will be done automatically at the build stage.

### Install to PATH
```sh
$ cargo install --git https://github.com/olliecheng/consensus.git --branch develop consensus

# or, from the local path
$ cargo install --path .
```

### Build
```sh
$ git clone https://github.com/olliecheng/consensus.git
$ cargo build --release
```
The binary can be found at `/target/release/consensus`.