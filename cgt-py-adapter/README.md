# `cgt-py-adapter`

A `python`/`sage` adapter to `cgt-rs`.

## Usage

Copy `cgt.py` to your sage working directory. This file is a python wrapper for the Rust implementation. To use it, you will need access to the `cgt-py-adapter` binary executable. You can obtain it either from the [releases tab](https://github.com/t4ccer/cgt-/releases/) or build from scratch using `cargo build --package cgt-py-adapter --release`.

See [`example.py`](example.py) file for example usage of Domineering and Snort games.

Note it it won't work with cloud sage providers like [sagecell](https://sagecell.sagemath.org/) or [cocalc](https://cocalc.com/features/sage) as you need local access to the `cgt-py-adapter`, thus you must use sage installed locally on your computer.
