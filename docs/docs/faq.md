# FAQ

## General

### How is cachebox different from `functools.lru_cache`?

`functools.lru_cache` is excellent for simple memoization, but cachebox adds:

- Multiple eviction policies (FIFO, RR, LRU, LFU, TTL, VTTL)
- Explicit cache objects you can share, inspect, pickle, or freeze
- Thread-safe storage implemented in Rust
- Configurable key makers and postprocessors (including default protection against mutating
  cached `dict`/`list`/`set` results)
- Per-key stampede prevention for concurrent misses
- Weighted capacity via `getsizeof`
- Hit/miss statistics via `cache_info()`

### Does cachebox work with PyPy?

Yes. Pre-built wheels target both CPython and PyPy for supported Python versions.

### Is the cache process-wide or per-process?

In-memory only, per process. There is no shared multi-process or multi-machine backend.
Use pickle (or your own serialization) if you need to move a snapshot to another process.

### Are keys required to be hashable?

Yes. Keys must be hashable, same as `dict`. Values may be any Python object.

---

## Capacity and size

### What does `maxsize=0` mean?

Unbounded. Internally the limit is set to `sys.maxsize`. Items are still subject to the eviction
policy only when you use a finite `maxsize` (or TTL for time-based classes).

### What is the difference between `len(cache)`, `current_size()`, and `capacity()`?

| Call | Meaning |
|------|---------|
| `len(cache)` | Number of key-value entries |
| `current_size()` | Sum of entry weights (`getsizeof`, or 1 per entry by default) |
| `capacity()` | Hash-table slots allocated (reallocation threshold), not `maxsize` |
| `remaining_size()` | `maxsize - current_size()` |
| `is_full()` | Whether cumulative weight has reached `maxsize` |

### Why did `Cache` raise `OverflowError`?

`Cache` never evicts. Once `current_size` reaches `maxsize`, new keys are rejected. Replace an
existing key, raise `maxsize`, free space with `pop`/`clear`, or switch to a policy-based class.

---

## Decorator

### Why doesn't my method have `.cache` / `.cache_info()`?

Those attributes are only attached when `cache` is a **concrete instance**. If you pass
`lambda self: self._cache`, manage the cache via the instance attribute yourself.

### Why do `f(1)` and `f(1.0)` share a cache entry?

The default `make_key` does not include types, and `1 == 1.0`. Use
[`make_typed_key`](api/utils.md#cachebox.utils.make_typed_key):

```python
@cachebox.cached(cachebox.LRUCache(128), key_maker=cachebox.make_typed_key)
def f(x):
    return x
```

### My recursive function deadlocks. What now?

Default stampede locks are non-reentrant. Use `lock=False` or `lock=threading.RLock`
(sync only). See [Cache stampede prevention](tips.md#cache-stampede-prevention).

### Can I use a different cache per request / tenant?

Yes — use a callable cache that returns the right instance, or build keys that embed the tenant
id in a shared cache:

```python
@cachebox.cached(cachebox.LRUCache(10_000), key_maker=lambda tenant, user_id: (tenant, user_id))
def get_user(tenant: str, user_id: int):
    ...
```

### Does `cachebox__ignore=True` still update statistics?

No. The function runs directly; the cache is neither read nor written, and hit/miss counters
are unchanged for that call.

---

## TTL

### When are expired items actually removed?

- **Lazy (default):** on subsequent operations that touch the cache.
- **Sweeper:** when `sweep_interval` is set, a daemon thread calls `expire()` periodically.
- **Manual:** call `expire()` yourself.

Lookups never return expired values; they behave as misses.

### Can some `VTTLCache` items never expire?

Yes. Omit `ttl` (or pass `None`) on insert. Those items are only removed under capacity pressure
after expiring items have been reclaimed.

### Does freezing a TTL cache pause the clock?

No. Expiration is based on wall-clock time on the underlying cache. Frozen only blocks write APIs.

### Are clock adjustments a problem?

TTL correctness depends on a monotonically advancing system clock. Large NTP steps or
suspend/resume can expire entries earlier or later than intended.

---

## Concurrency

### Is every operation thread-safe?

Individual cache method calls are thread-safe. Check-then-act sequences are not. Prefer
`setdefault` / `setdefault_with` / `@cached` for insert-if-missing patterns.

### Does stampede prevention work across processes?

No. Locks are in-process only (threading / asyncio).

### Can `setdefault_with`'s factory run twice?

Yes, under concurrent misses. The first successful insert wins; extra factory results are discarded
(and the winning value is returned). For single-flight guarantees use `@cached` with locking enabled.

---

## Persistence and copying

### Can I share a cache across machines?

Not built-in. Serialize with pickle (or extract items yourself) and load elsewhere.

### Why did pickling fail?

Common causes: non-picklable values, or a `lambda`/`local` function used as `getsizeof`.

### Are `keys()` / `items()` like `dict` views?

No. They are one-shot iterators with `len()` / `bool()`, not live views. See
[Iteration safety](tips.md#iteration-safety).

---

## Choosing an algorithm

See the full guide: [Choosing a Cache](algorithms.md).

**Short version:** start with `LRUCache` for general memoization, `TTLCache` when everything
shares one lifetime, `VTTLCache` for per-key TTLs, and `Cache` only for fixed non-stale sets.
