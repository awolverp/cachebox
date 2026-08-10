# Choosing a Cache

Cachebox ships seven implementations. They share the same dict-like surface area but differ
in **when** and **which** entries are removed under pressure (or over time).

## At a Glance

| Class | Eviction | Best for | Avoid when |
|-------|----------|----------|------------|
| [`Cache`](#cache) | None (raises `OverflowError` when full) | Fixed key sets that never go stale | Unbounded or volatile data |
| [`FIFOCache`](#fifocache) | Oldest insertion first | Predictable, insert-heavy workloads | Strong temporal locality |
| [`RRCache`](#rrcache) | Random | Uniform access, low overhead | Hit rate is critical |
| [`LRUCache`](#lrucache) | Least recently used | Temporal locality (most common choice) | Write-only / scan-once traffic |
| [`LFUCache`](#lfucache) | Least frequently used | Stable hot sets | Rapidly shifting access patterns |
| [`TTLCache`](#ttlcache) | Global TTL + FIFO among live entries | Uniform freshness windows | Per-key TTLs needed |
| [`VTTLCache`](#vttlcache) | Per-item TTL (optional non-expiring) | Mixed lifetimes | All items share one TTL |

Complexity (worst-case, amortized where marked with `~`):

| Class | get | insert | delete | popitem |
|-------|-----|--------|--------|---------|
| `Cache` | O(1) | O(1) | O(1) | N/A |
| `FIFOCache` | O(1) | O(1) | O(min(i, n−i)) | O(1)~ |
| `RRCache` | O(1) | O(1) | O(1) | O(min(i, n−i)) |
| `LRUCache` | O(1)~ | O(1)~ | O(1)~ | O(1)~ |
| `LFUCache` | O(1)~ | O(1)~ | O(min(i, n−i)) | O(1)~ |
| `TTLCache` | O(1) | O(1) | O(min(i, n−i)) | O(n) rare |
| `VTTLCache` | O(1)~ | O(1)~ | O(min(i, n−i)) | O(1)~ |

## Decision Guide

```
Do entries need to expire by time?
├── Yes — same TTL for all → TTLCache
├── Yes — different TTLs (or some never expire) → VTTLCache
└── No
    ├── Fixed set, never remove automatically → Cache
    ├── Access patterns roughly uniform → FIFOCache or RRCache
    ├── Recent items matter most → LRUCache  ← default choice
    └── A stable minority of keys is very hot → LFUCache
```

---

## Cache

Thread-safe hashmap with **no eviction policy**. When `maxsize` is reached, further inserts raise
`OverflowError` instead of removing anything.

```python
from cachebox import Cache

cache = Cache(maxsize=100, capacity=100)
cache.insert("key", "value")
print(cache["key"])  # value

# Overflow when full
cache.update({i: i for i in range(200)})
# OverflowError: The cache has reached the bound.

# popitem always fails — there is no eviction order
cache.popitem()  # OverflowError
```

**Use when:** compiled regexes, templates, config blobs — fixed keys that do not go stale.

**Avoid when:** the working set grows unboundedly or data can become stale.

---

## FIFOCache

First-In, First-Out. The oldest inserted item is always evicted first. Reads do **not** change
eviction order.

```python
from cachebox import FIFOCache

cache = FIFOCache(5, {i: i * 2 for i in range(5)})
cache["new-key"] = "new-value"  # evicts key 0
print(cache.first())            # oldest key (next popitem target)
print(cache.last())             # most recently inserted key
print(cache.popitem())          # (oldest_key, value)
```

**Use when:** eviction must be deterministic and auditable, or traffic is insert-heavy with
few re-reads.

**Avoid when:** the same keys are re-read often — LRU/LFU will hit more.

---

## RRCache

Random Replacement. When full, a uniformly random entry is evicted.

```python
from cachebox import RRCache

cache = RRCache(10, {i: i for i in range(10)})
print(cache.is_full())     # True
print(cache.random_key())  # e.g. 4
print(cache.popitem())     # random (key, value)
```

**Use when:** access is roughly uniform and you want cheap eviction with almost no bookkeeping.

**Avoid when:** a small hot set must stay resident — random eviction may drop hot keys.

---

## LRUCache

Least-Recently-Used. Every read and write promotes the key; when full, the key that has not been
touched for the longest time is removed.

```python
from cachebox import LRUCache

cache = LRUCache(0, {i: i * 2 for i in range(10)})  # maxsize=0 → unbounded

print(cache[0])                       # access key 0
print(cache.least_recently_used())    # 1
print(cache.most_recently_used())     # 0
print(cache.popitem())                # (1, 2) — LRU item

# peek: read without promoting
print(cache.peek(2))                  # 4
print(cache.least_recently_used())    # still 2 if nothing else was accessed
```

**Use when:** temporal locality exists (most application caches). Good default for `@cached`.

**Avoid when:** one-shot scans would pollute the cache, or you care about frequency more than recency.

---

## LFUCache

Least-Frequently-Used. The key with the lowest access count is evicted first. Ties are broken by
recency (older first).

```python
from cachebox import LFUCache

cache = LFUCache(5)
cache.insert("first", "A")
cache.insert("second", "B")

cache["first"]
cache["first"]
cache["second"]

assert cache.least_frequently_used() == "second"

for key, value, freq in cache.items_with_frequency():
    print(key, value, freq)
# second B 1
# first  A 2

# peek does not bump the frequency counter
cache.peek("first")
```

**Use when:** a stable subset of keys is repeatedly hot (popular products, common config keys).

**Avoid when:** access patterns shift quickly — historical frequency can keep cold keys around
(cache pollution). Prefer LRU in that case.

---

## TTLCache

Every entry shares one **global TTL**. At insert time each item gets `expires_at = now + global_ttl`.
Expired items are treated as misses and cleaned up lazily (or by a background sweeper).

When capacity is exceeded among still-live entries, eviction follows FIFO order.

```python
from cachebox import TTLCache
from datetime import timedelta
import time

cache = TTLCache(maxsize=0, global_ttl=2)
cache.update({i: str(i) for i in range(10)})

value, remaining = cache.get_with_expire(2)
print(value, remaining)  # '2'  ~1.99

print(cache.first())     # oldest key
print(cache.global_ttl)  # 2.0

cache["mykey"] = "value"
time.sleep(2)
cache["mykey"]  # KeyError — expired

# timedelta is accepted
cache2 = TTLCache(100, global_ttl=timedelta(minutes=5))

# Optional background sweeper (interval ≥ 1 second)
cache3 = TTLCache(1000, global_ttl=60, sweep_interval=30)
cache3.stop_sweeper()  # stop the daemon thread when done
```

**Use when:** data has a uniform freshness window (tokens, DNS, API responses, rate-limit windows).

**Avoid when:** different keys need different lifetimes — use `VTTLCache`.

!!! note "Lazy expiry"

    Without `sweep_interval`, expired entries linger until the next interaction
    (`insert`, `get`, iteration, `current_size`, …). See
    [TTL sweepers](tips.md#ttlcachevttlcache-background-thread).

---

## VTTLCache

**Variable** TTL: each insert can take its own lifetime. Items inserted without a TTL never expire
and are only removed under capacity pressure (after all expiring items are gone).

```python
from cachebox import VTTLCache
from datetime import datetime, timedelta, timezone
import time

# ttl= here applies only to the initial iterable, not as a global default
cache = VTTLCache(100, iterable={i: i for i in range(4)}, ttl=3)
time.sleep(3)
print(len(cache))  # 0 after interaction/expire

cache.insert("session", "tok", ttl=5)           # lives 5 seconds
cache.insert("config", {"theme": "dark"})       # never expires
cache.insert("short", "x", ttl=timedelta(seconds=2))
cache.insert("until", "y", ttl=datetime.now(timezone.utc) + timedelta(hours=1))

time.sleep(2)
print(cache.get("session"))  # tok
print(cache.get("short"))    # None

value, remaining = cache.get_with_expire("session")
# remaining is seconds left, or None for non-expiring entries
```

**Use when:** sessions, multi-tier data, or mixed freshness requirements live in one cache.

**Avoid when:** every item should share one TTL — `TTLCache` is simpler and slightly cheaper.

### VTTL insert/update signatures

Unlike other caches, write methods accept an optional `ttl`:

```python
cache.insert(key, value, ttl=None)
cache.update(mapping, ttl=None)
cache.setdefault(key, default=None, ttl=None)
cache.setdefault_with(key, factory, ttl=None)
```

`ttl` may be:

- `float` — seconds from now  
- `timedelta` — duration from now  
- `datetime` — absolute deadline  
- `None` — never expires  

---

## Shared Behaviors

Regardless of policy:

- **Thread-safe** — internal Rust mutexes protect all operations.
- **`maxsize=0`** — treated as unbounded.
- **`getsizeof`** — optional weighted capacity (see [Getting Started](getting-started.md#capacity-size-and-weighted-entries)).
- **`insert` preferred over `[]=`** when you need the previous value or cross-policy consistency.
- **Iterators are one-shot** — not live dict views; modifying the cache while iterating raises
  `RuntimeError`.

Full method lists: [API Reference — Classes](api/impls.md).
