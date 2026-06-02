<div align="center">

# Cachebox

*The fastest caching Python library written in Rust*

[**Documentation**](https://awolverp.github.io/cachebox) | [**Releases**](https://github.com/awolverp/cachebox/releases) | 
[**Benchmarks**](https://github.com/awolverp/cachebox-benchmark) | 
[**Issues**](https://github.com/awolverp/cachebox/issues/new)

[![License](https://img.shields.io/github/license/awolverp/cachebox.svg?style=flat-square)](https://github.com/awolverp/cachebox/blob/main/LICENSE)
[![Downloads](https://img.shields.io/pypi/dm/cachebox?style=flat-square&color=%23314bb5)](https://pepy.tech/projects/cachebox)

</div>

-------

> [!WARNING]\
> The new version v6 has incompatibilities with v5. For more info see [Migration Guide](https://awolverp.github.io/cachebox/migration).

### What does it do?
You can easily perform powerful caching operations in Python as fast as possible.
This can make your application a lot faster and it can be a good choice in complex applications.
**Ideal for optimizing large-scale applications** with efficient, low-overhead caching.

**Key Features:**
- 🚀 Extremely fast (10-50x faster than other caching libraries - [*benchmarks*](https://github.com/awolverp/cachebox-benchmark))
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
It uses the *Rust* language for high-performance.

- **🧮 SwissTable** \
It uses Google's high-performance SwissTable hash map. Thanks to [hashbrown](https://github.com/rust-lang/hashbrown).

- **✨ Low memory usage** \
It has very low memory usage.

- **⭐ Zero Dependency** \
As we said, `cachebox` is written in *Rust* so you don't have to install any other dependecies.

- **🧶 Thread safe** \
It's completely thread-safe and uses *Rust* mutex to prevent problems.

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

You can use cache alghoritms without the `cached` decorator -- just import the cache alghoritm you want and use it like a dictionary.
```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"

# You can also use `cache.get(key, default)`
assert cache.get("key") == "value"
```

## Learn more
Read the documentation for full information and learn more: [**Documentation**](https://awolverp.github.com/cachebox)

## License
This repository is licensed under the [MIT License](LICENSE)
