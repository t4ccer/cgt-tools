# `cgt_py`

`cgt-py` brings `cgt` to Python, with widgets that show and edit positions in Jupyter.
It is meant for exploring games in notebooks rather than for exhaustive searches of large search spaces, which are better done with the Rust library or [`cgt-cli`](https://github.com/t4ccer/cgt-tools/tree/main/cgt_cli).

```console
$ pip install cgt-py
```

The [installation guide](https://cgt.tools/guides/python-installation/) sets up a virtual environment and Jupyter, and the [API reference](https://cgt.tools/python/docs/latest/) lists everything the package has.

## Building from source

The widgets are written in Rust as well.
Their frontend, [`cgt_py_widgets`](https://github.com/t4ccer/cgt-tools/tree/main/cgt_py_widgets), is compiled to WebAssembly and bundled into `widget/bundle.js`, which the package embeds, and [`cgt_py_widgets_core`](https://github.com/t4ccer/cgt-tools/tree/main/cgt_py_widgets_core) has the types that the two ends exchange.
Building it needs `wasm-pack`, `webpack` and `maturin`, which `nix develop` provides.
From the root of [the repository](https://github.com/t4ccer/cgt-tools),

```console
$ make py
$ make notebook
```

`make py` builds the bundle, the type stub `cgt_py.pyi` and the wheel, and installs the wheel into `.venv`.
`make notebook` starts Jupyter from `.venv`.
