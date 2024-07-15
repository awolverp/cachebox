from . import _cachebox
import functools
import collections
import typing
import inspect


def make_key(args: tuple, kwds: dict):
    return (tuple(sorted(args)), frozenset(sorted(kwds.items())))


def make_hash_key(args: tuple, kwds: dict):
    return hash((tuple(sorted(args)), frozenset(sorted(kwds.items()))))


def make_typed_key(args: tuple, kwds: dict):
    args = tuple(sorted(args))
    args += tuple(type(i).__name__ for i in args)
    if kwds:
        args += ("|",) + tuple(type(v).__name__ for v in kwds.values())

    return (args, frozenset(sorted(kwds.items())))


_CacheInfo = collections.namedtuple(
    "CacheInfo", ["hits", "misses", "maxsize", "length", "cachememory"]
)


def cached(
    cache: typing.Union[_cachebox.BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Any] = make_key,
    clear_reuse: bool = False,
    info: bool = False,
):
    """
    Memoize your functions (async functions are supported) ...

    By `cache` param, set your cache and cache policy. (If is `None` or `dict`, `FIFOCache` will be used)

    By `key_maker` param, you can set your key maker, see examples below.

    The `clear_reuse` param will be passed to cache's `clear` method.

    If `info` is `True`, the function cache will tracked.

    Simple Example::

        @cachebox.cached(cachebox.LRUCache(128))
        def sum_as_string(a, b):
            return str(a+b)

        assert sum_as_string(1, 2) == "3"

        assert len(sum_as_string.cache) == 1
        sum_as_string.cache_clear()
        assert len(sum_as_string.cache) == 0

    Info Example::

        @cachebox.cached(cachebox.LRUCache(128), info=True)
        def sum_as_string(a, b):
            return str(a+b)

        assert sum_as_string(1, 2) == "3"
        assert sum_as_string(1, 2) == "3"

        info = sum_as_string.cache_info()
        assert info.misses == 1
        assert info.hits == 1

    Key Maker Example::

        def simple_key_maker(args: tuple, kwds: dict):
            return args[0].path

        @cachebox.cached(cachebox.LRUCache(128), key_maker=simple_key_maker)
        def request_handler(request: Request):
            return Response("hello man")

    Typed Example::

        @cachebox.cached(cachebox.LRUCache(128), key_maker=cachebox.make_typed_key)
        def sum_as_string(a, b):
            return str(a+b)

        # ...
    """

    if isinstance(cache, dict) or cache is None:
        cache = _cachebox.FIFOCache(0)

    if type(cache) is type or not isinstance(cache, _cachebox.BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    def decorator(func):
        if info:
            hits = 0
            misses = 0

            def cache_info():
                nonlocal hits, misses
                return _CacheInfo(hits, misses, cache.maxsize, len(cache), cache.__sizeof__())

            if inspect.iscoroutinefunction(func):

                async def wrapper(*args, **kwargs):
                    nonlocal hits, misses
                    key = key_maker(args, kwargs)
                    try:
                        result = cache[key]
                        hits += 1
                        return result
                    except KeyError:
                        misses += 1

                    result = await func(*args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            else:

                def wrapper(*args, **kwargs):
                    nonlocal hits, misses
                    key = key_maker(args, kwargs)
                    try:
                        result = cache[key]
                        hits += 1
                        return result
                    except KeyError:
                        misses += 1

                    result = func(*args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            def cache_clear():
                nonlocal hits, misses
                cache.clear(reuse=clear_reuse)
                hits = 0
                misses = 0

        else:
            if inspect.iscoroutinefunction(func):

                async def wrapper(*args, **kwargs):
                    key = key_maker(args, kwargs)
                    try:
                        return cache[key]
                    except KeyError:
                        pass

                    result = await func(*args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            else:

                def wrapper(*args, **kwargs):
                    key = key_maker(args, kwargs)
                    try:
                        return cache[key]
                    except KeyError:
                        pass

                    result = func(*args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            def cache_clear():
                cache.clear(reuse=clear_reuse)

            cache_info = None

        wrapper.cache = cache
        wrapper.cache_clear = cache_clear
        wrapper.cache_info = cache_info

        return functools.update_wrapper(wrapper, func)

    return decorator


def cachedmethod(
    cache: typing.Union[_cachebox.BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Any] = make_key,
    clear_reuse: bool = False,
    info: bool = False,
):
    """
    It works like `cached()`, but you can use it for class methods, because it will ignore `self` param.
    """
    if isinstance(cache, dict) or cache is None:
        cache = _cachebox.FIFOCache(0)

    if type(cache) is type or not isinstance(cache, _cachebox.BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    def decorator(func):
        if info:
            hits = 0
            misses = 0

            def cache_info():
                nonlocal hits, misses
                return _CacheInfo(hits, misses, cache.maxsize, len(cache), cache.__sizeof__())

            if inspect.iscoroutinefunction(func):

                async def wrapper(self, *args, **kwargs):
                    nonlocal hits, misses
                    key = key_maker(args, kwargs)
                    try:
                        result = cache[key]
                        hits += 1
                        return result
                    except KeyError:
                        misses += 1

                    result = await func(self, *args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            else:

                def wrapper(self, *args, **kwargs):
                    nonlocal hits, misses
                    key = key_maker(args, kwargs)
                    try:
                        result = cache[key]
                        hits += 1
                        return result
                    except KeyError:
                        misses += 1

                    result = func(self, *args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            def cache_clear():
                nonlocal hits, misses
                cache.clear(reuse=clear_reuse)
                hits = 0
                misses = 0

        else:
            if inspect.iscoroutinefunction(func):

                async def wrapper(self, *args, **kwargs):
                    key = key_maker(args, kwargs)
                    try:
                        return cache[key]
                    except KeyError:
                        pass

                    result = await func(self, *args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            else:

                def wrapper(self, *args, **kwargs):
                    key = key_maker(args, kwargs)
                    try:
                        return cache[key]
                    except KeyError:
                        pass

                    result = func(self, *args, **kwargs)

                    try:
                        return cache.setdefault(key, result)
                    except OverflowError:
                        return result

            def cache_clear():
                cache.clear(reuse=clear_reuse)

            cache_info = None

        wrapper.cache = cache
        wrapper.cache_clear = cache_clear
        wrapper.cache_info = cache_info

        return functools.update_wrapper(wrapper, func)

    return decorator


_K = typing.TypeVar("_K")
_V = typing.TypeVar("_V")

def items_in_order(cache: _cachebox.BaseCacheImpl[_K, _V]) -> typing.Generator[typing.Tuple[_K, _V], typing.Any, None]:
    """
    As you know `.items()` methods in implemetations doesn't iter items in order.
    You can use this method for iterating items in order.

    Supported classes:
        - `FIFOCache`
        - `LFUCache`
        - `LRUCache`
        - `TTLCache`
    """
    if isinstance(cache, _cachebox.FIFOCache):
        for i in range(len(cache)):
            key = cache.first(i)
            yield key, cache[key]
    
    elif isinstance(cache, _cachebox.LFUCache):
        for i in range(len(cache)):
            key = cache.least_frequently_used(i)
            yield key, cache.peek(key)
    
    elif isinstance(cache, _cachebox.LRUCache):
        for i in range(len(cache)):
            key = cache.least_recently_used(i)
            yield key, cache.peek(key)

    elif isinstance(cache, _cachebox.TTLCache):
        for i in range(len(cache)):
            key = cache.first(i)
            yield key, cache[key]

    else:
        raise TypeError(
            "can't iterate items in order for %r class" % type(cache).__name__
        )
