.venv:
	python3 -m venv .venv

.PHONY: repl
repl: .venv
	maturin develop
	.venv/bin/python -i -c 'from cgt_py import *'
