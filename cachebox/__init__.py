from ._cachebox import BaseCacheImpl as BaseCacheImpl
from ._cachebox import Cache as Cache
from ._cachebox import FIFOCache as FIFOCache
from ._cachebox import LFUCache as LFUCache
from ._cachebox import LRUCache as LRUCache
from ._cachebox import RRCache as RRCache
from ._cachebox import TTLCache as TTLCache
from ._cachebox import VTTLCache as VTTLCache
from ._core import __version__ as __version__
from ._core import _small_offset_feature as _small_offset_feature

# Utils
from .utils import Frozen as Frozen
from .utils import cached as cached
from .utils import is_cached as is_cached

# Key maker functions
from .utils import make_hash_key as make_hash_key
from .utils import make_key as make_key
from .utils import make_typed_key as make_typed_key

# Postprocess functions
from .utils import postprocess_copy as postprocess_copy
from .utils import postprocess_copy_mutables as postprocess_copy_mutables
from .utils import postprocess_deepcopy as postprocess_deepcopy
from .utils import postprocess_deepcopy_mutables as postprocess_deepcopy_mutables
