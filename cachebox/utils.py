from ._cachebox import BaseCacheImpl, FIFOCache
from collections import namedtuple
import functools
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

    def __str__(self) -> str:
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


def _copy_if_need(obj, tocopy=(dict, list, set)):
    from copy import copy

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

    key += tuple(type(v) for v in args)
    if kwds:
        key += tuple(type(v) for v in kwds.values())

    return key


CacheInfo = namedtuple("CacheInfo", ["hits", "misses", "maxsize", "length", "cachememory"])

_NOT_SETTED = object()


EVENT_MISS = 1
EVENT_HIT = 2


class _cached_wrapper(typing.Generic[VT]):
    def __init__(
        self,
        cache: BaseCacheImpl[typing.Any, VT],
        func: typing.Callable[..., VT],
        key_maker: typing.Callable[[tuple, dict], typing.Hashable],
        clear_reuse: bool,
        is_method: bool,
        *,
        callback: typing.Optional[typing.Callable[[int, typing.Any, VT], None]] = None,
    ) -> None:
        self.cache = cache
        self.func = func
        self.callback = callback
        self._key_maker = (
            (lambda args, kwds: key_maker(args[1:], kwds)) if is_method else (key_maker)
        )
        self.__reuse = clear_reuse
        self._hits = 0
        self._misses = 0

        self.instance = _NOT_SETTED

        functools.update_wrapper(self, func)

    def cache_info(self) -> CacheInfo:
        return CacheInfo(
            self._hits, self._misses, self.cache.maxsize, len(self.cache), self.cache.__sizeof__()
        )

    def cache_clear(self) -> None:
        self.cache.clear(reuse=self.__reuse)
        self._hits = 0
        self._misses = 0

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__}: {self.func}>"

    if not typing.TYPE_CHECKING:

        def __get__(self, instance, *args):
            self.instance = instance
            return self

    def __call__(self, *args, **kwds) -> VT:
        if self.instance is not _NOT_SETTED:
            args = (self.instance, *args)

        if kwds.pop("cachebox__ignore", False):
            return self.func(*args, **kwds)

        key = self._key_maker(args, kwds)
        try:
            result = self.cache[key]
            self._hits += 1

            if self.callback is not None:
                self.callback(EVENT_HIT, key, result)

            return _copy_if_need(result)
        except KeyError:
            self._misses += 1

        result = self.func(*args, **kwds)

        if self.callback is not None:
            self.callback(EVENT_MISS, key, result)

        self.cache[key] = result
        return _copy_if_need(result)


class _async_cached_wrapper(_cached_wrapper[VT]):
    async def __call__(self, *args, **kwds) -> VT:
        if self.instance is not _NOT_SETTED:
            args = (self.instance, *args)

        if kwds.pop("cachebox__ignore", False):
            return await self.func(*args, **kwds)

        key = self._key_maker(args, kwds)
        try:
            result = self.cache[key]
            self._hits += 1

            if self.callback is not None:
                awaitable = self.callback(EVENT_HIT, key, result)
                if inspect.isawaitable(awaitable):
                    await awaitable

            return _copy_if_need(result)
        except KeyError:
            self._misses += 1

        result = await self.func(*args, **kwds)
        self.cache[key] = result

        if self.callback is not None:
            awaitable = self.callback(EVENT_MISS, key, result)
            if inspect.isawaitable(awaitable):
                await awaitable

        return _copy_if_need(result)


def cached(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, VT], None]] = None,
    **kwargs,
):
    """
    a decorator that helps you to cache your functions and calculations with a lot of options.

    :param cache: set your cache and cache policy. (If is `None` or `dict`, `FIFOCache` will be used)

    :param key_maker: you can set your key maker, See [examples](https://github.com/awolverp/cachebox#function-cached).

    :param clear_reuse: The `clear_reuse` param will be passed to cache's `clear` method.

    :param callback: Every time the `cache` is used, callback is also called.
                     The callback arguments are: event number (see `EVENT_MISS` or `EVENT_HIT` variables), key, and then result.

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
    if isinstance(cache, dict) or cache is None:
        cache = FIFOCache(0)

    if type(cache) is type or not isinstance(cache, BaseCacheImpl):
        raise TypeError("we expected cachebox caches, got %r" % (cache,))

    if "info" in kwargs:
        import warnings

        warnings.warn(
            "'info' parameter is deprecated and no longer available.",
            DeprecationWarning,
        )

    @typing.overload
    def decorator(func: typing.Callable[..., VT]) -> _cached_wrapper[VT]: ...

    @typing.overload
    def decorator(
        func: typing.Callable[..., typing.Awaitable[VT]],
    ) -> _async_cached_wrapper[VT]: ...

    def decorator(func):
        if inspect.iscoroutinefunction(func):
            return _async_cached_wrapper(
                cache,
                func,
                key_maker=key_maker,
                clear_reuse=clear_reuse,
                is_method=kwargs.get("is_method", False),
                callback=callback,
            )

        return _cached_wrapper(
            cache,
            func,
            key_maker=key_maker,
            clear_reuse=clear_reuse,
            is_method=kwargs.get("is_method", False),
            callback=callback,
        )

    return decorator


def cachedmethod(
    cache: typing.Union[BaseCacheImpl, dict, None],
    key_maker: typing.Callable[[tuple, dict], typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: typing.Optional[typing.Callable[[int, typing.Any, VT], None]] = None,
    **kwargs,
):
    """
    this is excatly works like `cached()`, but ignores `self` parameters in hashing and key making.
    """
    kwargs["is_method"] = True
    return cached(cache, key_maker, clear_reuse, callback, **kwargs)


_K = typing.TypeVar("_K")
_V = typing.TypeVar("_V")


def items_in_order(cache: BaseCacheImpl[_K, _V]):
    import warnings

    warnings.warn(
        "'items_in_order' function is deprecated and no longer is available, because all '.items()' methods are ordered now.",
        DeprecationWarning,
    )

    return cache.items()


def is_cached(func: object) -> bool:
    """
    Check if a function/method cached by cachebox or not
    """
    return isinstance(func, (_cached_wrapper, _async_cached_wrapper))
