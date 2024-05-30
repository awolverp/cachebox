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
    LFUCache as LFUCache,
    RRCache as RRCache,
    # LRUCache as LRUCache,
    # VTTLCache as VTTLCache,
    # TTLCache as TTLCache,
    __version__ as __version__,
    __author__ as __author__,
    tuple_ptr_iterator as tuple_ptr_iterator,
    object_ptr_iterator as object_ptr_iterator,
    lfu_tuple_ptr_iterator as lfu_tuple_ptr_iterator,
    lfu_object_ptr_iterator as lfu_object_ptr_iterator,
)

from .utils import (
    cached as cached,
    cachedmethod as cachedmethod,
)
