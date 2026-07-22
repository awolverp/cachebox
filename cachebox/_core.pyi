import typing
from datetime import datetime, timedelta

from _typeshed import SupportsItems

_use_small_offset_feature: typing.Final[bool]
__version__: typing.Final[str]

KT = typing.TypeVar("KT", bound=typing.Hashable)
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

_IterableType: typing.TypeAlias = (
    typing.Dict[KT, VT]
    | SupportsItems[KT, VT]
    | BaseCacheImpl[KT, VT]
    | typing.Iterable[typing.Tuple[KT, VT]]
)

class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    Base implementation for cache classes.

    This abstract base class defines the generic structure for cache
    implementations.
    """

    def __new__(cls, *args, **kwds) -> typing.Self:
        """
        Allocates memory and returns an uninitialized instance.

        Warning:
            Using the returned instance before calling ``__init__`` is unsafe
            and causes panic errors.
        """
        ...

    def __init__(
        self,
        maxsize: int,
        iterable: _IterableType[KT, VT] | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT], int] | None = None,
    ) -> None:
        """
        Initializes a new instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero,
                the limit is set to ``sys.maxsize`` internally.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate cache capacity to minimize reallocations.
                Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair.
                When ``None``, each entry is assumed to have a size of 1
                (equivalent to ``lambda k, v: 1``). Use this to implement
                weighted caching - for example, sizing entries by memory
                footprint or byte length.

        Note:
            The cache can be pre-sized via ``capacity`` to reduce
            reallocations when the number of expected entries is known
            ahead of time.
        """
        ...

    @property
    def maxsize(self) -> int:
        """The configured ``maxsize``."""
        ...

    @property
    def getsizeof(self) -> typing.Callable[[KT, VT], int] | None:
        """The configured ``getsizeof`` function."""
        ...

    def current_size(self) -> int:
        """
        Returns the current total cumulative size of all stored entries.

        Returns:
            The sum of sizes of all entries currently in the cache.
        """
        ...

    def remaining_size(self) -> int:
        """
        Returns the remaining available size.

        Returns:
            The result of ``maxsize - current_size``.
        """
        ...

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        Returns:
            The current allocated capacity.
        """
        ...

    def __len__(self) -> int:
        """
        Returns the number of entries currently in the cache.

        Returns:
            The number of entries in the cache.
        """
        ...

    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: KT) -> bool: ...
    def contains(self, key: KT) -> bool:
        """
        Returns ``True`` if the cache contains an entry for ``key``.

        Equivalent to ``key in self``. Prefer this method over ``key in self``
        to keep code compatible across different cache policies.

        Args:
            key: The key to look up.

        Returns:
            ``True`` if the key exists in the cache, ``False`` otherwise.
        """
        ...

    def is_empty(self) -> bool:
        """
        Returns ``True`` if the cache is empty.

        Returns:
            ``True`` if the cache contains no entries.
        """
        ...

    def is_full(self) -> bool:
        """
        Returns ``True`` when the cumulative size has reached the maxsize limit.

        Returns:
            ``True`` if the cache is at capacity.
        """
        ...

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
    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> VT | DT: ...
    def pop(self, key: KT, default: DT = ...) -> typing.Union[VT, DT]:
        """
        Removes the specified key and returns the corresponding value.

        Args:
            key: The key to remove.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.

        Raises:
            KeyError: If the key is not found and no ``default`` is provided.
        """
        ...

    def __delitem__(self, key: KT) -> None: ...
    def popitem(self) -> typing.Tuple[KT, VT]: ...
    def drain(self, n: int) -> int:
        """
        Calls ``popitem()`` ``n`` times and returns the count of removed items.

        Args:
            n: The number of items to remove.

        Returns:
            The number of items successfully removed.
        """
        ...

    def shrink_to_fit(self) -> None:
        """Shrinks the internal allocation as close to the current length as possible."""
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from the cache.

        Args:
            reuse: If ``True``, retains the allocated memory for future reuse
                rather than freeing it. Defaults to ``False``.
        """
        ...

    def __eq__(self, other: typing.Any) -> bool: ...
    def __ne__(self, other: typing.Any) -> bool: ...
    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]: ...
    def values(self) -> typing.Iterable[VT]: ...
    def keys(self) -> typing.Iterable[KT]: ...
    def __iter__(self) -> typing.Iterator[KT]: ...
    def copy(self) -> typing.Self: ...
    def __copy__(self) -> typing.Self: ...
    def __getstate__(self) -> object: ...
    def __setstate__(self, state: object) -> None: ...
    def __repr__(self) -> str: ...

class Cache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe, memory-efficient key-value cache with no eviction policy.

    Items remain in the cache until manually removed or the cache is cleared.

    ``Cache`` is essentially a configurable hashmap-like store. When an item is
    inserted, it is stored directly without any ordering, priority tracking, or
    access metadata. If a maximum size is configured, insertions beyond that
    limit are rejected with an ``OverflowError``. All read and write operations
    are thread-safe.

    Because no eviction logic runs in the background, there is no overhead from
    tracking usage order, frequency counters, or expiry timestamps.

    |              | get   | insert  | delete | popitem |
    | ------------ | ----- | ------- | ------ | ------- |
    | Worse-case   | O(1)  | O(1)    | O(1)   | N/A     |

    Pros:
        - Minimal overhead: no bookkeeping for eviction means lower CPU and
          memory usage per entry compared to policy-based caches.
        - Predictable behavior: items are never silently removed, so cache hits
          are deterministic once an item is stored.
        - Thread-safe: safe for concurrent reads and writes out of the box.
        - Configurable capacity: a hard size limit prevents unbounded memory
          growth.

    Cons:
        - No automatic eviction: the cache can fill up and stop accepting new
          entries if a max size is set, requiring manual management.
        - Unordered: unlike a standard ``dict`` (Python 3.7+), insertion order
          is not preserved.
        - Not suitable for volatile data: stale entries persist forever unless
          explicitly invalidated.

    Use ``Cache`` when you have a fixed, well-known set of keys that are
    expensive to compute and never go stale (e.g. parsed config values,
    compiled regex patterns, loaded templates), and when the lowest possible
    overhead is required.

    Avoid it when cached data can become stale, when the working set is
    unpredictable in size, or when automatic memory pressure relief is needed.

    ```python
    from cachebox import Cache

    cache = Cache(maxsize=100, iterable=None, capacity=100)

    # behaves like a regular dict
    cache["key"] = "value"
    # using `.insert(key, value)` is recommended
    cache.insert("key", "value")

    print(cache["key"])  # value

    del cache["key"]
    cache["key"]  # KeyError: key

    # cachebox.Cache does not have any policy, so will raise OverflowError if the capacity is exceeded
    cache.update({i:i for i in range(200)})
    # OverflowError: The cache has reached the bound.
    ```
    """

    # | Class | get | insert | delete | popitem |
    # |---|---|---|---|---|
    # | \`Cache\` | O(1) | O(1) | O(1) | N/A |
    # | \`FIFOCache\` | O(1) | O(1) | O(min(i, n-i)) | O(1) |
    # | \`RRCache\` | O(1) | O(1) | O(1) | O(1) |
    # | \`LRUCache\` | O(1)~ | O(1)~ | O(1)~ | O(1)~ |
    # | \`LFUCache\` | O(1)~ | O(1)~ | O(min(i, n-i)) | O(1)~ |
    # | \`TTLCache\` | O(1)~ | O(1)~ | O(min(i, n-i)) | O(n) |
    # | \`VTTLCache\` | O(1)~ | O(1)~ | O(min(i, n-i)) | O(1)~ |

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).

        Raises:
            OverflowError: If the cache has reached its ``maxsize`` limit,
                since this class has no eviction algorithm.
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Always raises ``OverflowError``.

        ``Cache`` has no policy or algorithm to select an item for eviction.

        Raises:
            OverflowError: Always, because ``Cache`` has no eviction policy.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating. Items are not ordered.

        Returns:
            An iterable of ``(key, value)`` tuples.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating. Keys are not ordered.

        Returns:
            An iterable of keys.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating. Values are not ordered.

        Returns:
            An iterable of values.
        """
        ...

class FIFOCache(BaseCacheImpl[KT, VT]):
    """
    A cache with a First-In-First-Out (FIFO) eviction policy.

    When the cache is full, the oldest inserted item is always the first to be
    removed, regardless of how often it has been accessed.

    Items are stored in insertion order. When capacity is reached, the item
    that has been present the longest is evicted. There is no concept of
    "recently used" or "frequently used" - age alone determines eviction order.
    Conceptually it behaves like a queue: new items join the back and evictions
    come from the front.

    This implementation backs that queue with a double-ended queue for O(1)
    front removal, paired with a hash map for O(1) key lookups. Logical indices
    (a monotonically increasing counter) are stored in the table rather than
    physical deque positions, so eviction never requires rewriting the index.
    A ``front_offset`` counter recovers physical positions at read time as
    ``entries[table[key] - front_offset]``.

    |              | get   | insert  | delete           | popitem |
    | ------------ | ----- | ------- | ---------------- | ------- |
    | Worse-case   | O(1)  | O(1)    | O(min(i, n-i))   | O(n) - very rare |

    Pros:
        - Insert, lookup, and evict are all O(1) amortized.
        - Eviction order is fully deterministic and easy to reason about.
        - No per-read overhead: unlike LRU, FIFO requires no bookkeeping on
          cache hits.

    Cons:
        - Access-blind eviction: a hot item is evicted just as readily as one
          never read, hurting hit rates on workloads with temporal locality.
        - Logical-index indirection adds internal complexity vs. a naive queue.
        - A rare O(n) index rebase (when ``front_offset`` nears
          ``usize::MAX - isize::MAX``) introduces an occasional latency spike.

    Use ``FIFOCache`` when eviction order must be predictable and auditable,
    access patterns are roughly uniform, or read overhead must be minimal
    (insert-heavy workloads with infrequent re-reads).

    Avoid it when the workload has strong temporal locality; in those cases LRU
    or LFU will deliver meaningfully better hit rates.

    ```python
    from cachebox import FIFOCache

    cache = FIFOCache(5, {i:i*2 for i in range(5)})

    print(len(cache)) # 5
    cache["new-key"] = "new-value"
    print(len(cache)) # 5

    print(cache.get(3, "default-val")) # 6
    print(cache.get(6, "default-val")) # default-val

    print(cache.popitem()) # (1, 2)

    # Returns the first key in cache; this is the one which will be removed by `popitem()`.
    print(cache.first())
    ```
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the oldest item in the cache.

        Returns:
            A ``(key, value)`` tuple for the item that was inserted first.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in insertion order.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an ordered iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of keys in insertion order.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an ordered iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of values in insertion order.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key at position ``n`` in insertion order.

        The key at position 0 is the one that will be removed by ``popitem()``.

        Args:
            n: The index to look up. Defaults to 0 (the oldest item).

        Returns:
            The key at the given index.

        Raises:
            IndexError: If the cache is empty or ``n`` is out of range.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the most recently inserted key. Equivalent to ``self.first(-1)``.

        Returns:
            The key of the most recently inserted item.

        Raises:
            IndexError: If the cache is empty.
        """
        ...

class RRCache(BaseCacheImpl[KT, VT]):
    """A thread-safe, memory-efficient cache with a Random Replacement eviction policy.

    When the cache reaches its maximum size, a randomly selected item is
    evicted to make room for new entries.

    Items are stored without any ordering or priority tracking. The Random
    Replacement policy selects entries for eviction uniformly at random,
    ensuring fair treatment across all cached items regardless of access
    patterns.

    |              | get   | insert  | delete | popitem(i)     |
    | ------------ | ----- | ------- | ------ | -------------- |
    | Worse-case   | O(1)  | O(1)    | O(1)   | O(min(i, n-i)) |

    Pros:
        - Low overhead: computationally cheap compared to tracking access order
          or frequency.
        - Thread-safe: safe for concurrent reads and writes out of the box.
        - Configurable capacity: a hard size limit prevents unbounded memory
          growth while allowing new entries through automatic eviction.
        - No indefinite staleness: items are eventually replaced by the
          eviction policy.

    Cons:
        - Non-deterministic eviction: random selection means recently cached or
          frequently accessed items may be unexpectedly removed.
        - Unordered: insertion order is not preserved.
        - Less optimal than LRU/LFU on skewed access patterns.

    Use ``RRCache`` when the working set can grow unpredictably, access
    patterns are roughly uniform, and low overhead with simple eviction logic
    is preferred.

    Avoid it when access patterns are highly skewed, cache hits are
    mission-critical, or fine-grained eviction control is required.

    ```python
    from cachebox import RRCache

    cache = RRCache(10, {i:i for i in range(10)})
    print(cache.is_full()) # True
    print(cache.is_empty()) # False

    # Returns a random key
    print(cache.random_key()) # 4
    ```
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Randomly selects, removes, and returns a ``(key, value)`` pair.

        Returns:
            A randomly chosen ``(key, value)`` tuple.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating. Items are not ordered.

        Returns:
            An iterable of ``(key, value)`` tuples.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating. Keys are not ordered.

        Returns:
            An iterable of keys.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating. Values are not ordered.

        Returns:
            An iterable of values.
        """
        ...

    def random_key(self) -> KT:
        """
        Randomly selects and returns a key from the cache.

        Returns:
            A randomly chosen key.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

class LRUCache(BaseCacheImpl[KT, VT]):
    """
    A cache with a Least-Recently-Used (LRU) eviction policy.

    When the cache is full, the item that has not been accessed for the longest
    time is removed first, regardless of how many times it was accessed in the
    past.

    Items are tracked by access recency - every read or write promotes an item
    to "most recently used". When capacity is reached, the least recently used
    item (accessed longest ago) is evicted.

    This implementation pairs a doubly-linked list with a hash map. The list
    maintains items in access order (most recently used at the back, least
    recently used at the front); the hash map stores cursors into the list for
    O(1) lookups. On every access the item is moved to the back. On eviction
    the front item is removed. A running total enables O(1) capacity checks.

    |              | get   | insert  | delete(i) | popitem |
    | ------------ | ----- | ------- | --------- | ------- |
    | Worse-case   | O(1)~ | O(1)~   | O(1)~ | O(1)~ |

    Pros:
        - Excellent hit rates on temporal-locality workloads.
        - Insert, lookup, and evict are all O(1) amortized.
        - Automatically adapts to access patterns without manual tuning.
        - Per-hit cost is minimal (O(1) linked-list manipulation).

    Cons:
        - Per-read overhead from updating the linked list on every cache hit.
        - Burst traffic can keep a transiently hot item alive at the expense of
          items with better long-term utility.
        - Implementation complexity from doubly-linked list and cursor-based
          hash table.
        - Memory overhead from storing prev/next pointers for every entry.

    Use ``LRUCache`` when the workload exhibits temporal locality, hit rate is
    the primary metric, or access patterns are unknown or unpredictable.

    Avoid it for write-heavy workloads with few re-reads, ultra-low-latency
    requirements, or frequency-heavy bimodal access patterns (consider LFU
    instead).

    ```python
    from cachebox import LRUCache

    cache = LRUCache(0, {i:i*2 for i in range(10)})

    # access `1`
    print(cache[0]) # 0
    print(cache.least_recently_used()) # 1
    print(cache.popitem()) # (1, 2)

    # .peek() searches for a key-value in the cache and returns it without moving the key to recently used.
    print(cache.peek(2)) # 4
    print(cache.popitem()) # (3, 6)
    ```
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the least recently used item.

        Returns:
            A ``(key, value)`` tuple for the least recently used item.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in access order.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an ordered iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of keys in access order.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an ordered iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of values in access order.
        """
        ...

    def peek(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a key without updating its recency.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def least_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key that has not been accessed for the longest time.

        Returns:
            The least recently used key.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def most_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key that was accessed most recently.

        Returns:
            The most recently used key.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

class LFUCache(BaseCacheImpl[KT, VT]):
    """
    A cache with a Least-Frequently-Used (LFU) eviction policy.

    When the cache is full, the item with the lowest access count is evicted
    first. Ties in frequency are broken by recency - among equally rare items,
    the oldest is evicted.

    Access counts are tracked per key. This implementation uses a lazy binary
    min-heap keyed on access frequency, paired with a hash map that maps each
    key to its cursor (a stable pointer into the heap's backing buffer). The
    heap is "lazy": it does not restore the heap invariant after every frequency
    increment; instead it sets a dirty flag and defers re-sorting until the
    next eviction, amortising heap-maintenance cost across many hits.

    On a cache hit the frequency counter is incremented in O(1) and the heap is
    marked dirty. On eviction the heap is sorted if dirty, then the
    minimum-frequency item is popped in O(n log n) worst-case (amortised
    O(log n) under typical distributions). Lookups are O(1) via the hash map.

    |              | get   | insert  | delete(i)      | popitem |
    | ------------ | ----- | ------- | -------------- | ------- |
    | Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(1)~   |

    Pros:
        - Frequency-aware eviction protects hot items under heavy cache
          pressure.
        - O(1) cache hits: incrementing a counter and marking the heap dirty
          is constant-time work with no structural reorganisation.
        - Lazy heap sorting amortises the O(n log n) sort cost across many
          inserts and hits.

    Cons:
        - Eviction is O(n log n) worst-case, introducing latency spikes under
          adversarial access patterns.
        - Frequency counters accumulate indefinitely, causing "cache pollution"
          where historically hot but currently cold items monopolise capacity.
        - Access patterns must be skewed for LFU to outperform simpler
          policies; on uniform workloads the extra bookkeeping is pure overhead.

    Use ``LFUCache`` when the workload has a stable hot set, cache pollution
    from one-time scans is a concern, or hit rate matters more than worst-case
    eviction latency.

    Avoid it when access patterns shift rapidly (use LRU instead) or when all
    keys are accessed with roughly equal probability.

    ```python
    from cachebox import LFUCache

    cache = cachebox.LFUCache(5)
    cache.insert('first', 'A')
    cache.insert('second', 'B')

    # access 'first' twice
    cache['first']
    cache['first']

    # access 'second' once
    cache['second']

    assert cache.least_frequently_used() == 'second'
    assert cache.least_frequently_used(2) is None # 2 is out of range

    for item in cache.items_with_frequency():
        print(item)
    # ('second', 'B', 1)
    # ('first', 'A', 2)
    ```
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the least frequently used item.

        Returns:
            A ``(key, value)`` tuple for the item with the lowest access count.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in frequency order.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an ordered iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of keys in frequency order.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an ordered iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of values in frequency order.
        """
        ...

    def items_with_frequency(self) -> typing.Iterable[typing.Tuple[KT, VT, int]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs with their
        frequency counter.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in frequency order.
        """
        ...

    def peek(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a key without incrementing its frequency counter.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.
        """
        ...

    def least_frequently_used(self, n: int = 0) -> KT:
        """
        Returns the key with the lowest access count.

        Args:
            n: If given, returns the ``n``-th least frequently used key
                (0-indexed). Defaults to 0.

        Returns:
            The key with the ``n``-th lowest access count.

        Raises:
            IndexError: If the cache is empty or ``n`` is out of range.

        Warning:
            This method may re-sort the cache. Do not call it while iterating
            over the cache.
        """
        ...

class TTLCache(BaseCacheImpl[KT, VT]):
    """
    A cache with time-to-live (TTL) expiration.

    Items expire automatically after a configurable duration. Eviction follows
    a FIFO order among non-expired items when the cache is full.
    """

    def __init__(
        self,
        maxsize: int,
        global_ttl: float | timedelta,
        iterable: _IterableType[KT, VT] | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT], int] | None = None,
    ) -> None:
        """
        Initializes a new TTLCache instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero,
                the limit is set to ``sys.maxsize`` internally.
            global_ttl: Default time-to-live for all entries, in seconds or as
                a ``timedelta``.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate cache capacity to minimize reallocations.
                Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair.
                When ``None``, each entry is assumed to have a size of 1.
        """
        ...

    @property
    def global_ttl(self) -> float:
        """The configured ``global_ttl`` in seconds."""
        ...

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Inserts a key-value pair and returns the previous value if present.

        Equivalent to ``self[key] = value``, but returns a value. Prefer this
        method over direct assignment to keep code compatible across different
        cache policies.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the item that has been in the cache the longest.

        Returns:
            A ``(key, value)`` tuple for the oldest item.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in insertion order.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an ordered iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of keys in insertion order.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an ordered iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of values in insertion order.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the key at position ``n`` in insertion order.

        The key at position 0 is the one that will be removed by ``popitem()``.

        Args:
            n: The index to look up. Defaults to 0 (the oldest item).

        Returns:
            The key at the given index.

        Raises:
            IndexError: If the cache is empty or ``n`` is out of range.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the most recently inserted key. Equivalent to ``self.first(-1)``.

        Returns:
            The key of the most recently inserted item.

        Raises:
            IndexError: If the cache is empty.
        """
        ...

    def expire(self, *, reuse: bool = False) -> None:
        """
        Manually removes all expired key-value pairs from the cache.

        Args:
            reuse: If ``True``, retains the allocated memory for future reuse
                rather than freeing it. Defaults to ``False``.
        """
        ...

    def get_with_expire(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Retrieves a value along with its remaining TTL.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            A tuple of ``(value, remaining_ttl)`` where ``remaining_ttl`` is
            the expiration duration in seconds, or ``0.0`` if the key was not
            found.
        """
        ...

    def pop_with_expire(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Removes a key and returns its value along with its remaining TTL.

        Args:
            key: The key to remove.
            default: Value to return if the key is not found.

        Returns:
            A tuple of ``(value, remaining_ttl)`` where ``remaining_ttl`` is
            the expiration duration in seconds, or ``0.0`` if the key was not
            found.
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float]:
        """
        Removes and returns the oldest item along with its remaining TTL.

        Returns:
            A tuple of ``(key, value, remaining_ttl)`` where ``remaining_ttl``
            is the expiration duration in seconds.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items_with_expire(self) -> typing.Iterable[typing.Tuple[KT, VT, float]]:
        """
        Returns an ordered iterable of items with their remaining TTL.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value, remaining_ttl)`` tuples in insertion
            order, where ``remaining_ttl`` is in seconds.
        """
        ...

class VTTLCache(BaseCacheImpl[KT, VT]):
    """
    A cache with a Variable Time-To-Live (VTTL) eviction policy.

    Each item can be inserted with its own individual TTL (time-to-live). When
    an item's TTL expires, it is considered stale and will be evicted. Items
    inserted without a TTL never expire and are only evicted when the cache
    reaches capacity.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: _IterableType[KT, VT] | None = None,
        ttl: float | timedelta | datetime | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT], int] | None = None,
    ) -> None:
        """
        Initializes a new TTLCache instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero,
                the limit is set to ``sys.maxsize`` internally.
            iterable: Initial data to populate the cache.
            ttl: Time-to-live duration for ``iterable`` items. This *is not* a global ttl.
            capacity: Pre-allocate cache capacity to minimize reallocations.
                Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair.
                When ``None``, each entry is assumed to have a size of 1.
        """
        ...

    def insert(
        self,
        key: KT,
        value: VT,
        ttl: float | timedelta | datetime | None = None,
    ) -> typing.Optional[VT]:
        """
        Insert a key-value pair into the cache with an optional time-to-live (TTL).
        Returns the previous value associated with the key, if it existed.

        Args:
            key: The key to insert or update.
            value: The value to associate with ``key``.
            ttl: An optional time-to-live duration for the item.

        Returns:
            ``None`` if the key was not previously present; the old value if
            the key already existed (the key itself is not updated).
        """
        ...

    def update(
        self,
        iterable: _IterableType[KT, VT],
        ttl: float | timedelta | datetime | None = None,
    ) -> None:
        """
        Updates the cache with elements from a dictionary or iterable of key-value pairs.

        Args:
            iterable: A dictionary, object supporting ``items()``, another
                cache instance, or an iterable of ``(key, value)`` tuples.
            ttl: An optional time-to-live duration for items.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        ttl: float | timedelta | datetime | None = None,
    ) -> typing.Optional[VT | DT]:
        """
        Get `key`s value, or atomatically insert `default` and return it.
        If `key` exists, its current value is returned and `default` is ignored.
        Otherwise `default` is inserted for `key` and returned.

        Note:
            Use `setdefault_with`, if computing the value is expensive or has side
            effectes.

        Args:
            key: The key to look up or insert.
            default: The value to insert if ``key`` is not in the cache.
                Defaults to ``None``.
            ttl: An optional time-to-live duration for item.
        """
        ...

    def setdefault_with(
        self,
        key: KT,
        factory: typing.Callable[[], DT],
        ttl: float | timedelta | datetime | None = None,
    ) -> VT | DT:
        """
        Get `key`s value, or atomatically create and insert one via `factory`.
        If `key` exists, its current value is returned and `factory` is not called.
        Otherwise `factory` is called exactly once under an internal lock, its
        result is inserted and returned.

        Args:
            key: The key to look up or insert.
            factory: The factory to call and get default value from if ``key`` is not in the cache.
            ttl: An optional time-to-live duration for item.

        Warning:
            `factory` must not call back into this cache (deadlock risk) or block
            for long. If `factory` raises, nothing is inserted and the exception
            propagates.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes and returns the key-value pair that is closest to expiration.

        Returns:
            A tuple containing the key and value of the removed item.

        Raises:
            KeyError: If the cache is empty.
        """

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an ordered iterable of the cache's ``(key, value)`` pairs.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value)`` tuples in insertion order.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an ordered iterable of the cache's keys.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of keys in insertion order.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an ordered iterable of the cache's values.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of values in insertion order.
        """
        ...

    def expire(self, *, reuse: bool = False) -> None:
        """
        Manually removes all expired key-value pairs from the cache.

        Args:
            reuse: If ``True``, retains the allocated memory for future reuse
                rather than freeing it. Defaults to ``False``.
        """
        ...

    def get_with_expire(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Tuple[typing.Union[VT, DT], float | None]:
        """
        Retrieves a value along with its remaining TTL.

        Args:
            key: The key to look up.
            default: Value to return if the key is not found.

        Returns:
            A tuple of ``(value, remaining_ttl)`` where ``remaining_ttl`` is
            the expiration duration in seconds, or ``0.0`` if the key was not
            found.
        """
        ...

    def pop_with_expire(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Tuple[typing.Union[VT, DT], float | None]:
        """
        Removes a key and returns its value along with its remaining TTL.

        Args:
            key: The key to remove.
            default: Value to return if the key is not found.

        Returns:
            A tuple of ``(value, remaining_ttl)`` where ``remaining_ttl`` is
            the expiration duration in seconds, or ``0.0`` if the key was not
            found.
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float | None]:
        """
        Removes and returns the oldest item along with its remaining TTL.

        Returns:
            A tuple of ``(key, value, remaining_ttl)`` where ``remaining_ttl``
            is the expiration duration in seconds.

        Raises:
            KeyError: If the cache is empty.
        """
        ...

    def items_with_expire(self) -> typing.Iterable[typing.Tuple[KT, VT, float | None]]:
        """
        Returns an ordered iterable of items with their remaining TTL.

        Warning:
            Do not modify the cache while iterating.

        Returns:
            An iterable of ``(key, value, remaining_ttl)`` tuples in insertion
            order, where ``remaining_ttl`` is in seconds.
        """
        ...
