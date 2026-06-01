# Getting Started

This guide walks you through the most common cachebox patterns.
All cache classes behave like Python dictionaries unless noted otherwise.

## Using the `@cached` Decorator
The simplest way to cache a function's return value:

```python hl_lines="3"
import cachebox

@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, number + 1):
        fact *= num
    return fact

assert factorial(5) == 125
```

The first parameter `cache`, you can specify the cache instance it should use for caching.

```python hl_lines="4"
import cachebox

@cachebox.cached(
    cachebox.LRUCache(maxsize=128),
)
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, number + 1):
        fact *= num
    return fact

assert factorial(5) == 125
```

### Async Functions

Coroutines are supported out of the box:

```python
import cachebox

@cachebox.cached(cachebox.LRUCache(maxsize=128))
async def make_request(method: str, url: str) -> dict:
    response = await client.request(method, url)
    return response.json()
```

### Using a Custom Key Maker
There are 3 ready-to-use key maker functions, and by default the `@cached` decorator uses the simplest one of them.

You can use ready-to-use functions, or create a custom one.

=== "Standard way"
  
    ```python hl_lines="3 4 8"
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

=== "Using `lambda`"
  
    ```python hl_lines="5"
    import cachebox
    
    @cachebox.cached(
        cachebox.LRUCache(128), 
        key_maker=lambda request: request.path,
    )
    async def request_handler(request):
        return Response("hello")
    ```

Ready to use key makers are:

- [make_key function](api/utils.md#cachebox.utils.make_key)
- [make_typed_key function](api/utils.md#cachebox.utils.make_typed_key)
- [make_hash_key function](api/utils.md#cachebox.utils.make_hash_key)


### Callbacks on Cache Events
The `@cached` decorator supports callback on every hit/miss, using `callback` parameter.

```python hl_lines="3 4 5 6 7 11"
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

!!! tip

    May be a coroutine in async contexts.


### Setting a Postprocessor
The `@cached` decorator also supports postprocessors, using `postprocess` parameter.
It can be used as a transformer which applied before returning a result to the caller.

There are 3 ready-to-use key maker functions, and by default the `@cached` decorator uses
[`postprocess_copy_mutables` function](api/utils.md#cachebox.utils.postprocess_copy_mutables).

```python hl_lines="3 4 5 9"
import cachebox

def postprocess(result):
    print(f"RESULT: {result}")
    return result

@cachebox.cached(
    cachebox.LRUCache(0),
    postprocess=postprocess,
)
def add(a, b):
    return a + b

add(1, 2)   # RESULT: 3
```

Ready to use postprocessors:

- [postprocess_copy function](api/utils.md#cachebox.utils.postprocess_copy)
- [postprocess_copy_mutables function](api/utils.md#cachebox.utils.postprocess_copy_mutables)
- [postprocess_deepcopy function](api/utils.md#cachebox.utils.postprocess_deepcopy)
- [postprocess_deepcopy_mutables function](api/utils.md#cachebox.utils.postprocess_deepcopy_mutables)

!!! note

    Added since v6.0.0, and the `copy_level` parameter removed due to this feature.

### Bypass the Cache for a Single Call
Pass `cachebox__ignore=True` to skip the cache entirely:

```python
result = my_func(10, 20, cachebox__ignore=True)
```

### Cache on an Instance Method

```python hl_lines="6 8"
import cachebox

class MyService:
    def __init__(self, multiplier: int):
        self.multiplier = multiplier
        self._cache = cachebox.TTLCache(20, ttl=10)

    @cachebox.cached(lambda self: self._cache)
    def compute(self, char: str):
        return char * self.multiplier

svc = MyService(5)
assert svc.compute("a") == "aaaaa"
```

## Using a Cache Implemetations
You can use all cache implementations without `@cached` method.
You only need to import the classes you want and can work with them like a regular dictionaries
(except for [`VTTLCache`](api/impls.md#cachebox._cachebox.VTTLCache), this have some differences).

```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"
assert cache.get("missing", "default") == "default"
```

## Immutable (Frozen) Cache

Wrap any cache with `Frozen` to prevent further writes:

```python
from cachebox import Frozen, LRUCache

cache = LRUCache(10, {1: "a", 2: "b"})
frozen = Frozen(cache, ignore=False)

frozen[3] = "c"  # TypeError: This cache is frozen.
```

## Saving a Cache to Disk

Use Python's `pickle` module:

```python
import cachebox, pickle

cache = cachebox.LRUCache(100, {i: i for i in range(50)})

with open("cache.pkl", "wb") as f:
    pickle.dump(cache, f)

with open("cache.pkl", "rb") as f:
    loaded = pickle.load(f)

assert cache == loaded
```

## Next Steps

- Browse the full [API Reference](api/index.md) for every class and method.
- Check [Tips & Notes](tips.md) for copying caches and advanced patterns.
- Read the [Migration Guide](migration.md) if upgrading from v5.
