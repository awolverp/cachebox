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

## Copying a Cache
All cache classes support Python's `copy` module, both shallow-copy and deep-copy:

```python
import cachebox
import copy

cache = cachebox.LRUCache(100, {i: i for i in range(10)})

shallow = copy.copy(cache)       # shallow copy
deep    = copy.deepcopy(cache)   # deep copy
```

## Avoiding Cache Stampede

Cachebox uses a distributed lock system internally to prevent the
[cache stampede](https://en.wikipedia.org/wiki/Cache_stampede) problem —
multiple concurrent requests recomputing the same missing entry simultaneously.
No additional configuration is required.

## Pre-allocating Capacity
If you know roughly how many items a cache will hold, set `capacity` to avoid
hash table rehashing during initial population:

```python
cache = cachebox.LRUCache(maxsize=10_000, capacity=10_000)
```

## Thread Safety

All cache operations (reads, writes, eviction) are protected by internal Rust mutexes.
You do **not** need to add external synchronisation.

## TTL and Frozen Caches

!!! note

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
        You can use [get_cached_cache function](cachebox.utils.get_cached_cache) to prevent lint
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
        You can use [get_cached_cache_info function](cachebox.utils.get_cached_cache_info) to prevent lint
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
        You can use [clear_cached_cache function](cachebox.utils.clear_cached_cache) to prevent lint
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
        You can use [get_cached_callback function](cachebox.utils.get_cached_callback) to prevent lint
        & IDE warnings.

        ```python
        assert cachebox.get_cached_callback(add) is callback
        ```


## TTLCache/VTTLCache background thread
TODO
