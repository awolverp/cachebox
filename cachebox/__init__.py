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
    __version__ as __version__,
    __author__ as __author__,
    version_info as version_info,
)
