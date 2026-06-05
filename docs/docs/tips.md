# Tips & Notes

## Saving a Cache to a File

Cachebox does not include built-in persistence, but all cache classes support Python's
`pickle` module:

```python
import cachebox, pickle

cache = cachebox.LRUCache(100, {i: i for i in range(78)})

# Save
with open("cache.pkl", "wb") as f:
    pickle.dump(cache, f)

# Load
with open("cache.pkl", "rb") as f:
    loaded = pickle.load(f)

assert cache == loaded
assert cache.capacity() == loaded.capacity()
```

!!! note

    Don't set `lambda` as `getsizeof` for caches when you want to pickle them.

## Copying a Cache
All cache classes support Python's `copy` module, both shallow-copy and deep-copy:

```python
import cachebox
import copy

cache = cachebox.LRUCache(100, {i: i for i in range(10)})

shallow = copy.copy(cache)       # shallow copy
deep    = copy.deepcopy(cache)   # deep copy
```

## Pre-allocating Capacity
If you know roughly how many items a cache will hold, set `capacity` to avoid
hash table rehashing during initial population:

```python
cache = cachebox.LRUCache(maxsize=10_000, capacity=10_000)
```

## TTL and Frozen Caches
`Frozen` cannot prevent TTL expiration in `TTLCache` or `VTTLCache`.
Items will still expire naturally even when the cache is frozen.

```python
from cachebox import Frozen, TTLCache
import time

cache  = TTLCache(0, ttl=1, iterable={1: "a"})
frozen = Frozen(cache)
time.sleep(1)
print(len(frozen))  # 0 — expired despite being frozen
```

## Attached attributes to cached functions
When you use the `@cached` decorator, If *cache* isn't a lambda/function, these attributes will be attached to
your function:

=== "`cache` (property)"

    The cache class we're using for caching results.

    ```python hl_lines="9"
    import cachebox

    @cachebox.cached(
        cachebox.LFUCache(maxsize=20),
    )
    def add(a: int, b: int) -> int:
        return a + b

    assert type(add.cache) is cachebox.LFUCache
    ```

    !!! tip
        You can use [get_cached_cache function](api/utils.md#cachebox.utils.get_cached_cache) to prevent lint
        & IDE warnings.

        ```python
        assert type(cachebox.get_cached_cache(add)) is cachebox.LFUCache
        ```

=== "`cache_info` (callable)"

    By calling it, you will get a basic statistics.

    ```python hl_lines="9"
    import cachebox

    @cachebox.cached(
        cachebox.LFUCache(maxsize=20),
    )
    def add(a: int, b: int) -> int:
        return a + b

    cache_info = add.cache_info() # CacheInfo(hits=0, misses=0, maxsize=20, size=0)
    ```

    !!! tip
        You can use [get_cached_cache_info function](api/utils.md#cachebox.utils.get_cached_cache_info) to prevent lint
        & IDE warnings.

        ```python
        cache_info = cachebox.get_cached_cache_info(add) # CacheInfo(hits=0, misses=0, maxsize=20, size=0)
        ```

=== "`cache_clear` (callable)"

    Call it if you want to clear cache and reset statistics.

    ```python hl_lines="9"
    import cachebox

    @cachebox.cached(
        cachebox.LFUCache(maxsize=20),
    )
    def add(a: int, b: int) -> int:
        return a + b

    add.cache_clear()
    ```

    !!! tip
        You can use [clear_cached_cache function](api/utils.md#cachebox.utils.clear_cached_cache) to prevent lint
        & IDE warnings.

        ```python
        cachebox.clear_cached_cache(add)
        ```

=== "`callback` (property)"

    The configured `callback`.

    ```python hl_lines="12"
    import cachebox

    def callback(event, key, value): ...

    @cachebox.cached(
        cachebox.LFUCache(maxsize=20),
        callback=callback,
    )
    def add(a: int, b: int) -> int:
        return a + b

    assert add.callback is callback
    ```

    !!! tip
        You can use [get_cached_callback function](api/utils.md#cachebox.utils.get_cached_callback) to prevent lint
        & IDE warnings.

        ```python
        assert cachebox.get_cached_callback(add) is callback
        ```


## TTLCache/VTTLCache background thread
By default, both `TTLCache` and `VTTLCache` use **lazy expiry**: stale entries are
only cleaned up when the cache is interacted with (e.g. on insert, lookup, or
iteration). A completely idle cache will hold expired entries in memory until
the next interaction.

To reclaim expired entries proactively — independent of any method calls — pass a
`sweep_interval` to start a background sweeper thread:

```python
import cachebox
from datetime import timedelta

# Sweep every 30 seconds
ttl_cache = cachebox.TTLCache(maxsize=1000, global_ttl=60, sweep_interval=30)

# timedelta is also accepted
vttl_cache = cachebox.VTTLCache(maxsize=1000, sweep_interval=timedelta(seconds=30))
```

The thread is a **daemon thread**, meaning it will not prevent the Python process
from exiting when the main thread finishes.

!!! note

    `sweep_interval` must be **≥ 1 second**. Smaller values raise a `ValueError`:

    ```python
    cachebox.TTLCache(100, global_ttl=5, sweep_interval=0.5)
    # ValueError: sweep_interval must be more than 1 seconds.
    ```

```python
cache = cachebox.TTLCache(100, global_ttl=60, sweep_interval=30)
print(cache.sweep_interval)  # 30.0

# Without a sweeper, sweep_interval is None
cache2 = cachebox.TTLCache(100, global_ttl=60)
print(cache2.sweep_interval)  # None
```

Call `stop_sweeper()` when you want to halt background sweeping without
destroying the cache itself. This is useful when you need to pause periodic
eviction or cleanly shut down the thread before the cache goes out of scope:

```python
cache = cachebox.TTLCache(100, global_ttl=60, sweep_interval=10)

# ... later, during shutdown ...
cache.stop_sweeper()
```

!!! note

    The sweeper thread is also stopped automatically when the cache is garbage
    collected (via `__del__`), so manual cleanup is only necessary when explicit
    lifecycle control is required.

Use a **sweeper** when:
- The cache may be idle for long periods but memory should still be reclaimed.
- You need to bound the window in which stale data could be observed (e.g. via `items()` or `__iter__`).
- You are using `VTTLCache` with short, heterogeneous TTLs and want predictable cleanup.

Stick with **lazy expiry** when:
- The cache sees regular traffic and on-access cleanup is sufficient.
- You want to avoid any background thread overhead.
- Memory pressure from temporarily lingering stale entries is acceptable.

## Cache Stampede Prevention
A cache stampede occurs when many concurrent requests find the same key missing from the cache
and all proceed to recompute the value simultaneously that causing redundant work, resource spikes,
or even cascading failures under heavy load. The `@cached` decorator prevents this by default
using a per-key lock: once one caller begins computing a missing value, all other callers for the
same key wait for it to finish and then reuse the result.

Lock-based stampede prevention is enabled by default (`lock=True`). For sync
functions this uses `threading.Lock`; for async functions it uses `asyncio.Lock`:

=== "Sync"

    ```python
    import cachebox
    
    @cachebox.cached(cachebox.LRUCache(maxsize=256))
    def fetch_user(user_id: int) -> dict:
        # Only called once per user_id, even under concurrent load
        return expensive_db_query(user_id)
    ```

=== "Async"

    ```python
    import cachebox
    
    @cachebox.cached(cachebox.LRUCache(maxsize=256))
    async def fetch_user(user_id: int) -> dict:
        # Uses asyncio.Lock automatically for async functions
        return await expensive_db_query(user_id)
    ```

You can use your own lock type. anything that implements `contextlib.AbstractContextManager` for sync functions, or
`contextlib.AbstractAsyncContextManager` for async functions:

```python
import threading
import cachebox

# Use an RLock (re-entrant lock) instead of the default Lock
@cachebox.cached(cachebox.LRUCache(maxsize=256), lock=threading.RLock)
def fetch_user(user_id: int) -> dict:
    return expensive_db_query(user_id)
```

!!! warning
    Passing a synchronous lock to an async function (or vice versa) raises
    a TypeError at decoration time.

If your workload doesn't require it you can disable the lock entirely with `lock=False` or `lock=None`.
While the default lock is safe for most use cases, there are situations where keeping it enabled causes
problems or is simply unnecessary. *Recursive functions* are the most common case. Because `threading.Lock` is
non-reentrant, a cached recursive function will deadlock the moment it calls itself.

```python
# ❌ Deadlocks on any recursive call
@cachebox.cached(cachebox.LRUCache(maxsize=256))
def factorial(n: int) -> int:
    return 1 if n <= 1 else n * factorial(n - 1)
```

Other cases where disabling the lock is reasonable:

- *Cheap computations*: if recomputing a value is nearly free, the overhead of
lock contention outweighs the benefit of preventing duplicate work.
- *Single-threaded environments*: no concurrency means no stampedes; the lock
is pure overhead.
- *Already-serialised callers*: if your architecture guarantees that only one
caller can request a given key at a time (e.g. a task queue), the lock adds
nothing.

!!! note
    Disabling the lock does not make cache operations unsafe. all reads and
    writes are still protected by internal Rust mutexes. It only means that
    multiple threads may compute the same missing value simultaneously.
