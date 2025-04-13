from . import _core
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")


def _items_to_str(items, length):
    if length <= 50:
        return "{" + ", ".join(f"{k}: {v}" for k, v in items) + "}"

    c = 0
    left = []

    while c < length:
        k, v = next(items)

        if c <= 50:
            left.append(f"{k}: {v}")

        else:
            break

        c += 1

    return "{%s, ... %d more ...}" % (", ".join(left), length - c)


class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    Base implementation for cache classes in the cachebox library.
    
    This abstract base class defines the generic structure for cache implementations,
    supporting different key and value types through generic type parameters.
    Serves as a foundation for specific cache variants like Cache and FIFOCache.
    """
    pass


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

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[dict, typing.Iterable[tuple]] = None,
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
        """Equals to `self[key]`, but returns `default` if the cache don't have this key present."""
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache. Return the value for key if key is
        in the cache, else `default`.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.NoReturn:  # pragma: no cover
        raise NotImplementedError()

    def drain(self) -> typing.NoReturn:  # pragma: no cover
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
    Supports various operations like insertion, retrieval, deletion, and iteration with O(1) complexity.

    Attributes:
        maxsize: The maximum number of items the cache can hold.
        capacity: The initial capacity of the cache before resizing.

    Key features:
    - Deterministic item eviction order (oldest items removed first)
    - Efficient key-value storage and retrieval
    - Supports dictionary-like operations
    - Allows optional initial data population
    """

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
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

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

    Supports operations like insertion, retrieval, deletion, and iteration with O(1) complexity.
    """

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
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

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
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        return self._raw.insert(key, value)

    def peek(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).
        """
        try:
            return self._raw.peek(key)
        except _core.CoreKeyError:
            return default

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

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
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;
        """
        return self._raw.insert(key, value)

    def peek(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Searches for a key-value in the cache and returns it (without moving the key to recently used).
        """
        try:
            return self._raw.peek(key)
        except _core.CoreKeyError:
            return default

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Equals to `self[key]`, but returns `default` if the cache don't have this key present.
        """
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

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
