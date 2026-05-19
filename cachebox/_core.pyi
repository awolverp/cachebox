import typing

from _typeshed import SupportsItems

_IterableType: typing.TypeAlias = (
    typing.Dict[KT, VT]
    | SupportsItems[KT, VT]
    | BaseCacheImpl[KT, VT]
    | typing.Iterable[typing.Tuple[KT, VT]]
)

KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    Base implementation for cache classes in the cachebox library.

    This abstract base class defines the generic structure for cache implementations,
    supporting different key and value types through generic type parameters.
    Serves as a foundation for specific cache variants like Cache and FIFOCache.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: _IterableType[KT, VT] | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT]] | None = None,
    ) -> None: ...
    @property
    def maxsize(self) -> int: ...
    @property
    def getsizeof(self) -> typing.Callable[[KT, VT]] | None: ...
    def current_size(self) -> int: ...
    def remaining_size(self) -> int: ...
    def capacity(self) -> int: ...
    def __len__(self) -> int: ...
    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: KT) -> bool: ...
    def contains(self, key: KT) -> bool: ...
    def is_empty(self) -> bool: ...
    def is_full(self) -> bool: ...
    def insert(
        self, key: KT, value: VT, *args: typing.Any, **kwargs: typing.Any
    ) -> typing.Optional[VT]: ...
    def __setitem__(self, key: KT, value: VT) -> None: ...
    def update(
        self,
        iterable: _IterableType[KT, VT],
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> None: ...
    def get(
        self, key: KT, default: typing.Optional[DT] = None
    ) -> typing.Union[VT, DT]: ...
    def __getitem__(self, key: KT) -> VT: ...
    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT | DT]: ...
    def pop(self, key: KT, default: DT = ...) -> typing.Union[VT, DT]: ...
    def __delitem__(self, key: KT) -> None: ...
    def popitem(self) -> typing.Tuple[KT, VT]: ...
    def drain(self, n: int) -> int: ...
    def shrink_to_fit(self) -> None: ...
    def clear(self, *, reuse: bool = False) -> None: ...
    def __eq__(self, other: typing.Any) -> bool: ...
    def __ne__(self, other: typing.Any) -> bool: ...
    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]: ...
    def values(self) -> typing.Iterable[VT]: ...
    def keys(self) -> typing.Iterable[KT]: ...
    def __iter__(self) -> typing.Iterator[KT]: ...
    def copy(self) -> typing.Self: ...
    def __copy__(self) -> typing.Self: ...
    def __repr__(self) -> str: ...

class Cache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe, memory-efficient key-value cache with no eviction policy.
    items remain in the cache until manually removed or the cache is cleared.

    ## How It Works
    `Cache` is essentially a configurable hashmap-like store. When an item is inserted:
    - It is stored directly without any ordering, priority tracking, or access metadata.
    - If a maximum size is configured, insertions beyond that limit are rejected (raises OverflowError).
    - All read and write operations are thread-safe, making it safe for concurrent access without
      external locking.

    Because no eviction logic runs in the background, there is no overhead from tracking usage order,
    frequency counters, or expiry timestamps.

    ### Pros
    - Minimal overhead - no bookkeeping for eviction means lower CPU and memory usage per entry compared
      to policy-based caches.
    - Predictable behavior - items are never silently removed, so cache hits are deterministic once an
      item is stored.
    - Thread-safe - safe for concurrent reads and writes out of the box.
    - Configurable capacity - a hard size limit prevents unbounded memory growth.

    ### Cons
    - No automatic eviction - the cache can fill up and stop accepting new entries if a max size is set,
      requiring manual management.
    - Unordered - unlike a standard dict (Python 3.7+), insertion order is not preserved.
    - Not suitable for volatile data - stale entries persist forever unless explicitly invalidated.

    ## When to Use It
    `Cache` is the right choice when:
    - You have a fixed, well-known set of keys that are expensive to compute and never go stale
      (e.g., parsed config values, compiled regex patterns, loaded templates).
    - The cached data has no meaningful expiry - it's either always valid or always explicitly invalidated.
    - You need the lowest possible overhead and can guarantee the cache won't grow uncontrollably.

    Avoid it when cached data can become stale, when the working set is unpredictable in size, or when you need automatic
    memory pressure relief.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: _IterableType[KT, VT] | None = None,
        *,
        capacity: int = ...,
        getsizeof: typing.Callable[[KT, VT]] | None = ...,
    ) -> None:
        """
        Initialize a new Cache instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero, the limit is set to sys.maxsize internally.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate hash table capacity to minimize reallocations. Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair. When `None`, each
                    entry is assumed to have a size of 1 (equivalent to `lambda k, v: 1`).
                    Use this to implement weighted caching — for example, sizing entries by
                    memory footprint or byte length.

        The cache can be pre-sized via `capacity` to reduce hash table reallocations when
        the number of expected entries is known ahead of time.
        """
        ...

    @property
    def maxsize(self) -> int:
        """Returns the specified `maxsize`"""
        ...

    @property
    def getsizeof(self) -> typing.Callable[[KT, VT]] | None:
        """Returns the `getsizeof` function"""
        ...

    def current_size(self) -> int:
        """Returns the current total cumulative size consumed by all stored entries."""
        ...

    def remaining_size(self) -> int:
        """Returns the remaining size. Equals to `maxsize - current_size`"""
        ...

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        ...

    def __len__(self) -> int:
        """Returns the number of entries currently in the cache."""
        ...

    def contains(self, key: KT) -> bool:
        """
        Returns `true` if the cache contains an entry for `key`. Equals to `key in self`.

        It's recommended to use this method instead of `key in self`, as it keeps code
        compatible across different cache policies.
        """
        ...

    def is_empty(self) -> bool:
        """Returns `True` if cache is empty. Exactly like `bool(self)`."""
        ...

    def is_full(self) -> bool:
        """Returns `True` when the cumulative size has reached the maxsize limit."""
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        It's recommended to use this method instead of `self[key] = value`, as it keeps code
        compatible across different cache policies.

        Note: raises `OverflowError` if the cache reached the maxsize limit,
        because this class does not have any algorithm.
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
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
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Returns the value for key if key is in the cache, else default.
        """
        ...

    def pop(self, key: KT, default: DT = ...) -> typing.Union[VT, DT]:
        """
        Removes specified key and returns the corresponding value.

        If the key is not found, returns the `default` if given; otherwise, raise a KeyError.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Always raises `OverflowError` because `Cache` has neither policy nor algorithm to evict items."""
        ...

    def drain(self, n: int) -> int:
        """Calls the `popitem()` `n` times and returns count of removed items."""
        ...

    def shrink_to_fit(self) -> None:
        """Shrinks the internal allocation as close to the current length as possible."""
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If `reuse` is True, will not free the memory for reusing in the future.
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
