
<h1 align=center>
  Cachebox
</h1>
<p align="center">
    <em>The fastest caching Python library written in Rust</em>
</p>
<p align="center">
    <a href="https://github.com/awolverp/cachebox/releases"><b>Releases</b></a> | <a href="https://github.com/awolverp/cachebox-benchmark" target="_blank"><b>Benchmarks</b></a> | <a href="https://github.com/awolverp/cachebox/issues/new"><b>Issues</b></a>
</p>
<p align="center">
    <a href="https://github.com/awolverp/cachebox/blob/main/LICENSE">
        <img src="https://img.shields.io/github/license/awolverp/cachebox.svg?style=flat-square" alt="License">
    </a>
    <a href="https://github.com/awolverp/cachebox/releases">
        <img src="https://img.shields.io/github/v/release/awolverp/cachebox.svg?style=flat-square" alt="Release">
    </a>
    <a href="https://pypi.org/project/cachebox/">
        <img src="https://img.shields.io/pypi/pyversions/cachebox.svg?style=flat-square" alt="Python Versions">
    </a>
    <a href="https://pepy.tech/projects/cachebox">
        <img src="https://img.shields.io/pypi/dm/cachebox?style=flat-square&color=%23314bb5" alt="Downloads">
    </a>
</p>    

-------

### What does it do?
You can easily and powerfully perform caching operations in Python as fast as possible.
This can make your application very faster and it's a good choice in big applications.
**Ideal for optimizing large-scale applications** with efficient, low-overhead caching.

**Key Features:**
- 🚀 Extremely fast (10-50x faster than other caching libraries -- [*benchmarks*](https://github.com/awolverp/cachebox-benchmark))
- 📊 Minimal memory footprint (50% of standard dictionary memory usage)
- 🔥 Full-featured and user-friendly
- 🧶 Completely thread-safe
- 🔧 Tested and correct
- **\[R\]** written in Rust for maximum performance
- 🤝 Compatible with Python 3.9+ (PyPy and CPython)
- 📦 Supports 7 advanced caching algorithms

### Page Contents
- ❓ [**When i need caching and cachebox**](#when-i-need-caching-and-cachebox)
- 🌟 [**Why `cachebox`**](#why-cachebox)
- 🔧 [**Installation**](#installation)
- 💡 [**Preview**](#examples)
- 🎓 [**Getting started**](#getting-started)
- ✏️ [**Incompatible changes**](#%EF%B8%8F-incompatible-changes)
- 📌 [**Tips & Notes**](#tips-and-notes)

### When i need caching and cachebox
- 📈 **Frequently Data Access** \
  If you need to access the same data multiple times, caching can help reduce the number of database queries or API calls, improving performance.

- 💎 **Expensive Operations** \
  If you have operations that are computationally expensive, caching can help reduce the number of times these operations need to be performed.

- 🚗 **High Traffic Scenarios** \
  If your application has high user traffic, caching can help reduce the load on your server by reducing the number of requests that need to be processed.

- #️⃣ **Web Page Rendring** \
  If you are rendering web pages, caching can help reduce the time it takes to generate the page by caching the results of expensive operations. Caching HTML pages can speed up the delivery of static content.

- 🚧 **Rate Limiting** \
  If you have a rate limiting system in place, caching can help reduce the number of requests that need to be processed by the rate limiter. Also, caching can help you to manage rate limits imposed by third-party APIs by reducing the number of requests sent.

- 🤖 **Machine Learning Models** \
  If your application frequently makes predictions using the same input data, caching the results can save computation time.

### Why cachebox?
- **⚡ Rust** \
It uses *Rust* language to has high-performance.

- **🧮 SwissTable** \
It uses Google's high-performance SwissTable hash map. thanks to [hashbrown](https://github.com/rust-lang/hashbrown).

- **✨ Low memory usage** \
It has very low memory usage.

- **⭐ Zero Dependency** \
As we said, `cachebox` written in Rust so you don't have to install any other dependecies.

- **🧶 Thread safe** \
It's completely thread-safe and uses locks to prevent problems.

- **👌 Easy To Use** \
You only need to import it and choice your implementation to use and behave with it like a dictionary.

- **🚫 Avoids Cache Stampede** \
It avoids [cache stampede](https://en.wikipedia.org/wiki/Cache_stampede) by using a distributed lock system.


## Installation
cachebox is installable by `pip`:
```bash
pip3 install -U cachebox
```

> [!WARNING]\
> The new version v5 has some incompatible with v4, for more info please see [Incompatible changes](#incompatible-changes)

## Examples
The simplest example of **cachebox** could look like this:
```python
import cachebox

# Like functools.lru_cache, If maxsize is set to 0, the cache can grow without bound and limit.
@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(number: int) -> int:
    fact = 1
    for num in range(2, number + 1):
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

Also, unlike functools.lru_cache and other caching libraries, cachebox can copy `dict`, `list`, and `set` objects.
```python
@cachebox.cached(cachebox.LRUCache(maxsize=128))
def make_dict(name: str, age: int) -> dict:
   return {"name": name, "age": age}
>
d = make_dict("cachebox", 10)
assert d == {"name": "cachebox", "age": 10}
d["new-key"] = "new-value"

d2 = make_dict("cachebox", 10)
# `d2` will be `{"name": "cachebox", "age": 10, "new-key": "new-value"}` if you use other libraries
assert d2 == {"name": "cachebox", "age": 10}
```

You can use cache alghoritms without `cached` decorator -- just import what cache alghoritms you want and use it like a dictionary.
```python
from cachebox import FIFOCache

cache = FIFOCache(maxsize=128)
cache["key"] = "value"
assert cache["key"] == "value"

# You can also use `cache.get(key, default)`
assert cache.get("key") == "value"
```

## Getting started
There are 3 useful functions:
- [**cached**](#cached--decorator): a decorator that helps you to cache your functions and calculations with a lot of options.
- [**cachedmethod**](#cachedmethod--decorator): this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.
- [**is_cached**](#is_cached--function): check if a function/method cached by cachebox or not

And 9 classes:
- [**BaseCacheImpl**](#basecacheimpl-️-class): base-class for all classes.
- [**Cache**](#cache-️-class): A simple cache that has no algorithm; this is only a hashmap.
- [**FIFOCache**](#fifocache-️-class): the FIFO cache will remove the element that has been in the cache the longest.
- [**RRCache**](#rrcache-️-class): the RR cache will choice randomly element to remove it to make space when necessary.
- [**LRUCache**](#lrucache-️-class): the LRU cache will remove the element in the cache that has not been accessed in the longest time.
- [**LFUCache**](#lfucache-️-class): the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.
- [**TTLCache**](#ttlcache-️-class): the TTL cache will automatically remove the element in the cache that has expired.
- [**VTTLCache**](#vttlcache-️-class): the TTL cache will automatically remove the element in the cache that has expired when need.
- [**Frozen**](#frozen-️-class): you can use this class for freezing your caches.

You only need to import the class which you want, and behave with it like a dictionary (except for [VTTLCache](#vttlcache-️-class), this have some differences)

There are some examples for you with different methods for introducing those.
**All the methods you will see in the examples are common across all classes (except for a few of them).**

* * *

### `cached` (🎀 decorator)
Decorator to wrap a function with a memoizing callable that saves results in a cache.

**Parameters:**
- `cache`: Specifies a cache that handles and stores the results. if `None` or `dict`, `FIFOCache` will be used.

- `key_maker`: Specifies a function that will be called with the same positional and keyword
               arguments as the wrapped function itself, and which has to return a suitable
               cache key (must be hashable).

- `clear_reuse`: The wrapped function has a function named `clear_cache` that uses `cache.clear`
                 method to clear the cache. This parameter will be passed to cache's `clear` method.

- `callback`: Every time the `cache` is used, callback is also called.
              The callback arguments are: event number (see `EVENT_MISS` or `EVENT_HIT` variables), key, and then result.

- `copy_level`: The wrapped function always copies the result of your function and then returns it.
                This parameter specifies that the wrapped function has to copy which type of results.
                `0` means "never copy", `1` means "only copy `dict`, `list`, and `set` results" and
                `2` means "always copy the results".

<details>
<summary><b>Examples</b></summary>


A simple example:
```python
import cachebox

@cachebox.cached(cachebox.LRUCache(128))
def sum_as_string(a, b):
    return str(a+b)

assert sum_as_string(1, 2) == "3"

assert len(sum_as_string.cache) == 1
sum_as_string.cache_clear()
assert len(sum_as_string.cache) == 0
```

A key_maker example:
```python
import cachebox

def simple_key_maker(args: tuple, kwds: dict):
    return args[0].path

# Async methods are supported
@cachebox.cached(cachebox.LRUCache(128), key_maker=simple_key_maker)
async def request_handler(request: Request):
    return Response("hello man")
```

A typed key_maker example:
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
# CacheInfo(hits=0, misses=0, maxsize=9223372036854775807, length=0, memory=8)

# `.cache_clear()` clears the cache
sum_as_string.cache_clear()
```

callback example: *(Added in v4.2.0)*
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

</details>


> [!NOTE]\
> Recommended use `cached` method for **@staticmethod**s and use [`cachedmethod`](#function-cachedmethod) for **@classmethod**s;
> And set `copy_level` parameter to `2` on **@classmethod**s.
> ```python
> class MyClass:
>   def __init__(self, num: int) -> None:
>       self.num = num
>
>   @classmethod
>   @cachedmethod({}, copy_level=2)
>   def class_func(cls, num: int):
>       return cls(num)
>
>   @staticmethod
>   @cached({})
>   def static_func(num: int):
>       return num * 5
> ```

> [!TIP]\
> There's a new feature **since `v4.1.0`** that you can tell to a cached function that don't use cache for a call:
> ```python
> # with `cachebox__ignore=True` parameter, cachebox does not use cache and only calls the function and returns its result.
> sum_as_string(10, 20, cachebox__ignore=True)
> ```

* * *

### `cachedmethod` (🎀 decorator)
this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.

<details>
<summary><b>Example</b></summary>

```python
import cachebox

class MyClass:
    @cachebox.cachedmethod(cachebox.TTLCache(0, ttl=10))
    def my_method(self, name: str):
        return "Hello, " + name + "!"

c = MyClass()
c.my_method()
```

</details>

* * *

### `is_cached` (📦 function)
Checks that a function/method is cached by cachebox or not.

**Parameters:**
- `func`: The function/method to check.

<details>
<summary><b>Example</b></summary>

```python
import cachebox

@cachebox.cached(cachebox.FIFOCache(0))
def func():
    pass

assert cachebox.is_cached(func)
```

</details>

* * *

### `BaseCacheImpl` (🏗️ class)
Base implementation for cache classes in the cachebox library.

This abstract base class defines the generic structure for cache implementations,
supporting different key and value types through generic type parameters.
Serves as a foundation for specific cache variants like Cache and FIFOCache.

<details>
<summary><b>Example</b></summary>

```python
import cachebox

# subclass
class ClassName(cachebox.BaseCacheImpl):
    ...

# type-hint
def func(cache: BaseCacheImpl):
    ...

# isinstance
cache = cachebox.LFUCache(0)
assert isinstance(cache, cachebox.BaseCacheImpl)
```

</details>

* * *

### `Cache` (🏗️ class)
A thread-safe, memory-efficient hashmap-like cache with configurable maximum size.

Provides a flexible key-value storage mechanism with:
- Configurable maximum size (zero means unlimited)
- Lower memory usage compared to standard dict
- Thread-safe operations
- Useful memory management methods

Supports initialization with optional initial data and capacity,
and provides dictionary-like access with additional cache-specific operations.

> [!TIP]\
> Differs from standard `dict` by:
> - it is thread-safe and unordered, while dict isn't thread-safe and ordered (Python 3.6+).
> - it uses very lower memory than dict.
> - it supports useful and new methods for managing memory, while dict does not.
> - it does not support popitem, while dict does.
> - You can limit the size of Cache, but you cannot for dict.

|              | get   | insert  | delete | popitem |
| ------------ | ----- | ------- | ------ | ------- |
| Worse-case   | O(1)  | O(1)    | O(1)   | N/A     |

<details>
<summary><b>Example</b></summary>

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

</details>

* * *

### `FIFOCache` (🏗️ class)
A First-In-First-Out (FIFO) cache implementation with configurable maximum size and optional initial capacity.

This cache provides a fixed-size container that automatically removes the oldest items when the maximum size is reached.

**Key features**:
- Deterministic item eviction order (oldest items removed first)
- Efficient key-value storage and retrieval
- Supports dictionary-like operations
- Allows optional initial data population

|              | get   | insert  | delete       | popitem |
| ------------ | ----- | ------- | ------------- | ------- |
| Worse-case   | O(1)  | O(1) | O(min(i, n-i)) | O(1)  |

<details>
<summary><b>Example</b></summary>

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

</details>

* * *

### `RRCache` (🏗️ class)
A thread-safe cache implementation with Random Replacement (RR) policy.

This cache randomly selects and removes elements when the cache reaches its maximum size,
ensuring a simple and efficient caching mechanism with configurable capacity.

Supports operations like insertion, retrieval, deletion, and iteration with O(1) complexity.

|              | get   | insert  | delete | popitem |
| ------------ | ----- | ------- | ------ | ------- |
| Worse-case   | O(1)  | O(1)    | O(1)   | O(1)    |

<details>
<summary><b>Example</b></summary>

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

# Returns a random key
print(cache.random_key()) # 4
```

</details>

* * *

### `LRUCache` (🏗️ class)
Thread-safe Least Recently Used (LRU) cache implementation.

Provides a cache that automatically removes the least recently used items when
the cache reaches its maximum size. Supports various operations like insertion,
retrieval, and management of cached items with configurable maximum size and
initial capacity.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(1)~ | O(1)~ |

<details>
<summary><b>Example</b></summary>

```python
from cachebox import LRUCache

cache = LRUCache(0, {i:i*2 for i in range(10)})

# access `1`
print(cache[0]) # 0
print(cache.least_recently_used()) # 1
print(cache.popitem()) # (1, 2)

# .peek() searches for a key-value in the cache and returns it without moving the key to recently used.
print(cache.peek(2)) # 4
print(cache.popitem()) # (3, 6)

# Does the `popitem()` `n` times and returns count of removed items.
print(cache.drain(5)) # 5
```

</details>

* * *

### `LFUCache` (🏗️ class)
A thread-safe Least Frequently Used (LFU) cache implementation.

This cache removes elements that have been accessed the least number of times,
regardless of their access time. It provides methods for inserting, retrieving,
and managing cache entries with configurable maximum size and initial capacity.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(1)~ |

<details>
<summary><b>Example</b></summary>

```python
from cachebox import LFUCache

cache = cachebox.LFUCache(5)
cache.insert('first', 'A')
cache.insert('second', 'B')

# access 'first' twice
cache['first']
cache['first']

# access 'second' once
cache['second']

assert cache.least_frequently_used() == 'second'
assert cache.least_frequently_used(2) is None # 2 is out of range

for item in cache.items_with_frequency():
    print(item)
# ('second', 'B', 1)
# ('first', 'A', 2)
```

</details>

* * *

### `TTLCache` (🏗️ class)
A thread-safe Time-To-Live (TTL) cache implementation with configurable maximum size and expiration.

This cache automatically removes elements that have expired based on their time-to-live setting.
Supports various operations like insertion, retrieval, and iteration.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(n) |

<details>
<summary><b>Example</b></summary>

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

</details>

* * *

### `VTTLCache` (🏗️ class)
A thread-safe, time-to-live (TTL) cache implementation with per-key expiration policy.

This cache allows storing key-value pairs with optional expiration times. When an item expires,
it is automatically removed from the cache. The cache supports a maximum size and provides
various methods for inserting, retrieving, and managing cached items.

Key features:
- Per-key time-to-live (TTL) support
- Configurable maximum cache size
- Thread-safe operations
- Automatic expiration of items

Supports dictionary-like operations such as get, insert, update, and iteration.

|              | get   | insert  | delete(i) | popitem |
| ------------ | ----- | ------- | --------- | ------- |
| Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(1)~ |

> [!TIP]\
> `VTTLCache` vs `TTLCache`:
> - In `VTTLCache` each item has its own unique time-to-live, unlike `TTLCache`.
> - `VTTLCache` is generally slower than `TTLCache`.

<details>
<summary><b>Example</b></summary>

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

</details>

* * *

### `Frozen` (🏗️ class)
**This is not a cache**; This is a wrapper class that prevents modifications to an underlying cache implementation.

This class provides a read-only view of a cache, optionally allowing silent
suppression of modification attempts instead of raising exceptions.

<details>
<summary><b>Example</b></summary>

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

</details>

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

## ⚠️ Incompatible Changes
These are changes that are not compatible with the previous version:

**You can see more info about changes in [Changelog](CHANGELOG.md).**

#### CacheInfo's cachememory attribute renamed!
The `CacheInfo.cachememory` was renamed to `CacheInfo.memory`.

```python
@cachebox.cached({})
def func(a: int, b: int) -> str:
    ...

info = func.cache_info()

# Older versions
print(info.cachememory)

# New version
print(info.memory)
```

#### Errors in the `__eq__` method will not be ignored!
Now the errors which occurred while doing `__eq__` operations will not be ignored.

```python
class A:
    def __hash__(self):
        return 1

    def __eq__(self, other):
        raise NotImplementedError("not implemeneted")

cache = cachebox.FIFOCache(0, {A(): 10})

# Older versions:
cache[A()] # => KeyError

# New version:
cache[A()]
# Traceback (most recent call last):
# File "script.py", line 11, in <module>
#    cache[A()]
#    ~~~~~^^^^^
#  File "script.py", line 7, in __eq__
#   raise NotImplementedError("not implemeneted")
# NotImplementedError: not implemeneted
```

#### Cache comparisons will not be strict!
In older versions, cache comparisons depended on the caching algorithm. Now, they work just like dictionary comparisons.

```python
cache1 = cachebox.FIFOCache(10)
cache2 = cachebox.FIFOCache(10)

cache1.insert(1, 'first')
cache1.insert(2, 'second')

cache2.insert(2, 'second')
cache2.insert(1, 'first')

# Older versions:
cache1 == cache2 # False

# New version:
cache1 == cache2 # True
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

* * *

#### How to copy the caches?
You can use `copy.deepcopy` or `cache.copy` for copying caches. For example:
```python
import cachebox
cache = cachebox.LRUCache(100, {i:i for i in range(78)})

# shallow copy
shallow = cache.copy()

# deep copy
import copy
deep = copy.deepcopy(cache)
```

## License
This repository is licensed under the [MIT License](LICENSE)
