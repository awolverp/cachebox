# Migration Guide

This page documents breaking changes between major versions.

## v5 → v6

### `copy_level` on `@cached` is deprecated

`copy_level` no longer has any effect. Use `postprocess` instead for full control over returned values.

```python
# v5
@cachebox.cached(cachebox.RRCache(10), copy_level=2)
def add(a: int, b: int) -> dict:
    return {a: b}

# v6
@cachebox.cached(cachebox.RRCache(10), postprocess=cachebox.postprocess_copy)
def add(a: int, b: int) -> dict:
    return {a: b}
```

The default postprocessor is `postprocess_copy_mutables` (shallow-copy `dict`/`list`/`set` only).
Pass `postprocess=None` to return cached objects as-is.

### `TTLCache.ttl` renamed to `TTLCache.global_ttl`

The property and constructor parameter were renamed to avoid confusion with `VTTLCache`'s
per-item `ttl` argument.

```python
# v5
cache = cachebox.TTLCache(maxsize=125, ttl=10)
print(cache.ttl)

# v6
cache = cachebox.TTLCache(maxsize=125, global_ttl=10)
print(cache.global_ttl)
```

### `maxmemory` removed; use `getsizeof`

v5 offered a `maxmemory` limit. It caused large performance regressions and was removed.
Use `getsizeof` for weighted capacity instead:

```python
# v5
cache = cachebox.LRUCache(maxsize=125, maxmemory=1000)

# v6
import sys

def getsizeof(key, val):
    return sys.getsizeof(key) + sys.getsizeof(val)

cache = cachebox.LRUCache(maxsize=1000, getsizeof=getsizeof)
```

Related renames:

```python
# v5
print(cache.memory)

# v6
print(cache.current_size())
print(cache.remaining_size())
```

### `cachedmethod` removed

Deprecated in v5.1.0 and removed in v6. Use `cached` with a per-instance cache accessor:

```python
# v5
class Service:
    @cachebox.cachedmethod(cachebox.TTLCache(0, ttl=10))
    def my_method(self, name: str): ...

# v6
class Service:
    def __init__(self):
        self._cache = cachebox.TTLCache(0, global_ttl=10)

    @cachebox.cached(lambda self: self._cache)
    def my_method(self, name: str): ...
```

### `CacheInfo` fields

`cache_info()` now reports more detail:

```text
# approximate v5 shape (hits, misses, maxsize, …)
# v6
CacheInfo(hits, misses, maxsize, current_size, length, memory)
```

Update any code that unpacked or indexed the tuple by position.

---

## v4 → v5

### `CacheInfo.cachememory` renamed to `CacheInfo.memory`

```python
info = func.cache_info()

# v4
print(info.cachememory)

# v5
print(info.memory)
```

### `__eq__` errors are no longer swallowed

In v4, errors from a custom `__eq__` were converted to `KeyError`. In v5+ they propagate:

```python
class A:
    def __hash__(self):
        return 1

    def __eq__(self, other):
        raise NotImplementedError

cache = cachebox.FIFOCache(0, {A(): 10})

# v4: KeyError
# v5+: NotImplementedError
cache[A()]
```

### Cache equality is no longer order-dependent

Equality follows dictionary semantics (same keys and values), not insertion order:

```python
c1 = cachebox.FIFOCache(10)
c2 = cachebox.FIFOCache(10)

c1.insert(1, "a")
c1.insert(2, "b")
c2.insert(2, "b")
c2.insert(1, "a")

# v4: False
# v5+: True
print(c1 == c2)
```

### `cachedmethod` deprecated

Deprecated since v5.1.0 (removed in v6). Prefer `cached` with `lambda self: self._cache`.
