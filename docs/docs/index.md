---
title: Cachebox
description: The fastest caching Python library written in Rust
---

<div align="center">
    <h1>Cachebox</h1>
  <em>The fastest caching Python library written in Rust</em>
</div>

---

Cachebox lets you perform powerful caching operations in Python as fast as possible.
It can make your application significantly faster and is an excellent choice for complex,
high-scale applications.

## Key Features

<div class="grid cards" markdown>

- :rocket: **Extremely Fast**

    10–50x faster than other caching libraries - [see benchmarks](https://github.com/awolverp/cachebox-benchmark).

- :bar_chart: **Low Memory Usage**

    Only ~50% of the memory consumed by a standard Python dictionary.

- :thread: **Thread-Safe**

    All cache operations are fully thread-safe via internal locking.

- :package: **Zero Dependencies**

    Written entirely in Rust - no Python dependencies to install.

- :fire: **Full-Featured**

    7 caching algorithms, TTL support, decorators, callbacks, and more.

- :handshake: **Compatible**

    Works with Python 3.10+ on both CPython and PyPy.

</div>

## When Should I Use Caching?
- **Frequent Data Access**:  If you need to access the same data multiple times, caching can help reduce the number of database queries or API calls, improving performance.

- **Expensive Operations**:  If you have operations that are computationally expensive, caching can help reduce the number of times these operations need to be performed.

- **High Traffic Scenarios**:  If your application handles high traffic, caching can help reduce the load on your server by reducing the number of requests that need to be processed.

- **Web Page Rendering**:  If you are rendering web pages, caching can help reduce the time it takes to generate the page by caching the results of expensive rendering operations. Caching HTML pages can speed up the delivery of static content.

- **Rate Limiting**:  If you have a rate limiting system in place, caching can help reduce the number of requests that need to be processed by the rate limiter. Also, caching can help you to manage rate limits imposed by third-party APIs by reducing the number of requests sent.

- **Machine Learning Models**:  If your application frequently makes predictions using the same input data, caching the results can save computation time.


## Quick Example

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(maxsize=128))
def get_user(user_id: int) -> dict:
    # Expensive DB call - cached after first call
    return db.query("SELECT * FROM users WHERE id = ?", user_id)

# First call hits the database
user = get_user(42)

# Subsequent calls are served from cache instantly
user = get_user(42)
```
