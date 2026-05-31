# Migration Guide

This page documents breaking changes between major versions.

## v5 → v6
These are changes that are not compatible with the previous version:



### `copy_level` parameter has removed from `@cached`
We removed `copy_level` parameter from `@cached` decorator.
The new `postprocess` feature gives you more control on results.

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

### `TTLCache.ttl` has renamed to `TTLCache.global_ttl`
`TTLCache.ttl` has renamed to `TTLCache.global_ttl` because it was causing developers to confuse the usage of
`TTLCache.ttl` with `VTTLCache`'s `ttl` parameter.

```python
# v5
cache = cachebox.TTLCache(maxsize=125, ttl=10)
print(cache.ttl)

# v6
cache = cachebox.TTLCache(maxsize=125, global_ttl=10)
print(cache.global_ttl)
```

### Maxmemory limit has removed
In version 5, we could limit the cache classes by memory using `maxmemory` parameter.
But it caused performance `-75%`, and that was not the library targets. Our focus is on performance & speed.
So we removed it, but added a new parameter: `getsizeof`. A callable that computes the size of a key-value pair.
Now you can use this to implement weighted caching - for example, sizing entries by memory footprint or byte length.
This could cover `maxmemory`, while keeps performance on top.

```python
# v5
cache = cachebox.LRUCache(maxsize=125, maxmemory=1000)

# v6
import sys

def getsizeof(key, val):
    return sys.getsizeof(key) + sys.getsizeof(val)

cache = cachebox.LRUCache(maxsize=1000, getsizeof=getsizeof)
```

Due to this breaking change, we also removed `memory` property from cache classes, and
added new methods: `current_size` and `remaining_size`.

```python
# v5
print(cache.memory)

# v6
print(cache.current_size())
print(cache.remaining_size())
```

### `CacheInfo` fields have changed
The `cachebox.utils.CacheInfo` namedtuple fields has breaking changes:
- `memory` field removed.
- `length` renamed to `size`.

```python
info = cached_function.cache_info()

# v5
print(info.length)
print(info.memory)

# v6
print(info.size)
print(info.memory) # AttributeError
```

## v4 → v5
These are changes that are not compatible with the previous version:

### `CacheInfo.cachememory` renamed to `CacheInfo.memory`

```python
info = func.cache_info()

# v4
print(info.cachememory)

# v5
print(info.memory)
```

### `__eq__` errors are no longer silently swallowed

In v4, errors raised inside a custom `__eq__` method were caught and converted to a `KeyError`.
In v5, they propagate normally.

```python
class A:
    def __hash__(self): return 1
    def __eq__(self, other): raise NotImplementedError

cache = cachebox.FIFOCache(0, {A(): 10})

# v4: raises KeyError
# v5: raises NotImplementedError
cache[A()]
```

### Cache comparisons are no longer order-dependent

In v4, two caches with the same keys/values in different insertion order were considered unequal.
In v5, cache equality follows standard dictionary semantics.

```python
c1 = cachebox.FIFOCache(10)
c2 = cachebox.FIFOCache(10)

c1.insert(1, 'a'); c1.insert(2, 'b')
c2.insert(2, 'b'); c2.insert(1, 'a')

# v4: False  (order-dependent)
# v5: True   (dict-like)
print(c1 == c2)
```

### `cachedmethod` deprecated

`cachedmethod` is deprecated since v5.1.0. Use `cached` with a `lambda self:` cache accessor:

```python
# Before (v4)
@cachebox.cachedmethod(cachebox.TTLCache(0, ttl=10))
def my_method(self, name: str): ...

# After (v5.1.0+)
@cachebox.cached(lambda self: self._cache)
def my_method(self, name: str): ...
```
