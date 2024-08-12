"""
The fastest caching library written in Rust.

Example::

    from cachebox import TTLCache
    import time

    cache = TTLCache(1000, ttl=2)
    cache[0] = 1
    time.sleep(2)
    cache.get(0, None) # None
"""

from ._cachebox import (
    BaseCacheImpl as BaseCacheImpl,
    Cache as Cache,
    FIFOCache as FIFOCache,
    RRCache as RRCache,
    TTLCache as TTLCache,
    LRUCache as LRUCache,
    LFUCache as LFUCache,
    cache_iterator as cache_iterator,
    fifocache_iterator as fifocache_iterator,
    ttlcache_iterator as ttlcache_iterator,
    lrucache_iterator as lrucache_iterator,
    lfucache_iterator as lfucache_iterator,
    __version__ as __version__,
    __author__ as __author__,
    version_info as version_info,
)
