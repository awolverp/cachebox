import functools
import inspect
import typing
from copy import copy as _shallow_copy
from copy import deepcopy as _deep_copy

from ._cachebox import BaseCacheImpl, LRUCache
from ._wrappers import (
    AbstractAsyncContextManager,
    AbstractContextManager,
    CacheInfo,
    _async_cached_wrapper,
    _async_cached_wrapper_without_lock,
    _cached_wrapper,
    _cached_wrapper_without_lock,
    _Callback,
    _PostProcess,
)

if typing.TYPE_CHECKING:
    from ._core import _IterableType

KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")
FT = typing.TypeVar("FT", bound=typing.Callable[..., typing.Any])


_COPY_TYPES = frozenset((dict, list, set))


def postprocess_copy_mutables(value: VT) -> VT:
    """
    Shallow-copy *value* before returning it (only `dict`, `list`, and `set`)
    """
    if type(value) in _COPY_TYPES:
        return _shallow_copy(value)

    return value


def postprocess_copy(value: VT) -> VT:
    """Shallow-copy *value* before returning it"""
    return _shallow_copy(value)


def postprocess_deepcopy_mutables(value: VT) -> VT:
    """
    Deep-copy *value* before returning it (only `dict`, `list`, and `set`)
    """
    if type(value) in _COPY_TYPES:
        return _deep_copy(value)

    return value


def postprocess_deepcopy(value: VT) -> VT:
    """Deep-copy *value* before returning it"""
    return _deep_copy(value)


_KWDS_MARK = object()
_FAST_TYPES = frozenset((int, str))


def make_key(*args, **kwds) -> typing.Hashable:
    """
    Default cache key.

    Fast-path: a single ``int`` or ``str`` argument is returned as-is.
    Otherwise a plain tuple (plus a kwargs sentinel when needed) is returned.
    """
    if not kwds:
        if len(args) == 1 and type(args[0]) in _FAST_TYPES:
            return args[0]
        return args

    key = args + (_KWDS_MARK,)
    for item in kwds.items():
        key += item
    return key[0] if len(key) == 1 and type(key[0]) in _FAST_TYPES else key


def make_hash_key(*args, **kwds) -> int:
    """
    Key as the hash of all positional and keyword arguments.

    Avoids storing the raw argument tuple, at the cost of potential hash
    collisions mapping distinct inputs to the same cache slot.
    """
    if not kwds:
        return hash(args)
    key = args + (_KWDS_MARK,)
    for item in kwds.items():
        key += item
    return hash(key)


def make_typed_key(*args, **kwds) -> tuple:
    """
    Key that includes the runtime type of every argument.

    Ensures ``f(1)`` and ``f(1.0)`` are cached separately even though
    ``1 == 1.0``.
    """
    key: tuple = args
    if kwds:
        key += (_KWDS_MARK,)
        for item in kwds.items():
            key += item

    key += tuple(type(v) for v in args)
    if kwds:
        key += tuple(type(v) for v in kwds.values())

    return key


class Frozen(BaseCacheImpl[KT, VT]):  # pragma: no cover
    """
    A wrapper class that prevents modifications to an underlying cache implementation.

    This class provides a read-only view of a cache, optionally allowing silent
    suppression of modification attempts instead of raising exceptions.

    Example::

        from cachebox import Frozen, FIFOCache

        cache = FIFOCache(10, {1:1, 2:2, 3:3})

        frozen = Frozen(cache, ignore=True)
        print(frozen[1]) # 1
        print(len(frozen)) # 3

        # Frozen ignores this action and do nothing
        frozen.insert("key", "value")
        print(len(frozen)) # 3

        # Let's try with ignore=False
        frozen = Frozen(cache, ignore=False)

        frozen.insert("key", "value")
        # TypeError: This cache is frozen.
    """

    __slots__ = ("__cache", "ignore")

    def __init__(self, cls: BaseCacheImpl[KT, VT], ignore: bool = False) -> None:
        """
        Initialize a frozen cache wrapper.

        Args:
            cls: The underlying cache implementation to be frozen.
            ignore: If ``True``, silently ignores modification attempts; if ``False``, raises
                ``TypeError`` when modification is attempted. Default is ``False``.
        """
        assert isinstance(cls, BaseCacheImpl)
        assert type(cls) is not Frozen

        self.__cache = cls
        self.ignore = ignore

    def _guard(self) -> None:
        if not self.ignore:
            raise TypeError("This cache is frozen.")

    @property
    def cache(self) -> BaseCacheImpl[KT, VT]:
        """Returns the wrapped cache implementation."""
        return self.__cache

    @property
    def maxsize(self) -> int:
        """The configured ``maxsize``."""
        return self.__cache.maxsize

    @property
    def getsizeof(self) -> typing.Callable[[KT, VT], int] | None:
        """Callable or None: The configured ``getsizeof`` function."""
        return self.__cache.getsizeof

    def current_size(self) -> int:
        """
        Returns the current total cumulative size of all stored entries.

        Returns:
            The sum of sizes of all entries currently in the cache.
        """
        return self.__cache.current_size()

    def remaining_size(self) -> int:
        """
        Returns the remaining available size.

        Returns:
            The result of ``maxsize - current_size``.
        """
        return self.__cache.remaining_size()

    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        Returns:
            The current allocated capacity.
        """
        return self.__cache.capacity()

    def __len__(self) -> int:
        """
        Returns the number of entries currently in the cache.

        Returns:
            The number of entries in the cache.
        """
        return len(self.__cache)

    def __sizeof__(self) -> int:
        return self.__cache.__sizeof__()

    def __bool__(self) -> bool:
        return bool(self.__cache)

    def __contains__(self, key: KT) -> bool:
        return self.__cache.contains(key)

    def contains(self, key: KT) -> bool:
        """
        Returns ``True`` if the cache contains an entry for ``key``.

        Equivalent to ``key in self``. Prefer this method over ``key in self``
        to keep code compatible across different cache policies.

        Args:
            key: The key to look up.

        Returns:
            ``True`` if the key exists in the cache, ``False`` otherwise.
        """
        return self.__cache.contains(key)

    def is_empty(self) -> bool:
        """
        Returns ``True`` if the cache is empty.

        Returns:
            ``True`` if the cache contains no entries.
        """
        return self.__cache.is_empty()

    def is_full(self) -> bool:
        """
        Returns ``True`` when the cumulative size has reached the maxsize limit.

        Returns:
            ``True`` if the cache is at capacity.
        """
        return self.__cache.is_full()

    def insert(
        self,
        key: KT,
        value: VT,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT]:
        return self._guard()

    def __setitem__(self, key: KT, value: VT) -> None:
        return self._guard()

    def update(
        self,
        iterable: "_IterableType[KT, VT]",
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> None:
        return self._guard()

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        return self.__cache.get(key, default)

    def __getitem__(self, key: KT) -> VT:
        return self.__cache[key]

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT | DT]:
        return self._guard()

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Removes the specified key and returns the corresponding value.

        Args:
            key: The key to remove.
            default: Value to return if the key is not found.

        Returns:
            The value associated with ``key``, or ``default`` if not found.

        Raises:
            KeyError: If the key is not found and no ``default`` is provided.
        """
        return self._guard()  # type: ignore[return-value]

    def __delitem__(self, key: KT) -> None:
        return self._guard()

    def popitem(self) -> typing.Tuple[KT, VT]:
        return self._guard()  # type: ignore[return-value]

    def drain(self, n: int) -> int:
        """
        Calls ``popitem()`` ``n`` times and returns the count of removed items.

        Args:
            n: The number of items to remove.

        Returns:
            The number of items successfully removed.
        """
        return self._guard()  # type: ignore[return-value]

    def shrink_to_fit(self) -> None:
        """Shrinks the internal allocation as close to the current length as possible."""
        return self._guard()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from the cache.

        Args:
            reuse: If ``True``, retains the allocated memory for future reuse
                rather than freeing it. Defaults to ``False``.
        """
        return self._guard()

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        return self.__cache.items()

    def values(self) -> typing.Iterable[VT]:
        return self.__cache.values()

    def keys(self) -> typing.Iterable[KT]:
        return self.__cache.keys()

    def __iter__(self) -> typing.Iterator[KT]:
        return iter(self.__cache)

    def copy(self) -> "Frozen[KT, VT]":
        return Frozen(self.__cache.copy(), ignore=self.ignore)

    def __copy__(self) -> "Frozen[KT, VT]":
        return Frozen(self.__cache.copy(), ignore=self.ignore)

    def __repr__(self) -> str:
        return "Frozen(%s)" % repr(self.__cache)


def _cast_lock(
    iscoroutinefunction: bool,
    lock: (
        typing.Type[AbstractContextManager] | typing.Type[AbstractAsyncContextManager] | bool | None
    ) = True,
) -> typing.Type[AbstractContextManager] | typing.Type[AbstractAsyncContextManager] | None:
    import _thread
    import asyncio
    import threading

    if lock is None or lock is False:
        return None

    if lock is True:
        return asyncio.Lock if iscoroutinefunction else threading.Lock

    if iscoroutinefunction:
        if not hasattr(lock, "__aenter__"):
            raise TypeError("For async functions, you cannot use a regular synchronous lock.")

        return typing.cast(typing.Type[AbstractAsyncContextManager], lock)

    # threading.Lock, threading.RLock and _thread.allocate_lock are function
    if lock is threading.Lock or lock is threading.RLock or lock is _thread.allocate_lock:
        return typing.cast(typing.Type[AbstractContextManager], lock)

    if not hasattr(lock, "__enter__"):
        raise TypeError("For sync functions, you cannot use a asynchronous lock.")

    return typing.cast(typing.Type[AbstractContextManager], lock)


def cached(
    cache: BaseCacheImpl | dict | typing.Callable[..., BaseCacheImpl] | None = None,
    key_maker: typing.Callable[..., typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: _Callback | None = None,
    copy_level: int = 1,
    postprocess: _PostProcess | None = postprocess_copy_mutables,
    lock: (
        typing.Type[AbstractContextManager] | typing.Type[AbstractAsyncContextManager] | bool | None
    ) = True,
) -> typing.Callable[[FT], FT]:
    """
    Decorator to memoize function/method results.

    Args:
        cache: Cache instance, ``dict``, or callable ``(self) -> cache`` for
            per-instance caches. ``None`` defaults to an unbounded
            :class:`LRUCache`.
        key_maker: Converts ``(args, kwds)`` to a hashable key. Built-ins:
            :func:`make_key` (default), :func:`make_hash_key`,
            :func:`make_typed_key`.
        clear_reuse: Pass ``reuse=True`` to ``cache.clear()`` when
            :func:`cache_clear` is called.
        callback: Called as ``callback(event, key, value)`` on every hit/miss.
            May be a coroutine in async contexts.
        copy_level: It has been deprecated and no longer has any effect. Use
            the postprocess parameter instead.
        postprocess: Optional ``(value) -> value`` transform applied before
            returning a result to the caller. Ready-to-use options:

            * ``None`` - return the cached object as-is.
            * :func:`postprocess_copy` - shallow-copy.
            * :func:`postprocess_copy_mutables` - shallow-copy only `dict`, `list` and `set` (default).
            * :func:`postprocess_deepcopy` - deep-copy.
            * :func:`postprocess_deepcopy_mutables` - deep-copy only `dict`, `list` and `set`.
        lock: If ``None`` or ``False``, cache stampede preventation get disabled, but process is still thread-safe.
            If ``True``, will use ``threading.Lock`` or ``asyncio.Lock`` depends on wrapped function.
            Also you can pass anything that implemented ``contextlib.AbstractContextManager``
            (or ``contextlib.AbstractAsyncContextManager`` for async functions).
            (default is ``True``).

    Tip:
        Pass ``cachebox__ignore=True`` at call-time to bypass the cache.
        If *cache* isn't a lambda/function, these attributes will be attached to
        your function: ``cache`` (property), ``cache_info`` (callable), ``clear_cache`` (callable),
        and ``callback`` (property).

    Examples::

        @cachebox.cached(cachebox.LRUCache(128))
        def add(a, b):
            return a + b

        # Per-instance method cache
        class Foo:
            def __init__(self):
                self._cache = cachebox.LRUCache(0)

            @cachebox.cached(lambda self: self._cache)
            def compute(self, n):
                return n * 2
    """
    if copy_level != 1:
        import warnings

        warnings.warn(
            "`copy_level` parameter has been deprecated and no longer has any effect. Use the `postprocess` parameter instead",
            category=DeprecationWarning,
        )

    if cache is None:
        cache = LRUCache(0)
    elif type(cache) is dict:
        cache = LRUCache(0, cache)  # type: ignore[arg-type]

    cache_is_fn = callable(cache)
    if not isinstance(cache, BaseCacheImpl) and not cache_is_fn:
        raise TypeError("expected a cachebox cache or a callable, got %r" % (cache,))

    def decorator(func: FT) -> FT:
        iscoroutinefunction = inspect.iscoroutinefunction(func)
        lock_type = _cast_lock(iscoroutinefunction, lock)

        if not iscoroutinefunction and inspect.iscoroutinefunction(callback):
            raise TypeError("For sync functions, you cannot use a asynchronous callback")

        if lock_type:
            builder = _async_cached_wrapper if iscoroutinefunction else _cached_wrapper

            wrapper = builder(
                func,
                cache,  # type: ignore
                key_maker,
                clear_reuse,
                callback,
                postprocess,
                lock_type,  # type: ignore
            )
        else:
            builder = (
                _async_cached_wrapper_without_lock
                if iscoroutinefunction
                else _cached_wrapper_without_lock
            )
            wrapper = builder(
                func,
                cache,  # type: ignore
                key_maker,
                clear_reuse,
                callback,
                postprocess,
            )

        return functools.update_wrapper(wrapper, func)  # type: ignore[return-value]

    return decorator


def is_cached(func: object) -> bool:
    """
    Return ``True`` if *func* was decorated with :func:`cached`.

    Args:
        func: an object or function to check.
    """
    return hasattr(func, "cache") and isinstance(func.cache, BaseCacheImpl)  # type: ignore[union-attr]


def get_cached_cache(cached_func: object) -> BaseCacheImpl:
    """
    A way to get ``cached_func.cache``, without type-hint warnings.

    Args:
        cached_func: a function decorated with :func:`cached`.

    Warning:
        If *func* wasn't decorated with :func:`cached`, or you passed a lambda/function as *cache*
        to :func:`cached` decorator, raises ``AttributeError``.
    """
    return cached_func.cache  # type: ignore


def get_cached_cache_info(cached_func: object) -> CacheInfo:
    """
    A way to get ``cached_func.cache_info()``, without type-hint warnings.

    Args:
        cached_func: a function decorated with :func:`cached`.

    Warning:
        If *func* wasn't decorated with :func:`cached`, or you passed a lambda/function as *cache*
        to :func:`cached` decorator, raises ``AttributeError``.
    """
    return cached_func.cache_info()  # type: ignore


def get_cached_callback(cached_func: object) -> _Callback | None:
    """
    A way to get ``cached_func.callback``, without type-hint warnings.

    Args:
        cached_func: a function decorated with :func:`cached`.

    Warning:
        If *func* wasn't decorated with :func:`cached`, or you passed a lambda/function as *cache*
        to :func:`cached` decorator, raises ``AttributeError``.
    """
    return cached_func.callback  # type: ignore


def clear_cached_cache(cached_func: object) -> None:
    """
    A way to call ``cached_func.cache_clear()``, without type-hint warnings.

    Args:
        cached_func: a function decorated with :func:`cached`.

    Warning:
        If *func* wasn't decorated with :func:`cached`, or you passed a lambda/function as *cache*
        to :func:`cached` decorator, raises ``AttributeError``.
    """
    return cached_func.cache_clear()  # type: ignore
