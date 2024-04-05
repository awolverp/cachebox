# Cachebox
[**v2 Changelog**](https://github.com/awolverp/cachebox/blob/main/CHANGELOG.md#200---2024-03-09) | [**Releases**](https://github.com/awolverp/cachebox/releases)

The fastest caching library with different implementations, written in Rust.

- 🚀 3-21x faster than other libraries (like cachetools and cacheout)
- 🤯 Sometimes It works **as fast as dictionary**
- **(R)** written in Rust
- 🤝 Support Python 3.8 and above
- 📦 Over 7 cache algorithms are supported
- 🧶 Completely thread-safe

> 🚀 you can see benchmarks [**here**](https://github.com/awolverp/cachebox-benchmark).

**(@)** decorator example:
```python
from cachebox import cached, cachedmethod, TTLCache, LRUCache

# Keep coin price for no longer than a minute
@cached(TTLCache(maxsize=126, ttl=60))
def get_coin_price(coin_name):
    return web3_client.get_price(coin_name)

# Async functions are supported
@cached(LRUCache(maxsize=126))
async def get_coin_price(coin_name):
    return await async_web3_client.get_price(coin_name)

# You can pass `capacity` parameter.
# If `capacity` specified, the cache will be able to hold at
# least capacity elements without reallocating.
@cached(LRUCache(maxsize=126, capacity=100))
def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)

# methods are supported
class APIResource:
    @cachedmethod(
        TTLCache(126, ttl=10),
        # You can detemine how caching is done using `key_maker` parameter.
        key_maker=lambda args, kwds: args[0].client_ip
    )
    def get_information(self, request):
        ...
```

## Page Contents
- ⁉️ [What is caching?](#what-is-caching)
- ⁉️ [When i need caching?](#when-i-need-caching)
- 🎯 [Features](#features)
- 🛠️ [Installation](#installation)
- 🎓 [Usage](#API)
- 🚀 [Performance table](#performance-table)
- ⁉️ [Frequently Asked Questions](#frequently-asked-questions)
- 🆕 [*CHANGELOG*](CHANGELOG.md)
- ⏱️ [*BENCHMARK*](https://github.com/awolverp/cachebox-benchmark)

## What is caching?
In computing, caching improves performance by keeping recent or often-used data items in memory locations which are faster,
or computationally cheaper to access than normal memory stores. When the cache is full,
the algorithm must choose which items to discard to make room for new data. (*Wikipedia*)

## When i need caching?
1. Sometimes you have **functions that take a long time to execute**, and you need to call them each time.

```python
@cached(LRUCache(260))
def function(np_array):
    # big operations
    ...
```

2. Sometimes you need to **temporarily store data** in memory for a short period.

3. When dealing with **remote APIs**, Instead of making frequent API calls, store the responses in a cache.

```python
@cached(TTLCache(0, ttl=10))
def api_call(key):
    return api.call(key)
```

4. **Caching query results** from databases can enhance performance.

```python
@cached(TTLCache(0, ttl=1))
def select_user(id):
    return db.execute("SELECT * FROM users WHERE id=?", (id,))
```

and ...

## Installation
You can install **cachebox** from PyPi:
```sh
pip3 install -U cachebox
```

To verify that the library is installed correctly, run the following command:
```sh
python -c "import cachebox; print(cachebox.__version__)"
```

## API
All the implementations are support **mutable-mapping** methods (e.g `__setitem__`, `get`, `popitem`),
and there are some new methods for each implemetation.

These methods are available for all classes:
- `insert(key, value)`: an aliases for `__setitem__`

```python
>>> cache.insert(1, 1) # it equals to cache[1] = 1
```

- `capacity()`: Returns the number of elements the cache can hold without reallocating.

```python
>>> cache.update((i, i) for i in range(1000))
>>> cache.capacity()
1432
```

- `drain(n)`: According to cache algorithm, deletes and returns how many items removed from cache.

```python
>>> cache = LFUCache(10, {i:i for i in range(10)})
>>> cache.drain(8)
8
>>> len(cache)
2
>>> cache.drain(10)
2
>>> len(cache)
0
```

- `shrink_to_fit()`: Shrinks the capacity of the cache as much as possible.

```python
>>> cache = LRUCache(0, {i:i for i in range(10)})
>>> cache.capacity()
27
>>> cache.shrink_to_fit()
>>> cache.capacity()
11
```

### Cache
Fixed-size (or can be not) cache implementation without any policy,
So only can be fixed-size, or unlimited size cache.

```python
>>> from cachebox import Cache
>>> cache = Cache(100) # fixed-size cache
>>> cache = Cache(0) # unlimited-size cache
>>> cache = Cache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
>>> cache = Cache(2, {i:i for i in range(10)})
...
OverflowError: maximum size limit reached
```

**There're no new methods for this class.**

### FIFOCache
FIFO Cache implementation (First-In First-Out policy, very useful).

In simple terms, the FIFO cache will remove the element that has been in the cache the longest;
It behaves like a Python dictionary.

```python
>>> from cachebox import FIFOCache
>>> cache = FIFOCache(100) # fixed-size cache
>>> cache = FIFOCache(0) # unlimited-size cache
>>> cache = FIFOCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
```

**There're new methods:**
- `first`: returns the first inserted key (the oldest)
- `last`: returns the last inserted key (the newest)


### LFUCache
LFU Cache implementation (Least frequantly used policy).

In simple terms, the LFU cache will remove the element in the cache that has been accessed the least,
regardless of time.

```python
>>> from cachebox import LFUCache
>>> cache = LFUCache(100) # fixed-size cache
>>> cache = LFUCache(0) # unlimited-size cache
>>> cache = LFUCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
```

**There's a new method:**
- `least_frequently_used`: returns the key that has been accessed the least.


### RRCache
RRCache implementation (Random Replacement policy).

In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.

```python
>>> from cachebox import RRCache
>>> cache = RRCache(100) # fixed-size cache
>>> cache = RRCache(0) # unlimited-size cache
>>> cache = RRCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
```

**There're no new methods for this class.**


### LRUCache
LRU Cache implementation (Least recently used policy).

In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.

```python
>>> from cachebox import LRUCache
>>> cache = LRUCache(100) # fixed-size cache
>>> cache = LRUCache(0) # unlimited-size cache
>>> cache = LRUCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
```

**There're new methods:**
- `least_recently_used`: returns the key that has not been accessed in the longest time.
- `most_recently_used`: returns the key that has been accessed in the longest time.


### TTLCache
TTL Cache implementation (Time-to-live policy).

In simple terms, The TTL cache is one that evicts items that are older than a time-to-live.

```python
>>> from cachebox import TTLCache
>>> cache = TTLCache(100, 2) # fixed-size cache, 2 ttl value
>>> cache = TTLCache(0, 10) # unlimited-size cache, 10 ttl value
>>> cache = TTLCache(100, 5, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
```
**There're new methods:**
- `get_with_expire`: Works like `.get()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2})
>>> cache.get_with_expire(1)
(1, 1.23445675)
>>> cache.get_with_expire("no-exists")
(None, 0.0)
```

- `pop_with_expire`: Works like `.pop()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2})
>>> cache.pop_with_expire(1)
(1, 1.23445675)
>>> cache.pop_with_expire(1)
(None, 0.0)
```

- `popitem_with_expire`: Works like `.popitem()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2})
>>> cache.popitem_with_expire()
(1, 1, 1.23445675)
>>> cache.popitem_with_expire()
(2, 2, 1.94389545)
>>> cache.popitem_with_expire()
...
KeyError
```

### VTTLCache
VTTL Cache implementation (Time-to-live per-key policy)

Works like TTLCache, with this different that each key has own time-to-live value.

```python
>>> cache = VTTLCache(100) # fixed-size cache
>>> cache = VTTLCache(0) # unlimited-size cache

# initialize from dict or any iterable object;
# also these items will expire after 5 seconds
>>> cache = VTTLCache(100, {"key1": "value1", "key2": "value2"}, 5)

# initialize from dict or any iterable object;
# but these items never expire, because we pass None as them ttl value
>>> cache = VTTLCache(100, {"key1": "value1", "key2": "value2"}, None)
```

**There're new methods:**
- `insert(key, value, ttl)`: is different here. if you use `cache[key] = value` way, you cannot set ttl value for those item, but here you can.

```python
>>> cache.insert("key", "value", 10) # this item will expire after 10 seconds
>>> cache.insert("key", "value", None) # but this item never expire.
```

- `setdefault(key, default, ttl)`: Returns the value of the specified key.
If the key does not exist, insert the key, with the specified value.

- `update(iterable, ttl)`: inserts the specified items to the cache. The `iterable` can be a dictionary,
or an iterable object with key-value pairs.

```python
>>> cache = VTTLCache(20)
>>> cache.insert("key", "value", 10)
>>> cache.update({i:i for i in range(12)}, 2)
>>> len(cache)
13
>>> time.sleep(2)
>>> len(cache)
1
```

- `get_with_expire`: Works like `.get()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2}, 2)
>>> cache.get_with_expire(1)
(1, 1.9934)
>>> cache.get_with_expire("no-exists")
(None, 0.0)
```

- `pop_with_expire`: Works like `.pop()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2}, 2)
>>> cache.pop_with_expire(1)
(1, 1.99954)
>>> cache.pop_with_expire(1)
(None, 0.0)
```

- `popitem_with_expire`: Works like `.popitem()`, but also returns the remaining expiration.

```python
>>> cache.update({1: 1, 2: 2}, 2)
>>> cache.popitem_with_expire()
(1, 1, 1.9786564)
>>> cache.popitem_with_expire()
(2, 2, 1.97389545)
>>> cache.popitem_with_expire()
...
KeyError
```

## Performance table

> [!NOTE]\
> Operations which have an amortized cost are suffixed with a `*`. Operations with an expected cost are suffixed with a `~`.

|              | get(i) | insert(i)       | delete(i)      | update(m)        | popitem |
| ------------ | ------ | --------------- | -------------- | ---------------- | ------- |
| Cache        | O(1)~  | O(1)~*          | O(1)~          | O(m)~            | N/A     |
| FIFOCache    | O(1)~  | O(min(i, n-i))* | O(min(i, n-i)) | O(m*min(i, n-i)) | O(1)    |
| LFUCache     | O(1)~  | O(n)~*          | O(1)~          | O(m*n)~          | O(n)~*  |
| RRCache      | O(1)~  | O(1)~*          | O(1)~          | O(m)~            | O(1)~   |
| LRUCache     | O(1)~  | ?               | O(1)~          | ?                | O(1)    |
| TTLCache     | O(1)~  | O(min(i, n-i))* | O(min(i, n-i)) | O(m*min(i, n-i)) | O(1)    |
| VTTLCache    | O(1)~  | ?               | O(n-i)         | ?                | O(1)~   |

## Frequently asked questions
#### What is the difference between TTLCache and VTTLCache?
In `TTLCache`, you set an expiration time for all items, but in `VTTLCache`,
you can set a unique expiration time for each item.

|              | TTL         | Speed   |
| ------------ | ----------- | ------- |
| TTLCache     | One ttl for all items       | TTLCache is very faster than VTTLCache |
| VTTLCache    | Each item has unique expiration time | VTTLCache is slow in inserting |


#### Can we set maxsize to zero?
Yes, if you pass zero to maxsize, means there's no limit for items.

#### Migrate from cachetools to cachebox
*cachebox* syntax is very similar to *cachetools*.
Just change these:
```python
# If you pass infinity to a cache implementation, change it to zero.
cachetools.Cache(math.inf) -> cachebox.Cache(0)
# If you use `isinstance` for cachetools classes, change those.
isinstance(cache, cachetools.Cache) -> isinstance(cache, cachebox.BaseCacheImpl)
```

## License
Copyright (c) 2024 aWolverP - **MIT License**
