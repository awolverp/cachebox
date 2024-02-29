import typing
from typing import Iterable

__version__: str
__author__: str

K = typing.TypeVar("K", typing.Hashable)
V = typing.TypeVar("V")

class BaseCacheImpl(typing.Generic[K, V]):
    """
    This is only a base class of other caches and not implemented.

    Example::

        cache = cachebox.Cache(0)
        assert isinstance(cache, cachebox.BaseCacheImpl)
    """

    def __init__(self, maxsize: int, *, capacity: int = ...) -> None: ...
    def __setitem__(self, key: K, value: V) -> None:
        """
        See `.insert()`
        """
        ...

    def __getitem__(self, key: K) -> V:
        """
        Like `.get()` but raise `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: K) -> None:
        """
        See `.delete()`
        """
        ...

    def __contains__(self, key: K) -> bool: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __sizeof__(self) -> int: ...
    def __richcmp__(self, other: typing.Self) -> None:
        """
        Caches only support `==` and `!=`, and unfortunaly there's no good way to check these.

        Performance: O(capacity)~

        This method equals to::

            other_keys = other.keys()
            all((i in other_keys) for i in cache.keys())
        """
        ...

    def insert(self, key: K, value: V) -> None: ...
    def delete(self, key: K) -> None: ...
    def getmaxsize(self) -> int:
        """
        Returns the maxsize.
        """
        ...

    def keys(self) -> typing.List[V]: ...
    def values(self) -> typing.List[V]: ...
    def items(self) -> typing.List[typing.Tuple[K, V]]: ...
    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]: ...
    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]: ...
    def popitem(self) -> typing.Tuple[K, V]: ...
    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]: ...
    def update(self, iterable: typing.Union[typing.Dict[K, V], typing.Iterable[tuple]]) -> None: ...
    def clear(self, *, reuse: bool = True) -> None: ...

class Cache(BaseCacheImpl, typing.Generic[K, V]):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        A cache without any policy;
        it works like a dictionary but:
        - thread-safe
        - can be fixed-size
        - it isn't iterable
        - you can reserve memory before inserting items with `capacity` parameter ( increase speed )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Raise OverflowError if the `maxsize` reached;
        This means you should delete an item to decrease size before inserting new one.

        Performance: O(1)~*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(1)~
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an unsorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an unsorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (unsorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        This is not implemented for `Cache`.
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(1)~
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m)
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class FIFOCache(BaseCacheImpl, typing.Generic[K, V]):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        First In First Out Cache Implemention. ( details: https://en.wikipedia.org/wiki/FIFO_(computing_and_electronics) )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(min(i, n-i))*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(min(i, n-i))
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an sorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an sorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (sorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(min(i, n-i))
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the first item from cache.

        Performance: O(1)~
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(min(i, n-i))*
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * min(i, n-i))
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class LFUCache(BaseCacheImpl, typing.Generic[K, V]):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        Least Frequently Used Cache Implemention. ( details: https://en.wikipedia.org/wiki/Least_frequently_used )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(n)~*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(1)~
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an unsorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an unsorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (unsorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the least frequently used from cache.

        Performance: O(n)~*
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(n)~*
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * n)
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class RRCache(BaseCacheImpl):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        Random Replacement Cache Implemention. ( details: https://en.wikipedia.org/wiki/Least_frequently_used )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(1)~*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(1)~
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an unsorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an unsorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (unsorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns a random item from cache.

        Performance: O(1)~
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(1)~
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m)
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class LRUCache(BaseCacheImpl):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        Least Recently Used Cache Implemention. ( details: https://www.interviewcake.com/concept/java/lru-cache )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(1)~ or O(min(i, n-i))*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(1)~
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an sorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an sorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (sorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(min(i, n-i))*
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(min(i, n-i))~
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the least recently used item from cache.

        Performance: O(1)
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(min(i, n-i))*
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * min(i, n-i))
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class MRUCache(BaseCacheImpl):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        Most Recently Used Cache Implemention. ( details: https://en.wikipedia.org/wiki/Most_Recently_Used )

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(1)~ or O(min(i, n-i))*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(1)~
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an sorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an sorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (sorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(min(i, n-i))*
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(min(i, n-i))~
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the least recently used item from cache.

        Performance: O(1)
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(min(i, n-i))*
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * min(i, n-i))
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` is True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...

class TTLCache(BaseCacheImpl):
    def __init__(self, maxsize: int, ttl: float, *, capacity: int = ...) -> None:
        """
        LRU Cache Implementation With Per-Item TTL Value.

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.
        - `ttl` specify the each items time-to-live value, in seconds (cannot be zero or negative).

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V) -> None:
        """
        Stores key-value in cache.

        Performance: O(min(i, n-i))*
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(min(i, n-i))
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an sorted list of keys.

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an sorted list of values.

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (sorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(min(i, n-i))
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the first item from cache.

        Performance: O(n-i)
        """
        ...

    def setdefault(self, key: K, value: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(min(i, n-i))*
        """
        ...

    def update(self, iterable: typing.Union[typing.Dict[K, V], Iterable[tuple]]) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * min(i, n-i))
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` be True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...
    def get_with_expire(
        self, key: K, default: typing.Optional[V] = None
    ) -> typing.Tuple[typing.Optional[V], float]:
        """
        It works like `.get()` with the difference that it returns the expiration of item in seconds.

        If the key not found, returns (`default`, `0.0`)

        Performance: O(1)~
        """
        ...

    def pop_with_expire(
        self, key: K, default: typing.Optional[V] = None
    ) -> typing.Tuple[typing.Optional[V], float]:
        """
        It works like `.pop()` with the difference that it returns the expiration of item in seconds.

        If the key not found, returns (`default`, `0.0`)

        Performance: O(min(i, n-i))
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[K, V, float]:
        """
        It works like `.popitem()` with the difference that it returns the expiration of item in seconds.

        If the cache empty, raises `KeyError`.

        Performance: O(n-i)
        """
        ...

    def expire(self, *, reuse: bool = False) -> None:
        """
        Expired items will be removed from a cache only at the next mutating operation,
        e.g. `.insert()` or `.delete()`, and therefore may still claim memory.

        Calling this method removes all items whose time-to-live would have expired by time,
        and if `reuse` be True, keeps the allocated memory for reuse (default False).

        Performance: O(n-i)
        """
        ...

    def getttl(self) -> float: ...

class TTLCacheNoDefault(BaseCacheImpl):
    def __init__(self, maxsize: int, *, capacity: int = ...) -> None:
        """
        Time-aware Cache Implemention; With this cache, you can set its own expiration time for each key-value pair.

        #### Parameters:
        - `maxsize` specify the maximum size of cache ( zero means there's no limit ).
        - If `capacity` specified, the cache will be able to hold at least capacity elements without reallocating.
          If capacity is 0, the cache will not allocate.
          If `capacity` greater than `maxsize`, we use `maxsize` instead of this for `capacity`.

        #### Raise
        Raise OverflowError if the `maxsize` or the `capacity` be negative number.

        #### Thread-safe
        This cache is thread-safe.

        #### Note:
        this implemention is too slower than `TTLCache`,
        so if speed is very important to you, we recommend use `TTLCache` instead of this.
        """
        ...

    def __contains__(self, key: K) -> bool:
        """
        Returns `True` if key found.

        Performance: O(1)~
        """
        ...

    def insert(self, key: K, value: V, ttl: typing.Optional[float] = None) -> None:
        """
        Stores key-value in cache.

        `ttl` parameter: the time-to-live value for this key-value pair. `None` means infinite.

        Performance: ?
        """
        ...

    def delete(self, key: K) -> None:
        """
        Deletes the stored key-value from cache; raise KeyError if key not found.

        Performance: O(n-i)
        """
        ...

    def keys(self) -> typing.List[V]:
        """
        Returns an sorted list of keys (sorted).

        Performance: O(n)
        """
        ...

    def values(self) -> typing.List[V]:
        """
        Returns an sorted list of values (sorted).

        Performance: O(n)
        """
        ...

    def items(self) -> typing.List[typing.Tuple[K, V]]:
        """
        Returns a list that contains the key-value pairs of the cache, as tuples (sorted).

        Performance: O(n)
        """
        ...

    def get(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Returns value of specified key; returns `default` if key not found.

        Performance: O(1)~*
        """
        ...

    def pop(self, key: K, default: typing.Optional[V] = None) -> typing.Optional[V]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.

        Performance: O(n-i)
        """
        ...

    def popitem(self) -> typing.Tuple[K, V]:
        """
        Deletes and returns the first item from cache.

        Performance: O(1)~
        """
        ...

    def setdefault(
        self, key: K, value: typing.Optional[V] = None, ttl: typing.Optional[float] = None
    ) -> typing.Optional[V]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.

        Performance: O(min(i, n-i))*
        """
        ...

    def update(
        self,
        iterable: typing.Union[dict, typing.Iterable[tuple]],
        ttl: typing.Optional[float] = None,
    ) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Performance: O(m * min(i, n-i))
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the map, removing all key-value pairs.
        If `reuse` be True, keeps the allocated memory for reuse (default False).

        Performance: O(n) or O(capacity)
        """
        ...
    def get_with_expire(
        self, key: K, default: typing.Optional[V] = None
    ) -> typing.Tuple[typing.Optional[V], float]:
        """
        It works like `.get()` with the difference that it returns the expiration of item in seconds.

        If the key not found, returns (`default`, `0.0`)

        Performance: O(1)~
        """
        ...

    def pop_with_expire(
        self, key: K, default: typing.Optional[V] = None
    ) -> typing.Tuple[typing.Optional[V], float]:
        """
        It works like `.pop()` with the difference that it returns the expiration of item in seconds.

        If the key not found, returns (`default`, `0.0`)

        Performance: O(min(i, n-i))
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[K, V, float]:
        """
        It works like `.popitem()` with the difference that it returns the expiration of item in seconds.

        If the cache empty, raises `KeyError`.

        Performance: O(1)~
        """
        ...

    def expire(self, *, reuse: bool = False) -> None:
        """
        Expired items will be removed from a cache only at the next mutating operation,
        e.g. `.insert()` or `.delete()`, and therefore may still claim memory.

        Calling this method removes all items whose time-to-live would have expired by time,
        and if `reuse` be True, keeps the allocated memory for reuse (default False).
        """
        ...
