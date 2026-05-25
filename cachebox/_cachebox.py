import threading
import time
import typing
from datetime import timedelta

from ._core import BaseCacheImpl as BaseCacheImpl
from ._core import Cache as Cache
from ._core import FIFOCache as FIFOCache
from ._core import LFUCache as LFUCache
from ._core import LRUCache as LRUCache
from ._core import RRCache as RRCache

# private import
from ._core import TTLCache as _CoreTTLCache

if typing.TYPE_CHECKING:
    from ._core import _IterableType

KT = typing.TypeVar("KT", bound=typing.Hashable)
VT = typing.TypeVar("VT")


class TTLCache(_CoreTTLCache):
    """
    A Time-To-Live (TTL) cache eviction policy: each entry carries an expiration timestamp
    and is considered stale — and eligible for eviction — once that deadline has passed,
    regardless of how recently or frequently it was accessed.

    ## How It Works
    The TTL algorithm pairs time-based expiration with insertion-order eviction. Every entry
    is stamped with an absolute `expires_at` timestamp at insertion time (computed as
    `now + global_ttl`). Entries are stored in insertion order, and eviction proceeds from the
    front of that queue — but only after confirming the candidate has actually expired. A live
    entry at the front of the queue blocks eviction of everything behind it, so the cache may
    temporarily exceed capacity if the oldest entries are still fresh.

    Like `FIFOPolicy`, this implementation backs the queue with a `double-ended queue` for O(1)
    front removal and a `hash map` for O(1) key lookups. The same logical-index trick applies:
    the table stores monotonically increasing counters rather than physical deque positions, and
    a `front_offset` counter converts a logical index back to a physical one at read time via
    `entries[table[key] - front_offset]`. This keeps eviction and lookup O(1) without rewriting
    the table on every eviction. On top of that, every read checks `expires_at` against the current wall-clock time and
    treats any expired entry as a cache miss.

    Without `sweep_interval`, an expiry sweep is triggered automatically on every call to
    `insert`, `update`, `current_size`, `remaining_size`, `last`, `first`, `items`, `keys`,
    `values`, and `__iter__`. A completely idle cache will accumulate stale entries between
    these calls, but any normal interaction with the cache is sufficient to reclaim them.
    When `sweep_interval` is set, a background thread performs the sweep on that interval
    instead, reclaiming expired entries independent of any method calls.

    ### Pros
    - Insert, lookup, and evict are all O(1) amortized: the `front_offset` trick eliminates the O(n)
      index-shifting that a naïve implementation would require on every eviction.
    - Entries expire automatically without any background thread or explicit invalidation call.
      Stale data is never returned to the caller.
    - TTL expiry and insertion-order eviction compose cleanly: the oldest entry is always evicted
      first among those that have already expired.
    - A single `global_ttl` keeps configuration simple; every entry ages at the same rate.

    ### Cons

    - Wall-clock dependency. Correctness relies on a monotonically advancing system clock.
      Clock adjustments (NTP steps, suspend/resume) can cause entries to expire earlier or later
      than intended.
    - When `sweep_interval` is set, a background thread wakes on that interval to sweep and
      remove all expired entries. This adds a small amount of background CPU usage and
      introduces a reaper thread for the lifetime of the cache.
    - No per-entry TTL override. All entries share `global_ttl`; mixed expiry requirements need
      a different policy or a wrapper layer.
    - The rare O(n) index rebase (triggered when `front_offset` nears `usize::MAX - isize::MAX`)
      introduces an occasional latency spike. Amortized cost is negligible, but worst-case
      latency is unbounded in principle.

    ## When to use it
    Reach for `TTLPolicy` when:
    - Cached data has a natural freshness window: API responses, auth tokens, DNS records,
      rate-limit counters, or any value that becomes incorrect or unsafe after a known interval.
    - You need automatic expiry without a background reaper thread — expiry sweeps on common
      method calls are sufficient, or you want continuous reclamation via `sweep_interval`.
    - Access patterns are unpredictable or uniform enough that recency- or frequency-based
      eviction (LRU/LFU) would offer no meaningful advantage.

    Avoid it when:
    - Your workload has strong temporal locality and you need a best-effort hit rate policy —
      LRU will serve you better.
    - Per-entry TTL granularity is required. If different keys need different lifetimes,
      consider `VTTLCache`.
    - Your environment has an unreliable or adjustable system clock, where wall-clock-based
      expiry may behave unexpectedly.
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
        Initialize a new instance.

        Args:
            maxsize: Maximum number of elements the cache can hold. If zero, the limit is set to sys.maxsize internally.
            global_ttl: Time-to-live for every entry, either as seconds (float) or a timedelta. Applied at insertion time.
            iterable: Initial data to populate the cache.
            capacity: Pre-allocate cache capacity to minimize reallocations. Defaults to 0.
            getsizeof: A callable that computes the size of a key-value pair. When `None`, each
                    entry is assumed to have a size of 1 (equivalent to `lambda k, v: 1`).
                    Use this to implement weighted caching — for example, sizing entries by
                    memory footprint or byte length.
            sweep_interval: If set, starts a background thread that sweeps and removes all expired entries on this interval.
                    When None, expiry is lazy. Defaults to `None`. *It should be more than 1*.

        The cache can be pre-sized via `capacity` to reduce reallocations when
        the number of expected entries is known ahead of time.
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
        """Returns the configured `sweep_interval`."""
        return self._sweep_interval

    def _sweeper_thread(self, interval: float):
        while self._thread_is_running:
            time.sleep(interval)
            self.expire()

    def stop_sweeper(self) -> None:
        """Signals the sweeper thread to stop ( if is active )"""
        self._thread_is_running = False

    def __del__(self) -> None:
        self.stop_sweeper()
