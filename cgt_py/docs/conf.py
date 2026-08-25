import sys
import tomllib
from pathlib import Path

DOCS = Path(__file__).parent

# `pyo3_stub_gen_ext` and the API data it renders are written here by `cgt_py_stub_gen`
sys.path.insert(0, str(DOCS / "api"))

project = "cgt-tools"
author = "Tomasz Maciosowski"
release = tomllib.loads((DOCS.parent / "Cargo.toml").read_text())["package"]["version"]
version = release

extensions = [
    "pyo3_stub_gen_ext",
    "myst_parser",
    "sphinx.ext.intersphinx",
]

intersphinx_mapping = {"python": ("https://docs.python.org/3", None)}

html_theme = "furo"
html_title = f"cgt-tools {release}"
