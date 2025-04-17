
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
        <img src="https://img.shields.io/pypi/pyversions/cachebox.svg" alt="Python Versions">
    </a>
    <a href="https://pepy.tech/projects/cachebox">
        <img src="https://static.pepy.tech/badge/cachebox" alt="Downloads">
    </a>
</p>    

-------

### What does it do?
You can easily and powerfully perform caching operations in Python as fast as possible.
This can make your application very faster and it's a good choice in big applications.
**Ideal for optimizing large-scale applications** with efficient, low-overhead caching.

**Key Features:**
- 🚀 Extremely fast (10-50x faster than other caching libraries - [benchmarks](https://github.com/awolverp/cachebox-benchmark))
- 📊 Minimal memory footprint (50% of standard dictionary memory usage)
- 🔥 Full-featured and user-friendly
- 🧶 Completely thread-safe
- 🔧 Tested and correct
- **\[R\]** written in Rust for maximum performance
- 🤝 Compatible with Python 3.8+ (PyPy and CPython)
- 📦 Supports 7 advanced caching algorithms

### Page Contents
- [❓ **When i need caching and cachebox**](#when-i-need-caching-and-cachebox)
- [🌟 **Why `cachebox`**](#why-cachebox)
- [🔧 **Installation**](#installation)
- [💡 **Preview**](#example)
- [🎓 **Learn**](#learn)
- [✏️ **Incompatible changes**](#incompatible-changes)
- [📌 **Tips & Notes**](#tips-and-notes)

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

## Example
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

## Learn
There are 3 useful functions:
- [**cached**](#decorator-cached): a decorator that helps you to cache your functions and calculations with a lot of options.
- [**cachedmethod**](#decorator-cachedmethod): this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.
- [**is_cached**](#function-is_cached): check if a function/method cached by cachebox or not

And 9 classes:
- [**BaseCacheImpl**](#class-basecacheimpl): base-class for all classes.
- [**Cache**](#class-cache): A simple cache that has no algorithm; this is only a hashmap.
- [**FIFOCache**](#class-fifocache): the FIFO cache will remove the element that has been in the cache the longest.
- [**RRCache**](#class-rrcache): the RR cache will choice randomly element to remove it to make space when necessary.
- [**LRUCache**](#class-lrucache): the LRU cache will remove the element in the cache that has not been accessed in the longest time.
- [**LFUCache**](#class-lfucache): the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.
- [**TTLCache**](#class-ttlcache): the TTL cache will automatically remove the element in the cache that has expired.
- [**VTTLCache**](#class-vttlcache): the TTL cache will automatically remove the element in the cache that has expired when need.
- [**Frozen**](#class-frozen): you can use this class for freezing your caches.


### Decorator `cached`
Decorator to wrap a function with a memoizing callable that saves results in a cache.

<details>
<summary><b>Parameters</b></summary>

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

</details>

<details>
<summary><b>Examples</b></summary>


**A simple example:**
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
# CacheInfo(hits=0, misses=0, maxsize=9223372036854775807, length=0, memory=8)

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

### Decorator `cachedmethod`
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


### Function `is_cached`
Checks that a function/method is cached by cachebox or not.

<details>
<summary><b>Parameters</b></summary>

- `func`: The function/method to check.

</details>

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
