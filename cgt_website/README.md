# `cgt_website`

The [cgt.tools](https://cgt.tools) website. Leptos renders every page to static HTML at build time and hydrates only the interactive parts, as islands. The look is adapted from the [Doks](https://github.com/thuliteio/doks) theme, and the [Jost](https://github.com/indestructible-type/Jost) font is under the [SIL Open Font License](static/fonts/OFL.txt).

```console
$ make site
$ python3 -m http.server -d target/website
```

`make site` runs the `ssr` binary, which writes the pages and the files from `static/` to `target/website`, and then builds the islands and the web worker of the AI players ([`cgt_ai_web_worker`](../cgt_ai_web_worker)) with `wasm-pack` into `target/website/pkg`. Pages refer to assets by absolute paths, so serve the directory rather than opening the files directly.

## Configuration

What the site holds besides its own pages comes from a TOML file, [`website.toml`](website.toml) by default, and `make site SITE_CONFIG=<file>` builds from another one. Paths in it are relative to the file.

```toml
guides = ["guides/python-installation.md", "guides/custom-widgets.ipynb"]

[play.quelhas]
strong = { url = "https://example.com/quelhas-64x6.bin" }
quick = { path = "../checkpoints/quick/quelhas.bin" }
```

`guides` lists the guides in the order the site shows them. A Markdown guide is shown as written, and a notebook is first run in Jupyter to fill in its outputs.

`[play.quelhas]` names the models that the Quelhas page offers to play against, in the order of its menu, the first being the default. A model is a file written by `cgt-ai-train export` (see [`cgt_ai_train`](../cgt_ai_train)). The site serves a `path` model itself, while the page loads a `url` model straight from its address, which only works if that server sends an `Access-Control-Allow-Origin` header. Without models, the site has no Play section.
