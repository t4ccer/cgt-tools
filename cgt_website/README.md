# `cgt_website`

The [cgt.tools](https://cgt.tools) website. Leptos renders every page to static HTML at build time and hydrates only the interactive parts, as islands. The look is adapted from the [Doks](https://github.com/thuliteio/doks) theme, and the [Jost](https://github.com/indestructible-type/Jost) font is under the [SIL Open Font License](static/fonts/OFL.txt).

```console
$ make site
$ python3 -m http.server -d target/website
```

`make site` runs the `ssr` binary, which writes the pages and the files from `static/` to `target/website`, and then builds the islands with `wasm-pack` into `target/website/pkg`. Pages refer to assets by absolute paths, so serve the directory rather than opening the files directly.
