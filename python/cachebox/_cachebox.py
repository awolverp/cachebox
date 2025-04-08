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
    This is the base class of all cache classes such as Cache, FIFOCache, ...
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
        return "{}[{}/{}]({})".format(
            type(self).__name__,
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
        return "{}[{}/{}]({})".format(
            type(self).__name__,
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
        return "{}[{}/{}]({})".format(
            type(self).__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )
