<h1 align=center>
    cachebox
</h1>

<p align=center>
    <a href="CHANGELOG.md">Changelog</a> . <a href="https://github.com/awolverp/cachebox/releases">Releases</a>
    . <a href="APIReference.md">API Reference</a>
</p>

<p align=center>
    <em><b>The fastest caching Python library written in Rust</b></em>
</p>

<p align=center>
    <a href="https://github.com/awolverp/cachebox/issues/new">Did you find any bugs?</a>
</p>

> [!NOTE]\
> The new version v3 has some incompatible with v2, for more info please see [Incompatible changes](#incompatible-changes)

**What does it do?** \
You can easily and powerfully perform caching operations in Python as fast as possible.

**Features**:
- 🚀 5-20x faster than other caching libraries
- 📊 Very low memory usage (1/3 of dictionary)
- 🔥 Full-feature and easy-to-use
- **(R)** written in Rust with high-performance
- 🤝 Support Python 3.8 and above (PyPy & CPython)
- 📦 Over 7 cache algorithms are supported
- 🧶 Completely thread-safe (uses `RwLock`)

## Installing
Install it from PyPi:
```sh
pip3 install -U cachebox
```

## Page Contents
- ⁉️ [When i need caching?](#when-i-need-caching)
- 🤷‍♂️ [Why `cachebox`?](#why-cachebox)
- 🎓 [Examples](#examples)
- 💡 [Incompatible changes](#incompatible-changes)
- ⁉️ [Frequently Asked Questions](#faq)
- ⏱️ [*BENCHMARK*](https://github.com/awolverp/cachebox-benchmark)

## When i need caching?
There are some situations that you may need caching to improve your application speed:

1. Sometimes you have functions that take a long time to execute, and you need to call them each time.

2. Sometimes you need to temporarily store data in memory for a short period.

3. When dealing with remote APIs, Instead of making frequent API calls, store the responses in a cache.

4. Caching query results from databases can enhance performance.

5. and ...

### Why `cachebox`?
**Rust** - It uses *Rust* language to has high-performance.

**SwissTable** - It uses Google's high-performance SwissTable hash map. thanks to [hashbrown](https://github.com/rust-lang/hashbrown).

**Low memory usage** - It has very low memory usage.

**Zero-Dependecy** - As we said, `cachebox` written in Rust so you don't have to install any other dependecies.

**Thread-safe** - It's completely thread-safe and uses read-writer locks to prevent problems.

**Easy-To-Use** - You only need to import it and choice your implementation to use and behave with it like a dictionary.

## Examples

> [!TIP]\
> See [API Reference](APIReference.md) for more examples and references

**decorators** example:
```python
import cachebox

@cachebox.cached(cachebox.FIFOCache(maxsize=128))
def factorial(n):
    return n * factorial(n-1) if n else 1

# Like functools.lru_cache, If maxsize is set to 0, the cache can grow without bound and limit.
@cachebox.cached(cachebox.LRUCache(maxsize=0))
def count_vowels(sentence):
    return sum(sentence.count(vowel) for vowel in 'AEIOUaeiou')

# Async are also supported
@cachebox.cached(cachebox.TTLCache(maxsize=20, ttl=5))
async def get_coin_price(coin):
    return await client.get_coin_price(coin)

class Application:
    # Use cachedmethod for methods
    @cachebox.cachedmethod(cachebox.LFUCache(maxsize=20))
    def send(self, request):
        self._send_request(request)
```

**Implementations** example:
```python
import cachebox
import time

cache = cachebox.TTLCache(maxsize=5, ttl=3)
cache.insert("key", "value")
print(cache["key"]) # Output: value

time.sleep(3)
print(cache.get("key")) # Output: None
```

## Incompatible changes
These are changes that are not compatible with the previous version:

> [!NOTE]\
> You can see more info about changes in [Changelog](CHANGELOG.md).

#### Maxsize default value changed!
The change applied is that when you pass `0` as maxsize, the value of `sys.maxsize` is automatically used.

```python
import cachebox, sys
c = cachebox.Cache(0)

# In previous version:
assert c.maxsize == 0

# In new version:
assert c.maxsize == sys.maxsize
```

#### Iterators changed!
The change applied is that, in previous version you may make changes in cache after abtaining an iterator
from it and that did not cause an error, but now you cannot make changes in cache while using the iterator.

```python
import cachebox

c = cachebox.Cache(0, {i:i for i in range(100)})

for (key, value) in c.items():
    # This will raise RuntimeError, don't make changes
    del c[key]
```

#### Type-hint is now better!
In previous version, we couldn't use type-hints as possible as dictionary; now we can:

```python
import cachebox

# In previous version this will raises an exception; but now is OK.
c: cachebox.Cache[int, str] = cachebox.Cache(0)
```

#### Cache iterators are not ordered!
In previous versions, some caches such as `FIFOCache` can return ordered iterators, but now all of them
only can return unordered iterators.

```python
import cachebox

c = cachebox.FIFOCache(20)
for i in range(10):
    c.insert(i, i)

for key in c:
    print(key)
# (5, 5)
# (9, 9)
# (0, 0)
# ...
```

#### \_\_repr\_\_ changed to \_\_str\_\_
We changed the `__repr__` method to `__str__`:
```python
import cachebox
c = cachebox.Cache(0)

print(c)
# Output: Cache(0 / 9223372036854775807, capacity=0)

print(repr(c))
# Output: <cachebox._cachebox.Cache object at 0x7f96938f06a0>
```

## FAQ

<details>
    <summary><b>How do I preserve order while iterating?</b></summary>

On default, `.items()`, `.keys()` and `.values()` methods are unordered, so you have to do some more works to
have a ordered iteration.

For `FIFOCache`: [See here](APIReference.md#cacheboxfifocacheitems)\
For `LFUCache`: [See here](APIReference.md#cacheboxlfucacheitems)\
For `LRUCache`: [See here](APIReference.md#cacheboxlrucacheitems)\
For `TTLCache`: [See here](APIReference.md#cacheboxttlcacheitems)

> **NOTE**: Added in version 3.3.0

</details>

<details>
    <summary><b>How to migrate from cachetools to cachebox?</b></summary>

*cachebox* syntax is very similar to *cachetools*.
Just change these:
```python
# If you pass infinity to a cache implementation, change it to zero.
cachetools.Cache(math.inf) -> cachebox.Cache(0)
# If you use `isinstance` for cachetools classes, change those.
isinstance(cache, cachetools.Cache) -> isinstance(cache, cachebox.BaseCacheImpl)
```
</details>

<details>
    <summary><b>How to save caches in file?</b></summary>

there's no file-based implementation, but you can use `pickle` for saving caches in files. For example:
```python
import cachebox, pickle
c = cachebox.LRUCache(100, {i:i for i in range(78)})

with open("file", "wb") as fd:
    pickle.dump(c, fd)

with open("file", "rb") as fd:
    loaded = pickle.load(fd)

assert c == loaded
assert c.capacity() == loaded.capacity()
```

> **NOTE**: Added in version 3.1.0

</details>

<details>
    <summary><b>How to copy the caches?</b></summary>

Use `copy.deepcopy` for copying caches. For example:
```python
import cachebox, copy
c = cachebox.LRUCache(100, {i:i for i in range(78)})

copied = copy.deepcopy(c)

assert c == copied
assert c.capacity() == copied.capacity()
```

> **NOTE**: Added in version 3.1.0

</details>

<details>
    <summary><b>How to save caches before exiting the app?</b></summary>

You can use `atexit` (or also `signal`) and `pickle` module to do it.

For example:
```python
import cachebox, atexit, pickle
cache = cachebox.TTLCache(50, 10)

def _save_cache(c, filename):
    with open(filename, "wb") as fd:
        pickle.dump(c, fd)

atexit.register(_save_cache, cache, "cache.pickle")
```

> **NOTE**: Added in version 3.1.0

</details>

## License
cachebox is provided under the MIT license. See [LICENSE](LICENSE).

## Future Plans
TODO List:
- [x] Rewrite all cache algorithms and use low-level API hashmap
- [x] Change hashing system
- [x] Improve tests
- [x] Rewrite stub-file (`.pyi`)
- [x] Rewrite README.md
- [x] Write an API referenece
- [ ] Add new functions such as `cached_property`.
- [x] Add possible methods to implementations.
- [x] Make better type-hint for `cached` and `cachedmethod` (if possible).
