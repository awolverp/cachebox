__version__: str
__author__: str

class CoreKeyError(Exception):
    """
    An exception when a key is not found in a cache.
    This exception is internal to the library core and won't affect you.
    """
    ...
