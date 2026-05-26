from ._cachebox import BaseCacheImpl as BaseCacheImpl
from ._cachebox import Cache as Cache
from ._cachebox import FIFOCache as FIFOCache
from ._cachebox import LFUCache as LFUCache
from ._cachebox import LRUCache as LRUCache
from ._cachebox import RRCache as RRCache
from ._cachebox import TTLCache as TTLCache
from ._cachebox import VTTLCache as VTTLCache

try:
    from ._core import (
        _fifocache_small_offset as _fifocache_small_offset,  # type: ignore
    )
except ImportError:
    pass
