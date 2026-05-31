**cachebox** is available on PyPI. You can use *pip* or *uv* to install cachebox.
You can install MarkupEver using **pip**:

=== "Using pip"

    ```console
    $ pip install -U cachebox
    ```

=== "Using uv"

    ```console
    $ uv add cachebox
    ```

That's it - cachebox has **zero Python dependencies**. The Rust extension is distributed as a
pre-built wheel for all major platforms and Python versions.

!!! tip "Use Virtual Environments"

    It's recommended to use virtual environments for installing and managing libraries in Python.

!!! warning "Upgrading from v5 to v6"
    Version 6 introduces several breaking changes. Please review the
    [Migration Guide](migration.md) before upgrading.

## Verifying the Installation

```python
import cachebox
print(cachebox.__version__)
```
