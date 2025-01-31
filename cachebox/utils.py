from ._cachebox import BaseCacheImpl, FIFOCache
from collections import namedtuple, defaultdict
import functools
import warnings
import asyncio
import _thread
import inspect
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")


class Frozen(BaseCacheImpl, typing.Generic[KT, VT]):
    __slots__ = ("__cache", "ignore")

    def __init__(self, cls: BaseCacheImpl[KT, VT], ignore: bool = False) -> None:
        """
        **This is not a cache.** this class can freeze your caches and prevents changes.

        :param cls: your cache

        :param ignore: If False, will raise TypeError if anyone try to change cache. will do nothing otherwise.
        """
        assert isinstance(cls, BaseCacheImpl)
        assert type(cls) is not Frozen

        self.__cache = cls
        self.ignore = ignore

    @property
    def cache(self) -> BaseCacheImpl[KT, VT]:
        return self.__cache

    @property
    def maxsize(self) -> int:
        return self.__cache.maxsize

    def __len__(self) -> int:
        return len(self.__cache)

    def __sizeof__(self) -> int:
        return self.__cache.__sizeof__()

    def __bool__(self) -> bool:
        return bool(self.__cache)

    def __contains__(self, key: KT) -> bool:
        return key in self.__cache

    def __setitem__(self, key: KT, value: VT) -> None:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def __getitem__(self, key: KT) -> VT:
        return self.__cache[key]

    def __delitem__(self, key: KT) -> VT:
        if self.ignore:
            return  # type: ignore

        raise TypeError("This cache is frozen.")

    def __repr__(self) -> str:
        return f"<Frozen: {self.__cache}>"

    def __iter__(self) -> typing.Iterator[KT]:
        return iter(self.__cache)

    def __richcmp__(self, other, op: int) -> bool:
        return self.__cache.__richcmp__(other, op)

    def capacity(self) -> int:
        return self.__cache.capacity()

    def is_full(self) -> bool:
        return self.__cache.is_full()

    def is_empty(self) -> bool:
        return self.__cache.is_empty()

    def insert(self, key: KT, value: VT, *args, **kwargs) -> typing.Optional[VT]:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        return self.__cache.get(key, default)

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        if self.ignore:
            return  # type: ignore

        raise TypeError("This cache is frozen.")

    def setdefault(
        self, key: KT, default: typing.Optional[DT] = None, *args, **kwargs
    ) -> typing.Optional[typing.Union[VT, DT]]:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def popitem(self) -> typing.Tuple[KT, VT]:
        if self.ignore:
            return  # type: ignore

        raise TypeError("This cache is frozen.")

    def drain(self, n: int) -> int:
        if self.ignore:
            return  # type: ignore

        raise TypeError("This cache is frozen.")

    def clear(self, *, reuse: bool = False) -> None:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def shrink_to_fit(self) -> None:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def update(
        self, iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]]
    ) -> None:
        if self.ignore:
            return

        raise TypeError("This cache is frozen.")

    def keys(self) -> typing.Iterable[KT]:
        return self.__cache.keys()

    def values(self) -> typing.Iterable[VT]:
        return self.__cache.values()

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        return self.__cache.items()


class _LockWithCounter:
    """
    A threading/asyncio lock which count the waiters
    """

    __slots__ = ("lock", "waiters")

    def __init__(self, is_async: bool = False):
        self.lock = _thread.allocate_lock() if not is_async else asyncio.Lock()
        self.waiters = 0

    async def __aenter__(self) -> None:
        self.waiters += 1
        await self.lock.acquire()

    async def __aexit__(self, *args, **kwds) -> None:
        self.waiters -= 1
        self.lock.release()

    def __enter__(self) -> None:
        self.waiters += 1
        self.lock.acquire()

    def __exit__(self, *args, **kwds) -> None:
        self.waiters -= 1
        self.lock.release()


def _copy_if_need(obj, tocopy=(dict, list, set), level: int = 1):
    from copy import copy

    if level == 0:
        return obj

    if level == 2:
        return copy(obj)

    return copy(obj) if (type(obj) in tocopy) else obj


def make_key(args: tuple, kwds: dict, fasttype=(int, str)):
    key = args
    if kwds:
        key += (object,)
        for item in kwds.items():
            key += item

    if fasttype and len(key) == 1 and type(key[0]) in fasttype:
        return key[0]

    return key


def make_hash_key(args: tuple, kwds: dict):
    return hash(make_key(args, kwds))


def make_typed_key(args: tuple, kwds: dict):
    key = make_key(args, kwds, fasttype=())

    key += tuple(type(v) for v in args)  # type: ignore
    if kwds:
        key += tuple(type(v) for v in kwds.values())

    return key


CacheInfo = namedtuple("CacheInfo", ["hits", "misses", "maxsize", "length", "cachememory"])
EVENT_MISS = 1
EVENT_HIT = 2


def _cached_wrapper(
    func,
    cache: BaseCacheImpl,
    key_maker: typing.Callable[[tuple, dict], typing.Hashable],
    clear_reuse: bool,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]],
    copy_level: int,
    is_method: bool,
) -> None:
    _key_maker = (lambda args, kwds: key_maker(args[1:], kwds)) if is_method else key_maker

    hits = 0
    misses = 0
    locks = defaultdict(_LockWithCounter)
    exceptions = {}

    def _wrapped(*args, **kwds):
        nonlocal hits, misses, locks, exceptions

        if kwds.pop("cachebox__ignore", False):
            return func(*args, **kwds)

        key = _key_maker(args, kwds)

        # try to get result from cache
        try:
            result = cache[key]
            hits += 1

            if callback is not None:
                callback(EVENT_HIT, key, result)

            return _copy_if_need(result, level=copy_level)
        except KeyError:
            pass

        with locks[key]:
            if exceptions.get(key, None) is not None:
                e = exceptions[key] if locks[key].waiters > 1 else exceptions.pop(key)
                raise e

            try:
                result = cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = func(*args, **kwds)
                except Exception as e:
                    exceptions[key] = e
                    raise e

                else:
                    cache[key] = result
                    misses += 1
                    event = EVENT_MISS

        if callback is not None:
            callback(event, key, result)

        return _copy_if_need(result, level=copy_level)

    _wrapped.cache = cache
    _wrapped.callback = callback
    _wrapped.cache_info = lambda: CacheInfo(
        hits, misses, cache.maxsize, len(cache), cache.capacity()
    )

    def cache_clear():
        nonlocal misses, hits, locks
        cache.clear(reuse=clear_reuse)
        misses = 0
        hits = 0
        locks.clear()

    _wrapped.cache_clear = cache_clear

    return _wrapped


def _async_cached_wrapper(
    func,
    cache: BaseCacheImpl,
    key_maker: typing.Callable[[tuple, dict], typing.Hashable],
    clear_reuse: bool,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]],
    copy_level: int,
    is_method: bool,
) -> None:
    _key_maker = (lambda args, kwds: key_maker(args[1:], kwds)) if is_method else key_maker

    hits = 0
    misses = 0
    locks = defaultdict(lambda: _LockWithCounter(True))
    exceptions = {}

    async def _wrapped(*args, **kwds):
        nonlocal hits, misses, locks, exceptions

        if kwds.pop("cachebox__ignore", False):
            return await func(*args, **kwds)

        key = _key_maker(args, kwds)

        # try to get result from cache
        try:
            result = cache[key]
            hits += 1

            if callback is not None:
                awaitable = callback(EVENT_HIT, key, result)
                if inspect.isawaitable(awaitable):
                    await awaitable

            return _copy_if_need(result, level=copy_level)
        except KeyError:
            pass

        async with locks[key]:
            if exceptions.get(key, None) is not None:
                e = exceptions[key] if locks[key].waiters > 1 else exceptions.pop(key)
                raise e

            try:
                result = cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = await func(*args, **kwds)
                except Exception as e:
                    exceptions[key] = e
                    raise e

                else:
                    cache[key] = result
                    misses += 1
                    event = EVENT_MISS

        if callback is not None:
            awaitable = callback(event, key, result)
            if inspect.isawaitable(awaitable):
                await awaitable

        return _copy_if_need(result, level=copy_level)

    _wrapped.cache = cache
    _wrapped.callback = callback
    _wrapped.cache_info = lambda: CacheInfo(
        hits, misses, cache.maxsize, len(cache), cache.capacity()
    )

    def cache_clear():
        nonlocal misses, hits, locks
        cache.clear(reuse=clear_reuse)
        misses = 0
        hits = 0
        locks.clear()

    _wrapped.cache_clear = cache_clear

    return _wrapped


def cached(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]] = None,
    copy_level: int = 1,
    always_copy: typing.Optional[bool] = None,
):
    """
    Decorator to wrap a function with a memoizing callable that saves results in a cache.

    :param cache: Specifies a cache that handles and stores the results. if `None` or `dict`, `FIFOCache` will be used.

    :param key_maker: Specifies a function that will be called with the same positional and keyword
                      arguments as the wrapped function itself, and which has to return a suitable
                      cache key (must be hashable).

    :param clear_reuse: The wrapped function has a function named `clear_cache` that uses `cache.clear`
                        method to clear the cache. This parameter will be passed to cache's `clear` method.

    :param callback: Every time the `cache` is used, callback is also called.
                     The callback arguments are: event number (see `EVENT_MISS` or `EVENT_HIT` variables), key, and then result.

    :param copy_level: The wrapped function always copies the result of your function and then returns it.
                       This parameter specifies that the wrapped function has to copy which type of results.
                       `0` means "never copy", `1` means "only copy `dict`, `list`, and `set` results" and
                       `2` means "always copy the results".

    Example::

        @cachebox.cached(cachebox.LRUCache(128))
        def sum_as_string(a, b):
            return str(a+b)

        assert sum_as_string(1, 2) == "3"

        assert len(sum_as_string.cache) == 1
        sum_as_string.cache_clear()
        assert len(sum_as_string.cache) == 0

    See more: [documentation](https://github.com/awolverp/cachebox#function-cached)
    """
    if cache is None:
        cache = FIFOCache(0)

    if type(cache) is dict:
        cache = FIFOCache(0, cache)

    if not isinstance(cache, BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    if always_copy is not None:
        warnings.warn(
            "'always_copy' parameter is deprecated and will be removed in future; use 'copy_level' instead",
            category=DeprecationWarning,
        )
        if always_copy is True:
            copy_level = 2

    def decorator(func):
        if inspect.iscoroutinefunction(func):
            wrapper = _async_cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, False
            )
        else:
            wrapper = _cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, False
            )

        return functools.update_wrapper(wrapper, func)

    return decorator


def cachedmethod(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]] = None,
    copy_level: int = 1,
    always_copy: typing.Optional[bool] = None,
):
    """
    this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.
    """
    if cache is None:
        cache = FIFOCache(0)

    if type(cache) is dict:
        cache = FIFOCache(0, cache)

    if not isinstance(cache, BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    if always_copy is not None:
        warnings.warn(
            "'always_copy' parameter is deprecated and will be removed in future; use 'copy_level' instead",
            category=DeprecationWarning,
        )
        if always_copy is True:
            copy_level = 2

    def decorator(func):
        if inspect.iscoroutinefunction(func):
            wrapper = _async_cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, True
            )
        else:
            wrapper = _cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, True
            )

        return functools.update_wrapper(wrapper, func)

    return decorator


def is_cached(func: object) -> bool:
    """
    Check if a function/method cached by cachebox or not
    """
    return hasattr(func, "cache") and isinstance(func.cache, BaseCacheImpl)
