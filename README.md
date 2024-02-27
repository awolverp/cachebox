# Cachebox
**Cachebox** is a Python library (written in Rust) that provides memoization and cache implementions with
different cache replecement policies.

> [!NOTE]\
> This library is very faster than cachetools and other libraries (between 5x-10x) and use lower memory than, [*you can see benchmarks here*](BENCHMARK.md).

```python
from cachebox import cached, TTLCache, LRUCache

# Keep coin price for no longer than a minute
@cached(TTLCache(maxsize=126, ttl=60))
def get_coin_price(coin_name):
    return web3_client.get_price(coin_name)

# Async functions are supported
@cached(LRUCache(maxsize=126))
async def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)

# You can pass `capacity` parameter.
# If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
@cached(LRUCache(maxsize=126, capacity=100))
def get_coin_price(coin_name):
    return web3_client.get_price(coin_name)
```

**Page Content**:
- [Should i use memoization?](#what-is-caching-and-why-to-use-it)
- [Features](#features)
- [Installation](#installation)
- [Tutorial](#tutorial)
    - [difference between `TTLCache` and `TTLCacheNoDefault`](#what-is-the-difference-between-ttlcache-and-ttlcachenodefault)
- [Frequently Asked Questions](#frequently-asked-questions)

### What is caching and why to use it?
Wikipeda:
> In computing, caching improves performance by keeping recent or often-used data
 items in memory locations which are faster, or computationally cheaper to access,
 than normal memory stores. When the cache is full, the algorithm must choose which
 items to discard to make room for new data. 

Researchgate:
> Cache replacement policies play important roles in efficiently processing the current
 big data applications. The performance of any high performance computing system is highly
 depending on the performance of its cache memory. 


### Features
Pros:
- Thread-safe (uses Rusts `RwLock`)
- You can use it `async` and `sync`
- Varius cache alghoritms (supports *8* cache alghoritms)
- Super fast (is written in Rust language)
- High performance

Cons:
- Does not support iterating (for `values`, `keys` and `items` methods)
- Does not support *PyPy*

Supported:
- `Cache`: Simple cache implemention without any policy and alghoritm.
- `FIFOCache`: First In First Out cache implemention.
- `LFUCache`: Least Frequently Used cache implemention.
- `RRCache`: Random Replacement cache implemention.
- `LRUCache`: Least Recently Used cache implemention.
- `MRUCache`: Most Recently Used cache implemention.
- `TTLCache`: LRU Cache Implementation With Per-Item TTL Value.
- `TTLCacheNoDefault`: Time-aware Cache Implemention; With this cache, you can set its own expiration time for each key-value pair.


## Installation
You can install **cachebox** from PyPi:
```sh
pip3 install -U cachebox
```

Now you can use it:
```python
>>> import cachebox
>>> cachebox.__version__
'...'
```

## Tutorial
This package is very easy to use. You can use all implementions like a dictionary;
they're supported all `abc.MutableMapping` methods.
But *there are some new methods* you can see in examples.

For instance, see **LRUCache** example:
```python
import cachebox

cache = cachebox.LRUCache(10)
cache.insert("key", "value") # or cache["key"] = "value"
cache.delete("key") # or `del cache["key"]`

# `.clear()` method has new parameter `reuse`
# pass True to keeps the allocated memory for reuse (default False).
cache.clear(reuse=True)
```

And there are new methods for `TTLCache` and `TTLCacheNoDefault`. You can see those methods
in these examples:

**TTLCache** example:
```python
import cachebox

cache = cachebox.TTLCache(10, ttl=2)
cache.insert(1, "value1") # or cache[1] = "value1"
cache.insert(2, "value2") # or cache[2] = "value2"
cache.insert(3, "value3") # or cache[3] = "value3"

# It works like `.get()` with the difference that it returns the expiration of item in seconds.
cache.get_with_expire(1)
# Output: ('value1', 1.971873426437378)

# It works like `.popitem()` with the difference that it returns the expiration of item in seconds.
cache.popitem_with_expire()
# Output: (1, 'value1', 1.961873426437378)

# It works like `.pop()` with the difference that it returns the expiration of item in seconds.
cache.pop_with_expire(2)
# Output: ('value2', 1.951873426437378)

# Calling this method removes all items whose time-to-live would have expired by time,
# and if `reuse` be True, keeps the allocated memory for reuse (default False).
cache.expire(reuse=False)
```

**TTLCacheNoDefault** example:
```python
import cachebox

# TTLCacheNoDefault have not ttl parameter here.
cache = cachebox.TTLCacheNoDefault(10)
cache.insert(1, "value1", ttl=10) # this key-pair is available for no longer than 10 seconds
cache.insert(2, "value2", ttl=2) # this key-pair is available for no longer than 2 seconds
cache.setdefault(3, "value3", ttl=6) # this key-pair is available for no longer than 6 seconds
cache.insert(4, "value4", ttl=None) # this key-pair never expire

# It works like `.get()` with the difference that it returns the expiration of item in seconds.
cache.get_with_expire(1)
# Output: ('value1', 9.971873426437378)

# It works like `.popitem()` with the difference that it returns the expiration of item in seconds.
cache.popitem_with_expire()
# Output: (2, 'value2', 1.961873426437378)

# It works like `.pop()` with the difference that it returns the expiration of item in seconds.
cache.pop_with_expire(4) 
# Output: ('value4', 0.0)
```

### What is the difference between TTLCache and TTLCacheNoDefault?
In `TTLCache`, you set an expiration time for all items, but in `TTLCacheNoDefault`,
you can set a unique expiration time for each item.

|              | TTL         | Speed   |
| ------------ | ----------- | ------- |
| TTLCache     | One ttl for all items       | TTLCache is very faster than TTLCacheNoDefault |
| TTLCacheNoDefault    | Each item has unique expiration time | TTLCacheNoDefault is very slow in inserting |


## Frequently asked questions
#### Can we set maxsize to zero?
Yes, if you pass zero to maxsize, means there's no limit for items.

#### I use cachetools, how to change it to cachebox?
*cachebox* syntax is very similar to *cachetools*.
Just change these items:
```python
# If you use `isinstance` for cachetools classes, change those.
isinstance(cache, cachetools.Cache) -> isinstance(cache, cachebox.BaseCacheImpl)

# If you pass `None` to `cached()`, change it to `dict`.
@cachetools.cached(None) -> @cachebox.cached({})

# If you use `cache.maxsize`, change it to `cache.getmaxsize()`
cache.maxsize -> cache.getmaxsize()
```

## License
Copyright (c) 2024 aWolverP - **MIT License**
