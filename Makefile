PIP = .venv/bin/pip
PYTHON = .venv/bin/python
WHEELS = target/wheels/venv

.venv:
	python3 -m venv .venv
	$(PIP) install ipykernel

.PHONY: py
py: .venv
	wasm-pack build ./cgt_py_widgets --target web --out-dir pkg
	env -C ./cgt_py_widgets webpack
	# env -C ./cgt_py maturin develop
	rm -rf $(WHEELS)
	env -C ./cgt_py maturin build --interpreter ../$(PYTHON) --out ../$(WHEELS)
	$(PIP) install --force-reinstall $(WHEELS)/*.whl
