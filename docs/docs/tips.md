# Tips & Notes

## Saving a Cache to a File

Cachebox does not include built-in persistence, but all cache classes support Python's
`pickle` module:

```python
import cachebox
import pickle

cache = cachebox.LRUCache(100, {i: i for i in range(78)})

with open("cache.pkl", "wb") as f:
    pickle.dump(cache, f)

with open("cache.pkl", "rb") as f:
    loaded = pickle.load(f)

assert cache == loaded
assert cache.capacity() == loaded.capacity()
```

!!! note

    Do not set a `lambda` as `getsizeof` if you intend to pickle the cache. Use a module-level
    or otherwise picklable function instead.

## Copying a Cache

All cache classes support `copy.copy` and `copy.deepcopy`, and expose `.copy()` for a shallow copy:

```python
import cachebox
import copy

cache = cachebox.LRUCache(100, {i: i for i in range(10)})

shallow = copy.copy(cache)       # or cache.copy()
deep = copy.deepcopy(cache)
```

## Pre-allocating Capacity

If you know roughly how many items a cache will hold, set `capacity` to avoid hash-table
rehashing during the initial fill:

```python
cache = cachebox.LRUCache(maxsize=10_000, capacity=10_000)
```

This only reserves table slots; it does not change `maxsize` or eviction behavior.

## Weighted Caching with `getsizeof`

By default each entry contributes size `1` toward `maxsize`. Pass `getsizeof(key, value) -> int`
to size entries by memory, payload length, or any other weight:

```python
import cachebox
import sys

def memsize(key, value):
    return sys.getsizeof(key) + sys.getsizeof(value)

cache = cachebox.LRUCache(maxsize=1_000_000, getsizeof=memsize)
cache.insert("blob", b"x" * 10_000)

print(cache.current_size())    # weight of stored entries
print(cache.remaining_size())  # maxsize - current_size
print(len(cache))              # number of keys (not weight)
```

Eviction runs when inserting an entry would exceed `maxsize` (policy classes) or raises
`OverflowError` (`Cache`).

## TTL and Frozen Caches

`Frozen` blocks write APIs but **cannot** stop TTL expiration. Items still expire on the
underlying `TTLCache` / `VTTLCache`:

```python
from cachebox import Frozen, TTLCache
import time

cache = TTLCache(0, global_ttl=1, iterable={1: "a"})
frozen = Frozen(cache)
time.sleep(1)
print(len(frozen))  # 0 — expired despite being frozen
```

## Attributes Attached to Cached Functions

When you use `@cached` with a **cache instance** (not a lambda/callable), these attributes are
attached to the wrapper:

=== "`cache` (property)"

    The cache object used for storage.

    ```python
    import cachebox

    @cachebox.cached(cachebox.LFUCache(maxsize=20))
    def add(a: int, b: int) -> int:
        return a + b

    assert type(add.cache) is cachebox.LFUCache
    ```

    Prefer the typed helper to silence IDE warnings:

    ```python
    assert type(cachebox.get_cached_cache(add)) is cachebox.LFUCache
    ```

=== "`cache_info` (callable)"

    Returns a `CacheInfo` namedtuple:

    ```text
    CacheInfo(hits, misses, maxsize, current_size, length, memory)
    ```

    | Field | Meaning |
    |-------|---------|
    | `hits` | Number of cache hits since last clear |
    | `misses` | Number of cache misses since last clear |
    | `maxsize` | Cache `maxsize` |
    | `current_size` | Sum of entry weights (`current_size()`) |
    | `length` | Number of keys (`len(cache)`) |
    | `memory` | Approximate allocation size (`__sizeof__()`) |

    ```python
    import cachebox

    @cachebox.cached(cachebox.LFUCache(maxsize=20))
    def add(a: int, b: int) -> int:
        return a + b

    info = add.cache_info()
    # CacheInfo(hits=0, misses=0, maxsize=20, current_size=0, length=0, memory=...)
    ```

    ```python
    info = cachebox.get_cached_cache_info(add)
    ```

=== "`cache_clear` (callable)"

    Clears the cache and resets hit/miss counters. Respects `clear_reuse` from the decorator
    (`reuse=True` keeps the hash-table allocation).

    ```python
    add.cache_clear()
    # or
    cachebox.clear_cached_cache(add)
    ```

=== "`callback` (property)"

    The configured callback (or `None`).

    ```python
    def callback(event, key, value): ...

    @cachebox.cached(cachebox.LFUCache(20), callback=callback)
    def add(a, b):
        return a + b

    assert add.callback is callback
    assert cachebox.get_cached_callback(add) is callback
    ```

Detect wrappers with [`is_cached`](api/utils.md#cachebox.utils.is_cached):

```python
assert cachebox.is_cached(add)
assert not cachebox.is_cached(lambda x: x)
```

## TTLCache / VTTLCache Background Thread

By default both classes use **lazy expiry**: stale entries are cleaned when the cache is
touched (`insert`, lookup, iteration, `current_size`, …). An idle cache keeps expired entries
in memory until then.

Pass `sweep_interval` (≥ 1 second) to start a **daemon** background thread that calls
`expire()` on a fixed schedule:

```python
import cachebox
from datetime import timedelta

ttl_cache = cachebox.TTLCache(maxsize=1000, global_ttl=60, sweep_interval=30)

vttl_cache = cachebox.VTTLCache(
    maxsize=1000,
    sweep_interval=timedelta(seconds=30),
)
```

```python
cache = cachebox.TTLCache(100, global_ttl=60, sweep_interval=30)
print(cache.sweep_interval)  # 30.0

cache2 = cachebox.TTLCache(100, global_ttl=60)
print(cache2.sweep_interval)  # None
```

Stop the thread explicitly when you need a clean shutdown:

```python
cache = cachebox.TTLCache(100, global_ttl=60, sweep_interval=10)
# ... later ...
cache.stop_sweeper()
```

The sweeper also stops when the cache is garbage-collected (`__del__`).

!!! note

    Values below 1 second raise `ValueError`:

    ```python
    cachebox.TTLCache(100, global_ttl=5, sweep_interval=0.5)
    # ValueError: sweep_interval must be more than 1 seconds.
    ```

**Prefer a sweeper when:**

- The cache may sit idle for long periods but memory should still be reclaimed.
- You need a tighter bound on how long stale data can appear in `items()` / `__iter__`.
- `VTTLCache` holds short, mixed TTLs and you want predictable cleanup.

**Prefer lazy expiry when:**

- Traffic is regular and on-access cleanup is enough.
- You want zero background threads.
- Temporary retention of expired entries is acceptable.

You can also call `expire()` yourself at any time:

```python
cache.expire()
cache.expire(reuse=True)  # keep table allocation
```

## Cache Stampede Prevention

A **cache stampede** happens when many concurrent callers miss the same key and all recompute
the value. `@cached` prevents this by default with a **per-key lock**: one caller computes while
others wait and then reuse the result.

Default locks: `threading.Lock` (sync) or `asyncio.Lock` (async). Override with any type that
implements `AbstractContextManager` / `AbstractAsyncContextManager`:

=== "Sync (default)"

    ```python
    import cachebox

    @cachebox.cached(cachebox.LRUCache(maxsize=256))
    def fetch_user(user_id: int) -> dict:
        return expensive_db_query(user_id)
    ```

=== "Async (default)"

    ```python
    import cachebox

    @cachebox.cached(cachebox.LRUCache(maxsize=256))
    async def fetch_user(user_id: int) -> dict:
        return await expensive_db_query(user_id)
    ```

=== "Custom lock type"

    ```python
    import threading
    import cachebox

    @cachebox.cached(cachebox.LRUCache(maxsize=256), lock=threading.RLock)
    def fetch_user(user_id: int) -> dict:
        return expensive_db_query(user_id)
    ```

!!! warning

    Passing a sync lock to an async function (or vice versa) raises `TypeError` at decoration time.
    An async callback on a sync function is also rejected.

Disable locking with `lock=False` or `lock=None` when stampedes are impossible or cheap to tolerate:

```python
# Recursive functions deadlock under a non-reentrant Lock — disable or use RLock
@cachebox.cached(cachebox.LRUCache(256), lock=False)
def factorial(n: int) -> int:
    return 1 if n <= 1 else n * factorial(n - 1)
```

Other cases where disabling the lock is reasonable:

- **Cheap computations** — lock contention costs more than duplicate work.
- **Single-threaded** environments — no concurrency, pure overhead.
- **Already serialised callers** — e.g. a single-worker queue.

!!! note

    Disabling the lock does **not** make cache storage unsafe. Reads and writes remain protected
    by internal Rust mutexes. It only allows concurrent recomputation of the same missing key.

Errors raised while computing under a lock are propagated to waiters for the same key so they
fail consistently instead of each retrying independently.

## Mutable Return Values

The default postprocessor shallow-copies `dict`, `list`, and `set` on return so callers cannot
corrupt the cache by mutating the result. If you return other mutable containers (custom objects,
`bytearray`, nested structures you will mutate in place), set an explicit postprocessor:

```python
@cachebox.cached(cachebox.LRUCache(128), postprocess=cachebox.postprocess_deepcopy)
def load_config():
    return {"nested": {"flag": True}}
```

Or disable copying for maximum speed when values are immutable:

```python
@cachebox.cached(cachebox.LRUCache(128), postprocess=None)
def fib(n: int) -> int:
    ...
```

## Thread Safety of Cache Objects

All cache classes are safe for concurrent reads and writes from multiple threads. Individual
method calls are atomic with respect to the internal map. Compound sequences
(`if key not in cache: cache[key] = ...`) are **not** atomic — use `setdefault` /
`setdefault_with` or `@cached` for single-flight insertion patterns.

## Iteration Safety

`keys()`, `values()`, `items()`, and policy-specific iterators (`items_with_frequency`,
`items_with_expire`, …) return **one-shot** iterators, not live dict views:

```python
cache = cachebox.LRUCache(10, {i: i for i in range(5)})
it = cache.keys()
print(len(it))   # items left to yield
print(bool(it))  # True if anything left

for k in it:
    pass
# second pass is empty
```

Do not modify the cache while an iterator is alive — every method on that iterator raises
`RuntimeError` if the cache changed.