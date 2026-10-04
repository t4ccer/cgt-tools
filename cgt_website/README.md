# `cgt_website`

The [cgt.tools](https://cgt.tools) website.
Leptos renders every page to static HTML at build time and hydrates only the interactive parts, as islands.
The look is adapted from the [Doks](https://github.com/thuliteio/doks) theme, and the [Jost](https://github.com/indestructible-type/Jost) font is under the [SIL Open Font License](static/fonts/OFL.txt).

```console
$ make site
$ python3 -m http.server -d target/website
```

`make site` runs the `ssr` binary, which writes the pages and the files from `static/` to `target/website`, and then builds the islands and the web worker of the AI players ([`cgt_ai_web_worker`](../cgt_ai_web_worker)) with `wasm-pack` into `target/website/pkg`.
Pages refer to assets by absolute paths, so serve the directory rather than opening the files directly.

The binary runs the notebook guides in Jupyter, with the `cgt-py` that `make site` builds and installs into `.venv` first, and screenshots their widgets in headless Chrome, which it looks for on `PATH` unless `CHROME` names it.

## Configuration

What the site holds besides its own pages comes from a TOML file, [`website.toml`](website.toml) by default, and `make site SITE_CONFIG=<file>` builds from another one.
Paths in it are relative to the file.

```toml
guides = ["guides/python-installation.md", "guides/custom-widgets.ipynb"]

[play.quelhas]
easy = { path = "./checkpoints/quelhas_easy.bin" }
strong = { url = "https://example.com/quelhas-64x6.bin" }

[play.fjords]
easy = { path = "./checkpoints/fjords_easy.bin" }
```

`guides` lists the guides in the order the site shows them.
A Markdown guide is shown as written, and a notebook is first run in Jupyter to fill in its outputs.

`[play.<game>]` names the models that the page of a game, `quelhas` or `fjords`, offers to play against, in the order of its menu, the first being the default.
A model is a file written by `cgt-ai-train export` (see [`cgt_ai_train`](../cgt_ai_train)).
The site serves a `path` model itself, while the page loads a `url` model straight from its address, which only works if that server sends an `Access-Control-Allow-Origin` header.
A game without models can still be played on the site, by two players.

## Python API reference

The Python API reference is rendered from the `api_reference.json` that `make stub` writes next to the type stub of [`cgt_py`](../cgt_py).
The checked out source is shown as `unstable`, and every released version from its snapshot in [`python-api`](python-api).
`make python-api` adds the snapshot of the current version, which `version-bump.sh` does on every release.
