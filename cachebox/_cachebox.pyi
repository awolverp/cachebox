"""
cachebox core ( written in Rust )
"""

import typing

__version__: str
__author__: str

version_info: typing.Tuple[int, int, int, bool]
""" (major, minor, patch, is_beta) """

KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    This is the base class of all cache classes such as Cache, FIFOCache, ...

    Do not try to call its constructor, this is only for type-hint.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None: ...
    @staticmethod
    def __class_getitem__(*args) -> None: ...
    @property
    def maxsize(self) -> int: ...
    def __len__(self) -> int: ...
    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: KT) -> bool: ...
    def __setitem__(self, key: KT, value: VT) -> None: ...
    def __getitem__(self, key: KT) -> VT: ...
    def __delitem__(self, key: KT) -> VT: ...
    def __str__(self) -> str: ...
    def __iter__(self) -> typing.Iterator[KT]: ...
    def __richcmp__(self, other, op: int) -> bool: ...
    def __getstate__(self) -> object: ...
    def __getnewargs__(self) -> tuple: ...
    def __setstate__(self, state: object) -> None: ...
    def capacity(self) -> int: ...
    def is_full(self) -> bool: ...
    def is_empty(self) -> bool: ...
    def insert(self, key: KT, value: VT) -> typing.Optional[VT]: ...
    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]: ...
    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]: ...
    def setdefault(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Optional[VT | DT]: ...
    def popitem(self) -> typing.Tuple[KT, VT]: ...
    def drain(self, n: int) -> int: ...
    def clear(self, *, reuse: bool = False) -> None: ...
    def shrink_to_fit(self) -> None: ...
    def update(
        self, iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]]
    ) -> None: ...
    def keys(self) -> typing.Iterable[KT]: ...
    def values(self) -> typing.Iterable[VT]: ...
    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]: ...

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

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.

        Note: raises `OverflowError` if the cache reached the maxsize limit,
        because this class does not have any algorithm.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        Note: raises `OverflowError` if the cache reached the maxsize limit,
        because this class does not have any algorithm.
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.NoReturn: ...  # not implemented for this class
    def drain(self, n: int) -> typing.NoReturn: ...  # not implemented for this class
    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the cache to fit len(self) elements.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Note: raises `OverflowError` if the cache reached the maxsize limit.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.
        """
        ...

class FIFOCache(BaseCacheImpl[KT, VT]):
    """
    FIFO Cache implementation - First-In First-Out Policy (thread-safe).

    In simple terms, the FIFO cache will remove the element that has been in the cache the longest
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

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest
        """
        ...

    def drain(self, n: int) -> int:
        """
        Does the `popitem()` `n` times and returns count of removed items.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the first key in cache; this is the one which will be removed by `popitem()` (if n == 0).

        By using `n` parameter, you can browse order index by index.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the last key in cache.
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

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the cache to fit len(self) elements.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Note: raises `OverflowError` if the cache reached the maxsize limit.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.
        """
        ...

class TTLCache(BaseCacheImpl[KT, VT]):
    """
    TTL Cache implementation - Time-To-Live Policy (thread-safe).

    In simple terms, the TTL cache will automatically remove the element in the cache that has expired.
    """

    def __init__(
        self,
        maxsize: int,
        ttl: float,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        TTL Cache implementation - Time-To-Live Policy (thread-safe).

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param ttl: specifies the time-to-live value for each element in cache (in seconds); cannot be zero or negative.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest
        """
        ...

    def drain(self, n: int) -> int:
        """
        Does the `popitem()` `n` times and returns count of removed items.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the oldest key in cache; this is the one which will be removed by `popitem()` (if n == 0).

        By using `n` parameter, you can browse order index by index.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the newest key in cache.
        """
        ...

    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining time-to-live.
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining time-to-live.
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[KT, VT, float]:
        """
        Works like `.popitem()`, but also returns the remaining time-to-live.
        """
        ...

class LRUCache(BaseCacheImpl[KT, VT]):
    """
    LRU Cache implementation - Least recently used policy (thread-safe).

    In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.
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

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def peek(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest
        """
        ...

    def drain(self, n: int) -> int:
        """
        Does the `popitem()` `n` times and returns count of removed items.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def least_recently_used(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has not been accessed in the longest time.
        """
        ...

    def most_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed in the shortest time.
        """
        ...

class LFUCache(BaseCacheImpl[KT, VT]):
    """
    LFU Cache implementation - Least frequantly used policy (thread-safe).

    In simple terms, the LFU cache will remove the element in the cache that has been accessed the least, regardless of time
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

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def peek(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without increasing frequenctly counter).
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest
        """
        ...

    def drain(self, n: int) -> int:
        """
        Does the `popitem()` `n` times and returns count of removed items.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def update(self, iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        ...

    def least_frequently_used(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed the least, regardless of time.
        """
        ...

class VTTLCache(BaseCacheImpl[KT, VT]):
    """
    VTTL Cache implementation - Time-To-Live Per-Key Policy (thread-safe).

    In simple terms, the TTL cache will automatically remove the element in the cache that has expired when need.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        ttl: typing.Optional[float] = 0.0,
        *,
        capacity: int = ...,
    ) -> None:
        """
        VTTL Cache implementation - Time-To-Live Per-Key Policy (thread-safe).

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param ttl: specifies the time-to-live value for each element in cache (in seconds); cannot be zero or negative.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        ...

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Set self[key] to value.

        Recommended to use `.insert()` method here.
        """
        ...

    def __getitem__(self, key: KT) -> VT:
        """
        Returns self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def __delitem__(self, key: KT) -> VT:
        """
        Deletes self[key].

        Note: raises `KeyError` if key not found.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.
        """
        ...

    def is_full(self) -> bool:
        """
        Equivalent directly to `len(self) == self.maxsize`
        """
        ...

    def is_empty(self) -> bool:
        """
        Equivalent directly to `len(self) == 0`
        """
        ...

    def insert(self, key: KT, value: VT, ttl: typing.Optional[float] = None) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but:
        - Here you can set ttl for key-value ( with `self[key] = value` you can't )
        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
        and the old value is returned. The key is not updated, though;
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value.

        If the key is not found, returns the `default`.
        """
        ...

    def setdefault(
        self, key: KT, default: typing.Optional[DT] = None, ttl: typing.Optional[float] = None
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest
        """
        ...

    def drain(self, n: int) -> int:
        """
        Does the `popitem()` `n` times and returns count of removed items.
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        ...

    def update(
        self,
        iterable: typing.Iterable[typing.Tuple[KT, VT]] | typing.Dict[KT, VT],
        ttl: typing.Optional[float] = None,
    ) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the oldest key in cache; this is the one which will be removed by `popitem()` (if n == 0).

        By using `n` parameter, you can browse order index by index.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the newest key in cache.
        """
        ...

    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining time-to-live.
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining time-to-live.
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[KT, VT, float]:
        """
        Works like `.popitem()`, but also returns the remaining time-to-live.
        """
        ...

class cache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...

class fifocache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...

class ttlcache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...

class lrucache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...

class lfucache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...

class vttlcache_iterator:
    def __len__(self) -> int: ...
    def __iter__(self) -> typing.Iterator: ...
    def __next__(self) -> typing.Any: ...
