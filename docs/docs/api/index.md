# API Reference

Cachebox's public surface is small: cache classes, the `@cached` decorator, key/postprocess
helpers, and a few inspection utilities.

## Package exports

```python
from cachebox import (
    # Cache classes
    BaseCacheImpl,
    Cache,
    FIFOCache,
    RRCache,
    LRUCache,
    LFUCache,
    TTLCache,
    VTTLCache,
    # Decorator & helpers
    cached,
    Frozen,
    is_cached,
    get_cached_cache,
    get_cached_cache_info,
    get_cached_callback,
    clear_cached_cache,
    # Key makers
    make_key,
    make_typed_key,
    make_hash_key,
    # Postprocessors
    postprocess_copy,
    postprocess_copy_mutables,
    postprocess_deepcopy,
    postprocess_deepcopy_mutables,
    # Callback events
    EVENT_HIT,
    EVENT_MISS,
    # Version
    __version__,
)
```

## Sections

| Section | Contents |
|---------|----------|
| [Classes](impls.md) | `BaseCacheImpl`, `Cache`, `FIFOCache`, `RRCache`, `LRUCache`, `LFUCache`, `TTLCache`, `VTTLCache` |
| [Utilities](utils.md) | `cached`, `Frozen`, key makers, postprocessors, inspection helpers |

## Quick links for common tasks

| Task | Start here |
|------|------------|
| Memoize a function | [`cached`](utils.md#cachebox.utils.cached) |
| Pick an eviction policy | [Choosing a Cache](../algorithms.md) |
| Per-instance method cache | [Getting Started — methods](../getting-started.md#caching-instance-methods) |
| Per-item TTL | [`VTTLCache`](impls.md#cachebox._cachebox.VTTLCache) |
| Read-only wrapper | [`Frozen`](utils.md#cachebox.utils.Frozen) |
| Weighted capacity | `getsizeof` on any class constructor |

## Base protocol

All caches implement `BaseCacheImpl` and support:

- Dict protocol: `[]`, `get`, `in` / `contains`, `len`, `keys` / `values` / `items`, `update`, `pop`, `clear`
- Capacity: `maxsize`, `current_size()`, `remaining_size()`, `capacity()`, `is_full()`, `is_empty()`
- Mutation helpers: `insert`, `setdefault`, `setdefault_with`, `popitem`, `drain`, `shrink_to_fit`
- Copy / pickle: `copy`, `__copy__`, `__getstate__` / `__setstate__`

Policy-specific APIs (e.g. `peek`, `first`/`last`, `get_with_expire`) are documented on each class.
