from ._cachebox import BaseCacheImpl, FIFOCache
from collections import namedtuple, defaultdict
import functools
import asyncio
import _thread
import inspect
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")
FT = typing.TypeVar("FT", bound=typing.Callable[..., typing.Any])


class Frozen(BaseCacheImpl[KT, VT]):  # pragma: no cover
    """
    A wrapper class that prevents modifications to an underlying cache implementation.

    This class provides a read-only view of a cache, optionally allowing silent
    suppression of modification attempts instead of raising exceptions.
    """

    __slots__ = ("__cache", "ignore")

    def __init__(self, cls: BaseCacheImpl[KT, VT], ignore: bool = False) -> None:
        """
        Initialize a frozen cache wrapper.

        :param cls: The underlying cache implementation to be frozen
        :type cls: BaseCacheImpl[KT, VT]
        :param ignore: If True, silently ignores modification attempts; if False, raises TypeError when modification is attempted
        :type ignore: bool, optional
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

    def __delitem__(self, key: KT) -> None:
        if self.ignore:
            return None

        raise TypeError("This cache is frozen.")

    def __repr__(self) -> str:
        return f"<Frozen: {self.__cache}>"

    def __iter__(self) -> typing.Iterator[KT]:
        return iter(self.__cache)

    def __richcmp__(self, other: typing.Any, op: int) -> bool:
        return self.__cache.__richcmp__(other, op)

    def capacity(self) -> int:
        return self.__cache.capacity()

    def is_full(self) -> bool:
        return self.__cache.is_full()

    def is_empty(self) -> bool:
        return self.__cache.is_empty()

    def insert(self, key: KT, value: VT, *args, **kwargs) -> typing.Optional[VT]:
        if self.ignore:
            return None

        raise TypeError("This cache is frozen.")

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        return self.__cache.get(key, default)

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        if self.ignore:
            return None  # type: ignore[return-value]

        raise TypeError("This cache is frozen.")

    def setdefault(
        self, key: KT, default: typing.Optional[DT] = None, *args, **kwargs
    ) -> typing.Optional[typing.Union[VT, DT]]:
        if self.ignore:
            return None

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
        self,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]],
        *args,
        **kwargs,
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
    A lock with a counter to track the number of waiters.

    This class provides a lock mechanism that supports both synchronous and asynchronous contexts,
    with the ability to track the number of threads or coroutines waiting to acquire the lock.
    """

    __slots__ = ("lock", "waiters")

    def __init__(self, is_async: bool = False):
        self.lock = _thread.allocate_lock() if not is_async else asyncio.Lock()
        self.waiters = 0

    async def __aenter__(self) -> None:
        self.waiters += 1
        await self.lock.acquire()  # type: ignore[misc]

    async def __aexit__(self, *args, **kwds) -> None:
        self.waiters -= 1
        self.lock.release()

    def __enter__(self) -> None:
        self.waiters += 1
        self.lock.acquire()

    def __exit__(self, *args, **kwds) -> None:
        self.waiters -= 1
        self.lock.release()


def _copy_if_need(obj: VT, tocopy=(dict, list, set), level: int = 1) -> VT:
    from copy import copy

    if level == 0:
        return obj

    if level == 2:
        return copy(obj)

    return copy(obj) if (type(obj) in tocopy) else obj


def make_key(args: tuple, kwds: dict, fasttype=(int, str)):
    """
    Create a hashable key from function arguments for caching purposes.

    Args:
        args (tuple): Positional arguments to be used in key generation.
        kwds (dict): Keyword arguments to be used in key generation.
        fasttype (tuple, optional): Types that can be directly used as keys. Defaults to (int, str).

    Returns:
        A hashable key representing the function arguments, optimized for simple single-argument cases.
    """
    key = args
    if kwds:
        key += (object,)
        for item in kwds.items():
            key += item

    if fasttype and len(key) == 1 and type(key[0]) in fasttype:
        return key[0]

    return key


def make_hash_key(args: tuple, kwds: dict):
    """
    Create a hashable hash key from function arguments for caching purposes.

    Args:
        args (tuple): Positional arguments to be used in key generation.
        kwds (dict): Keyword arguments to be used in key generation.

    Returns:
        int: A hash value representing the function arguments.
    """
    return hash(make_key(args, kwds))


def make_typed_key(args: tuple, kwds: dict):
    """
    Create a hashable key from function arguments that includes type information.

    Args:
        args (tuple): Positional arguments to be used in key generation.
        kwds (dict): Keyword arguments to be used in key generation.

    Returns:
        A hashable key representing the function arguments, including the types of the arguments.
    """
    key = make_key(args, kwds, fasttype=())

    key += tuple(type(v) for v in args)
    if kwds:
        key += tuple(type(v) for v in kwds.values())

    return key


CacheInfo = namedtuple("CacheInfo", ["hits", "misses", "maxsize", "length", "memory"])
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
):
    _key_maker = (lambda args, kwds: key_maker(args[1:], kwds)) if is_method else key_maker

    hits = 0
    misses = 0
    locks: defaultdict[typing.Hashable, _LockWithCounter] = defaultdict(_LockWithCounter)
    exceptions: typing.Dict[typing.Hashable, BaseException] = {}

    def _wrapped(*args, **kwds):
        nonlocal hits, misses, locks, exceptions

        if kwds.pop("cachebox__ignore", False):
            return func(*args, **kwds)

        key = _key_maker(args, kwds)

        # try to get result from cache
        try:
            result = cache[key]
        except KeyError:
            pass
        else:
            # A NOTE FOR ME: we don't want to catch KeyError exceptions from `callback`
            # so don't wrap it with try except
            hits += 1

            if callback is not None:
                callback(EVENT_HIT, key, result)

            return _copy_if_need(result, level=copy_level)

        with locks[key]:
            if exceptions.get(key, None) is not None:
                cached_error = exceptions[key] if locks[key].waiters > 1 else exceptions.pop(key)
                raise cached_error

            try:
                result = cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = func(*args, **kwds)
                except Exception as e:
                    if locks[key].waiters > 1:
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

    def cache_clear() -> None:
        nonlocal misses, hits, locks, exceptions
        cache.clear(reuse=clear_reuse)
        misses = 0
        hits = 0
        locks.clear()
        exceptions.clear()

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
):
    _key_maker = (lambda args, kwds: key_maker(args[1:], kwds)) if is_method else key_maker

    hits = 0
    misses = 0
    locks: defaultdict[typing.Hashable, _LockWithCounter] = defaultdict(lambda: _LockWithCounter(True))
    exceptions: typing.Dict[typing.Hashable, BaseException] = {}

    async def _wrapped(*args, **kwds):
        nonlocal hits, misses, locks, exceptions

        if kwds.pop("cachebox__ignore", False):
            return await func(*args, **kwds)

        key = _key_maker(args, kwds)

        # try to get result from cache
        try:
            result = cache[key]
        except KeyError:
            pass
        else:
            # A NOTE FOR ME: we don't want to catch KeyError exceptions from `callback`
            # so don't wrap it with try except
            hits += 1

            if callback is not None:
                awaitable = callback(EVENT_HIT, key, result)
                if inspect.isawaitable(awaitable):
                    await awaitable

            return _copy_if_need(result, level=copy_level)

        async with locks[key]:
            if exceptions.get(key, None) is not None:
                cached_error = exceptions[key] if locks[key].waiters > 1 else exceptions.pop(key)
                raise cached_error

            try:
                result = cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = await func(*args, **kwds)
                except Exception as e:
                    if locks[key].waiters > 1:
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

    def cache_clear() -> None:
        nonlocal misses, hits, locks, exceptions
        cache.clear(reuse=clear_reuse)
        misses = 0
        hits = 0
        locks.clear()
        exceptions.clear()

    _wrapped.cache_clear = cache_clear

    return _wrapped


def cached(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]] = None,
    copy_level: int = 1,
) -> typing.Callable[[FT], FT]:
    """
    Decorator to create a memoized cache for function results.

    Wraps a function to automatically cache and retrieve its results based on input parameters.

    Args:
        cache (BaseCacheImpl, dict, optional): Cache implementation to store results. Defaults to FIFOCache.
        key_maker (Callable, optional): Function to generate cache keys from function arguments. Defaults to make_key.
        clear_reuse (bool, optional): Whether to reuse cache during clearing. Defaults to False.
        callback (Callable, optional): Function called on cache hit/miss events. Defaults to None.
        copy_level (int, optional): Level of result copying. Defaults to 1.

    Returns:
        Callable: Decorated function with caching capabilities.

    Example::

        @cachebox.cached(cachebox.LRUCache(128))
        def sum_as_string(a, b):
            return str(a+b)

        assert sum_as_string(1, 2) == "3"

        assert len(sum_as_string.cache) == 1
        sum_as_string.cache_clear()
        assert len(sum_as_string.cache) == 0
    """
    if cache is None:
        cache = FIFOCache(0)

    if type(cache) is dict:
        cache = FIFOCache(0, cache)

    if not isinstance(cache, BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    def decorator(func: FT) -> FT:
        if inspect.iscoroutinefunction(func):
            wrapper = _async_cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, False
            )
        else:
            wrapper = _cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, False
            )

        return functools.update_wrapper(wrapper, func)  # type: ignore[return-value]

    return decorator


def cachedmethod(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, typing.Any], typing.Any]] = None,
    copy_level: int = 1,
) -> typing.Callable[[FT], FT]:
    """
    Decorator to create a method-specific memoized cache for function results.

    Similar to `cached()`, but ignores `self` parameter when generating cache keys.

    Args:
        cache (BaseCacheImpl, dict, optional): Cache implementation to store results. Defaults to FIFOCache.
        key_maker (Callable, optional): Function to generate cache keys from function arguments. Defaults to make_key.
        clear_reuse (bool, optional): Whether to reuse cache during clearing. Defaults to False.
        callback (Callable, optional): Function called on cache hit/miss events. Defaults to None.
        copy_level (int, optional): Level of result copying. Defaults to 1.

    Returns:
        Callable: Decorated method with method-specific caching capabilities.
    """
    if cache is None:
        cache = FIFOCache(0)

    if type(cache) is dict:
        cache = FIFOCache(0, cache)

    if not isinstance(cache, BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    def decorator(func: FT) -> FT:
        if inspect.iscoroutinefunction(func):
            wrapper = _async_cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, True
            )
        else:
            wrapper = _cached_wrapper(
                func, cache, key_maker, clear_reuse, callback, copy_level, True
            )

        return functools.update_wrapper(wrapper, func)  # type: ignore[return-value]

    return decorator


def is_cached(func: object) -> bool:
    """
    Check if a function/method cached by cachebox or not
    """
    return hasattr(func, "cache") and isinstance(func.cache, BaseCacheImpl)
