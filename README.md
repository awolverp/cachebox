# cachebox

![image](https://img.shields.io/pypi/v/cachebox.svg)
![image](https://img.shields.io/pypi/l/cachebox.svg)
![image](https://img.shields.io/pypi/pyversions/cachebox.svg)
![image](https://static.pepy.tech/badge/cachebox)
![python-test](https://github.com/awolverp/cachebox/actions/workflows/python-test.yml/badge.svg)

[**Releases**](https://github.com/awolverp/cachebox/releases) | [**Benchmarks**](https://github.com/awolverp/cachebox-benchmark) | [**Issues**](https://github.com/awolverp/cachebox/issues/new)

**The fastest caching Python library written in Rust**

### What does it do?
You can easily and powerfully perform caching operations in Python as fast as possible.
This can make your application very faster and it's a good choice in big applications.

- 🚀 10-50x faster than other caching libraries.
- 📊 Very low memory usage (1/2 of dictionary).
- 🔥 Full-feature and easy-to-use
- 🧶 Completely thread-safe
- 🔧 Tested and correct
- **\[R\]** written in Rust that has high-performance
- 🤝 Support Python 3.8+ (PyPy & CPython)
- 📦 Over 7 cache algorithms are supported

## Page Content
- [**When i need caching and cachebox?**](#when-i-need-caching-and-cachebox)
- [**Why `cachebox`?**](#why-cachebox)
- [**Installation**](#installation)
- [**Example**](#example)
- [**Learn**](#learn)
- [**Incompatible changes**](#incompatible-changes)
- [**Tips & Notes**](#tips-and-notes)

## When i need caching and cachebox?
**📈 Frequent Data Access** \
If your application frequently accesses the same data, caching can helps you.

**💎 Expensive Operations** \
When data retrieval involves costly operations such as database queries or API calls, caching can save time and resources.

**🚗 High Traffic Scenarios** \
In big applications with high user traffic caching can help by reducing the number of operations.

**#️⃣ Web Page Rendering** \
Caching HTML pages can speed up the delivery of static content.

**🚧 Rate Limiting** \
Caching can help you to manage rate limits imposed by third-party APIs by reducing the number of requests sent.

**🤖 Machine Learning Models** \
If your application frequently makes predictions using the same input data, caching the results can save computation time.

**And a lot of other situations ...**

## Why cachebox?
**⚡ Rust** \
It uses *Rust* language to has high-performance.

**🧮 SwissTable** \
It uses Google's high-performance SwissTable hash map. thanks to [hashbrown](https://github.com/rust-lang/hashbrown).

**✨ Low memory usage** \
It has very low memory usage.

**⭐ Zero-Dependecy** \
As we said, `cachebox` written in Rust so you don't have to install any other dependecies.

**🧶 Thread-safe** \
It's completely thread-safe and uses locks to prevent problems.

**👌 Easy-To-Use** \
You only need to import it and choice your implementation to use and behave with it like a dictionary.

## Installation
cachebox is installable by `pip`:
```bash
pip3 install -U cachebox
```

> [!WARNING]\
> The new version v4 has some incompatible with v3, for more info please see [Incompatible changes](#incompatible-changes)

## Example
The simplest example of **cachebox** could look like this:
```python
import cachebox

# Like functools.lru_cache, If maxsize is set to 0, the cache can grow without bound and limit.
@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, n + 1):
        fact *= num
    return fact

assert factorial(5) == 125
assert len(factorial.cache) == 1

# Async are also supported
@cachebox.cached(cachebox.LRUCache(maxsize=128))
async def make_request(method: str, url: str) -> dict:
    response = await client.request(method, url)
    return response.json()
```

> [!NOTE]\
> Unlike functools.lru_cache and other caching libraries, cachebox will copy `dict`, `list`, and `set`.
> ```python
> @cachebox.cached(cachebox.LRUCache(maxsize=128))
> def make_dict(name: str, age: int) -> dict:
>    return {"name": name, "age": age}
>
> d = make_dict("cachebox", 10)
> assert d == {"name": "cachebox", "age": 10}
> d["new-key"] = "new-value"
> 
> d2 = make_dict("cachebox", 10)
> # `d2` will be `{"name": "cachebox", "age": 10, "new-key": "new-value"}` if you use other libraries
> assert d2 == {"name": "cachebox", "age": 10}
> ```

## Learn
There are 2 decorators:
- [**cached**](#function-cached): a decorator that helps you to cache your functions and calculations with a lot of options.
- [**cachedmethod**](#function-cachedmethod): this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.
- [**is_cached**](#function-is_cached)

There are 9 classes:
- [**BaseCacheImpl**](#class-basecacheimpl): base-class for all classes.
- [**Cache**](#class-cache): A simple cache that has no algorithm; this is only a hashmap.
- [**FIFOCache**](#class-fifocache): the FIFO cache will remove the element that has been in the cache the longest.
- [**RRCache**](#class-rrcache): the RR cache will choice randomly element to remove it to make space when necessary.
- [**TTLCache**](#class-ttlcache): the TTL cache will automatically remove the element in the cache that has expired.
- [**LRUCache**](#class-lrucache): the LRU cache will remove the element in the cache that has not been accessed in the longest time.
- [**LFUCache**](#class-lfucache): the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.
- [**VTTLCache**](#class-vttlcache): the TTL cache will automatically remove the element in the cache that has expired when need.
- [**Frozen**](#class-frozen): you can use this class for freezing your caches.

Using this library is very easy and you only need to import cachebox and then use these classes like a dictionary (or use its decorator such as `cached` and `cachedmethod`).

There are some examples for you with different methods for introducing those. \
**All the methods you will see in the examples are common across all classes (except for a few of them).**

* * *

### *function* cached

a decorator that helps you to cache your functions and calculations with a lot of options.

**A simple example:**
```python
import cachebox

# Parameters:
#   - `cache`: your cache and cache policy.
#   - `key_maker`: you can set your key maker, see examples below.
#   - `clear_cache`: will be passed to cache's `clear` method when clearing cache.
#   - `callback`: Every time the `cache` is used, callback is also called. See examples below.
@cachebox.cached(cachebox.LRUCache(128))
def sum_as_string(a, b):
    return str(a+b)

assert sum_as_string(1, 2) == "3"

assert len(sum_as_string.cache) == 1
sum_as_string.cache_clear()
assert len(sum_as_string.cache) == 0
```

**A key_maker example:**
```python
import cachebox

def simple_key_maker(args: tuple, kwds: dict):
    return args[0].path

# Async methods are supported
@cachebox.cached(cachebox.LRUCache(128), key_maker=simple_key_maker)
async def request_handler(request: Request):
    return Response("hello man")
```

**A typed key_maker example:**
```python
import cachebox

@cachebox.cached(cachebox.LRUCache(128), key_maker=cachebox.make_typed_key)
def sum_as_string(a, b):
    return str(a+b)

sum_as_string(1.0, 1)
sum_as_string(1, 1)
print(len(sum_as_string.cache)) # 2
```

You have also manage functions' caches with `.cache` attribute as you saw in examples.
Also there're more attributes and methods you can use:
```python
import cachebox

@cachebox.cached(cachebox.LRUCache(0))
def sum_as_string(a, b):
    return str(a+b)

print(sum_as_string.cache)
# LRUCache(0 / 9223372036854775807, capacity=0)

print(sum_as_string.cache_info())
# CacheInfo(hits=0, misses=0, maxsize=9223372036854775807, length=0, cachememory=8)

# `.cache_clear()` clears the cache
sum_as_string.cache_clear()
```

**callback example:** (Added in v4.2.0)
```python
import cachebox

def callback_func(event: int, key, value):
    if event == cachebox.EVENT_MISS:
        print("callback_func: miss event", key, value)
    elif event == cachebox.EVENT_HIT:
        print("callback_func: hit event", key, value)
    else:
        # unreachable code
        raise NotImplementedError

@cachebox.cached(cachebox.LRUCache(0), callback=callback_func)
def func(a, b):
    return a + b

assert func(1, 2) == 3
# callback_func: miss event (1, 2) 3

assert func(1, 2) == 3 # hit
# callback_func: hit event (1, 2) 3

assert func(1, 2) == 3 # hit again
# callback_func: hit event (1, 2) 3

assert func(5, 4) == 9
# callback_func: miss event (5, 4) 9
```

> [!TIP]\
> There's a new feature **since `v4.1.0`** that you can tell to a cached function that don't use cache for a call:
> ```python
> # with `cachebox__ignore=True` parameter, cachebox does not use cache and only calls the function and returns its result.
> sum_as_string(10, 20, cachebox__ignore=True)
> ```

> [!NOTE]\
> You can see [LRUCache here](#class-lrucache).

* * *

### *function* cachedmethod

this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.

```python
import cachebox

class MyClass:
    @cachebox.cachedmethod(cachebox.TTLCache(0, ttl=10))
    def my_method(self, name: str):
        return "Hello, " + name + "!"

c = MyClass()
c.my_method()
```

> [!NOTE]\
> You can see [TTLCache here](#class-ttlcache).

* * *

### *function* is_cached

Check if a function/method cached by cachebox or not

```python
import cachebox

@cachebox.cached(cachebox.FIFOCache(0))
def func():
    pass

assert cachebox.is_cached(func)
```

> [!NOTE]\
> You can see [TTLCache here](#class-ttlcache).

* * *

### *class* BaseCacheImpl
This is the base class of all cache classes such as Cache, FIFOCache, ... \
Do not try to call its constructor, this is only for type-hint.

```python
import cachebox

class ClassName(cachebox.BaseCacheImpl):
    # ...

def func(cache: BaseCacheImpl):
    # ...

cache = cachebox.LFUCache(0)
assert isinstance(cache, cachebox.BaseCacheImpl)
```

* * *

### *class* Cache
A simple cache that has no algorithm; this is only a hashmap.

> [!TIP]\
> **`Cache` vs `dict`**:
> - it is thread-safe and unordered, while `dict` isn't thread-safe and ordered (Python 3.6+).
> - it uses very lower memory than `dict`.
> - it supports useful and new methods for managing memory, while `dict` does not.
> - it does not support `popitem`, while `dict` does.
> - You can limit the size of `Cache`, but you cannot for `dict`.

|              | get   | insert  | delete | popitem |
| ------------ | ----- | ------- | ------ | ------- |
| Worse-case   | O(1)  | O(1)    | O(1)   | N/A     |

```python
from cachebox import Cache

# These parameters are common in classes:
# By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
# By `iterable` param, you can create cache from a dict or an iterable.
# If `capacity` param is given, cache attempts to allocate a new hash table with at
# least enough capacity for inserting the given number of elements without reallocating.
cache = Cache(maxsize=100, iterable=None, capacity=100)

# you can behave with it like a dictionary
cache["key"] = "value"
# or you can use `.insert(key, value)` instead of that (recommended)
cache.insert("key", "value")

print(cache["key"]) # value

del cache["key"]
cache["key"] # KeyError: key

# cachebox.Cache does not have any policy, so will raise OverflowError if reached the bound.
cache.update({i:i for i in range(200)})
# OverflowError: The cache has reached the bound.
```

* * *

### *class* FIFOCache
FIFO Cache implementation - First-In First-Out Policy (thread-safe).

In simple terms, the FIFO cache will remove the element that has been in the cache the longest.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)  | O(1)    | O(min(i, n-i))    | O(1)    |

```python
from cachebox import FIFOCache

cache = FIFOCache(5, {i:i*2 for i in range(5)})

print(len(cache)) # 5
cache["new-key"] = "new-value"
print(len(cache)) # 5

print(cache.get(3, "default-val")) # 6
print(cache.get(6, "default-val")) # default-val

print(cache.popitem()) # (1, 2)

# insert method returns a value:
# - If the cache did not have this key present, None is returned.
# - If the cache did have this key present, the value is updated, and the old value is returned.
print(cache.insert(3, "val")) # 6
print(cache.insert("new-key", "val")) # None

# Returns the first key in cache; this is the one which will be removed by `popitem()`.
print(cache.first())
```

* * *

### *class* RRCache
RRCache implementation - Random Replacement policy (thread-safe).

In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.

|              | get   | insert  | delete | popitem |
| ------------ | ----- | ------- | ------ | ------- |
| Worse-case   | O(1)  | O(1)    | O(1)   | O(1)~   |

```python
from cachebox import RRCache

cache = RRCache(10, {i:i for i in range(10)})
print(cache.is_full()) # True
print(cache.is_empty()) # False

# Returns the number of elements the map can hold without reallocating.
print(cache.capacity()) # 28

# Shrinks the cache to fit len(self) elements.
cache.shrink_to_fit()
print(cache.capacity()) # 10

print(len(cache)) # 10
cache.clear()
print(len(cache)) # 0
```

* * *

### *class* TTLCache
TTL Cache implementation - Time-To-Live Policy (thread-safe).

In simple terms, the TTL cache will automatically remove the element in the cache that has expired.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(n) |

```python
from cachebox import TTLCache
import time

# The `ttl` param specifies the time-to-live value for each element in cache (in seconds); cannot be zero or negative.
cache = TTLCache(0, ttl=2)
cache.update({i:str(i) for i in range(10)})

print(cache.get_with_expire(2)) # ('2', 1.99)

# Returns the oldest key in cache; this is the one which will be removed by `popitem()` 
print(cache.first()) # 0

cache["mykey"] = "value"
time.sleep(2)
cache["mykey"] # KeyError
```

* * *

### *class* LRUCache
LRU Cache implementation - Least recently used policy (thread-safe).

In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(1)~ | O(1)~ |

```python
from cachebox import LRUCache

cache = LRUCache(0, {i:i*2 for i in range(10)})

# access `1`
print(cache[0]) # 0
print(cache.popitem()) # (1, 2)

# .peek() searches for a key-value in the cache and returns it without moving the key to recently used.
print(cache.peek(2)) # 4
print(cache.popitem()) # (3, 6)

# Does the `popitem()` `n` times and returns count of removed items.
print(cache.drain(5)) # 5
```

* * *

### *class* LFUCache
LFU Cache implementation - Least frequantly used policy (thread-safe).

In simple terms, the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(n) | O(n) |

```python
from cachebox import LFUCache

cache = cachebox.LFUCache(5)
cache.insert(1, 1)
cache.insert(2, 2)

# access 1 twice
cache[1]
cache[1]

# access 2 once
cache[2]

assert cache.least_frequently_used() == 2
assert cache.least_frequently_used(2) is None # 2 is out of range

for item in cache.items():
    print(item)
# (2, '2')
# (1, '1')
```

> [!TIP]\
> `.items()`, `.keys()`, and `.values()` are ordered (v4.0+)

* * *

### *class* VTTLCache
VTTL Cache implementation - Time-To-Live Per-Key Policy (thread-safe).

In simple terms, the TTL cache will automatically remove the element in the cache that has expired when need.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(n) | O(n) |

```python
from cachebox import VTTLCache
import time

# The `ttl` param specifies the time-to-live value for `iterable` (in seconds); cannot be zero or negative.
cache = VTTLCache(100, iterable={i:i for i in range(4)}, ttl=3)
print(len(cache)) # 4
time.sleep(3)
print(len(cache)) # 0

# The "key1" is exists for 5 seconds
cache.insert("key1", "value", ttl=5)
# The "key2" is exists for 2 seconds
cache.insert("key2", "value", ttl=2)

time.sleep(2)
# "key1" is exists for 3 seconds
print(cache.get("key1")) # value

# "key2" has expired
print(cache.get("key2")) # None
```

> [!TIP]
> **`VTTLCache` vs `TTLCache`:**
> - In `VTTLCache` each item has its own unique time-to-live, unlike `TTLCache`.
> - `VTTLCache` is generally slower than `TTLCache`.

* * *

### *class* Frozen
**This is not a cache.** this class can freeze your caches and prevents changes ❄️.

```python
from cachebox import Frozen, FIFOCache

cache = FIFOCache(10, {1:1, 2:2, 3:3})

# parameters:
#   cls: your cache
#   ignore: If False, will raise TypeError if anyone try to change cache. will do nothing otherwise.
frozen = Frozen(cache, ignore=True)
print(frozen[1]) # 1
print(len(frozen)) # 3

# Frozen ignores this action and do nothing
frozen.insert("key", "value")
print(len(frozen)) # 3

# Let's try with ignore=False
frozen = Frozen(cache, ignore=False)

frozen.insert("key", "value")
# TypeError: This cache is frozen.
```

> [!NOTE]\
> The **Frozen** class can't prevent expiring in [TTLCache](#ttlcache) or [VTTLCache](#vttlcache).
>
> For example:
> ```python
> cache = TTLCache(0, ttl=3, iterable={i:i for i in range(10)})
> frozen = Frozen(cache)
> 
> time.sleep(3)
> print(len(frozen)) # 0
> ```

## Incompatible changes
These are changes that are not compatible with the previous version:

**You can see more info about changes in [Changelog](CHANGELOG.md).**

* * *

#### Pickle serializing changed!
If you try to load bytes that has dumped by pickle in previous version, you will get `TypeError` exception.
There's no way to fix that 💔, but it's worth it.

```python
import pickle

with open("old-version.pickle", "rb") as fd:
    pickle.load(fd) # TypeError: ...
```

* * *

#### Iterators changed!
In previous versions, the iterators are not ordered; but now all of iterators are ordered.
this means all of `.keys()`, `.values()`, `.items()`, and `iter(cache)` methods are ordered now.

For example:
```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=4)
for i in range(4):
    cache[i] = str(i)

for key in cache:
    print(key)
# 0
# 1
# 2
# 3
```

* * *

#### `.insert()` method changed!
In new version, the `.insert()` method has a small change that can help you in coding.

`.insert()` equals to `self[key] = value`, but:
- If the cache did not have this key present, **None is returned**.
- If the cache did have this key present, the value is updated,
and **the old value is returned**. The key is not updated, though;

For example:
```python
from cachebox import LRUCache

lru = LRUCache(10, {"a": "b", "c": "d"})

print(lru.insert("a", "new-key")) # "b"
print(lru.insert("no-exists", "val")) # None
```

## Tips and Notes
#### How to save caches in files?
there's no built-in file-based implementation, but you can use `pickle` for saving caches in files. For example:
```python
import cachebox
import pickle
c = cachebox.LRUCache(100, {i:i for i in range(78)})

with open("file", "wb") as fd:
    pickle.dump(c, fd)

with open("file", "rb") as fd:
    loaded = pickle.load(fd)

assert c == loaded
assert c.capacity() == loaded.capacity()
```

> [!TIP]\
> For more, see this [issue](https://github.com/awolverp/cachebox/issues/8).

> [!NOTE]\
> Supported since version 3.1.0

* * *

#### How to copy the caches?
Use `copy.deepcopy` or `copy.copy` for copying caches. For example:
```python
import cachebox, copy
c = cachebox.LRUCache(100, {i:i for i in range(78)})

copied = copy.copy(c)

assert c == copied
assert c.capacity() == copied.capacity()
```

> [!NOTE]\
> Supported since version 3.1.0

## License
This repository is licensed under the [MIT License](LICENSE)
