PIP = .venv/bin/pip
PYTHON = .venv/bin/python
WHEELS = target/wheels/venv
DEPS = target/make

VERSION := $(shell sed -n 's/^version = "\(.*\)"$$/\1/p' cgt_py/Cargo.toml | head -1)

WIDGETS_WASM = cgt_py_widgets/pkg/cgt_py_widgets_bg.wasm
BUNDLE = cgt_py/widget/bundle.js
WHEEL_STAMP = $(DEPS)/wheel.stamp
INSTALL_STAMP = $(DEPS)/install.stamp
STUB_STAMP = $(DEPS)/stub.stamp

SITE = target/website
DOCS_ROOT = $(SITE)/python/docs
DOCS_NAME ?= v$(VERSION)
DOCS_OUT = $(DOCS_ROOT)/$(DOCS_NAME)

CARGO_DEP_WIDGETS = target/wasm32-unknown-unknown/release/cgt_py_widgets.d
CARGO_DEP_PY = target/debug/libcgt_py.d
CARGO_DEP_STUB = target/debug/cgt_py_stub_gen.d
WIDGETS_DEP = $(DEPS)/$(notdir $(WIDGETS_WASM)).d
PY_DEP = $(DEPS)/$(notdir $(WHEEL_STAMP)).d
STUB_DEP = $(DEPS)/$(notdir $(STUB_STAMP)).d

MANIFESTS = Cargo.toml Cargo.lock $(wildcard */Cargo.toml)

define cargo-dep
sed 's|^[^:]*:|$@:|' $(1) > $(DEPS)/$(@F).d; \
tr ' ' '\n' < $(1) | sed -e '1d' -e '/^$$/d' -e 's|$$|:|' >> $(DEPS)/$(@F).d
endef

.venv:
	python3 -m venv .venv
	$(PIP) install ipykernel

$(DEPS):
	mkdir -p $(DEPS)

.PHONY: clean
clean:
	git clean -Xdf

.PHONY: py
py: $(INSTALL_STAMP)

.PHONY: stub
stub: $(STUB_STAMP)

.PHONY: site
site:
	mkdir -p $(SITE)
	cp -r website/. $(SITE)/

.PHONY: docs
docs: $(DOCS_OUT)/index.html

.PHONY: docs-latest
docs-latest: | site
	mkdir -p $(DOCS_ROOT)/latest
	printf '<meta http-equiv="refresh" content="0; url=../%s/">\n' '$(DOCS_NAME)' \
	  > $(DOCS_ROOT)/latest/index.html

$(WIDGETS_DEP) $(PY_DEP) $(STUB_DEP): ;

$(WIDGETS_WASM): $(MANIFESTS) $(WIDGETS_DEP) | $(DEPS)
	wasm-pack build ./cgt_py_widgets --target web --out-dir pkg
	$(call cargo-dep,$(CARGO_DEP_WIDGETS))
	touch $@

$(BUNDLE): $(WIDGETS_WASM) cgt_py_widgets/index.js cgt_py_widgets/webpack.config.js
	env -C ./cgt_py_widgets webpack
	touch $@

$(WHEEL_STAMP): $(BUNDLE) $(STUB_STAMP) $(MANIFESTS) $(PY_DEP) cgt_py/pyproject.toml | .venv $(DEPS)
	rm -rf $(WHEELS)
	env -C ./cgt_py maturin build --interpreter ../$(PYTHON) --out ../$(WHEELS)
	$(call cargo-dep,$(CARGO_DEP_PY))
	touch $@

$(INSTALL_STAMP): $(WHEEL_STAMP) | .venv $(DEPS)
	$(PIP) install --force-reinstall $(WHEELS)/*.whl
	touch $@

$(STUB_STAMP): $(BUNDLE) $(MANIFESTS) $(STUB_DEP) cgt_py/pyproject.toml | .venv $(DEPS)
	rm -rf cgt_py/docs/api
	PYO3_PYTHON=$(abspath $(PYTHON)) cargo run --quiet -p cgt_py --bin cgt_py_stub_gen
	$(call cargo-dep,$(CARGO_DEP_STUB))
	touch $@

$(DOCS_OUT)/index.html: $(STUB_STAMP) cgt_py/docs/conf.py cgt_py/docs/index.rst | site
	rm -rf $(DOCS_OUT)
	sphinx-build --builder html --doctree-dir $(DEPS)/doctrees/$(DOCS_NAME) \
	  cgt_py/docs $(DOCS_OUT)

-include $(DEPS)/*.d
