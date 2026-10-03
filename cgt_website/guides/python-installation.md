# Installing cgt-py

`cgt-py` brings `cgt-tools` to Python.
In this guide we install it with `pip`, set up a virtual environment on systems such as Debian that do not let `pip` install into the system's Python, and add Jupyter, which the interactive widgets run in.

## Installing with pip

`cgt-py` needs Python 3.10 or newer.
On Linux and Windows, `pip` installs a ready-built package:

```console
$ pip install cgt-py
```

On other systems, macOS among them, there is no ready-built package, and `pip` compiles `cgt-py` from its source code.
That needs a Rust compiler, which [rustup](https://rustup.rs) installs.

We can check the installation by computing the temperature of a game:

```console
$ python3 -c "import cgt_py; print(cgt_py.CanonicalForm('{1|-1}').temperature)"
DyadicRationalNumber('1')
```

## Debian and Ubuntu

Debian 12 and later, and Ubuntu 23.04 and later, leave the Python that comes with the system to their own package manager.
There, `pip install` refuses to install anything and stops with

```console
error: externally-managed-environment
```

Instead, we install `cgt-py` into a virtual environment, which is a separate Python installation in a directory of our choice.
Creating one needs the `python3-venv` package:

```console
$ sudo apt install python3-venv
# Create the environment, once
$ python3 -m venv ~/.venvs/cgt
# Activate it, in every new terminal
$ source ~/.venvs/cgt/bin/activate
(cgt) $ pip install cgt-py
```

While the environment is active, `python` and `pip` refer to it, so the commands in the other guides work unchanged.

## Jupyter

The widgets run in Jupyter, which we install next to `cgt-py`, in the same environment, and start from there:

```console
$ pip install notebook anywidget
$ jupyter notebook
```

## Hosted Notebooks

Inside a running notebook a cell with

```console
%pip install cgt-py
```

installs into the environment of the notebook's kernel, which is useful in notebooks hosted online (Google Colab, CoCalc, etc.), where there is no terminal.
