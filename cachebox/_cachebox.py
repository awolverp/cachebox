import threading
import time
import typing
from datetime import datetime, timedelta

from ._core import BaseCacheImpl as BaseCacheImpl
from ._core import Cache as Cache
from ._core import FIFOCache as FIFOCache
from ._core import LFUCache as LFUCache
from ._core import LRUCache as LRUCache
from ._core import RRCache as RRCache

# private import
from ._core import TTLCache as _CoreTTLCache
from ._core import VTTLCache as _CoreVTTLCache

if typing.TYPE_CHECKING:
    from ._core import _IterableType

KT = typing.TypeVar("KT", bound=typing.Hashable)
VT = typing.TypeVar("VT")


class TTLCache(_CoreTTLCache[KT, VT]):
    """
    A cache with a Time-To-Live (TTL) eviction policy.

    Each entry carries an expiration timestamp and is considered stale — and
    eligible for eviction — once that deadline has passed, regardless of how
    recently or frequently it was accessed.

    Every entry is stamped with an absolute ``expires_at`` timestamp at
    insertion time (computed as ``now + global_ttl``). Entries are stored in
    insertion order and eviction proceeds from the front of that queue, but
    only after confirming the candidate has actually expired. A live entry at
    the front of the queue blocks eviction of everything behind it, so the
    cache may temporarily exceed capacity if the oldest entries are still
    fresh.

    Like ``FIFOCache``, this implementation backs the queue with a
    double-ended queue for O(1) front removal and a hash map for O(1) key
    lookups. The same logical-index trick applies: the table stores
    monotonically increasing counters rather than physical deque positions, and
    a ``front_offset`` counter converts a logical index back to a physical one
    at read time via ``entries[table[key] - front_offset]``. This keeps
    eviction and lookup O(1) without rewriting the table on every eviction.
    Every read also checks ``expires_at`` against the current wall-clock time
    and treats any expired entry as a cache miss.

    Without ``sweep_interval``, an expiry sweep is triggered automatically on
    every call to ``insert``, ``update``, ``current_size``, ``remaining_size``,
    ``last``, ``first``, ``items``, ``keys``, ``values``, and ``__iter__``. A
    completely idle cache will accumulate stale entries between these calls,
    but any normal interaction is sufficient to reclaim them. When
    ``sweep_interval`` is set, a background thread performs the sweep on that
    interval instead, reclaiming expired entries independent of method calls.

    |              | get   | insert  | delete           | popitem |
    | ------------ | ----- | ------- | ---------------- | ------- |
    | Worse-case   | O(1)  | O(1)    | O(min(i, n-i))   | O(n) - very rare |

    Pros:
        - Insert, lookup, and evict are all O(1) amortized: the
          ``front_offset`` trick eliminates the O(n) index-shifting that a
          naive implementation would require on every eviction.
        - Entries expire automatically without a background thread or explicit
          invalidation call; stale data is never returned to the caller.
        - TTL expiry and insertion-order eviction compose cleanly: the oldest
          expired entry is always evicted first.
        - A single ``global_ttl`` keeps configuration simple; every entry ages
          at the same rate.

    Cons:
        - Wall-clock dependency: correctness relies on a monotonically
          advancing system clock. Clock adjustments (NTP steps,
          suspend/resume) can cause entries to expire earlier or later than
          intended.
        - When ``sweep_interval`` is set, a background thread wakes on that
          interval to remove all expired entries, adding a small amount of
          background CPU usage for the lifetime of the cache.
        - No per-entry TTL override: all entries share ``global_ttl``; mixed
          expiry requirements need a different policy or a wrapper layer.
        - A rare O(n) index rebase (triggered when ``front_offset`` nears
          ``usize::MAX - isize::MAX``) introduces an occasional latency spike;
          amortised cost is negligible but worst-case latency is unbounded in
          principle.

    Use ``TTLCache`` when cached data has a natural freshness window (API
    responses, auth tokens, DNS records, rate-limit counters), when automatic
    expiry without a background reaper is sufficient, or when access patterns
    are unpredictable enough that recency- or frequency-based eviction would
    offer no meaningful advantage.

    Avoid it when strong temporal locality makes LRU a better fit, when
    per-entry TTL granularity is required (consider ``VTTLCache`` instead), or
    when the system clock is unreliable or subject to adjustment.

    Example::

        from cachebox import TTLCache
        import time

        cache = TTLCache(0, global_ttl=2)
        cache.update({i:str(i) for i in range(10)})

        print(cache.get_with_expire(2)) # ('2', 1.99)

        # Returns the oldest key in cache; this is the one which will be removed by `popitem()`
        print(cache.first()) # 0

        cache["mykey"] = "value"
        time.sleep(2)
        cache["mykey"] # KeyError
    """

    def __init__(
        self,
        maxsize: int,
        global_ttl: float | timedelta,
        iterable: _IterableType[KT, VT] | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT]] | None = None,
        sweep_interval: float | timedelta | None = None,
    ) -> None:
        """
        Initializes a new TTLCache instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero,
                the limit is set to ``sys.maxsize`` internally.
            global_ttl: Time-to-live for every entry, as seconds (float) or a
                ``timedelta``. Applied at insertion time.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate cache capacity to minimize reallocations.
                Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair.
                When ``None``, each entry is assumed to have a size of 1
                (equivalent to ``lambda k, v: 1``). Use this to implement
                weighted caching — for example, sizing entries by memory
                footprint or byte length.
            sweep_interval: If set, starts a background thread that sweeps and
                removes all expired entries on this interval (in seconds or as
                a ``timedelta``). When ``None``, expiry is lazy. Defaults to
                ``None``. Must be greater than or equal to 1.

        Note:
            The cache can be pre-sized via ``capacity`` to reduce
            reallocations when the number of expected entries is known
            ahead of time.

        Raises:
            ValueError: If ``sweep_interval`` is set to a value less than 1.
        """
        super().__init__(
            maxsize,
            global_ttl,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )

        self._thread: threading.Thread | None = None
        self._thread_is_running: bool = False

        if sweep_interval is not None:
            if isinstance(sweep_interval, timedelta):
                sweep_interval = sweep_interval.total_seconds()

            if sweep_interval < 1:
                raise ValueError("sweep_interval must be more than 1 seconds.")

            self._thread_is_running = True
            self._thread = threading.Thread(
                target=self._sweeper_thread,
                args=(sweep_interval,),
                daemon=True,
            )
            self._thread.start()

        self._sweep_interval = sweep_interval

    @property
    def sweep_interval(self) -> float | None:
        """The configured ``sweep_interval`` in seconds."""
        return self._sweep_interval

    def _sweeper_thread(self, interval: float):
        while self._thread_is_running:
            time.sleep(interval)
            self.expire()

    def stop_sweeper(self) -> None:
        """Signals the background sweeper thread to stop, if one is active."""
        self._thread_is_running = False

    def __del__(self) -> None:
        self.stop_sweeper()


class VTTLCache(_CoreVTTLCache[KT, VT]):
    """
    A cache with a Variable Time-To-Live (VTTL) eviction policy.

    Each item can be inserted with its own individual TTL (time-to-live). When
    an item's TTL expires, it is considered stale and will be evicted. Items
    inserted without a TTL never expire and are only evicted when the cache
    reaches capacity.

    Expiration is managed lazily by default: stale entries are not removed
    immediately when they expire, but are cleaned up on the next access or
    when the cache needs to reclaim capacity. Optionally, a ``sweep_interval``
    can be configured to spawn a background thread that proactively removes
    expired items on a fixed schedule, bounding the window in which stale
    data can be observed or memory held unnecessarily.

    Internally, a lazy-evaluated min-heap tracks expiration deadlines. The
    heap is only fully sorted when needed (e.g. during eviction), keeping
    insert costs low on average. A hash table stores cursors into the heap for
    O(1) key lookups. A running total enables O(1) capacity checks.

    When the cache is full and eviction is needed, expired items are reclaimed
    first (in expiration order, cheapest deadline first). If no expired items
    exist, the item with the nearest upcoming expiration is evicted. Items with
    no TTL are the last resort and are evicted only when all expiring items
    have been exhausted.

    |              | get   | insert  | delete(i)      | popitem |
    | ------------ | ----- | ------- | -------------- | ------- |
    | Worse-case   | O(1)~ | O(1)~   | O(min(i, n-i)) | O(1)~   |

    Pros:
        - Per-item TTL control: each entry can have a different lifetime.
        - Expired items are reclaimed before live items, maximising useful
          capacity.
        - Lazy expiry avoids background threads and timer overhead by default.
        - Optional background sweeping bounds stale-data visibility and memory
          retention when lazy eviction is insufficient.
        - Insert, lookup, and evict are O(1) amortized (O(log n) worst-case
          during heap rebalancing).
        - TTL-free items coexist naturally alongside expiring ones.

    Cons:
        - Without sweeping, stale items may linger in memory until the next
          access or eviction pressure forces a cleanup.
        - With sweeping, a background thread is running for the lifetime of
          the cache, adding concurrency overhead and requiring thread-safe
          internal locking.
        - Slightly higher per-insert cost compared to pure LRU/LFU.
        - No guarantee on the exact eviction moment for expired items in lazy
          mode; callers that require strict TTL enforcement should validate
          timestamps on read, or configure a sufficiently short
          ``sweep_interval``.

    Use ``VTTLCache`` when different items have different natural lifetimes
    (e.g. session tokens, API responses with varying freshness requirements,
    or multi-tier data with mixed staleness tolerances). Set
    ``sweep_interval`` when bounded staleness or proactive memory reclamation
    is required.

    Avoid it when all items share a uniform TTL (consider ``TTLCache`` instead),
    when strict and immediate expiry is a hard requirement, or when memory pressure
    from temporarily lingering stale entries is unacceptable and a background thread
    is not an option.

    Example::

        from cachebox import VTTLCache
        import time

        cache = VTTLCache(100, iterable={i:i for i in range(4)}, ttl=3)
        print(len(cache)) # 4
        time.sleep(3)
        print(len(cache)) # 0

        # The "key1" is exists for 5 seconds
        cache.insert("key1", "value", ttl=5)
        # The "key2" is exists for 2 seconds
        cache.insert("key2", "value", ttl=2)

        time.sleep(2)
        # "key1" is exists for 3 seconds
        print(cache.get("key1")) # value

        # "key2" has expired
        print(cache.get("key2")) # None
    """

    def __init__(
        self,
        maxsize: int,
        iterable: _IterableType[KT, VT] | None = None,
        ttl: float | timedelta | datetime | None = None,
        *,
        capacity: int = 0,
        getsizeof: typing.Callable[[KT, VT]] | None = None,
        sweep_interval: float | timedelta | None = None,
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
                When ``None``, each entry is assumed to have a size of 1
                (equivalent to ``lambda k, v: 1``). Use this to implement
                weighted caching — for example, sizing entries by memory
                footprint or byte length.
            sweep_interval: If set, starts a background thread that sweeps and
                removes all expired entries on this interval (in seconds or as
                a ``timedelta``). When ``None``, expiry is lazy. Defaults to
                ``None``. Must be greater than or equal to 1.

        Note:
            The cache can be pre-sized via ``capacity`` to reduce
            reallocations when the number of expected entries is known
            ahead of time.

        Raises:
            ValueError: If ``sweep_interval`` is set to a value less than 1.
        """
        super().__init__(
            maxsize,
            iterable,
            ttl,
            capacity=capacity,
            getsizeof=getsizeof,
        )

        self._thread: threading.Thread | None = None
        self._thread_is_running: bool = False

        if sweep_interval is not None:
            if isinstance(sweep_interval, timedelta):
                sweep_interval = sweep_interval.total_seconds()

            if sweep_interval < 1:
                raise ValueError("sweep_interval must be more than 1 seconds.")

            self._thread_is_running = True
            self._thread = threading.Thread(
                target=self._sweeper_thread,
                args=(sweep_interval,),
                daemon=True,
            )
            self._thread.start()

        self._sweep_interval = sweep_interval

    @property
    def sweep_interval(self) -> float | None:
        """The configured ``sweep_interval`` in seconds."""
        return self._sweep_interval

    def _sweeper_thread(self, interval: float):
        while self._thread_is_running:
            time.sleep(interval)
            self.expire()

    def stop_sweeper(self) -> None:
        """Signals the background sweeper thread to stop, if one is active."""
        self._thread_is_running = False

    def __del__(self) -> None:
        self.stop_sweeper()
