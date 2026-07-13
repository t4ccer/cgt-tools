PIP = .venv/bin/pip
CGT_VERSION = 0.10.0

.venv:
	python3 -m venv .venv
	$(PIP) install ipykernel

.PHONY: py
py: .venv
	wasm-pack build ./cgt_py_widgets --target web --out-dir pkg
	env -C ./cgt_py_widgets webpack
	# env -C ./cgt_py maturin develop
	env -C ./cgt_py maturin build
	$(PIP) install --force-reinstall ./target/wheels/cgt_py-$(CGT_VERSION)-cp313-cp313-linux_x86_64.whl
