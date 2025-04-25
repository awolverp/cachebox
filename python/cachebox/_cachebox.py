from . import _core
from ._core import BaseCacheImpl
from datetime import timedelta, datetime
import copy as _std_copy
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")


def _items_to_str(items: typing.Iterable[typing.Any], length) -> str:
    if length <= 50:
        return "{" + ", ".join(f"{k!r}: {v!r}" for k, v in items) + "}"

    c = 0
    left = []

    while c < length:
        k, v = next(items)  # type: ignore[call-overload]

        if c <= 50:
            left.append(f"{k!r}: {v!r}")

        else:
            break

        c += 1

    return "{%s, ... %d more ...}" % (", ".join(left), length - c)


class IteratorView(typing.Generic[VT]):
    __slots__ = ("iterator", "func")

    def __init__(self, iterator, func: typing.Callable[[tuple], typing.Any]):
        self.iterator = iterator
        self.func = func

    def __iter__(self):
        self.iterator = self.iterator.__iter__()
        return self

    def __next__(self) -> VT:
        return self.func(self.iterator.__next__())


class Cache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe, memory-efficient hashmap-like cache with configurable maximum size.

    Provides a flexible key-value storage mechanism with:
    - Configurable maximum size (zero means unlimited)
    - Lower memory usage compared to standard dict
    - Thread-safe operations
    - Useful memory management methods

    Differs from standard dict by:
    - Being thread-safe
    - Unordered storage
    - Size limitation
    - Memory efficiency
    - Additional cache management methods

    Supports initialization with optional initial data and capacity,
    and provides dictionary-like access with additional cache-specific operations.
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[dict, typing.Iterable[tuple], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new Cache instance.

        Args:
            maxsize (int): Maximum number of elements the cache can hold. Zero means unlimited.
            iterable (Union[Cache, dict, tuple, Generator, None], optional): Initial data to populate the cache. Defaults to None.
            capacity (int, optional): Pre-allocate hash table capacity to minimize reallocations. Defaults to 0.

        Creates a new cache with specified size constraints and optional initial data. The cache can be pre-sized
        to improve performance when the number of expected elements is known in advance.
        """
        self._raw = _core.Cache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        Note: raises `OverflowError` if the cache reached the maxsize limit,
        because this class does not have any algorithm.
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache. Return the value for key if key is
        in the cache, else `default`.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.NoReturn:  # pragma: no cover
        raise NotImplementedError()

    def drain(self, n: int) -> typing.NoReturn:  # pragma: no cover
        raise NotImplementedError()

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Note: raises `OverflowError` if the cache reached the maxsize limit.
        """
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, Cache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, Cache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def copy(self) -> "Cache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "Cache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "Cache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )


class FIFOCache(BaseCacheImpl[KT, VT]):
    """
    A First-In-First-Out (FIFO) cache implementation with configurable maximum size and optional initial capacity.

    This cache provides a fixed-size container that automatically removes the oldest items when the maximum size is reached.
    Supports various operations like insertion, retrieval, deletion, and iteration.

    Attributes:
        maxsize: The maximum number of items the cache can hold.
        capacity: The initial capacity of the cache before resizing.

    Key features:
    - Deterministic item eviction order (oldest items removed first)
    - Efficient key-value storage and retrieval
    - Supports dictionary-like operations
    - Allows optional initial data population
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new FIFOCache instance.

        Args:
            maxsize: The maximum number of items the cache can hold.
            iterable: Optional initial data to populate the cache. Can be another FIFOCache,
                      a dictionary, tuple, generator, or None.
            capacity: Optional initial capacity of the cache before resizing. Defaults to 0.
        """
        self._raw = _core.FIFOCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair into the cache, returning the previous value if the key existed.

        Equivalent to `self[key] = value`, but with additional return value semantics:

        - If the key was not previously in the cache, returns None.
        - If the key was already present, updates the value and returns the old value.
          The key itself is not modified.

        Args:
            key: The key to insert.
            value: The value to associate with the key.

        Returns:
            The previous value associated with the key, or None if the key was not present.
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value] # type: ignore[return-value]

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Removes the element that has been in the cache the longest."""
        try:
            return self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, FIFOCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, FIFOCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the first key in cache; this is the one which will be removed by `popitem()` (if n == 0).

        By using `n` parameter, you can browse order index by index.
        """
        if n < 0:
            n = len(self._raw) + n

        if n < 0:
            return None

        return self._raw.get_index(n)

    def last(self) -> typing.Optional[KT]:
        """
        Returns the last key in cache. Equals to `self.first(-1)`.
        """
        return self._raw.get_index(len(self._raw) - 1)

    def copy(self) -> "FIFOCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "FIFOCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "FIFOCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )


class RRCache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe cache implementation with Random Replacement (RR) policy.

    This cache randomly selects and removes elements when the cache reaches its maximum size,
    ensuring a simple and efficient caching mechanism with configurable capacity.

    Supports operations like insertion, retrieval, deletion, and iteration.
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new RRCache instance.

        Args:
            maxsize (int): Maximum size of the cache. A value of zero means unlimited capacity.
            iterable (dict or Iterable[tuple], optional): Initial data to populate the cache. Defaults to None.
            capacity (int, optional): Preallocated capacity for the cache to minimize reallocations. Defaults to 0.

        Note:
            - The cache size limit is immutable after initialization.
            - If an iterable is provided, the cache will be populated using the update method.
        """
        self._raw = _core.RRCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair into the cache, returning the previous value if the key existed.

        Equivalent to `self[key] = value`, but with additional return value semantics:

        - If the key was not previously in the cache, returns None.
        - If the key was already present, updates the value and returns the old value.
          The key itself is not modified.

        Args:
            key: The key to insert.
            value: The value to associate with the key.

        Returns:
            The previous value associated with the key, or None if the key was not present.
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Randomly selects and removes a (key, value) pair from the cache."""
        try:
            return self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def random_key(self) -> KT:
        """
        Randomly selects and returns a key from the cache.
        Raises `KeyError` If the cache is empty.
        """
        try:
            return self._raw.random_key()
        except _core.CoreKeyError:
            raise KeyError() from None

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, RRCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, RRCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def copy(self) -> "RRCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "RRCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "RRCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )


class LRUCache(BaseCacheImpl[KT, VT]):
    """
    Thread-safe Least Recently Used (LRU) cache implementation.

    Provides a cache that automatically removes the least recently used items when
    the cache reaches its maximum size. Supports various operations like insertion,
    retrieval, and management of cached items with configurable maximum size and
    initial capacity.

    Key features:
    - Configurable maximum cache size
    - Optional initial capacity allocation
    - Thread-safe operations
    - Efficient key-value pair management
    - Supports initialization from dictionaries or iterables
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new LRU Cache instance.

        Args:
            maxsize (int): Maximum size of the cache. Zero indicates unlimited size.
            iterable (dict | Iterable[tuple], optional): Initial data to populate the cache.
            capacity (int, optional): Pre-allocated capacity for the cache to minimize reallocations.

        Notes:
            - The cache size is immutable after initialization.
            - If an iterable is provided, it will be used to populate the cache.
        """
        self._raw = _core.LRUCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair into the cache, returning the previous value if the key existed.

        Equivalent to `self[key] = value`, but with additional return value semantics:

        - If the key was not previously in the cache, returns None.
        - If the key was already present, updates the value and returns the old value.
          The key itself is not modified.

        Args:
            key: The key to insert.
            value: The value to associate with the key.

        Returns:
            The previous value associated with the key, or None if the key was not present.
        """
        return self._raw.insert(key, value)

    def peek(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).
        """
        try:
            return self._raw.peek(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the least recently used item from the cache and returns it as a (key, value) tuple.
        Raises KeyError if the cache is empty.
        """
        try:
            return self._raw.popitem()
        except _core.CoreKeyError:  # pragma: no cover
            raise KeyError() from None

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, LRUCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, LRUCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def least_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has not been accessed in the longest time.
        """
        return self._raw.least_recently_used()

    def most_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed in the shortest time.
        """
        return self._raw.most_recently_used()

    def copy(self) -> "LRUCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "LRUCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "LRUCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )


class LFUCache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe Least Frequently Used (LFU) cache implementation.

    This cache removes elements that have been accessed the least number of times,
    regardless of their access time. It provides methods for inserting, retrieving,
    and managing cache entries with configurable maximum size and initial capacity.

    Key features:
    - Thread-safe cache with LFU eviction policy
    - Configurable maximum size and initial capacity
    - Supports initialization from dictionaries or iterables
    - Provides methods for key-value management similar to dict
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new Least Frequently Used (LFU) cache.

        Args:
            maxsize (int): Maximum size of the cache. A value of zero means unlimited size.
            iterable (dict or Iterable[tuple], optional): Initial data to populate the cache.
            capacity (int, optional): Initial hash table capacity to minimize reallocations. Defaults to 0.

        The cache uses a thread-safe LFU eviction policy, removing least frequently accessed items when the cache reaches its maximum size.
        """
        self._raw = _core.LFUCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair into the cache, returning the previous value if the key existed.

        Equivalent to `self[key] = value`, but with additional return value semantics:

        - If the key was not previously in the cache, returns None.
        - If the key was already present, updates the value and returns the old value.
          The key itself is not modified.

        Args:
            key: The key to insert.
            value: The value to associate with the key.

        Returns:
            The previous value associated with the key, or None if the key was not present.
        """
        return self._raw.insert(key, value)

    def peek(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Union[VT, DT]:  # pragma: no cover
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).
        """
        try:
            return self._raw.peek(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the least frequently used (LFU) item from the cache.
        """
        try:
            return self._raw.popitem()
        except _core.CoreKeyError:  # pragma: no cover
            raise KeyError() from None

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, LFUCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, LFUCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: (x[0], x[1]))

    def items_with_frequency(self) -> IteratorView[typing.Tuple[KT, VT, int]]:
        """
        Returns an iterable view - containing tuples of `(key, value, frequency)` - of the cache's items along with their access frequency.

        Notes:
            - The returned iterator should not be used to modify the cache.
            - Frequency represents how many times the item has been accessed.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def least_frequently_used(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed the least, regardless of time.

        If n is given, returns the nth least frequently used key.

        Notes:
            - This method may re-sort the cache which can cause iterators to be stopped.
            - Do not use this method while using iterators.
        """
        if n < 0:
            n = len(self._raw) + n

        if n < 0:
            return None

        return self._raw.least_frequently_used(n)

    def copy(self) -> "LFUCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "LFUCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "LFUCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            # NOTE: we cannot use self._raw.items() here because iterables a tuples of (key, value, frequency)
            _items_to_str(self.items(), len(self._raw)),
        )


class TTLCache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe Time-To-Live (TTL) cache implementation with configurable maximum size and expiration.

    This cache automatically removes elements that have expired based on their time-to-live setting.
    Supports various operations like insertion, retrieval, and iteration.
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        ttl: typing.Union[float, timedelta],
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new TTL cache instance.

        Args:
            maxsize: Maximum number of elements the cache can hold.
            ttl: Time-to-live for cache entries, either as seconds or a timedelta.
            iterable: Optional initial items to populate the cache, can be a dict or iterable of tuples.
            capacity: Optional initial capacity for the underlying cache storage. Defaults to 0.

        Raises:
            ValueError: If the time-to-live (ttl) is not a positive number.
        """
        if isinstance(ttl, timedelta):
            ttl = ttl.total_seconds()

        if ttl <= 0:
            raise ValueError("ttl must be a positive number and non-zero")

        self._raw = _core.TTLCache(maxsize, ttl, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    @property
    def ttl(self) -> float:
        return self._raw.ttl()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair into the cache, returning the previous value if the key existed.

        Equivalent to `self[key] = value`, but with additional return value semantics:

        - If the key was not previously in the cache, returns None.
        - If the key was already present, updates the value and returns the old value.
          The key itself is not modified.

        Args:
            key: The key to insert.
            value: The value to associate with the key.

        Returns:
            The previous value associated with the key, or None if the key was not present.
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key).value()
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def get_with_expire(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Retrieves the value and expiration duration for a given key from the cache.

        Returns a tuple containing the value associated with the key and its duration.
        If the key is not found, returns the default value and 0.0 duration.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            A tuple of (value, duration), where value is the cached value or default,
            and duration is the time-to-live for the key (or 0.0 if not found).
        """
        try:
            pair = self._raw.get(key)
        except _core.CoreKeyError:
            return default, 0.0  # type: ignore[return-value]
        else:
            return (pair.value(), pair.duration())

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key).value()
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop_with_expire(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Removes the specified key from the cache and returns its value and expiration duration.

        If the key is not found, returns the default value and 0.0 duration.

        Args:
            key: The key to remove from the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            A tuple of (value, duration), where value is the cached value or default,
            and duration is the time-to-live for the key (or 0.0 if not found).
        """
        try:
            pair = self._raw.remove(key)
        except _core.CoreKeyError:
            return default, 0.0  # type: ignore[return-value]
        else:
            return (pair.value(), pair.duration())

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Return the value for key if key is in the cache, else default.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Removes the element that has been in the cache the longest."""
        try:
            val = self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None
        else:
            return val.pack2()

    def popitem_with_expire(self) -> typing.Tuple[KT, VT, float]:
        """
        Removes and returns the element that has been in the cache the longest, along with its key and expiration duration.

        If the cache is empty, raises a KeyError.

        Returns:
            A tuple of (key, value, duration), where:
            - key is the key of the removed item
            - value is the value of the removed item
            - duration is the time-to-live for the removed item
        """
        try:
            val = self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None
        else:
            return val.pack3()

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union[dict, typing.Iterable[tuple]]) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key).value()
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, TTLCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, TTLCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items_with_expire(self) -> IteratorView[typing.Tuple[KT, VT, float]]:
        """
        Returns an iterable object of the cache's items (key-value pairs along with their expiration duration).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.pack3())

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.pack2())

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.key())

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.value())

    def first(self, n: int = 0) -> typing.Optional[KT]:  # pragma: no cover
        """
        Returns the first key in cache; this is the one which will be removed by `popitem()` (if n == 0).

        By using `n` parameter, you can browse order index by index.
        """
        if n < 0:
            n = len(self._raw) + n

        if n < 0:
            return None

        return self._raw.get_index(n)

    def last(self) -> typing.Optional[KT]:
        """
        Returns the last key in cache. Equals to `self.first(-1)`.
        """
        return self._raw.get_index(len(self._raw) - 1)

    def expire(self) -> None:  # pragma: no cover
        """
        Manually removes expired key-value pairs from memory and releases their memory.

        Notes:
            - This operation is typically automatic and does not require manual invocation.
        """
        self._raw.expire()

    def copy(self) -> "TTLCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "TTLCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "TTLCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d, ttl=%f](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            self._raw.ttl(),
            _items_to_str(self.items(), len(self._raw)),
        )


class VTTLCache(BaseCacheImpl[KT, VT]):
    """
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
    """

    __slots__ = ("_raw",)

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Union[dict, typing.Iterable[tuple]], None] = None,
        ttl: typing.Union[float, timedelta, datetime, None] = None,  # This is not a global TTL!
        *,
        capacity: int = 0,
    ) -> None:
        """
        Initialize a new VTTLCache instance.

        Args:
            maxsize (int): Maximum size of the cache. Zero indicates unlimited size.
            iterable (dict or Iterable[tuple], optional): Initial data to populate the cache.
            ttl (float or timedelta or datetime, optional): Time-to-live duration for `iterable` items.
            capacity (int, optional): Preallocated capacity for the cache to minimize reallocations.

        Raises:
            ValueError: If provided TTL is zero or negative.
        """
        self._raw = _core.VTTLCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable, ttl)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(
        self, key: KT, value: VT, ttl: typing.Union[float, timedelta, datetime, None] = None
    ) -> typing.Optional[VT]:
        """
        Insert a key-value pair into the cache with an optional time-to-live (TTL).
        Returns the previous value associated with the key, if it existed.

        Args:
            key (KT): The key to insert.
            value (VT): The value to associate with the key.
            ttl (float or timedelta or datetime, optional): Time-to-live duration for the item.
                If a timedelta or datetime is provided, it will be converted to seconds.

        Raises:
            ValueError: If the provided TTL is zero or negative.
        """
        if ttl is not None:  # pragma: no cover
            if isinstance(ttl, timedelta):
                ttl = ttl.total_seconds()

            elif isinstance(ttl, datetime):
                ttl = (ttl - datetime.now()).total_seconds()

            if ttl <= 0:
                raise ValueError("ttl must be positive and non-zero")

        return self._raw.insert(key, value, ttl)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            The value associated with the key, or the default value if the key is not found.
        """
        try:
            return self._raw.get(key).value()
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def get_with_expire(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Retrieves the value and expiration duration for a given key from the cache.

        Returns a tuple containing the value associated with the key and its duration.
        If the key is not found, returns the default value and 0.0 duration.

        Args:
            key: The key to look up in the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            A tuple of (value, duration), where value is the cached value or default,
            and duration is the time-to-live for the key (or 0.0 if not found).
        """
        try:
            pair = self._raw.get(key)
        except _core.CoreKeyError:
            return default, 0.0  # type: ignore[return-value]
        else:
            return (pair.value(), pair.duration())

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key).value()
        except _core.CoreKeyError:
            return default  # type: ignore[return-value]

    def pop_with_expire(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Removes the specified key from the cache and returns its value and expiration duration.

        If the key is not found, returns the default value and 0.0 duration.

        Args:
            key: The key to remove from the cache.
            default: The value to return if the key is not present in the cache. Defaults to None.

        Returns:
            A tuple of (value, duration), where value is the cached value or default,
            and duration is the time-to-live for the key (or 0.0 if not found).
        """
        try:
            pair = self._raw.remove(key)
        except _core.CoreKeyError:
            return default, 0.0  # type: ignore[return-value]
        else:
            return (pair.value(), pair.duration())

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        ttl: typing.Union[float, timedelta, datetime, None] = None,
    ) -> typing.Union[VT, DT]:
        """
        Inserts a key-value pair into the cache with an optional time-to-live (TTL).

        If the key is not in the cache, it will be inserted with the default value.
        If the key already exists, its current value is returned.

        Args:
            key: The key to insert or retrieve from the cache.
            default: The value to insert if the key is not present. Defaults to None.
            ttl: Optional time-to-live for the key. Can be a float (seconds), timedelta, or datetime.
                 If not specified, the key will not expire.

        Returns:
            The value associated with the key, either existing or the default value.

        Raises:
            ValueError: If the provided TTL is not a positive value.
        """
        if ttl is not None:  # pragma: no cover
            if isinstance(ttl, timedelta):
                ttl = ttl.total_seconds()

            elif isinstance(ttl, datetime):
                ttl = (ttl - datetime.now()).total_seconds()

            if ttl <= 0:
                raise ValueError("ttl must be positive and non-zero")

        return self._raw.setdefault(key, default, ttl)

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair that is closest to expiration.

        Returns:
            A tuple containing the key and value of the removed item.

        Raises:
            KeyError: If the cache is empty.
        """
        try:
            val = self._raw.popitem()
        except _core.CoreKeyError:  # pragma: no cover
            raise KeyError() from None
        else:
            return val.pack2()

    def popitem_with_expire(self) -> typing.Tuple[KT, VT, float]:
        """
        Removes and returns the key-value pair that is closest to expiration, along with its expiration duration.

        Returns:
            A tuple containing the key, value, and expiration duration of the removed item.

        Raises:
            KeyError: If the cache is empty.
        """
        try:
            val = self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None
        else:
            return val.pack3()

    def drain(self, n: int) -> int:  # pragma: no cover
        """Does the `popitem()` `n` times and returns count of removed items."""
        if n <= 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(
        self,
        iterable: typing.Union[dict, typing.Iterable[tuple]],
        ttl: typing.Union[float, timedelta, datetime, None] = None,
    ) -> None:
        """Updates the cache with elements from a dictionary or an iterable object of key/value pairs."""
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        if ttl is not None:  # pragma: no cover
            if isinstance(ttl, timedelta):
                ttl = ttl.total_seconds()

            elif isinstance(ttl, datetime):
                ttl = (ttl - datetime.now()).total_seconds()

            if ttl <= 0:
                raise ValueError("ttl must be positive and non-zero")

        self._raw.update(iterable, ttl)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value, None)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key).value()
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, VTTLCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, VTTLCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items_with_expire(self) -> IteratorView[typing.Tuple[KT, VT, float]]:
        """
        Returns an iterable object of the cache's items (key-value pairs along with their expiration duration).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.pack3())

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.pack2())

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.key())

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        """
        return IteratorView(self._raw.items(), lambda x: x.value())

    def expire(self) -> None:  # pragma: no cover
        """
        Manually removes expired key-value pairs from memory and releases their memory.

        Notes:
            - This operation is typically automatic and does not require manual invocation.
        """
        self._raw.expire()

    def copy(self) -> "VTTLCache[KT, VT]":
        """Returns a shallow copy of the cache"""
        return self.__copy__()

    def __copy__(self) -> "VTTLCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.copy(self._raw)
        return copied

    def __deepcopy__(self, memo) -> "VTTLCache[KT, VT]":
        cls = type(self)
        copied = cls.__new__(cls)
        copied._raw = _std_copy.deepcopy(self._raw, memo)
        return copied

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        cls = type(self)

        return "%s.%s[%d/%d](%s)" % (
            cls.__module__,
            cls.__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self.items(), len(self._raw)),
        )
