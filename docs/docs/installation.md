# Installation

**cachebox** is available on [PyPI](https://pypi.org/project/cachebox/). Install it with pip or uv.

=== "Using pip"

    ```console
    $ pip install -U cachebox
    ```

=== "Using uv"

    ```console
    $ uv add cachebox
    ```

That's it — cachebox has **zero Python dependencies**. The Rust extension is distributed as a
pre-built wheel for all major platforms and Python versions (CPython and PyPy).

!!! tip "Use virtual environments"

    Prefer a virtual environment when installing and managing Python packages.

!!! warning "Upgrading from v5 to v6"

    Version 6 introduces several breaking changes. Review the
    [Migration Guide](migration.md) before upgrading.

## Requirements

| Requirement | Details |
|-------------|---------|
| Python | 3.10 or newer |
| Implementations | CPython, PyPy |
| OS | Linux, macOS, Windows |
| Extra deps | None |

## Verifying the Installation

```python
import cachebox

print(cachebox.__version__)
```

If the import succeeds and a version string is printed, the wheel was installed correctly.

## Building from Source

Pre-built wheels cover most users. To build from source you need a Rust toolchain and
[maturin](https://www.maturin.rs/):

```console
$ git clone https://github.com/awolverp/cachebox.git
$ cd cachebox
$ pip install maturin
$ maturin develop --release
```
