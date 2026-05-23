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
    ) -> None:
        """
        Initialize a new instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero, the limit is set to sys.maxsize internally.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate cache capacity to minimize reallocations. Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair. When `None`, each
                    entry is assumed to have a size of 1 (equivalent to `lambda k, v: 1`).
                    Use this to implement weighted caching — for example, sizing entries by
                    memory footprint or byte length.

        The cache can be pre-sized via `capacity` to reduce reallocations when
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

    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: KT) -> bool: ...
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
    def pop(self, key: KT, default: DT = ...) -> typing.Union[VT, DT]:
        """
        Removes specified key and returns the corresponding value.

        If the key is not found, returns the `default` if given; otherwise, raise a KeyError.
        """
        ...

    def __delitem__(self, key: KT) -> None: ...
    def popitem(self) -> typing.Tuple[KT, VT]: ...
    def drain(self, n: int) -> int:
        """
        Calls the `popitem()` `n` times and returns count of removed items.
        """
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
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Returns the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Always raises `OverflowError` because `Cache` has neither policy nor algorithm to evict items."""
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

class FIFOCache(BaseCacheImpl[KT, VT]):
    """
    A First-In-First-Out (FIFO) cache eviction policy: when the cache is full, the oldest
    inserted item is always the first to be removed, regardless of how often it has been accessed.

    ## How It Works
    The FIFO algorithm is one of the simplest cache eviction strategies. Items are stored in
    insertion order, and when the cache reaches capacity, the item that has been there the
    longest is evicted to make room. There is no concept of "recently used" or "frequently used"
    - age alone determines eviction order. Conceptually, it behaves like a queue: new items
    join the back, and evictions come from the front.

    This implementation backs that queue with a `double-ended queue` for O(1) front removal,
    paired with a `hash map` for O(1) key lookups. Rather than storing physical indices into
    the deque (which shift every time an item is evicted from the front), the table stores
    logical indices - a monotonically increasing counter assigned at insertion time.
    A separate `front_offset` counter tracks how many items have ever been evicted; the physical
    position of any key is recovered at read time as `entries[table[key] - front_offset]`,
    keeping both eviction and lookup O(1) without any per-eviction rewriting of the table.

    ### Pros
    - Insert, lookup, and evict are all O(1) amortized: the `front_offset` trick eliminates the O(n)
      index-shifting that a native implementation would require on every eviction.
    - Eviction order is fully deterministic: the oldest item always goes first, independent of access
      patterns, making behaviour easy to reason about and reproduce in tests.
    - No per-read overhead. Unlike LRU, FIFO requires no bookkeeping on cache hits.

    ### Cons
    - Access-blind eviction. A hot item accessed thousands of times is evicted just as readily as one
      that has never been read. Hit rates suffer on workloads with strong temporal locality.
    - The logical-index indirection adds a layer of internal complexity compared to a naïve queue-based cache.
    - The rare O(n) index rebase (triggered when `front_offset` nears `usize::MAX - isize::MAX`) introduces
      an occasional latency spike. Amortized cost is negligible, but worst-case latency is unbounded in principle.

    ## When to use it
    Reach for `FIFOPolicy` when:
    - Eviction order must be predictable and auditable: streaming pipelines, sequential batch processors, or
      any context where deterministic behaviour simplifies debugging.
    - Access patterns are roughly uniform, so there is no meaningful "hot" subset of keys that a recency or
      frequency-aware policy could exploit.
    - Read overhead must be minimal: FIFO's zero-cost hits make it preferable to LRU in insert-heavy workloads
      with infrequent re-reads.

    Avoid it when your workload has strong temporal locality. If recently or frequently accessed items are likely
    to be needed again soon, an LRU or LFU policy will deliver meaningfully better hit rates.
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        It's recommended to use this method instead of `self[key] = value`, as it keeps code
        compatible across different cache policies.
        """
        ...

    def update(self, iterable: _IterableType[KT, VT]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Returns the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the element that has been in the cache the longest.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are ordered.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are ordered.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are ordered.
        """
        ...

    def first(self, n: int = 0) -> typing.Optional[KT]:
        """
        Returns the first key in cache; this is the one which will be removed by `popitem()` (if n == 0).
        By using `n` parameter, you can browse order index by index.

        Raises `IndexError` if cache is empty, or `n` is out of range.
        """
        ...

    def last(self) -> typing.Optional[KT]:
        """
        Returns the last key in cache. Equals to `self.first(-1)`.

        Raises `IndexError` if cache is empty.
        """
        ...

class RRCache(BaseCacheImpl[KT, VT]):
    """
    A thread-safe, memory-efficient key-value cache with Random Replacement eviction policy.
    When the cache reaches its maximum size, an item is randomly selected and
    evicted to make room for new entries.

    ## How It Works
    `RRCache` is a configurable hashmap-like store with automatic eviction. When an item is inserted:
    - It is stored directly without any ordering or priority tracking.
    - If a maximum size is configured and the cache is full, a random entry is evicted to make room
      for the new item.
    - All read and write operations are thread-safe, making it safe for concurrent access without
      external locking.

    The Random Replacement policy selects entries for eviction uniformly at random, ensuring fair
    treatment across all cached items regardless of access patterns.

    ### Pros
    - Low overhead: Random Replacement is computationally cheap compared to tracking access order or frequency.
    - Thread-safe: safe for concurrent reads and writes out of the box.
    - Configurable capacity: a hard size limit prevents unbounded memory growth while allowing new entries
      through automatic eviction.
    - No staleness issues: items persist only as long as they remain unselected by the eviction policy,
      preventing indefinite accumulation of stale data.

    ### Cons
    - Non-deterministic eviction: random selection means you cannot predict which entry will be removed,
      potentially evicting recently cached or frequently accessed items.
    - Unordered: insertion order is not preserved.
    - Less optimal than LRU/LFU: for workloads with skewed access patterns, Random Replacement will
      evict frequently used items more often than policy-aware caches.

    ## When to Use It
    `RRCache` is the right choice when:
    - You have a working set that can grow unpredictably and requires automatic memory management.
    - Access patterns are relatively uniform and predictable, so random eviction is not significantly
      worse than smarter policies.
    - You need low computational overhead and simple eviction logic.
    - You want to prevent unbounded memory growth without the complexity of tracking usage metadata.

    Avoid it when you have highly skewed access patterns (where certain items are accessed far more
    frequently than others), when cache hits are mission-critical and predictability matters, or when
    you need fine-grained control over what gets evicted.
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        It's recommended to use this method instead of `self[key] = value`, as it keeps code
        compatible across different cache policies.
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
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Returns the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """Randomly selects and removes a (key, value) pair from the cache."""
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

    def random_key(self) -> KT:
        """
        Randomly selects and returns a key from the cache.
        Raises `KeyError` If the cache is empty.
        """
        ...

class LRUCache(BaseCacheImpl[KT, VT]):
    """
    A Least-Recently-Used (LRU) cache eviction policy: when the cache is full,
    the item that has not been accessed for the longest time is removed first,
    regardless of how many times it was accessed in the past.

    ## How It Works
    The LRU algorithm is one of the most widely used cache eviction strategies in
    practice. Items are tracked by their access recency—every time an item is read
    or written, it becomes the most recently used. When the cache reaches capacity,
    the least recently used item (the one that was accessed longest ago) is
    evicted to make room for new entries.

    This implementation pairs a doubly-linked list with a hash map. The linked list
    maintains items in access order: the most recently used item sits at the back,
    and the least recently used at the front. The hash map stores pointers (cursors)
    into this list, enabling O(1) key lookups. On every access—read or write—the
    accessed item is moved to the back of the list, promoting it to "most recently used"
    status. When eviction is needed, the front item is removed.

    The doubly-linked list structure is critical: it permits O(1) removal and
    reinsertion of any item anywhere in the ordering, without requiring a full rebuild
    or index shifting. A running total tracks the current size of cached items,
    allowing capacity checks in constant time.

    ### Pros
    - **Excellent hit rates on temporal locality.** Workloads where recently or
      frequently accessed items are likely to be needed again soon benefit dramatically
      from LRU's recency-aware eviction. Real-world caches (CPU L1/L2, database
      buffers, CDN edges) rely on this principle.
    - **Insert, lookup, and evict are all O(1) amortized.** The doubly-linked list
      and hash map combination guarantees no per-operation index shifting or traversals.
    - **Automatic adaptation to access patterns.** Hot keys naturally migrate to the
      back of the list and stay there, while cold keys drift toward eviction. No
      manual tuning of weights or thresholds is needed.
    - **Per-hit cost is minimal.** While LRU does require bookkeeping on reads (moving
      an item to the back), this bookkeeping is O(1) and adds negligible overhead to most
      workloads.

    ### Cons
    - **Per-read overhead.** Every cache hit requires updating the linked list (removing
      the item from its current position and reinserting it at the back), which is
      measurably slower than FIFO's zero-cost hits on read-heavy workloads.
    - **Burst traffic can skew eviction.** A single item accessed many times in rapid
      succession will be kept alive indefinitely, even if other keys have better long-term
      utility. Recency is a proxy for future use, not a guarantee.
    - **Implementation complexity.** The doubly-linked list and cursor-based hash table add
      internal complexity compared to simpler policies like FIFO.
    - **Memory overhead.** Storing doubly-linked pointers (prev/next) for every cached item
      consumes extra memory compared to array-based alternatives.

    ## When to use it
    Reach for `LRUPolicy` when:
    - Your workload exhibits temporal locality—recently accessed items are likely to be
      needed again soon. Databases, web caches, and CPU caches all exhibit this pattern.
    - Hit rate is your primary metric. If maximizing the proportion of requests served
      from the cache matters more than minimizing per-hit latency, LRU is typically the
      best general-purpose choice.
    - Access patterns are unknown or unpredictable. LRU's automatic adaptation makes it a safe
      default when you cannot statically analyze what keys will be hot.
    - You need a standard, battle-tested algorithm. LRU is the de facto eviction policy in most
      production systems; it is well-understood, widely supported, and easy to reason about.

    Avoid it when:
    - Your workload is write-heavy with few or no re-reads. FIFO's zero per-hit bookkeeping
      will outperform LRU if the cache is rarely hit.
    - You need sub-microsecond latency on every operation. The linked-list manipulation on each
      read can add measurable overhead in ultra-low-latency systems.
    - Access patterns are bimodal or exhibit frequency-heavy behavior (a small set of items is
      accessed far more often than others). An LFU policy may deliver better hit rates in such cases.
    """

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        It's recommended to use this method instead of `self[key] = value`, as it keeps code
        compatible across different cache policies.
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
        """
        ...

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Optional[VT | DT]:
        """
        Inserts key with a value of default if key is not in the cache.

        Returns the value for key if key is in the cache, else default.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Removes the least recently used item from the cache and returns it as a (key, value) tuple.
        Raises KeyError if the cache is empty.
        """
        ...

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are ordered.
        """
        ...

    def keys(self) -> typing.Iterable[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are ordered.
        """
        ...

    def values(self) -> typing.Iterable[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are ordered.
        """
        ...

    def peek(
        self,
        key: KT,
        default: typing.Optional[DT] = ...,
    ) -> typing.Union[VT, DT]:
        """
        Retrieves the value for a given key from the cache (without promoting the key).

        Returns the value associated with the key if present, otherwise returns the specified default value.
        Equivalent to `self[key]`, but provides a fallback default if the key is not found.
        """

    def least_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has not been accessed in the longest time.

        Raises `KeyError` if cache is empty.
        """
        ...

    def most_recently_used(self) -> typing.Optional[KT]:
        """
        Returns the key in the cache that has been accessed in the shortest time.

        Raises `KeyError` if cache is empty.
        """
        ...
