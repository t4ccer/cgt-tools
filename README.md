# cgt-tools

Combinatorial Game Theory toolkit in Rust and Python.
[cgt.tools](https://cgt.tools) has guides, the Python API reference, and games to play against AI players.

## Crates

- [`cgt`](https://docs.rs/cgt/latest/cgt/), at the root of the repository, is the Rust library.
  It computes canonical forms and thermographs of short games, handles impartial, loopy and misère games, and evaluates and draws positions of games such as Domineering, Snort, Amazons and Konane.
- [`cgt_derive`](./cgt_derive) has the derive macros of `cgt`.
- [`cgt_cli`](./cgt_cli) is `cgt-cli`, a command line program that evaluates positions, runs exhaustive and genetic searches, and turns their results into LaTeX tables.
- [`cgt_py`](./cgt_py) is `cgt-py`, the Python bindings, with Jupyter widgets that show and edit positions.
- [`cgt_py_widgets`](./cgt_py_widgets) is the frontend of these widgets, compiled to WebAssembly.
- [`cgt_py_widgets_core`](./cgt_py_widgets_core) has the types that the bindings and the widget frontend exchange.
- [`cgt_ai_core`](./cgt_ai_core) has the rules of Quelhas and Fjords and the Monte Carlo tree search of AlphaZero-style players.
- [`cgt_ai_model`](./cgt_ai_model) has the policy and value networks of these players, written with [Burn](https://burn.dev).
- [`cgt_ai_train`](./cgt_ai_train) trains the networks by self-play and exports them for the website.
- [`cgt_ai_web_worker`](./cgt_ai_web_worker) plays with an exported network in the browser.
- [`cgt_website`](./cgt_website) builds [cgt.tools](https://cgt.tools).

`nix develop` provides the tools that all crates need to build.

## Credits

- Library is heavily inspired by https://github.com/aaron-siegel/cgsuite
- Library in Haskell that provides basic functions to work with combinatorial games https://github.com/kamekura/haskell-cgt

## Citing

If you found the toolkit useful for your work please consider citing it.

### BibTeX

```bib
@misc{maciosowskiCgttools,
  title = {cgt-tools},
  author = {Maciosowski, Tomasz},
  howpublished = {https://cgt.tools},
  abstract = {Combinatorial Game Theory toolkit},
  copyright = {AGPL-3.0}
}
```

### BibLaTeX

```bib
@software{maciosowskiCgttools,
  title = {cgt-tools},
  author = {Maciosowski, Tomasz},
  url = {https://cgt.tools},
  abstract = {Combinatorial Game Theory toolkit}
}
```

## License

Copyright (C) 2023-2026 Tomasz Maciosowski (t4ccer)

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program.
If not, see <https://www.gnu.org/licenses/>.
