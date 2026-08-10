# Getting Started

This guide covers the most common cachebox patterns. All cache classes behave like Python
dictionaries unless noted otherwise.

## Using the `@cached` Decorator

The simplest way to memoize a function's return value:

```python
import cachebox

@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, number + 1):
        fact *= num
    return fact

assert factorial(5) == 120
assert factorial(5) == 120  # served from cache
```

The first argument is the cache instance used for storage. Pass `None` (or omit it) to get an
unbounded `LRUCache`. A plain `dict` is also accepted and converted to an unbounded `LRUCache`.

```python
@cachebox.cached()                       # unbounded LRUCache
def f(x): ...

@cachebox.cached(cachebox.LRUCache(128)) # bounded LRUCache
def g(x): ...
```

### Async Functions

Coroutines are supported out of the box. Stampede prevention uses `asyncio.Lock` automatically:

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(maxsize=128))
async def make_request(method: str, url: str) -> dict:
    response = await client.request(method, url)
    return response.json()
```

### Custom Key Makers

By default `@cached` uses [`make_key`](api/utils.md#cachebox.utils.make_key), which builds a
hashable key from positional and keyword arguments. You can supply your own:

=== "Named function"

    ```python
    import cachebox

    def path_key(request):
        return request.path

    @cachebox.cached(
        cachebox.LRUCache(128),
        key_maker=path_key,
    )
    async def request_handler(request):
        return Response("hello")
    ```

=== "Lambda"

    ```python
    import cachebox

    @cachebox.cached(
        cachebox.LRUCache(128),
        key_maker=lambda request: request.path,
    )
    async def request_handler(request):
        return Response("hello")
    ```

Built-in key makers:

| Function | Behavior |
|----------|----------|
| [`make_key`](api/utils.md#cachebox.utils.make_key) | Default. Fast path for a single `int`/`str`; otherwise a tuple of args (+ kwargs). |
| [`make_typed_key`](api/utils.md#cachebox.utils.make_typed_key) | Like `make_key`, but includes runtime types so `f(1)` and `f(1.0)` are distinct. |
| [`make_hash_key`](api/utils.md#cachebox.utils.make_hash_key) | Stores `hash(args…)` only — smaller keys, risk of rare collisions. |

### Callbacks on Cache Events

Pass a `callback` to observe every hit and miss:

```python
import cachebox

def on_cache_event(event: int, key, value):
    if event == cachebox.EVENT_MISS:
        print(f"MISS  key={key}")
    elif event == cachebox.EVENT_HIT:
        print(f"HIT   key={key}")

@cachebox.cached(
    cachebox.LRUCache(0),
    callback=on_cache_event,
)
def add(a, b):
    return a + b

add(1, 2)   # MISS  key=(1, 2)
add(1, 2)   # HIT   key=(1, 2)
```

`EVENT_MISS` is `1` and `EVENT_HIT` is `2`. In async contexts the callback may be a coroutine;
it is awaited automatically.

### Postprocessors

A postprocessor transforms the cached value **before** it is returned to the caller. This is
how cachebox protects against accidental mutation of cached `dict`/`list`/`set` objects.

The default is [`postprocess_copy_mutables`](api/utils.md#cachebox.utils.postprocess_copy_mutables):
`dict`, `list`, and `set` results are shallow-copied on every return; other types are returned as-is.

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(128))
def make_dict(name: str, age: int) -> dict:
    return {"name": name, "age": age}

d = make_dict("cachebox", 10)
d["new-key"] = "new-value"

d2 = make_dict("cachebox", 10)
# Without copying, d2 would also contain "new-key"
assert d2 == {"name": "cachebox", "age": 10}
```

Ready-to-use postprocessors:

| Function | Behavior |
|----------|----------|
| `None` | Return the cached object as-is (no copy). |
| [`postprocess_copy_mutables`](api/utils.md#cachebox.utils.postprocess_copy_mutables) | Shallow-copy `dict`/`list`/`set` only (**default**). |
| [`postprocess_copy`](api/utils.md#cachebox.utils.postprocess_copy) | Shallow-copy every value. |
| [`postprocess_deepcopy_mutables`](api/utils.md#cachebox.utils.postprocess_deepcopy_mutables) | Deep-copy `dict`/`list`/`set` only. |
| [`postprocess_deepcopy`](api/utils.md#cachebox.utils.postprocess_deepcopy) | Deep-copy every value. |

```python
@cachebox.cached(
    cachebox.LRUCache(0),
    postprocess=cachebox.postprocess_deepcopy,
)
def build_tree():
    return {"children": [{"id": 1}]}
```

### Bypass the Cache for a Call

Pass `cachebox__ignore=True` to execute the function without reading or writing the cache:

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(128))
def add(a, b):
    print("computing...")
    return a + b

add(1, 2)  # computing...
add(1, 2)  # from cache

add(1, 2, cachebox__ignore=True)  # computing...
# Only this call is uncached; future calls still use the cache
```

### Caching Instance Methods

For instance methods, each object usually needs its own cache. Pass a callable that receives
`self` and returns the cache:

```python
import cachebox

class MyService:
    def __init__(self, multiplier: int):
        self.multiplier = multiplier
        self._cache = cachebox.TTLCache(20, global_ttl=10)

    @cachebox.cached(lambda self: self._cache)
    def compute(self, char: str):
        return char * self.multiplier

svc1 = MyService(2)
svc2 = MyService(5)

assert svc1.compute("x") == "xx"
assert svc2.compute("x") == "xxxxx"
# Entries created by svc1 are not visible to svc2
```

When the cache is a callable, `self`/`cls` is **excluded** from the cache key automatically.

!!! note "No helper attributes on methods"

    When you pass a lambda/callable as `cache`, the wrapper does **not** attach `.cache`,
    `.cache_info()`, or `.cache_clear()`. Manage the cache object yourself (for example via
    `self._cache`).

### Caching `@staticmethod`s

Static methods do not receive `self` or `cls`. Provide a cache instance directly; it is shared
by all callers:

```python
import cachebox

class TextUtils:
    @staticmethod
    @cachebox.cached(cachebox.LRUCache(128))
    def normalize(text: str) -> str:
        print("normalizing...")
        return text.strip().lower()

TextUtils.normalize(" Hello ")
TextUtils.normalize(" Hello ")  # cached
```

### Caching `@classmethod`s

Class methods receive `cls`. The cache can live on the class and be selected dynamically:

```python
import cachebox

class UserRepository:
    _cache = cachebox.LRUCache(128)

    @classmethod
    @cachebox.cached(lambda cls: cls._cache)
    def get_user(cls, user_id: int):
        print("loading user...")
        return {"id": user_id}

UserRepository.get_user(1)
UserRepository.get_user(1)  # cached
```

With inheritance, each subclass can own its cache while sharing the method:

```python
import cachebox

class BaseRepository:
    _cache = cachebox.LRUCache(128)

    @classmethod
    @cachebox.cached(lambda cls: cls._cache)
    def get_item(cls, item_id):
        return f"{cls.__name__}:{item_id}"

class ProductRepository(BaseRepository):
    _cache = cachebox.LRUCache(128)

class OrderRepository(BaseRepository):
    _cache = cachebox.LRUCache(128)
```

## Using Cache Classes Directly

You can use every cache implementation without `@cached`. They support the usual dict operations
(`[]`, `get`, `in`, `len`, `keys`/`values`/`items`, …) plus cache-specific methods.

```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"
assert cache.get("missing", "default") == "default"
```

Prefer [`.insert(key, value)`](api/impls.md#cachebox._core.Cache.insert) over `__setitem__` when
you need the previous value or want code that stays consistent across policies.

```python
old = cache.insert("key", "new-value")  # returns previous value or None
```

### Common Constructor Parameters

All cache classes accept:

| Parameter | Meaning |
|-----------|---------|
| `maxsize` | Capacity limit. `0` means unbounded (`sys.maxsize` internally). |
| `iterable` | Optional initial data (`dict`, another cache, or `(key, value)` pairs). |
| `capacity` | Pre-allocate the hash table to reduce reallocations. |
| `getsizeof` | Callable `(key, value) -> int` for weighted sizing. Default: every entry has size `1`. |

`TTLCache` and `VTTLCache` add TTL-related parameters — see [Choosing a Cache](algorithms.md)
and the [API reference](api/impls.md).

### Capacity, Size, and Weighted Entries

```python
import cachebox
import sys

# maxsize counts entries (each entry size = 1 by default)
cache = cachebox.LRUCache(maxsize=100)
cache.insert("a", 1)
assert cache.current_size() == 1
assert cache.remaining_size() == 99
assert not cache.is_full()
assert not cache.is_empty()

# Weighted: size is computed by getsizeof
def entry_size(key, value):
    return sys.getsizeof(key) + sys.getsizeof(value)

weighted = cachebox.LRUCache(maxsize=10_000, getsizeof=entry_size)
weighted.insert("user:1", {"name": "Ada"})
print(weighted.current_size())   # sum of entry sizes
print(weighted.remaining_size()) # maxsize - current_size
```

When the cache is full, policy-based classes evict items; plain `Cache` raises `OverflowError`.

### `setdefault` and `setdefault_with`

```python
cache = cachebox.Cache(maxsize=10)

# Insert only if missing
value = cache.setdefault("key", "default")

# Lazy factory — called only on miss (lock is released while it runs)
value = cache.setdefault_with("key", lambda: expensive_compute())
```

!!! warning "Concurrent misses"

    If two threads miss the same key, `factory` may run more than once. The first successful
    insert wins. For single-flight computation, prefer `@cached` with stampede prevention enabled.

### Drain, Clear, and Shrink

```python
cache = cachebox.FIFOCache(10, {i: i for i in range(10)})

# Evict n items according to the policy (FIFO here: oldest first)
removed = cache.drain(3)
assert removed == 3

cache.clear()              # free memory
cache.clear(reuse=True)    # keep allocation for reuse
cache.shrink_to_fit()      # shrink allocation close to current length
```

### Inspecting Capacity

```python
cache = cachebox.LRUCache(maxsize=1000, capacity=1000)
print(cache.capacity())  # slots without reallocation
print(cache.maxsize)     # configured maxsize
print(len(cache))        # number of entries
```

## Immutable (Frozen) Caches

Wrap any cache with `Frozen` to block further writes:

```python
from cachebox import Frozen, LRUCache

cache = LRUCache(10, {1: "a", 2: "b"})
frozen = Frozen(cache, ignore=False)

frozen[3] = "c"  # TypeError: This cache is frozen.

# With ignore=True, mutations are silently ignored
frozen = Frozen(cache, ignore=True)
frozen[3] = "c"  # no-op
assert 3 not in frozen

# The underlying cache remains mutable
cache[3] = "c"
assert frozen[3] == "c"
```

TTL expiry still runs on the underlying `TTLCache` / `VTTLCache` even when frozen.

## Saving a Cache to Disk

All cache classes support `pickle`:

```python
import cachebox
import pickle

cache = cachebox.LRUCache(100, {i: i for i in range(50)})

with open("cache.pkl", "wb") as f:
    pickle.dump(cache, f)

with open("cache.pkl", "rb") as f:
    loaded = pickle.load(f)

assert cache == loaded
```

Do not use a `lambda` as `getsizeof` if you need to pickle the cache — picklable callables only.

## Next Steps

- [Choosing a Cache](algorithms.md) — pick the right eviction policy
- [Tips & Notes](tips.md) — stampede prevention, sweepers, attached attributes
- [FAQ](faq.md) — common questions
- [API Reference](api/index.md) — full method documentation
