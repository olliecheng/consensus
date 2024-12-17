# nailpolish summary

Quickly review the quality and duplicate rate of the dataset.
The reads must first have been [indexed](./generate-index.md).

## Usage

```shell
$ nailpolish summary --help
Generate a summary of duplicate statistics from an index file

Usage: nailpolish summary [OPTIONS] --index <INDEX>

Options:
      --index <INDEX>    the index file
      --output <OUTPUT>  output file [default: summary.html]
  -h, --help             Print help
```

## Output

[See an example summary output file.](../assets/summary.html)

<iframe src="../assets/summary.html" style="width: 100%; height: 60vh; min-height: 500px;"></iframe>