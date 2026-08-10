---
title: Cachebox
description: The fastest caching Python library written in Rust
---

<div align="center">
    <h1>Cachebox</h1>
  <em>The fastest caching Python library written in Rust</em>
</div>

---

Cachebox is a high-performance, in-memory caching library for Python. It is written in Rust,
has zero Python dependencies, and exposes a familiar dict-like API so you can drop it into
existing code with minimal friction.

## Key Features

<div class="grid cards" markdown>

- :rocket: **Extremely Fast**

    10–50× faster than other caching libraries — [see benchmarks](https://github.com/awolverp/cachebox-benchmark).

- :bar_chart: **Low Memory Usage**

    Roughly half the memory of a standard Python dictionary for equivalent contents.

- :thread: **Thread-Safe**

    All cache operations are protected by internal Rust mutexes.

- :package: **Zero Dependencies**

    Distributed as pre-built wheels — no Rust toolchain required at install time.

- :fire: **Full-Featured**

    Seven eviction policies, TTL support, `@cached` decorator, callbacks, and more.

- :handshake: **Compatible**

    Python 3.10+ on CPython and PyPy.

</div>

## When Should I Use Caching?

- **Frequent data access** — avoid repeated database queries or API calls for the same keys.
- **Expensive operations** — memoize pure, costly computations so they run only once per input.
- **High traffic** — absorb load spikes by serving hot data from memory.
- **Web page rendering** — cache fragments or full pages that are expensive to generate.
- **Rate limiting** — track counters and windows, or reduce calls to third-party APIs.
- **Machine learning** — cache predictions for repeated inputs to save inference time.

## Quick Example

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(maxsize=128))
def get_user(user_id: int) -> dict:
    # Expensive DB call — cached after the first call
    return db.query("SELECT * FROM users WHERE id = ?", user_id)

# First call hits the database
user = get_user(42)

# Subsequent calls with the same arguments are served from cache
user = get_user(42)
```

Use a cache class directly when you need full control over keys and lifetime:

```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"
assert cache.get("missing", "default") == "default"
```

## What's Next?

| Page | Description |
|------|-------------|
| [Installation](installation.md) | Install from PyPI with pip or uv |
| [Getting Started](getting-started.md) | Decorators, key makers, methods, and common patterns |
| [Choosing a Cache](algorithms.md) | Which algorithm to pick for your workload |
| [Tips & Notes](tips.md) | Pickling, copying, TTL sweepers, stampede prevention |
| [FAQ](faq.md) | Common questions and edge cases |
| [API Reference](api/index.md) | Full class and function documentation |
| [Migration Guide](migration.md) | Breaking changes between major versions |
