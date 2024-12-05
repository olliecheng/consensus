# Quick Start

This quick start guide will walk you through installing Nailpolish and running it on a small demo dataset.
The demo dataset is a small subset of the _scmixology2_ Chromium 10x droplet-based dataset, sequenced using
Nanopore technology, released by [Tian et al. (2021)](https://doi.org/10.1186/s13059-021-02525-6).

Our Flexiplex tool is used to demultiplex the dataset.

## Install

For x64 Linux, run:

```shell
curl --proto '=https' --tlsv1.2 -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/nightly_develop/nailpolish" -o nailpolish
chmod +x nailpolish
```
