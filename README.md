<div align="center">

# Cachebox

_The fastest caching Python library written in Rust_

[![badge](https://shieldcn.dev/badge/Documentation-ff6e42.svg?logo=materialformkdocs)](https://awolverp.github.io/cachebox/) [![badge](https://shieldcn.dev/badge/Benchmarks-ff6e42.svg?logo=ri%3ATbBrandSpeedtest)](https://github.com/awolverp/cachebox-benchmark)
[![Buy Me A Coffee](https://shieldcn.dev/badge/Buy%20Me%20A%20Coffee-ff6e42.svg?logo=lu%3ACoffee)](https://payrequest.me/ali-pooralijan-awolverp)

[![PyPi](https://shieldcn.dev/pypi/cachebox.svg?variant=branded&font=geist-mono&size=xs)](https://pypi.org/project/cachebox)
[![Monthly Downloads](https://shieldcn.dev/pypi/dm/cachebox.svg?variant=branded&font=geist-mono&size=xs)](https://pypi.org/project/cachebox)
[![Python Version](https://shieldcn.dev/pypi/python/cachebox.svg?variant=branded&font=geist-mono&size=xs)](https://pypi.org/project/cachebox)

[![CI](https://shieldcn.dev/github/ci/awolverp/cachebox.svg?variant=outline&font=geist-mono&size=xs&animate=pulse)](https://github.com/awolverp/cachebox/actions?query=branch%3Amain)
[![Last Commit](https://shieldcn.dev/github/last-commit/awolverp/cachebox.svg?variant=outline&font=geist-mono&size=xs)](https://github.com/awolverp/cachebox/commits/main)
[![License](https://shieldcn.dev/github/awolverp/cachebox/license.svg?variant=outline&font=geist-mono&size=xs)](https://github.com/awolverp/cachebox/blob/main/LICENSE)

</div>

---

> [!WARNING]\
> The new version v6 has incompatibilities with v5. For more info see [Migration Guide](https://awolverp.github.io/cachebox/migration).

### What does it do?

You can easily perform powerful caching operations in Python as fast as possible.
This can make your application a lot faster and it can be a good choice in complex applications.
**Ideal for optimizing large-scale applications** with efficient, low-overhead caching.

**Key Features:**

- 🚀 Extremely fast (10-50x faster than other caching libraries - [_benchmarks_](https://github.com/awolverp/cachebox-benchmark))
- 📊 Minimal memory footprint
- 🔥 Full-featured and user-friendly
- 🧶 Completely thread-safe
- 🔧 Tested and correct
- **\[R\]** written in Rust for maximum performance
- 🤝 Compatible with Python 3.10+ (PyPy and CPython)
- 📦 Supports 7 advanced caching algorithms

### When do I need caching?

- 📈 **Frequent Data Access** \
  If you need to access the same data multiple times, caching can help reduce the number of database queries or API calls, improving performance.

- 💎 **Expensive Operations** \
  If you have operations that are computationally expensive, caching can help reduce the number of times these operations need to be performed.

- 🚗 **High Traffic Scenarios** \
  If your application handles high traffic, caching can help reduce the load on your server by reducing the number of requests that need to be processed.

- #️⃣ **Web Page Rendering** \
  If you are rendering web pages, caching can help reduce the time it takes to generate the page by caching the results of expensive rendering operations. Caching HTML pages can speed up the delivery of static content.

- 🚧 **Rate Limiting** \
  If you have a rate limiting system in place, caching can help reduce the number of requests that need to be processed by the rate limiter. Also, caching can help you to manage rate limits imposed by third-party APIs by reducing the number of requests sent.

- 🤖 **Machine Learning Models** \
  If your application frequently makes predictions using the same input data, caching the results can save computation time.

### Why `cachebox`?

- **⚡ Rust** \
  It uses the _Rust_ language for high-performance.

- **🧮 SwissTable** \
  It uses Google's high-performance SwissTable hash map. Thanks to [hashbrown](https://github.com/rust-lang/hashbrown).

- **✨ Low memory usage** \
  It has very low memory usage.

- **⭐ Zero Dependency** \
  As we said, `cachebox` is written in _Rust_ so you don't have to install any other dependencies.

- **🧶 Thread safe** \
  It's completely thread-safe and uses _Rust_ mutex to prevent problems.

- **👌 Easy To Use** \
  You only need to import it and choose a cache implementation to use.

- **🚫 Avoids Cache Stampede** \
  It avoids [cache stampede](https://en.wikipedia.org/wiki/Cache_stampede) by using a distributed lock system.

## Installation

cachebox is installable via `pip`:

```bash
pip3 install -U cachebox
```

## Examples

The simplest example of **cachebox** could look like this:

```python
import cachebox

@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, number + 1):
        fact *= num
    return fact

assert factorial(5) == 125

# coroutines are also supported
@cachebox.cached(cachebox.LRUCache(maxsize=128))
async def make_request(method: str, url: str) -> dict:
    response = await client.request(method, url)
    return response.json()
```

Unlike `functools.lru_cache` and other caching libraries, cachebox can copy `dict`, `list`, and `set` objects.

```python
@cachebox.cached(cachebox.LRUCache(maxsize=128))
def make_dict(name: str, age: int) -> dict:
   return {"name": name, "age": age}
>
d = make_dict("cachebox", 10)
assert d == {"name": "cachebox", "age": 10}
d["new-key"] = "new-value"

d2 = make_dict("cachebox", 10)
# `d2` will be `{"name": "cachebox", "age": 10, "new-key": "new-value"}` if you use other libraries
assert d2 == {"name": "cachebox", "age": 10}
```

You can use cache algorithms without the `cached` decorator -- just import the cache algorithm you want and use it like a dictionary.

```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"

# You can also use `cache.get(key, default)`
assert cache.get("key") == "value"
```

## Learn more

Read the documentation for full information and learn more: [**Documentation**](https://awolverp.github.io/cachebox/)

## Contributors

[![Contributers](https://contrib.rocks/image?repo=awolverp/cachebox)](https://github.com/awolverp/cachebox/graphs/contributors)

## License

This repository is licensed under the [MIT License](LICENSE)
