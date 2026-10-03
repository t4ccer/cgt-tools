CLEAN_PY_ENV = env -u PYTHONPATH -u _PYTHON_SYSCONFIGDATA_NAME -u PYTHONHOME
PIP = $(CLEAN_PY_ENV) .venv/bin/pip
PYTHON = .venv/bin/python
VENV_CFG = .venv/pyvenv.cfg
WHEELS = target/wheels/venv
DEPS = target/make

VERSION := $(shell sed -n 's/^version = "\(.*\)"$$/\1/p' cgt_py/Cargo.toml | head -1)

WIDGETS_WASM = cgt_py_widgets/pkg/cgt_py_widgets_bg.wasm
BUNDLE = cgt_py/widget/bundle.js
WHEEL_STAMP = $(DEPS)/wheel.stamp
INSTALL_STAMP = $(DEPS)/install.stamp
STUB_STAMP = $(DEPS)/stub.stamp

SITE = target/website
API_JSON = cgt_py/docs/api/api_reference.json
API_SNAPSHOTS = cgt_website/python-api

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

$(VENV_CFG):
	python3 -m venv --system-site-packages .venv
	$(CLEAN_PY_ENV) $(PYTHON) -m ipykernel install --prefix .venv --name cgt --display-name "cgt (NixOS venv)"

.venv: $(VENV_CFG)

$(DEPS):
	mkdir -p $(DEPS)

.PHONY: clean
clean:
	git clean -Xdf

.PHONY: py
py: $(INSTALL_STAMP)

.PHONY: notebook
notebook: $(INSTALL_STAMP)
	$(CLEAN_PY_ENV) $(PYTHON) -m notebook

.PHONY: stub
stub: $(STUB_STAMP)

.PHONY: site
site: $(STUB_STAMP)
	cargo run --quiet --release -p cgt_website --features ssr -- $(SITE) unstable=$(API_JSON) \
	  $(foreach api,$(wildcard $(API_SNAPSHOTS)/*.json),$(basename $(notdir $(api)))=$(api))
	wasm-pack build ./cgt_website --target web --no-typescript --no-pack \
	  --out-dir $(abspath $(SITE))/pkg --out-name cgt_website -- --features hydrate
	# gh-pages is published with `git add`, which would skip everything this ignores
	rm -f $(SITE)/pkg/.gitignore

# Released versions are rendered from these snapshots, because the stub can only be generated for
# the checked out version
.PHONY: python-api
python-api: $(STUB_STAMP)
	cp $(API_JSON) $(API_SNAPSHOTS)/v$(VERSION).json

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
	$(CLEAN_PY_ENV) env -C ./cgt_py maturin build --interpreter ../$(PYTHON) --out ../$(WHEELS)
	$(call cargo-dep,$(CARGO_DEP_PY))
	touch $@

$(INSTALL_STAMP): $(WHEEL_STAMP) $(VENV_CFG) | $(DEPS)
	$(PIP) install --force-reinstall $(WHEELS)/*.whl
	touch $@

$(STUB_STAMP): $(BUNDLE) $(MANIFESTS) $(STUB_DEP) cgt_py/pyproject.toml | .venv $(DEPS)
	rm -rf cgt_py/docs/api
	$(CLEAN_PY_ENV) PYO3_PYTHON=$(abspath $(PYTHON)) cargo run --quiet -p cgt_py --bin cgt_py_stub_gen
	$(call cargo-dep,$(CARGO_DEP_STUB))
	touch $@

-include $(DEPS)/*.d
