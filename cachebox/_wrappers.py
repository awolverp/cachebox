import inspect
import typing
from collections import namedtuple
from contextlib import AbstractAsyncContextManager, AbstractContextManager

from cachebox import BaseCacheImpl

_PostProcess: typing.TypeAlias = typing.Callable[[typing.Any], typing.Any]
_Callback: typing.TypeAlias = typing.Callable[[int, typing.Any, typing.Any], typing.Any]


class _Lock:
    __slots__ = ("_lock", "waiters")

    def __init__(self, lock: AbstractContextManager) -> None:
        self._lock = lock
        self.waiters = 0

    def __enter__(self) -> None:
        self.waiters += 1
        self._lock.__enter__()

    def __exit__(self, *_) -> None:
        self.waiters -= 1
        self._lock.__exit__(*_)


class _AsyncLock:
    __slots__ = ("_lock", "waiters")

    def __init__(self, lock: AbstractAsyncContextManager) -> None:
        self._lock = lock
        self.waiters = 0

    async def __aenter__(self) -> None:
        self.waiters += 1
        await self._lock.__aenter__()

    async def __aexit__(self, *_) -> None:
        self.waiters -= 1
        await self._lock.__aexit__(*_)


CacheInfo = namedtuple(
    "CacheInfo", ("hits", "misses", "maxsize", "current_size", "length", "memory")
)
EVENT_MISS = 1
EVENT_HIT = 2


def _create_cache_info(cache, hits, misses) -> CacheInfo:
    return CacheInfo(
        hits,
        misses,
        cache.maxsize,
        cache.current_size(),
        len(cache),
        cache.__sizeof__(),
    )


def _cached_wrapper_without_lock(
    func,
    cache: BaseCacheImpl | typing.Callable,
    key_maker: typing.Callable[[tuple, dict], typing.Hashable],
    clear_reuse: bool,
    callback: typing.Callable[[int, typing.Any, typing.Any], None] | None,
    postprocess: _PostProcess | None,
):
    cache_is_fn = callable(cache)

    # Per-instance caches receive `self` as args[0]; exclude it from the ke
    _make_key = (
        (lambda a, k: key_maker(*a[1:], **k))
        if cache_is_fn
        else (lambda a, k: key_maker(*a, **k))
    )

    hits = 0
    misses = 0

    def _wrapped(*args, **kwds):
        nonlocal hits, misses

        # Passing `cachebox__ignore=True` bypasses the cache and
        # calls the function directly.
        if kwds.pop("cachebox__ignore", False):
            return func(*args, **kwds)

        _cache: BaseCacheImpl = cache(args[0]) if cache_is_fn else cache  # type: ignore[arg-type]
        key = _make_key(args, kwds)

        try:
            result = _cache[key]
            hits += 1
            if callback is not None:
                callback(EVENT_HIT, key, result)

            return postprocess(result) if postprocess is not None else result
        except KeyError:
            pass

        result = func(*args, **kwds)
        _cache.insert(key, result)
        hits += 1
        if callback is not None:
            callback(EVENT_MISS, key, result)

        return postprocess(result) if postprocess is not None else result

    if not cache_is_fn:
        _wrapped.cache = cache  # type: ignore[attr-defined]
        _wrapped.cache_info = lambda: _create_cache_info(cache, hits, misses)  # type: ignore

        def cache_clear() -> None:
            nonlocal hits, misses
            cache.clear(reuse=clear_reuse)  # type: ignore[union-attr]
            hits = 0
            misses = 0

        _wrapped.cache_clear = cache_clear  # type: ignore[attr-defined]

    _wrapped.callback = callback  # type: ignore[attr-defined]
    return _wrapped


async def _call_async_callback(callback, event, key, result):
    if callback is None:
        return

    ret = callback(event, key, result)
    if inspect.isawaitable(ret):
        await ret


def _async_cached_wrapper_without_lock(
    func,
    cache: BaseCacheImpl | typing.Callable,
    key_maker: typing.Callable[[tuple, dict], typing.Hashable],
    clear_reuse: bool,
    callback: typing.Callable[[int, typing.Any, typing.Any], None] | None,
    postprocess: _PostProcess | None,
):
    cache_is_fn = callable(cache)

    # Per-instance caches receive `self` as args[0]; exclude it from the ke
    _make_key = (
        (lambda a, k: key_maker(*a[1:], **k))
        if cache_is_fn
        else (lambda a, k: key_maker(*a, **k))
    )

    hits = 0
    misses = 0

    async def _wrapped(*args, **kwds):
        nonlocal hits, misses

        # Passing `cachebox__ignore=True` bypasses the cache and
        # calls the function directly.
        if kwds.pop("cachebox__ignore", False):
            return func(*args, **kwds)

        _cache: BaseCacheImpl = cache(args[0]) if cache_is_fn else cache  # type: ignore[arg-type]
        key = _make_key(args, kwds)

        try:
            result = _cache[key]
            hits += 1
            await _call_async_callback(callback, EVENT_HIT, key, result)

            return postprocess(result) if postprocess is not None else result
        except KeyError:
            pass

        result = await func(*args, **kwds)
        _cache.insert(key, result)
        hits += 1
        await _call_async_callback(callback, EVENT_MISS, key, result)

        return postprocess(result) if postprocess is not None else result

    if not cache_is_fn:
        _wrapped.cache = cache  # type: ignore[attr-defined]
        _wrapped.cache_info = lambda: _create_cache_info(cache, hits, misses)  # type: ignore

        def cache_clear() -> None:
            nonlocal hits, misses
            cache.clear(reuse=clear_reuse)  # type: ignore[union-attr]
            hits = 0
            misses = 0

        _wrapped.cache_clear = cache_clear  # type: ignore[attr-defined]

    _wrapped.callback = callback  # type: ignore[attr-defined]
    return _wrapped


def _cached_wrapper(
    func,
    cache: BaseCacheImpl | typing.Callable,
    key_maker: typing.Callable[[tuple, dict], typing.Hashable],
    clear_reuse: bool,
    callback: typing.Callable[[int, typing.Any, typing.Any], None] | None,
    postprocess: _PostProcess | None,
    lock_type: typing.Type[AbstractContextManager],
):
    cache_is_fn = callable(cache)

    # Per-instance caches receive `self` as args[0]; exclude it from the key
    _make_key = (
        (lambda a, k: key_maker(*a[1:], **k))
        if cache_is_fn
        else (lambda a, k: key_maker(*a, **k))
    )

    hits = 0
    misses = 0

    locks: dict[typing.Hashable, _Lock] = {}
    pending_errors: dict[typing.Hashable, BaseException] = {}

    def _wrapped(*args, **kwds):
        nonlocal hits, misses

        # Passing `cachebox__ignore=True` bypasses the cache and
        # calls the function directly.
        if kwds.pop("cachebox__ignore", False):
            return func(*args, **kwds)

        _cache: BaseCacheImpl = cache(args[0]) if cache_is_fn else cache  # type: ignore[arg-type]
        key = _make_key(args, kwds)

        # Most calls are expected to hit the cache; avoid acquiring a lock.
        # Implementations are thread-safe.
        try:
            result = _cache[key]
            hits += 1
            if callback is not None:
                callback(EVENT_HIT, key, result)

            return postprocess(result) if postprocess is not None else result
        except KeyError:
            pass

        lock = locks.get(key)
        if lock is None:
            locks[key] = lock = _Lock(lock_type())

        # Acquire the per-key lock so that only one task computes the value
        # while the rest wait.
        with lock:
            # Re-raise any exception stored by a previous owner so that all
            # waiters fail with the same error.
            err = pending_errors.get(key)
            if err is not None:
                if lock.waiters == 0:
                    del pending_errors[key]

                raise err

            # Re-check the cache; a previous waiter may have already populated
            # it while we were waiting for the lock.
            try:
                result = _cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = func(*args, **kwds)
                except Exception as exc:
                    if lock.waiters > 0:
                        pending_errors[key] = exc
                    raise
                else:
                    _cache[key] = result
                    misses += 1
                    event = EVENT_MISS

            if lock.waiters == 0:
                locks.pop(key, None)

        if callback is not None:
            callback(event, key, result)

        return postprocess(result) if postprocess is not None else result

    if not cache_is_fn:
        _wrapped.cache = cache  # type: ignore[attr-defined]
        _wrapped.cache_info = lambda: _create_cache_info(cache, hits, misses)  # type: ignore

        def cache_clear() -> None:
            nonlocal hits, misses
            cache.clear(reuse=clear_reuse)  # type: ignore[union-attr]
            hits = misses = 0
            locks.clear()
            pending_errors.clear()

        _wrapped.cache_clear = cache_clear  # type: ignore[attr-defined]

    _wrapped.callback = callback  # type: ignore[attr-defined]
    return _wrapped


def _async_cached_wrapper(
    func,
    cache: BaseCacheImpl | typing.Callable,
    key_maker: typing.Callable[..., typing.Hashable],
    clear_reuse: bool,
    callback: _Callback | None,
    postprocess: _PostProcess | None,
    lock_type: typing.Type[AbstractAsyncContextManager],
):
    cache_is_fn = callable(cache)
    _make_key = (
        (lambda a, k: key_maker(*a[1:], **k))
        if cache_is_fn
        else (lambda a, k: key_maker(*a, **k))
    )

    hits = 0
    misses = 0
    locks: dict[typing.Hashable, _AsyncLock] = {}
    pending_errors: dict[typing.Hashable, BaseException] = {}

    async def _wrapped(*args, **kwds):
        nonlocal hits, misses

        # Passing `cachebox__ignore=True` bypasses the cache and
        # calls the function directly.
        if kwds.pop("cachebox__ignore", False):
            return await func(*args, **kwds)

        _cache: BaseCacheImpl = cache(args[0]) if cache_is_fn else cache  # type: ignore[arg-type]
        key = _make_key(args, kwds)

        try:
            result = _cache[key]
            hits += 1
            await _call_async_callback(callback, EVENT_HIT, key, result)

            return postprocess(result) if postprocess is not None else result
        except KeyError:
            pass

        lock = locks.get(key)
        if lock is None:
            locks[key] = lock = _AsyncLock(lock_type())

        async with lock:
            err = pending_errors.get(key)
            if err is not None:
                if lock.waiters == 0:
                    del pending_errors[key]

                raise err

            try:
                result = _cache[key]
                hits += 1
                event = EVENT_HIT
            except KeyError:
                try:
                    result = await func(*args, **kwds)
                except Exception as exc:
                    if lock.waiters > 0:
                        pending_errors[key] = exc
                    raise
                else:
                    _cache[key] = result
                    misses += 1
                    event = EVENT_MISS

            if lock.waiters == 0:
                locks.pop(key, None)

        await _call_async_callback(callback, event, key, result)

        return postprocess(result) if postprocess is not None else result

    if not cache_is_fn:
        _wrapped.cache = cache  # type: ignore[attr-defined]
        _wrapped.cache_info = lambda: _create_cache_info(cache, hits, misses)  # type: ignore

        def cache_clear() -> None:
            nonlocal hits, misses
            cache.clear(reuse=clear_reuse)  # type: ignore[union-attr]
            hits = misses = 0
            locks.clear()
            pending_errors.clear()

        _wrapped.cache_clear = cache_clear  # type: ignore[attr-defined]

    _wrapped.callback = callback  # type: ignore[attr-defined]
    return _wrapped
