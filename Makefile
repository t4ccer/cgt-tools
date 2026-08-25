PIP = .venv/bin/pip
PYTHON = .venv/bin/python
WHEELS = target/wheels/venv
DEPS = target/make

WIDGETS_WASM = cgt_py_widgets/pkg/cgt_py_widgets_bg.wasm
BUNDLE = cgt_py_widgets/dist/bundle.js
WHEEL_STAMP = $(DEPS)/wheel.stamp
INSTALL_STAMP = $(DEPS)/install.stamp

CARGO_DEP_WIDGETS = target/wasm32-unknown-unknown/release/cgt_py_widgets.d
CARGO_DEP_PY = target/debug/libcgt_py.d
WIDGETS_DEP = $(DEPS)/$(notdir $(WIDGETS_WASM)).d
PY_DEP = $(DEPS)/$(notdir $(WHEEL_STAMP)).d

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

.PHONY: py
py: $(INSTALL_STAMP)

$(WIDGETS_DEP) $(PY_DEP): ;

$(WIDGETS_WASM): $(MANIFESTS) $(WIDGETS_DEP) | $(DEPS)
	wasm-pack build ./cgt_py_widgets --target web --out-dir pkg
	$(call cargo-dep,$(CARGO_DEP_WIDGETS))
	touch $@

$(BUNDLE): $(WIDGETS_WASM) cgt_py_widgets/index.js cgt_py_widgets/webpack.config.js
	env -C ./cgt_py_widgets webpack
	touch $@

$(WHEEL_STAMP): $(BUNDLE) $(MANIFESTS) $(PY_DEP) cgt_py/pyproject.toml | .venv $(DEPS)
	rm -rf $(WHEELS)
	env -C ./cgt_py maturin build --interpreter ../$(PYTHON) --out ../$(WHEELS)
	$(call cargo-dep,$(CARGO_DEP_PY))
	touch $@

$(INSTALL_STAMP): $(WHEEL_STAMP) | .venv $(DEPS)
	$(PIP) install --force-reinstall $(WHEELS)/*.whl
	touch $@

-include $(DEPS)/*.d
