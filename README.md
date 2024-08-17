# cachebox

[![image](https://img.shields.io/pypi/v/cachebox.svg)](https://pypi.python.org/pypi/cachebox)
[![image](https://img.shields.io/pypi/l/cachebox.svg)](https://github.com/astral-sh/cachebox/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/cachebox.svg)](https://pypi.python.org/pypi/cachebox)
[![image](https://static.pepy.tech/badge/cachebox)](https://pypi.python.org/pypi/cachebox)

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

# Unlike functools.lru_cache and other caching libraries, cachebox will copy dict, list, and set results.
@cachebox.cached(cachebox.LRUCache(maxsize=128))
def make_dict(name: str, age: int) -> dict:
    return {"name": name, "age": age}

d = make_dict("cachebox", 10)
assert d == {"name": "cachebox", "age": 10}
d["new-key"] = "new-value"
d2 = make_dict("cachebox", 10)
# `d2` will be `{"name": "cachebox", "age": 10, "new-key": "new-value"}` if you use other libraries
assert d2 == {"name": "cachebox", "age": 10}

# Async are also supported
@cachebox.cached(cachebox.LRUCache(maxsize=128))
async def make_request(method: str, url: str) -> dict:
    response = await client.request(method, url)
    return response.json()
```

## Learn
There are 9 implementation:
- **BaseCacheImpl**: base-class for all classes.
- **Cache**: A simple cache that has no algorithm; this is only a hashmap.
- **FIFOCache**: the FIFO cache will remove the element that has been in the cache the longest.
- **RRCache**: the RR cache will choice randomly element to remove it to make space when necessary.
- **TTLCache**: the TTL cache will automatically remove the element in the cache that has expired.
- **LRUCache**: the LRU cache will remove the element in the cache that has not been accessed in the longest time.
- **LFUCache**: the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.
- **VTTLCache**: the TTL cache will automatically remove the element in the cache that has expired when need.
- **Frozen**: you can use this class for freezing your caches.

Using this library is very easy and you only need to import cachebox and then use these classes like a dictionary (or use its decorator such as `cached` and `cachedmethod`).

There are some examples for you with different methods for introducing those. for more, please see [API Reference](APIReference.md).

> [!NOTE]\
> All the methods you will see in the examples are common across all classes (except for a few of them).

* * *

### BaseCacheImpl
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

### Cache
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

### FIFOCache
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

### RRCache
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

### TTLCache
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

### LRUCache
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

### LFUCache
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

### VTTLCache
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

### Frozen
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

## Incompatible changes
These are changes that are not compatible with the previous version:

> [!NOTE]\
> You can see more info about changes in [Changelog](CHANGELOG.md).

* * *

#### Pickle serializing changed!
If you try to load bytes that has dumped by pickle in previous version, you will get `TypeError` exception.
There's no way to fix that 💔, but it's worth it.

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
