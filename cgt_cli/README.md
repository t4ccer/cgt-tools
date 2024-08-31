# `cgt-cli`

`cgt-cli` is a program to run exhaustive search of Domineering positions to get their canonical forms and temperatures.

## Installation

### Building from source (Recommended)

To build `cgt-cli` from source clone the repository and install [Rust toolchain](https://www.rust-lang.org/tools/install) (`rustc`, `cargo`). If you are using [Nix](https://nixos.org/) you can use `nix develop` to bootstrap the development environment for you.

```console
$ git clone https://github.com/t4ccer/cgt-tools.git
$ cd cgt-tools
$ cargo build --package cgt_cli --release
```

You will find the binary in `./target/release` directory created by `cargo`.

### Releases page

> [!WARNING]
> There is no stable version of `cgt-cli` yet and releases are published very infrequently thus it is recommended to build from source.

Once in a while when release is published, GNU/Linux and Windows pre-built binaries are published in the [releases tab](https://github.com/t4ccer/cgt-tools/releases/). GNU/Linux pre-built binaries have some problems on my system (See [Building from source](#building-from-source)), but Windows ones seems to work (confirmed with [Wine](https://www.winehq.org/)).

## Usage

Once you have `cgt-cli` you can use it to print available options.

```console
$ cgt-cli --help
```

You can furher call `cgt-cli` with `--help` on subcommands e.g.

```console
$ cgt-cli snort --help
```
