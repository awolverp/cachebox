# cachebox: API Reference

All caches are built on `mapping` and support all methods that Python dictionary has.
You can behave with them same as Python dictionary (only for `VTTLCache` is a little different).

**Performances:**

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
| VTTLCache    | O(1)~  | ?               | O(1)~          | ?                | O(1)~   |

**Content**:
- [BaseCacheImpl](#cacheboxbasecacheimpl)

- [Cache](#cacheboxcache)
    - [\_\_init\_\_](#cacheboxcache__init__)
    - [is_full](#cacheboxcacheis_full)
    - [is_empty](#cacheboxcacheis_empty)
    - [insert](#cacheboxcacheinsert)
    - [get](#cacheboxcacheget)
    - [capacity](#cacheboxcachecapacity)
    - [clear](#cacheboxcacheclear)
    - [pop](#cacheboxcachepop)
    - [setdefault](#cacheboxcachesetdefault)
    - [update](#cacheboxcacheupdate)
    - [shrink_to_fit](#cacheboxcacheshrink_to_fit)
    - [items](#cacheboxcacheitems)
    - [keys](#cacheboxcachekeys)
    - [values](#cacheboxcachevalues)

- [FIFOCache](#cacheboxfifocache)
    - [\_\_init\_\_](#cacheboxfifocache__init__)
    - [is_full](#cacheboxfifocacheis_full)
    - [is_empty](#cacheboxfifocacheis_empty)
    - [insert](#cacheboxfifocacheinsert)
    - [get](#cacheboxfifocacheget)
    - [capacity](#cacheboxfifocachecapacity)
    - [clear](#cacheboxfifocacheclear)
    - [pop](#cacheboxfifocachepop)
    - [setdefault](#cacheboxfifocachesetdefault)
    - [update](#cacheboxfifocacheupdate)
    - [shrink_to_fit](#cacheboxfifocacheshrink_to_fit)
    - [items](#cacheboxfifocacheitems)
    - [keys](#cacheboxfifocachekeys)
    - [values](#cacheboxfifocachevalues)
    - [first](#cacheboxfifocachefirst)
    - [last](#cacheboxfifocachelast)

- [LFUCache](#cacheboxlfucache)
    - [\_\_init\_\_](#cacheboxlfucache__init__)
    - [is_full](#cacheboxlfucacheis_full)
    - [is_empty](#cacheboxlfucacheis_empty)
    - [insert](#cacheboxlfucacheinsert)
    - [get](#cacheboxlfucacheget)
    - [capacity](#cacheboxlfucachecapacity)
    - [clear](#cacheboxlfucacheclear)
    - [pop](#cacheboxlfucachepop)
    - [setdefault](#cacheboxlfucachesetdefault)
    - [update](#cacheboxlfucacheupdate)
    - [shrink_to_fit](#cacheboxlfucacheshrink_to_fit)
    - [items](#cacheboxlfucacheitems)
    - [keys](#cacheboxlfucachekeys)
    - [values](#cacheboxlfucachevalues)
    - [least_frequently_used](#cacheboxlfucacheleast_frequently_used)

- [RRCache](#cacheboxrrcache)
    - [\_\_init\_\_](#cacheboxrrcache__init__)
    - [is_full](#cacheboxrrcacheis_full)
    - [is_empty](#cacheboxrrcacheis_empty)
    - [insert](#cacheboxrrcacheinsert)
    - [get](#cacheboxrrcacheget)
    - [capacity](#cacheboxrrcachecapacity)
    - [clear](#cacheboxrrcacheclear)
    - [pop](#cacheboxrrcachepop)
    - [setdefault](#cacheboxrrcachesetdefault)
    - [update](#cacheboxrrcacheupdate)
    - [shrink_to_fit](#cacheboxrrcacheshrink_to_fit)
    - [items](#cacheboxrrcacheitems)
    - [keys](#cacheboxrrcachekeys)
    - [values](#cacheboxrrcachevalues)

- [LRUCache](#cacheboxlrucache)
    - [\_\_init\_\_](#cacheboxlrucache__init__)
    - [is_full](#cacheboxlrucacheis_full)
    - [is_empty](#cacheboxlrucacheis_empty)
    - [insert](#cacheboxlrucacheinsert)
    - [get](#cacheboxlrucacheget)
    - [capacity](#cacheboxlrucachecapacity)
    - [clear](#cacheboxlrucacheclear)
    - [pop](#cacheboxlrucachepop)
    - [setdefault](#cacheboxlrucachesetdefault)
    - [update](#cacheboxlrucacheupdate)
    - [shrink_to_fit](#cacheboxlrucacheshrink_to_fit)
    - [items](#cacheboxlrucacheitems)
    - [keys](#cacheboxlrucachekeys)
    - [values](#cacheboxlrucachevalues)
    - [least_recently_used](#cacheboxlrucacheleast_recently_used)
    - [most_recently_used](#cacheboxlrucachemost_recently_used)
    
- [TTLCache](#cacheboxttlcache)
    - [\_\_init\_\_](#cacheboxttlcache__init__)
    - [is_full](#cacheboxttlcacheis_full)
    - [is_empty](#cacheboxttlcacheis_empty)
    - [insert](#cacheboxttlcacheinsert)
    - [get](#cacheboxttlcacheget)
    - [capacity](#cacheboxttlcachecapacity)
    - [clear](#cacheboxttlcacheclear)
    - [pop](#cacheboxttlcachepop)
    - [setdefault](#cacheboxttlcachesetdefault)
    - [update](#cacheboxttlcacheupdate)
    - [shrink_to_fit](#cacheboxttlcacheshrink_to_fit)
    - [items](#cacheboxttlcacheitems)
    - [keys](#cacheboxttlcachekeys)
    - [values](#cacheboxttlcachevalues)
    - [get_with_expire](#cacheboxttlcacheget_with_expire)
    - [pop_with_expire](#cacheboxttlcachepop_with_expire)
    - [popitem_with_expire](#cacheboxttlcachepopitem_with_expire)

- [VTTLCache](#cacheboxttlcache)
    - [\_\_init\_\_](#cacheboxvttlcache__init__)
    - [is_full](#cacheboxvttlcacheis_full)
    - [is_empty](#cacheboxvttlcacheis_empty)
    - [insert](#cacheboxvttlcacheinsert)
    - [get](#cacheboxvttlcacheget)
    - [capacity](#cacheboxvttlcachecapacity)
    - [clear](#cacheboxvttlcacheclear)
    - [pop](#cacheboxvttlcachepop)
    - [setdefault](#cacheboxvttlcachesetdefault)
    - [update](#cacheboxvttlcacheupdate)
    - [shrink_to_fit](#cacheboxvttlcacheshrink_to_fit)
    - [items](#cacheboxvttlcacheitems)
    - [keys](#cacheboxvttlcachekeys)
    - [values](#cacheboxvttlcachevalues)
    - [get_with_expire](#cacheboxvttlcacheget_with_expire)
    - [pop_with_expire](#cacheboxvttlcachepop_with_expire)
    - [popitem_with_expire](#cacheboxvttlcachepopitem_with_expire)

## cachebox.BaseCacheImpl

A base class for all cache algorithms;
Do not try to call its constructor, this is only for type-hint.

You can use it for type hint:
```python
cache: BaseCacheImpl[int, str] = create_a_cache()
```

Or use it for checking types:
```python
assert isinstance(cachebox.Cache(0), BaseCacheImpl)
assert isinstance(cachebox.LRUCache(0), BaseCacheImpl)
# ...
```

## cachebox.Cache
A simple cache that has no algorithm; this is only a hashmap.

**`Cache` vs `dict`**:
- it is thread-safe and unordered, while `dict` isn't thread-safe and ordered (Python 3.6+).
- it uses very lower memory than `dict`.
- it supports useful and new methods for managing memory, while `dict` does not.
- it does not support `popitem`, while `dict` does.
- You can limit the size of `Cache`, but you cannot for `dict`.

### cachebox.Cache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

First example:
```python
cache = cachebox.Cache(100) # 100 is limit size
cache.insert("key", "value")
assert cache["key"] == "value"
```

Second example:
```python
cache = cachebox.Cache(0) # zero means infinity
cache.insert("key", "value")
assert cache["key"] == "value"
```

### cachebox.Cache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.Cache(20)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.Cache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.Cache(20)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.Cache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.Cache(0)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.Cache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.Cache(0)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.Cache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.Cache(0)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.Cache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.Cache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.Cache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.Cache(0)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.Cache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.Cache(0, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.Cache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.Cache(100)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.Cache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.Cache(0, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.Cache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.Cache(10, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.Cache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.Cache(10, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.Cache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.Cache(10, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

--------

## cachebox.FIFOCache
FIFO Cache implementation - First-In First-Out Policy (thread-safe).

In simple terms, the FIFO cache will remove the element that has been in the cache the longest::

```
    A      B
    |      |
  |---|  |---|  |---|  |---|
1 |   |  | B |  |   |  |   |
2 | A |  | A |  | B |  |   |
  |---|  |---|  |---|  |---|
                  |      |
                  A      B
```

### cachebox.FIFOCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

Example:
```python
cache = cachebox.FIFOCache(2)
cache.insert("a", 1)
cache.insert("b", 2)
assert "a" in cache and "b" in cache

cache.insert("c", 3)
assert "a" not in cache
assert "b" in cache and "c" in cache

assert cache.popitem() == ("b", 2)
```

### cachebox.FIFOCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.FIFOCache(20)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.FIFOCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.FIFOCache(20)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.FIFOCache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.FIFOCache(0)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.FIFOCache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.FIFOCache(0)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.FIFOCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.FIFOCache(0)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.FIFOCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.FIFOCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.FIFOCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.FIFOCache(0)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.FIFOCache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.FIFOCache(0, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.FIFOCache.popitem
Removes and returns the key-value pair that has been in the cache the longest.

### cachebox.FIFOCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.FIFOCache(0, {i:i for i in range(10)})
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.FIFOCache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Example:
```python
cache = cachebox.FIFOCache(100)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.FIFOCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.FIFOCache(0, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.FIFOCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.FIFOCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.FIFOCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

### cachebox.FIFOCache.first
Returns the first key in cache; this is the one which will be removed by `popitem()`.

Example:
```python
cache = cachebox.FIFOCache(3)
cache.insert(1, 1)
cache.insert(2, 2)
cache.insert(3, 3)

assert cache.first() == 1
assert cache.popitem() == (1, 1)
```

### cachebox.FIFOCache.last
Returns the last key in cache.

Example:
```python
cache = cachebox.FIFOCache(3)
cache.insert(1, 1)
cache.insert(2, 2)
assert cache.last() == 2

cache.insert(3, 3)
assert cache.last() == 3
```

-------

## cachebox.LFUCache
LFU Cache implementation - Least frequantly used policy (thread-safe).

In simple terms, the LFU cache will remove the element in the cache that has been accessed the least,
regardless of time::

```
                        E
                        |
|------|  |------|  |------|
| A(1) |  | B(2) |  | B(2) |
| B(1) |  | A(1) |  | E(1) |
|------|  |------|  |------|
          access B  A dropped
```

### cachebox.LFUCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

Example:
```python
cache = cachebox.LFUCache(2)
cache.insert("a", 1)
cache.insert("b", 2)
assert "a" in cache and "b" in cache

# get "a"
assert cache["a"] == 1

cache.insert("c", 3)
assert "b" not in cache
assert "a" in cache and "c" in cache

assert cache.popitem() == ("c", 3)
```

### cachebox.LFUCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.LFUCache(20)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.LFUCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.LFUCache(20)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.LFUCache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.LFUCache(0)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.LFUCache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.LFUCache(0)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.LFUCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.LFUCache(0)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.LFUCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.LFUCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.LFUCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.LFUCache(0)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.LFUCache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.LFUCache(0, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.LFUCache.popitem
Removes and returns the key-value pair in the cache that has been accessed the least, regardless of time.

### cachebox.LFUCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.LFUCache(0, {i:i for i in range(10)})
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.LFUCache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Example:
```python
cache = cachebox.LFUCache(100)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.LFUCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.LFUCache(0, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.LFUCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.LFUCache(10, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.LFUCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.LFUCache(10, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.LFUCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.LFUCache(10, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

### cachebox.LFUCache.least_frequently_used
Returns the key in the cache that has been accessed the least, regardless of time.

Example:
```python
cache = cachebox.LFUCache(5)
cache.insert(1, 1)
cache.insert(2, 2)

# access 1 twice
cache[1]
cache[1]

# access 2 once
cache[2]

assert cache.least_frequently_used() == 2
```

-------

## cachebox.RRCache
RRCache implementation - Random Replacement policy (thread-safe).

In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.

### cachebox.RRCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

Example:
```python
cache = cachebox.RRCache(2)
cache.insert("a", 1)
cache.insert("b", 2)
assert "a" in cache and "b" in cache

# get "a"
assert cache["a"] == 1

cache.insert("c", 3)
assert len(cache) == 2
```

### cachebox.RRCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.RRCache(20)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.RRCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.RRCache(20)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.RRCache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.RRCache(0)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.RRCache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.RRCache(0)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.RRCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.RRCache(0)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.RRCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.RRCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.RRCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.RRCache(0)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.RRCache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.RRCache(0, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.RRCache.popitem
Choices randomly element, removes, and returns it.

### cachebox.RRCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.RRCache(0, {i:i for i in range(10)})
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.RRCache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Example:
```python
cache = cachebox.RRCache(100)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.RRCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.RRCache(0, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.RRCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.RRCache(10, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.RRCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.RRCache(10, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.RRCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.RRCache(10, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

--------


## cachebox.LRUCache
LRU Cache implementation - Least recently used policy (thread-safe).

In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.

```
                        E
                        |
|------|  |------|  |------|
|  A   |  |  B   |  |  B   |
|  B   |  |  A   |  |  E   |
|------|  |------|  |------|
          access B  A dropped
```

### cachebox.LRUCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

Example:
```python
cache = cachebox.LRUCache(2)
cache.insert("a", 1)
cache.insert("b", 2)
assert "a" in cache and "b" in cache

# get "a"
assert cache["a"] == 1

cache.insert("c", 3)
assert "b" not in cache

# get "a" again
assert cache["a"] == 1
assert cache.popitem() == ("c", 3)
```

### cachebox.LRUCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.LRUCache(20)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.LRUCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.LRUCache(20)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.LRUCache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.LRUCache(0)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.LRUCache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.LRUCache(0)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.LRUCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.LRUCache(0)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.LRUCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.LRUCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.LRUCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.LRUCache(0)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.LRUCache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.LRUCache(0, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.LRUCache.popitem
Removes and returns the key-value pair that has not been accessed in the longest time.

### cachebox.LRUCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.LRUCache(0, {i:i for i in range(10)})
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.LRUCache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Example:
```python
cache = cachebox.LRUCache(100)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.LRUCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.LRUCache(0, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.LRUCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.LRUCache(10, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.LRUCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.LRUCache(10, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.LRUCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.LFUCache(10, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```


### cachebox.LRUCache.least_recently_used
Returns the key in the cache that has not been accessed in the longest time.

Example:
```python
cache = cachebox.LRUCache(2)
cache.insert(1, 1)
cache.insert(2, 2)

# get 1
assert cache[1] == 1
assert cache.least_recently_used() == 2
```

### cachebox.LRUCache.most_recently_used
Returns the key in the cache that has been accessed in the shortest time.

Example:
```python
cache = cachebox.LRUCache(2)
cache.insert(1, 1)
cache.insert(2, 2)

# get 1
assert cache[1] == 1
assert cache.most_recently_used() == 1
```

------

## cachebox.TTLCache
TTL Cache implementation - Time-To-Live Policy (thread-safe).

In simple terms, the TTL cache will automatically remove the element in the cache that has expired:

```
|-------|                   |-------|
| A(3s) |                   |       |
| B(7s) |  -- after 4s -->  | B(3s) |
| C(9s) |                   | C(5s) |
|-------|                   |-------|
```

### cachebox.TTLCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- ttl (`float`): The `ttl` param specifies the time-to-live value for each element in cache (in seconds); cannot be zero or negative.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

Example:
```python
cache = cachebox.TTLCache(5, ttl=3) # 3 seconds
cache.insert(1, 1)
assert cache.get(1) == 1

time.sleep(3)
assert cache.get(1) is None
```

### cachebox.TTLCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.TTLCache(20, 10)
for i in range(20):
    cache[i] = i

assert cache.is_full()
```

### cachebox.TTLCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.TTLCache(20, 10)
assert cache.is_empty()
cache[0] = 0
assert not cache.is_empty()
```

### cachebox.TTLCache.insert

**Parameters**:
- key
- value

Inserts a new key-value into the cache.

An alias for `__setitem__`.

Example:
```python
cache = cachebox.TTLCache(0, 10)
cache.insert("key", "value") # cache["key"] = "value"
assert cache["key"] == "value"
```

### cachebox.TTLCache.get

**Parameters**:
- key
- default (*optional*)

Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.TTLCache(0, 10)
cache.insert("key", "value")
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.TTLCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.TTLCache(0, 10)
assert cache.capacity() == 0
cache.insert(0, 0)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.TTLCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0)
assert cache.capacity() == 100
```

### cachebox.TTLCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.TTLCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.TTLCache(0, 10)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.TTLCache.setdefault

**Parameters**:
- key
- default (*optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

Note: raises `OverflowError` if the cache reached the maxsize limit.

Example:
```python
cache = cachebox.TTLCache(0, 10, {"exists", 1})
assert cache["exists"] == 1

assert cache.setdefault("exists", 2) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2) == 2
assert cache["no-exists"] == 2
```

### cachebox.TTLCache.popitem
Removes and returns the oldest key-value pair from the cache.

### cachebox.TTLCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.TTLCache(0, 5, {i:i for i in range(10)})
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.TTLCache.update

**Parameters**:
- iterable (`iterable | dict`)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

Example:
```python
cache = cachebox.TTLCache(100, 5)
cache.update({1: 1, 2: 2, 3: 3})
assert len(cache) == 3
```

### cachebox.TTLCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.TTLCache(0, 3, {i:i for i in range(4)})
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.TTLCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.TTLCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.TTLCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

### cachebox.TTLCache.get_with_expire

**Parameters:**
- key
- default (*optional*)

Works like `.get()`, but also returns the remaining time-to-live.

Example:
```python
cache = cachebox.TTLCache(10, 1)
cache.insert("key", "value")

value, remaining = cache.get_with_expire("key")
assert value == "value"
assert 0.0 < remaining < 1.0

value, remaining = cache.get_with_expire("no-exists")
assert value is None
assert remaining == 0.0
```


### cachebox.TTLCache.pop_with_expire

**Parameters:**
- key
- default (*optional*)

Works like `.pop()`, but also returns the remaining time-to-live.

Example:
```python
cache = cachebox.TTLCache(10, 1)
cache.insert("key", "value")

value, remaining = cache.pop_with_expire("key")
assert value == "value"
assert 0.0 < remaining < 1.0

value, remaining = cache.pop_with_expire("key")
assert value is None
assert remaining == 0.0
```

### cachebox.TTLCache.popitem_with_expire
Works like `.popitem()`, but also returns the remaining time-to-live.

-------

## cachebox.VTTLCache
VTTL Cache Implementation - Time-To-Live Per-Key Policy (thread-safe).

In simple terms, the TTL cache will automatically remove the element in the cache that has expired.

**`VTTLCache` vs `TTLCache`**:
- In `VTTLCache` each item has its own unique time-to-live, unlike `TTLCache`.
- `VTTLCache` insert is slower than `TTLCache`.

### cachebox.VTTLCache.\_\_init\_\_

**Parameters**:
- maxsize (`int`): By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
- iterable (`tuple | dict`, *optional*): By `iterable` param, you can create cache from a dict or an iterable.
- ttl (`float`, *optional*): The `ttl` param specifies the time-to-live value for `iterable` key-value pairs (None means no time-to-live). Note that this is the time-to-live value for all key-value pairs in `iterable` param.
- capacity (int, *optional*): If `capacity` param is given, cache attempts to allocate a new hash table with at
least enough capacity for inserting the given number of elements without reallocating.

First Example:
```python
cache = cachebox.VTTLCache(5)
cache.insert(1, 1, ttl=2)
cache.insert(2, 2, ttl=5)
cache.insert(3, 3, ttl=1)
assert cache.get(1) == 1

time.sleep(1)
assert cache.get(3) is None
```

Second Example:
```python
cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, ttl=5)
assert len(cache) == 10
time.sleep(5)
assert len(cache) == 0
```

### cachebox.VTTLCache.is_full
Returns `True` if cache has reached the maxsize limit.

Example:
```python
cache = cachebox.VTTLCache(20)
for i in range(20):
    cache.insert(i, i, None)

assert cache.is_full()
```

### cachebox.VTTLCache.is_empty
Returns `True` if cache is empty.

Example:
```python
cache = cachebox.VTTLCache(20)
assert cache.is_empty()
cache.insert(1, 1, None)
assert not cache.is_empty()
```

### cachebox.VTTLCache.insert

**Parameters**:
- key
- value
- ttl (`float`)

Inserts a new key-value into the cache.

The `ttl` param specifies the time-to-live value for this key-value pair;
cannot be zero or negative.
Set `None` to keep alive key-value pair for always.

> [!NOTE]\
> This method is different from `__setitem__` here.

> [!NOTE]\
> With this method you can specify time-to-live value, but with `__setitem__` you cannot.

Example:
```python
cache = cachebox.VTTLCache(0)
cache.insert(1, 1, ttl=3)
cache.insert(2, 2, ttl=None)
assert 1 in cache
assert 2 in cache

time.sleep(3)
assert 1 not in cache
assert 2 in cache
```

### cachebox.VTTLCache.get

**Parameters**:
- key
- default (*optional*)
Searches for a key-value in the cache and returns it.

Unlike `__getitem__`, if the key-value not found, returns `default`.

Example:
```python
cache = cachebox.VTTLCache(0)
cache.insert("key", "value", 3)
assert cache.get("key") == "value"
assert cache.get("no-exists") is None
assert cache.get("no-exists", "default") == "default"
```

### cachebox.VTTLCache.capacity
Returns the number of elements the map can hold without reallocating.

First example:
```python
cache = cachebox.VTTLCache(0)
assert cache.capacity() == 0
cache.insert(0, 0, None)
assert cache.capacity() >= 1
```

Second example:
```python
cache = cachebox.VTTLCache(0, capacity=100)
assert cache.capacity() == 100
cache.insert(0, 0, None)
assert cache.capacity() == 100
```

### cachebox.VTTLCache.clear

**Parameters**:
- reuse (`bool`, *optional*): if `reuse` is `True`, will not free the memory for reusing in the future (default is False).

Removes all elements from the cache.

### cachebox.VTTLCache.pop

**Parameters**:
- key
- default (*optional*)

Removes a key from the cache, returning it.

Example:
```python
cache = cachebox.VTTLCache(0, 10)
cache.insert("key", "value")
assert len(cache) == 1
assert cache.pop("key") == "value"
assert len(cache) == 0
```

### cachebox.VTTLCache.setdefault

**Parameters**:
- key
- default (*optional*)
- ttl (`float`, *optional*)

Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

for `ttl` param see `insert()` method.

Example:
```python
cache = cachebox.VTTLCache(0, {"exists", 1}, 10)
assert cache["exists"] == 1

assert cache.setdefault("exists", 2, ttl=3) == 1
assert cache["exists"] == 1

assert cache.setdefault("no-exists", 2, ttl=3) == 2
assert cache["no-exists"] == 2
```

### cachebox.VTTLCache.popitem
Removes and returns the key-value pair that is near to be expired.

### cachebox.VTTLCache.drain

**Parameters**:
- n (`int`)

Do the `popitem()`, `n` times and returns count of removed items.

Example:
```python
cache = cachebox.VTTLCache(0, {i:i for i in range(10)}, ttl=5)
assert len(cache) == 10
assert cache.drain(8) == 8
assert len(cache) == 2
assert cache.drain(10) == 2
```

### cachebox.VTTLCache.update

**Parameters**:
- iterable (`iterable | dict`)
- ttl (`float`, *optional*)

Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

For `ttl` param see `insert()` method.

Example:
```python
cache = cachebox.VTTLCache(100)
cache.update({1: 1, 2: 2, 3: 3}, ttl=3)
assert len(cache) == 3
time.sleep(3)
assert len(cache) == 0
```

### cachebox.VTTLCache.shrink_to_fit
Shrinks the capacity of the cache as much as possible.

Example:
```python
cache = cachebox.VTTLCache(0, {i:i for i in range(4)}, 5)
assert cache.capacity() == 14 # maybe greater or lower, this is just example
cache.shrinks_to_fit()
assert cache.capacity() >= 4
```

### cachebox.VTTLCache.items
Returns an iterable object of the cache's items (key-value pairs).

Notes:
- You should not make any changes in cache while using this iterable object.
- Items are not ordered.

Example:
```python
cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, ttl=None)
for (key, value) in cache.items():
    print(key, value)
# (3, 3)
# (9, 9)
# ...
```

### cachebox.VTTLCache.keys
Returns an iterable object of the cache's keys.

Notes:
- You should not make any changes in cache while using this iterable object.
- Keys are not ordered.

Example:
```python
cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, ttl=None)
for key in cache.keys():
    print(key)
# 5
# 0
# ...
```

### cachebox.VTTLCache.values
Returns an iterable object of the cache's values.

Notes:
- You should not make any changes in cache while using this iterable object.
- Values are not ordered.

Example:
```python
cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, ttl=None)
for key in cache.values():
    print(key)
# 5
# 0
# ...
```

### cachebox.VTTLCache.get_with_expire

**Parameters:**
- key
- default (*optional*)

Works like `.get()`, but also returns the remaining time-to-live.

Example:
```python
cache = cachebox.VTTLCache(10)
cache.insert("key", "value", ttl=1)

value, remaining = cache.get_with_expire("key")
assert value == "value"
assert 0.0 < remaining < 1.0

value, remaining = cache.get_with_expire("no-exists")
assert value is None
assert remaining == 0.0
```

### cachebox.VTTLCache.pop_with_expire

**Parameters:**
- key
- default (*optional*)

Works like `.pop()`, but also returns the remaining time-to-live.

Example:
```python
cache = cachebox.VTTLCache(10)
cache.insert("key", "value", ttl=1)

value, remaining = cache.pop_with_expire("key")
assert value == "value"
assert 0.0 < remaining < 1.0

value, remaining = cache.pop_with_expire("key")
assert value is None
assert remaining == 0.0
```

### cachebox.VTTLCache.popitem_with_expire
Works like `.popitem()`, but also returns the remaining time-to-live.
