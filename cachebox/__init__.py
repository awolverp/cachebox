from ._core import BaseCacheImpl as BaseCacheImpl
from ._core import Cache as Cache
from ._core import FIFOCache as FIFOCache
from ._core import LFUCache as LFUCache
from ._core import LRUCache as LRUCache
from ._core import RRCache as RRCache

try:
    from ._core import (
        _fifocache_small_offset as _fifocache_small_offset,  # type: ignore
    )
except ImportError:
    pass
