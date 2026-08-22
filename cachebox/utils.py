import functools
import inspect
import typing
import _thread
import asyncio
import threading
import warnings
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
    """ Default cache key.

    Fast-path: a single `int or str argument is returned as-is.
    Otherwise a plain tuple (plus a kwargs sentinel when needed) is returned.
    """

    if not kwds:
        if len(args) == 1 and type(args[0]) in _FAST_TYPES:
            return args[0]

        return args


def make_hash_key(*args, **kwds) -> int:
    """
    Return the hash of all positional and keyword arguments.

    Note:
        The returned integer is not collision-free. Distinct argument
        combinations may produce the same hash value.
    """
    if not kwds:
        return hash(args)

    key = [*args, _KWDS_MARK]

    for item in kwds.items():
        key.extend(item)

    return hash(tuple(key))


def make_typed_key(*args, **kwds) -> tuple:
    """
    Key that includes the exact runtime type of every argument.

    Ensures ``f(1)`` and ``f(1.0)`` are cached separately even though
    ``1 == 1.0``.
    """
    key = [*args]

    if kwds:
        key.append(_KWDS_MARK)

        for item in kwds.items():
            key.extend(item)

    key.extend(type(value) for value in args)

    if kwds:
        key.extend(type(value) for value in kwds.values())

    return tuple(key)


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
            ignore: If ``True``, silently ignores modification attempts.
                If ``False``, raises ``TypeError`` when modification is
                attempted. Defaults to ``False``.

        Raises:
            TypeError: If ``cls`` is not a ``BaseCacheImpl`` instance or
                is already a ``Frozen`` cache.
        """
        if not isinstance(cls, BaseCacheImpl):
            raise TypeError(
                f"expected a BaseCacheImpl instance, got {type(cls).__name__!r}"
            )

        if type(cls) is Frozen:
            raise TypeError("cannot wrap an already-frozen cache")

        self.__cache = cls
        self.ignore = ignore

    def _guard(self) -> None:
        """
        Guard against modification attempts.

        Raises:
            TypeError: If the cache is frozen and ``ignore`` is ``False``.
        """
        if not self.ignore:
            raise TypeError("This cache is frozen.")

    @property
    def cache(self) -> BaseCacheImpl[KT, VT]:
        """Return the wrapped cache implementation."""
        return self.__cache

    @property
    def maxsize(self) -> int:
        """Return the configured maximum cache size."""
        return self.__cache.maxsize

    @property
    def getsizeof(self) -> typing.Callable[[KT, VT], int] | None:
        """Return the configured ``getsizeof`` callable, or ``None``."""
        return self.__cache.getsizeof

    def current_size(self) -> int:
        """
        Return the current cumulative size of all stored entries.

        Returns:
            The sum of the sizes of all entries currently stored in the cache.
        """
        return self.__cache.current_size()

    def remaining_size(self) -> int:
        """
        Return the remaining available cache size.

        Returns:
            The result of ``maxsize - current_size``.
        """
        return self.__cache.remaining_size()

    def capacity(self) -> int:
        """
        Return the current allocated cache capacity.

        Returns:
            The number of elements the underlying map can hold without
            reallocating.
        """
        return self.__cache.capacity()

    def __len__(self) -> int:
        """
        Return the number of entries currently stored in the cache.

        Returns:
            The number of entries currently in the cache.
        """
        return len(self.__cache)

    def __sizeof__(self) -> int:
        """Return the memory size reported by the underlying cache."""
        return self.__cache.__sizeof__()

    def __bool__(self) -> bool:
        """Return ``True`` if the underlying cache contains any entries."""
        return bool(self.__cache)

    def __contains__(self, key: KT) -> bool:
        """Return whether ``key`` exists in the underlying cache."""
        return self.__cache.contains(key)

    def contains(self, key: KT) -> bool:
        """
        Return whether ``key`` exists in the cache.

        This is equivalent to ``key in self`` and is provided for
        compatibility across different cache policies.

        Args:
            key: The key to look up.

        Returns:
            ``True`` if the key exists in the cache, otherwise ``False``.
        """
        return self.__cache.contains(key)

    def is_empty(self) -> bool:
        """
        Return whether the cache is empty.

        Returns:
            ``True`` if the cache contains no entries, otherwise ``False``.
        """
        return self.__cache.is_empty()

    def is_full(self) -> bool:
        """
        Return whether the cache has reached its maximum size.

        Returns:
            ``True`` if the cumulative size has reached the ``maxsize`` limit,
            otherwise ``False``.
        """
        return self.__cache.is_full()

    def insert(
        self,
        key: KT,
        value: VT,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT]:
        """
        Attempt to insert an item into the cache.

        Since this cache is frozen, the operation is never forwarded to the
        underlying cache.

        Args:
            key: The key associated with the value.
            value: The value to insert.
            *args: Additional positional arguments accepted for compatibility.
            **kwargs: Additional keyword arguments accepted for compatibility.

        Returns:
            ``None`` when ``ignore=True``.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def __setitem__(self, key: KT, value: VT) -> None:
        """
        Attempt to assign an item using subscription syntax.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def update(
        self,
        iterable: "_IterableType[KT, VT]",
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> None:
        """
        Attempt to update the cache with multiple items.

        The operation is never forwarded to the underlying cache.

        Args:
            iterable: An iterable containing cache entries.
            *args: Additional positional arguments accepted for compatibility.
            **kwargs: Additional keyword arguments accepted for compatibility.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def get(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
    ) -> typing.Union[VT, DT]:
        """
        Return the value associated with ``key`` without modifying the cache.

        Args:
            key: The key to look up.
            default: The value returned when ``key`` is not present.

        Returns:
            The cached value or ``default`` when the key is absent.
        """
        return self.__cache.get(key, default)

    def __getitem__(self, key: KT) -> VT:
        """
        Return the value associated with ``key``.

        Args:
            key: The key to look up.

        Returns:
            The cached value.

        Raises:
            KeyError: If ``key`` does not exist in the cache.
        """
        return self.__cache[key]

    def setdefault(
        self,
        key: KT,
        default: typing.Optional[DT] = None,
        *args: typing.Any,
        **kwargs: typing.Any,
    ) -> typing.Optional[VT | DT]:
        """
        Attempt to insert ``default`` when ``key`` is missing.

        Since this cache is frozen, no modification is performed.

        Args:
            key: The key to look up.
            default: The value that would normally be inserted.
            *args: Additional positional arguments accepted for compatibility.
            **kwargs: Additional keyword arguments accepted for compatibility.

        Returns:
            ``None`` when ``ignore=True``.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def pop(
        self,
        key: KT,
        default: DT = None,
    ) -> typing.Union[VT, DT]:
        """
        Attempt to remove and return the value associated with ``key``.

        Since this cache is frozen, the operation is never performed.

        Args:
            key: The key to remove.
            default: Value to return if the key is not found.

        Returns:
            ``None`` when ``ignore=True``.

        Raises:
            TypeError: If ``ignore=False``.
            KeyError: Normally raised when ``key`` is missing and no default
                is provided, but the frozen cache blocks the operation first.
        """
        self._guard()  # type: ignore[return-value]

    def __delitem__(self, key: KT) -> None:
        """
        Attempt to delete ``key`` from the cache.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        Attempt to remove and return an arbitrary cache entry.

        Returns:
            ``None`` when ``ignore=True``.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()  # type: ignore[return-value]

    def drain(self, n: int) -> int:
        """
        Attempt to remove up to ``n`` entries from the cache.

        Args:
            n: The maximum number of entries to remove.

        Returns:
            ``None`` when ``ignore=True``.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()  # type: ignore[return-value]

    def shrink_to_fit(self) -> None:
        """
        Attempt to release unused internal allocation.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Attempt to remove all entries from the cache.

        Args:
            reuse: If ``True``, the underlying allocation would normally be
                retained for future reuse. Since this cache is frozen, no
                modification is performed.

        Raises:
            TypeError: If ``ignore=False``.
        """
        self._guard()

    def items(self) -> typing.Iterable[typing.Tuple[KT, VT]]:
        """Return an iterable over the cache's key-value pairs."""
        return self.__cache.items()

    def values(self) -> typing.Iterable[VT]:
        """Return an iterable over the cache's values."""
        return self.__cache.values()

    def keys(self) -> typing.Iterable[KT]:
        """Return an iterable over the cache's keys."""
        return self.__cache.keys()

    def __iter__(self) -> typing.Iterator[KT]:
        """Return an iterator over the cache's keys."""
        return iter(self.__cache)

    def copy(self) -> "Frozen[KT, VT]":
        """
        Return a frozen copy of the underlying cache.

        Returns:
            A new ``Frozen`` instance containing a copy of the underlying
            cache and preserving the current ``ignore`` setting.
        """
        return Frozen(self.__cache.copy(), ignore=self.ignore)

    def __copy__(self) -> "Frozen[KT, VT]":
        """
        Return a shallow copy of this frozen cache wrapper.

        Returns:
            A new ``Frozen`` instance containing a copy of the underlying
            cache and preserving the current ``ignore`` setting.
        """
        return Frozen(self.__cache.copy(), ignore=self.ignore)

    def __repr__(self) -> str:
        """Return the string representation of the frozen cache."""
        return f"Frozen({self.__cache!r})"


def _cast_lock(
    iscoroutinefunction: bool,
    lock: (
        typing.Type[AbstractContextManager]
        | typing.Type[AbstractAsyncContextManager]
        | bool
        | None
    ) = True,
) -> (
    typing.Type[AbstractContextManager]
    | typing.Type[AbstractAsyncContextManager]
    | None
):
    """
    Validate and normalize the lock configuration for a cached wrapper.

    Args:
        iscoroutinefunction: Whether the wrapped function is asynchronous.
        lock: Lock type, context manager type, or a boolean controlling
            automatic lock selection.

    Returns:
        The appropriate lock type, or ``None`` when locking is disabled.

    Raises:
        TypeError: If an incompatible lock is supplied for the wrapped
            function type.
    """
    if lock is None or lock is False:
        return None

    if lock is True:
        return asyncio.Lock if iscoroutinefunction else threading.Lock

    if iscoroutinefunction:
        if not hasattr(lock, "__aenter__"):
            raise TypeError(
                "For async functions, you cannot use a regular synchronous lock."
            )

        return typing.cast(typing.Type[AbstractAsyncContextManager], lock)

    # threading.Lock, threading.RLock and _thread.allocate_lock are functions.
    if (
        lock is threading.Lock
        or lock is threading.RLock
        or lock is _thread.allocate_lock
    ):
        return typing.cast(typing.Type[AbstractContextManager], lock)

    if not hasattr(lock, "__enter__"):
        raise TypeError(
            "For sync functions, you cannot use a asynchronous lock."
        )

    return typing.cast(typing.Type[AbstractContextManager], lock)


def cached(
    cache: BaseCacheImpl | dict | typing.Callable[..., BaseCacheImpl] | None = None,
    key_maker: typing.Callable[..., typing.Hashable] = make_key,
    clear_reuse: bool = False,
    callback: _Callback | None = None,
    copy_level: int = 1,
    postprocess: _PostProcess | None = postprocess_copy_mutables,
    lock: (
        typing.Type[AbstractContextManager]
        | typing.Type[AbstractAsyncContextManager]
        | bool
        | None
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

        copy_level: Deprecated and no longer has any effect. Use the
            ``postprocess`` parameter instead.

        postprocess: Optional ``(value) -> value`` transform applied before
            returning a result to the caller. Ready-to-use options:

            * ``None`` - return the cached object as-is.
            * :func:`postprocess_copy` - shallow-copy.
            * :func:`postprocess_copy_mutables` - shallow-copy only
              ``dict``, ``list`` and ``set`` (default).
            * :func:`postprocess_deepcopy` - deep-copy.
            * :func:`postprocess_deepcopy_mutables` - deep-copy only
              ``dict``, ``list`` and ``set``.

        lock: If ``None`` or ``False``, cache stampede prevention is disabled,
            while the underlying cache remains thread-safe.

            If ``True``, ``threading.Lock`` is used for synchronous functions
            and ``asyncio.Lock`` for asynchronous functions.

            A compatible context manager type may also be supplied.

    Tip:
        Pass ``cachebox__ignore=True`` at call-time to bypass the cache.

        If *cache* is not a lambda/function, the following attributes are
        attached to the decorated function:

        * ``cache`` - the underlying cache.
        * ``cache_info`` - cache statistics callable.
        * ``clear_cache`` - cache clearing callable.
        * ``callback`` - callback property.

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
        warnings.warn(
            "`copy_level` parameter has been deprecated and no longer has any "
            "effect. Use the `postprocess` parameter instead",
            category=DeprecationWarning,
        )

    if cache is None:
        cache = LRUCache(0)
    elif type(cache) is dict:
        cache = LRUCache(0, cache)  # type: ignore[arg-type]

    cache_is_fn = callable(cache)

    if not isinstance(cache, BaseCacheImpl) and not cache_is_fn:
        raise TypeError(
            "expected a cachebox cache or a callable, got %r" % (cache,)
        )

    def decorator(func: FT) -> FT:
        iscoroutinefunction = inspect.iscoroutinefunction(func)
        lock_type = _cast_lock(iscoroutinefunction, lock)

        if not iscoroutinefunction and inspect.iscoroutinefunction(callback):
            raise TypeError(
                "For sync functions, you cannot use a asynchronous callback"
            )

        if lock_type:
            builder = (
                _async_cached_wrapper
                if iscoroutinefunction
                else _cached_wrapper
            )

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

        return functools.update_wrapper(
            wrapper,
            func,
        )  # type: ignore[return-value]

    return decorator


def is_cached(func: object) -> bool:
    """
    Return ``True`` if *func* was decorated with :func:`cached`.

    Args:
        func: Object or function to inspect.

    Returns:
        ``True`` when *func* exposes a cache managed by :func:`cached`,
        otherwise ``False``.
    """
    return hasattr(func, "cache") and isinstance(
        func.cache,
        BaseCacheImpl,
    )  # type: ignore[union-attr]

def get_cached_cache(cached_func: object) -> BaseCacheImpl:
    """
    Return the cache attached to a cached function without type-checker warnings.

    Args:
        cached_func: A function decorated with :func:`cached`.

    Returns:
        The underlying :class:`BaseCacheImpl` instance.

    Raises:
        AttributeError: If *cached_func* was not decorated with :func:`cached`
            or uses a callable cache factory.
    """
    return cached_func.cache  # type: ignore


def get_cached_cache_info(cached_func: object) -> CacheInfo:
    """
    Return cache statistics for a cached function without type-checker warnings.

    Args:
        cached_func: A function decorated with :func:`cached`.

    Returns:
        The :class:`CacheInfo` object returned by ``cache_info()``.

    Raises:
        AttributeError: If *cached_func* does not expose ``cache_info``.
    """
    return cached_func.cache_info()  # type: ignore


def get_cached_callback(cached_func: object) -> _Callback | None:
    """
    Return the callback attached to a cached function.

    Args:
        cached_func: A function decorated with :func:`cached`.

    Returns:
        The configured callback, or ``None`` when no callback is configured.

    Raises:
        AttributeError: If *cached_func* does not expose ``callback``.
    """
    return cached_func.callback  # type: ignore


def clear_cached_cache(cached_func: object) -> None:
    """
    Clear the cache attached to a cached function.

    Args:
        cached_func: A function decorated with :func:`cached`.

    Raises:
        AttributeError: If *cached_func* does not expose ``cache_clear``.
    """
    cached_func.cache_clear()  # type: ignore
