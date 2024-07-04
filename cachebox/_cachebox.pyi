"""
cachebox core ( written in Rust )
"""

import typing

__version_: str
__author__: str

version_info: typing.Tuple[int, int, int, bool]
""" (major, minor, patch, is_beta) """

KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    A base class for all cache algorithms;
    Do not try to call its constructor, this is only for type-hint.

    You can use it for type hint::

        cache: BaseCacheImpl[int, str] = create_a_cache()

    Or use it for type checking::

        assert isinstance(cachebox.Cache(0), BaseCacheImpl)
        assert isinstance(cachebox.LRUCache(0), BaseCacheImpl)
        # ...
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        Do not try to call me, i'm not implemeneted.
        """
        ...

    @property
    def maxsize(self) -> int: ...
    def is_full(self) -> bool: ...
    def is_empty(self) -> bool: ...
    def __len__(self) -> int: ...
    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __setitem__(self, key: KT, value: VT) -> None: ...
    def insert(self, key: KT, value: VT) -> None: ...
    def __getitem__(self, key: KT) -> VT: ...
    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]: ...
    def __delitem__(self, key: KT) -> None: ...
    def __contains__(self, key: KT) -> bool: ...
    def capacity(self) -> int: ...
    def clear(self, *, reuse: bool = False) -> None: ...
    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]: ...
    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]: ...
    def popitem(self) -> typing.Tuple[KT, VT]: ...
    def drain(self, n: int) -> int: ...
    def update(
        self, iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]]
    ) -> None: ...
    def shrink_to_fit(self) -> None: ...
    def items(self) -> tuple_ptr_iterator[KT, VT]: ...
    def __iter__(self) -> object_ptr_iterator[KT]: ...
    def keys(self) -> object_ptr_iterator[KT]: ...
    def values(self) -> object_ptr_iterator[VT]: ...
    def __eq__(self, other: typing.Self) -> bool: ...
    def __ne__(self, other: typing.Self) -> bool: ...
    def __str__(self) -> str: ...
    def __getstate__(self) -> typing.Any:
        """
        This is unstable
        """
        ...
    def __setstate__(self, state: typing.Any):
        """
        This is unstable
        """
        ...
    def __getnewargs__(self) -> tuple: ...

class Cache(BaseCacheImpl[KT, VT]):
    """
    A simple cache that has no algorithm; this is only a hashmap.

    `Cache` vs `dict`:
    - it is thread-safe and unordered, while `dict` isn't thread-safe and ordered (Python 3.6+).
    - it uses very lower memory than `dict`.
    - it supports useful and new methods for managing memory, while `dict` does not.
    - it does not support `popitem`, while `dict` does.
    - You can limit the size of `Cache`, but you cannot for `dict`.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        A simple cache that has no algorithm; this is only a hashmap.

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        First example::

            cache = cachebox.Cache(100) # 100 is limit size
            cache.insert("key", "value")
            assert cache["key"] == "value"

        Second example::

            cache = cachebox.Cache(0) # zero means infinity
            cache.insert("key", "value")
            assert cache["key"] == "value"
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.Cache(20)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.Cache(20)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        Note: raises `OverflowError` if the cache reached the maxsize limit.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.Cache(0)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it.

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.Cache(0)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.Cache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.Cache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.Cache(0)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Note: raises `OverflowError` if the cache reached the maxsize limit.

        Example::

            cache = cachebox.Cache(0, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        It is not implemented for this cache; there's no algorithms.
        """
        ...

    def drain(self, n: int) -> int:
        """
        It is not implemented for this cache; there's no algorithms.
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Note: raises `OverflowError` if the cache reached the maxsize limit.

        Example::

            cache = cachebox.Cache(100)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.Cache(0, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.Cache(10, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...
        """
        ...

    def keys(self) -> object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.Cache(10, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

    def values(self) -> object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.Cache(10, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

class FIFOCache(BaseCacheImpl[KT, VT]):
    """
    FIFO Cache implementation - First-In First-Out Policy (thread-safe).

    In simple terms, the FIFO cache will remove the element that has been in the cache the longest::

          A      B
          |      |
        |---|  |---|  |---|  |---|
      1 |   |  | B |  |   |  |   |
      2 | A |  | A |  | B |  |   |
        |---|  |---|  |---|  |---|
                        |      |
                        A      B
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        FIFO Cache implementation - First-In First-Out Policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        Example::

            cache = cachebox.FIFOCache(2)
            cache.insert("a", 1)
            cache.insert("b", 2)
            assert "a" in cache and "b" in cache

            cache.insert("c", 3)
            assert "a" not in cache
            assert "b" in cache and "c" in cache

            assert cache.popitem() == ("b", 2)
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.FIFOCache(20)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.FIFOCache(20)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.FIFOCache(0)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it.

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.FIFOCache(0)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.FIFOCache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.FIFOCache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.FIFOCache(0)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Example::

            cache = cachebox.FIFOCache(0, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair that has been in the cache the longest.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.FIFOCache(0, {i:i for i in range(10)})
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Example::

            cache = cachebox.FIFOCache(100)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.FIFOCache(0, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...

        Ordered Example::

            cache = cachebox.FIFOCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.first(i)
                print(key, cache[key])

            # (0, 0)
            # (1, 1)
            # (2, 2)
        """
        ...

    def keys(self) -> object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.FIFOCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                print(cache.first(i))

            # 0
            # 1
            # 2
        """
        ...

    def values(self) -> object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.FIFOCache(10, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.FIFOCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.first(i)
                print(cache[key])

            # 0
            # 1
            # 2
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the first key in cache; this is the one which will be removed by `popitem()`.

        Example::

            cache = cachebox.FIFOCache(3)
            cache.insert(1, 1)
            cache.insert(2, 2)
            cache.insert(3, 3)

            assert cache.first() == 1
            assert cache.popitem() == (1, 1)
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the last key in cache.

        Example::

            cache = cachebox.FIFOCache(3)
            cache.insert(1, 1)
            cache.insert(2, 2)
            assert cache.last() == 2

            cache.insert(3, 3)
            assert cache.last() == 3
        """
        ...

class LFUCache(BaseCacheImpl[KT, VT]):
    """
    LFU Cache implementation - Least frequantly used policy (thread-safe).

    In simple terms, the LFU cache will remove the element in the cache that has been accessed the least,
    regardless of time::

                               E
                               |
        |------|  |------|  |------|
        | A(1) |  | B(2) |  | B(2) |
        | B(1) |  | A(1) |  | E(1) |
        |------|  |------|  |------|
                  access B  A dropped
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        LFU Cache implementation - Least frequantly used policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        Example::

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
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.LFUCache(20)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.LFUCache(20)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.LFUCache(0)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (and increase the frequently counter).

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.LFUCache(0)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def peek(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without increasing the frequently counter).

        Example::

            cache = cachebox.LFUCache(0)
            cache.insert("key", "value")
            assert cache.peek("key") == "value"
            assert cache.peek("no-exists") is None
            assert cache.peek("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.LFUCache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.LFUCache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.LFUCache(0)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Example::

            cache = cachebox.LFUCache(0, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair in the cache that has been accessed the least, regardless of time.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.LFUCache(0, {i:i for i in range(10)})
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Example::

            cache = cachebox.LFUCache(100)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.LFUCache(0, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> lfu_tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.LFUCache(10, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...

        Ordered Example::

            cache = cachebox.LFUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.least_frequently_used(i)
                print(key, cache.peek(key))

            # (0, 0)
            # (1, 1)
            # (2, 2)
        """
        ...

    def __iter__(self) -> lfu_object_ptr_iterator[KT]: ...
    def keys(self) -> lfu_object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.LFUCache(10, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.LFUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                print(cache.least_frequently_used(i))

            # 0
            # 1
            # 2
        """
        ...

    def values(self) -> lfu_object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.LFUCache(10, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.LFUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.least_frequently_used(i)
                print(cache.peek(key))

            # 0
            # 1
            # 2
        """
        ...

    def least_frequently_used(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed the least, regardless of time.

        Example::

            cache = cachebox.LFUCache(5)
            cache.insert(1, 1)
            cache.insert(2, 2)

            # access 1 twice
            cache[1]
            cache[1]

            # access 2 once
            cache[2]

            assert cache.least_frequently_used() == 2
            assert cache.least_frequently_used(0) == 2
            assert cache.least_frequently_used(1) == 1
            assert cache.least_frequently_used(2) is None # 2 is out of bound
        """
        ...

class RRCache(BaseCacheImpl[KT, VT]):
    """
    RRCache implementation - Random Replacement policy (thread-safe).

    In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        RRCache implementation - Random Replacement policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        Example::

            cache = cachebox.RRCache(2)
            cache.insert("a", 1)
            cache.insert("b", 2)
            assert "a" in cache and "b" in cache

            # get "a"
            assert cache["a"] == 1

            cache.insert("c", 3)
            assert len(cache) == 2
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.RRCache(20)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.RRCache(20)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.RRCache(0)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it.

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.RRCache(0)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.RRCache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.RRCache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.RRCache(0)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Example::

            cache = cachebox.RRCache(0, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Choices randomly element, removes, and returns it.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.RRCache(0, {i:i for i in range(10)})
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Example::

            cache = cachebox.RRCache(100)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.RRCache(0, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.RRCache(10, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...
        """
        ...

    def keys(self) -> object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.RRCache(10, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

    def values(self) -> object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.RRCache(10, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

class LRUCache(BaseCacheImpl[KT, VT]):
    """
    LRU Cache implementation - Least recently used policy (thread-safe).

    In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.

                               E
                               |
        |------|  |------|  |------|
        |  A   |  |  B   |  |  B   |
        |  B   |  |  A   |  |  E   |
        |------|  |------|  |------|
                  access B  A dropped
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        LRU Cache implementation - Least recently used policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        Example::

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
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.LRUCache(20)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.LRUCache(20)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.LRUCache(0)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (and moves the key to recently used).

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.LRUCache(0)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def peek(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).

        Example::

            cache = cachebox.LRUCache(0)
            cache.insert("key", "value")
            assert cache.peek("key") == "value"
            assert cache.peek("no-exists") is None
            assert cache.peek("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.LRUCache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.LRUCache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.LRUCache(0)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Example::

            cache = cachebox.LRUCache(0, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair that has not been accessed in the longest time.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.LRUCache(0, {i:i for i in range(10)})
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Example::

            cache = cachebox.LRUCache(100)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.LRUCache(0, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.LRUCache(10, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...

        Ordered Example::

            cache = cachebox.LRUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.least_recently_used(i)
                print(key, cache.peek(key))

            # (0, 0)
            # (1, 1)
            # (2, 2)
        """
        ...

    def keys(self) -> object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.LRUCache(10, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.LRUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                print(cache.least_recently_used(i))

            # 0
            # 1
            # 2
        """
        ...

    def values(self) -> object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.LRUCache(10, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.LRUCache(3, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.least_recently_used(i)
                print(cache.peek(key))

            # 0
            # 1
            # 2
        """
        ...

    def least_recently_used(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has not been accessed in the longest time.

        Example::

            cache = cachebox.LRUCache(2)
            cache.insert(1, 1)
            cache.insert(2, 2)

            # get 1
            assert cache[1] == 1
            assert cache.least_recently_used() == 2
        """
        ...

    def most_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed in the shortest time.

        Example::

            cache = cachebox.LRUCache(2)
            cache.insert(1, 1)
            cache.insert(2, 2)

            # get 1
            assert cache[1] == 1
            assert cache.most_recently_used() == 1
        """
        ...

class TTLCache(BaseCacheImpl[KT, VT]):
    """
    TTL Cache implementation - Time-To-Live Policy (thread-safe).

    In simple terms, the TTL cache will automatically remove the element in the cache that has expired::

        |-------|                   |-------|
        | A(3s) |                   |       |
        | B(7s) |  -- after 4s -->  | B(3s) |
        | C(9s) |                   | C(5s) |
        |-------|                   |-------|
    """

    def __init__(
        self,
        maxsize: int,
        ttl: float,
        iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]][typing.Tuple[KT, VT]]
        | typing.Dict[KT, VT] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        TTL Cache implementation - Time-To-Live Policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        The `ttl` param specifies the time-to-live value for each element in cache (in seconds); cannot be zero or negative.

        By `iterable` param, you can create cache from a dict or an iterable.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        Example::

            cache = cachebox.TTLCache(5, ttl=3) # 3 seconds
            cache.insert(1, 1)
            assert cache.get(1) == 1

            time.sleep(3)
            assert cache.get(1) is None
        """
        ...

    @property
    def ttl(self) -> float: ...
    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.TTLCache(20, 10)
            for i in range(20):
                cache[i] = i

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.TTLCache(20, 10)
            assert cache.is_empty()
            cache[0] = 0
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT) -> None:
        """
        Inserts a new key-value into the cache.

        An alias for `__setitem__`.

        Example::

            cache = cachebox.TTLCache(0, 10)
            cache.insert("key", "value") # cache["key"] = "value"
            assert cache["key"] == "value"
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it.

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.TTLCache(0, 10)
            cache.insert("key", "value")
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.TTLCache(0, 10)
            assert cache.capacity() == 0
            cache.insert(0, 0)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.TTLCache(0, 10, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.TTLCache(0, 10)
            cache.insert("key", "value")
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        Example::

            cache = cachebox.TTLCache(0, 10, {"exists", 1})
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the oldest key-value pair from the cache.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.TTLCache(0, 5, {i:i for i in range(10)})
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(self, iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Example::

            cache = cachebox.TTLCache(100, 5)
            cache.update({1: 1, 2: 2, 3: 3})
            assert len(cache) == 3
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.TTLCache(0, 3, {i:i for i in range(4)})
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> ttl_tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...

        Ordered Example::

            cache = cachebox.TTLCache(3, 5, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.first(i)
                print(key, cache[key])

            # (0, 0)
            # (1, 1)
            # (2, 2)
        """
        ...

    def __iter__(self) -> ttl_object_ptr_iterator[KT]: ...
    def keys(self) -> ttl_object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.TTLCache(3, 5, {i:i for i in range(3)})
            for i in range(len(cache)):
                print(cache.first(i))

            # 0
            # 1
            # 2
        """
        ...

    def values(self) -> ttl_object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.TTLCache(10, 3, {i:i for i in range(10)})
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...

        Ordered Example::

            cache = cachebox.TTLCache(3, 5, {i:i for i in range(3)})
            for i in range(len(cache)):
                key = cache.first(i)
                print(cache[key])

            # 0
            # 1
            # 2
        """
        ...

    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining time-to-live.

        Example::

            cache = cachebox.TTLCache(10, 1)
            cache.insert("key", "value")

            value, remaining = cache.get_with_expire("key")
            assert value == "value"
            assert 0.0 < remaining < 1.0

            value, remaining = cache.get_with_expire("no-exists")
            assert value is None
            assert remaining == 0.0
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining time-to-live.

        Example::

            cache = cachebox.TTLCache(10, 1)
            cache.insert("key", "value")

            value, remaining = cache.pop_with_expire("key")
            assert value == "value"
            assert 0.0 < remaining < 1.0

            value, remaining = cache.pop_with_expire("key")
            assert value is None
            assert remaining == 0.0
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float]:
        """
        Works like `.popitem()`, but also returns the remaining time-to-live.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the oldest key from the cache; this is the one which will be removed by `popitem()`.

        Example::

            cache = cachebox.TTLCache(3, ttl=3)
            cache.insert(1, 1)
            cache.insert(2, 2)
            cache.insert(3, 3)

            assert cache.first() == 1
            assert cache.popitem() == (1, 1)
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the newest key from the cache.

        Example::

            cache = cachebox.TTLCache(3, ttl=3)
            cache.insert(1, 1)
            cache.insert(2, 2)
            assert cache.last() == 2

            cache.insert(3, 3)
            assert cache.last() == 3
        """
        ...

class VTTLCache(BaseCacheImpl[KT, VT]):
    """
    VTTL Cache Implementation - Time-To-Live Per-Key Policy (thread-safe).

    In simple terms, the TTL cache will automatically remove the element in the cache that has expired.

    `VTTLCache` vs `TTLCache`:
    - In `VTTLCache` each item has its own unique time-to-live, unlike `TTLCache`.
    - `VTTLCache` insert is slower than `TTLCache`.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        ttl: typing.Optional[float] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        VTTL Cache Implementation - Time-To-Live Per-Key Policy (thread-safe).

        By `maxsize` param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        By `iterable` param, you can create cache from a dict or an iterable.

        The `ttl` param specifies the time-to-live value for `iterable` key-value pairs (None means no time-to-live).
        Note that this is the time-to-live value for all key-value pairs in `iterable` param.

        If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.

        First Example::

            cache = cachebox.VTTLCache(5)
            cache.insert(1, 1, ttl=2)
            cache.insert(2, 2, ttl=5)
            cache.insert(3, 3, ttl=1)
            assert cache.get(1) == 1

            time.sleep(1)
            assert cache.get(3) is None

        Second Example::

            cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, ttl=5)
            assert len(cache) == 10
            time.sleep(5)
            assert len(cache) == 0
        """
        ...

    def is_full(self) -> bool:
        """
        Returns `True` if cache has reached the maxsize limit.

        Example::

            cache = cachebox.VTTLCache(20)
            for i in range(20):
                cache.insert(i, i, None)

            assert cache.is_full()
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns `True` if cache is empty.

        Example::

            cache = cachebox.VTTLCache(20)
            assert cache.is_empty()
            cache.insert(1, 1, None)
            assert not cache.is_empty()
        """
        ...

    def insert(self, key: KT, value: VT, ttl: typing.Optional[float]) -> None:
        """
        Inserts a new key-value into the cache.

        The `ttl` param specifies the time-to-live value for this key-value pair;
        cannot be zero or negative.
        Set `None` to keep alive key-value pair for always.

        Notes:
        - This method is different from `__setitem__` here.
        - With this method you can specify time-to-live value, but with `__setitem__` you cannot.

        Example::

            cache = cachebox.VTTLCache(0)
            cache.insert(1, 1, ttl=3)
            cache.insert(2, 2, ttl=None)
            assert 1 in cache
            assert 2 in cache

            time.sleep(3)
            assert 1 not in cache
            assert 2 in cache
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it.

        Unlike `__getitem__`, if the key-value not found, returns `default`.

        Example::

            cache = cachebox.VTTLCache(0)
            cache.insert("key", "value", 3)
            assert cache.get("key") == "value"
            assert cache.get("no-exists") is None
            assert cache.get("no-exists", "default") == "default"
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        First example::

            cache = cachebox.VTTLCache(0)
            assert cache.capacity() == 0
            cache.insert(0, 0, ttl=None)
            assert cache.capacity() >= 1

        Second example::

            cache = cachebox.VTTLCache(0, capacity=100)
            assert cache.capacity() == 100
            cache.insert(0, 0, ttl=None)
            assert cache.capacity() == 100
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all elements from the cache.

        if `reuse` is `True`, will not free the memory for reusing in the future.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes a key from the cache, returning it.

        Example::

            cache = cachebox.VTTLCache(0)
            cache.insert("key", "value", ttl=3)
            assert len(cache) == 1
            assert cache.pop("key") == "value"
            assert len(cache) == 0
        """
        ...

    def setdefault(
        self, key: KT, default: DT = None, ttl: typing.Optional[float] = None
    ) -> typing.Union[VT, DT]:
        """
        Returns the value of a key (if the key is in cache). If not, it inserts key with a value to the cache.

        for `ttl` param see `insert()` method.

        Example::

            cache = cachebox.VTTLCache(0, {"exists", 1}, 10)
            assert cache["exists"] == 1

            assert cache.setdefault("exists", 2, ttl=3) == 1
            assert cache["exists"] == 1

            assert cache.setdefault("no-exists", 2, ttl=3) == 2
            assert cache["no-exists"] == 2
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair that is near to be expired.
        """
        ...

    def drain(self, n: int) -> int:
        """
        Do the `popitem()`, `n` times and returns count of removed items.

        Example::

            cache = cachebox.VTTLCache(0, {i:i for i in range(10)}, 5)
            assert len(cache) == 10
            assert cache.drain(8) == 8
            assert len(cache) == 2
            assert cache.drain(10) == 2
        """
        ...

    def update(
        self,
        iterable: typing.Union[typing.Iterable[KT, VT], typing.Dict[KT, VT]],
        ttl: typing.Optional[float] = None,
    ) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        For `ttl` param see `insert()` method.

        Example::

            cache = cachebox.VTTLCache(100)
            cache.update({1: 1, 2: 2, 3: 3}, ttl=3)
            assert len(cache) == 3
            time.sleep(3)
            assert len(cache) == 0
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.

        Example::

            cache = cachebox.VTTLCache(0, {i:i for i in range(4)}, 5)
            assert cache.capacity() == 14 # maybe greater or lower, this is just example
            cache.shrinks_to_fit()
            assert cache.capacity() >= 4
        """
        ...

    def items(self) -> vttl_tuple_ptr_iterator[KT, VT]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.

        Example::

            cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, None)
            for (key, value) in cache.items():
                print(key, value)
            # (3, 3)
            # (9, 9)
            # ...
        """
        ...

    def __iter__(self) -> vttl_object_ptr_iterator[KT]: ...
    def keys(self) -> vttl_object_ptr_iterator[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.

        Example::

            cache = cachebox.VTTLCache(10, {i:i for i in range(10)}, None)
            for key in cache.keys():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

    def values(self) -> vttl_object_ptr_iterator[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.

        Example::

            cache = cachebox.TTLCache(10, {i:i for i in range(10)}, None)
            for key in cache.values():
                print(key)
            # 5
            # 0
            # ...
        """
        ...

    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining time-to-live.

        Example::

            cache = cachebox.VTTLCache(10)
            cache.insert("key", "value", ttl=1)

            value, remaining = cache.get_with_expire("key")
            assert value == "value"
            assert 0.0 < remaining < 1.0

            value, remaining = cache.get_with_expire("no-exists")
            assert value is None
            assert remaining == 0.0
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining time-to-live.

        Example::

            cache = cachebox.VTTLCache(10)
            cache.insert("key", "value", ttl=1)

            value, remaining = cache.pop_with_expire("key")
            assert value == "value"
            assert 0.0 < remaining < 1.0

            value, remaining = cache.pop_with_expire("key")
            assert value is None
            assert remaining == 0.0
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float]:
        """
        Works like `.popitem()`, but also returns the remaining time-to-live.
        """
        ...

class tuple_ptr_iterator(typing.Generic[KT, VT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> typing.Tuple[KT, VT]: ...

class object_ptr_iterator(typing.Generic[KT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> KT: ...

class lfu_tuple_ptr_iterator(typing.Generic[KT, VT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> typing.Tuple[KT, VT]: ...

class lfu_object_ptr_iterator(typing.Generic[KT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> KT: ...

class ttl_tuple_ptr_iterator(typing.Generic[KT, VT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> typing.Tuple[KT, VT]: ...

class ttl_object_ptr_iterator(typing.Generic[KT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> KT: ...

class vttl_tuple_ptr_iterator(typing.Generic[KT, VT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> typing.Tuple[KT, VT]: ...

class vttl_object_ptr_iterator(typing.Generic[KT]):
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Self: ...
    def __next__(self) -> KT: ...
