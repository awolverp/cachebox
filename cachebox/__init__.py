from ._core import BaseCacheImpl as BaseCacheImpl
from ._core import Cache as Cache
from ._core import FIFOCache as FIFOCache

try:
    from ._core import (
        _fifocache_small_offset as _fifocache_small_offset,  # type: ignore
    )
except ImportError:
    pass
