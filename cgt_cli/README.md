# `cgt-cli`

`cgt-cli` is a command line program that evaluates game positions, exhaustive and genetic searches, and transforms evaluation and search results into graphic latex figures.

## Installation

### Building from source (Recommended)

To build `cgt-cli` from the source clone the repository and install [Rust toolchain](https://www.rust-lang.org/tools/install) (`rustc`, `cargo`). If you are using [Nix](https://nixos.org/) you can use `nix develop` to bootstrap the development environment for you.

```console
$ git clone https://github.com/t4ccer/cgt-tools.git
$ cd cgt-tools
$ cargo build --package cgt_cli --release
```

You will find the binary in `./target/release` directory created by `cargo`.

### Releases page

> [!WARNING]
> There is no stable version of `cgt-cli` yet and releases are published very infrequently thus it is recommended to build from source.

Once in a while when the release is published, GNU/Linux and Windows pre-built binaries are published in the [releases tab](https://github.com/t4ccer/cgt-tools/releases/). GNU/Linux pre-built binaries have some problems on my system (See [Building from source](#building-from-source)), but Windows ones seem to work (confirmed with [Wine](https://www.winehq.org/)).

## Usage

Once you have `cgt-cli` you can use it to print available options.

```console
$ cgt-cli --help
```

You can further call `cgt-cli` with `--help` on subcommands e.g.

```console
$ cgt-cli snort --help
```

### Filtering results

This section assumes running `cgt-cli` on unix-like system (system needs to support split between stdout and stderr and piping outputs). While `cgt-cli` compiles on Windows and Darwin (macOS) author does not run proprietary systems to check if this section applies.

This section requires [jq](https://jqlang.github.io/jq/) to be installed.

`cgt-cli` will output "debug" information to standard error and computer-readable JSON object to standard output. This can be used to pipe into files and reused later or pipe into other programs, like `jq`, to filter output data.

#### Example: Get only temperature of Snort position

```bash
# `2>/dev/null` will silent debug information
# `| jq '.temperature' --raw-output` will filter the output and strip quotes
cgt-cli snort graph --edges '0-1,1-2' --no-graphviz 2>/dev/null | jq '.temperature' --raw-output
```
