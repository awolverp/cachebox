from cachebox import (
    Frozen,
    LRUCache,
    cached,
    make_typed_key,
    make_key,
    cachedmethod,
    EVENT_HIT,
    EVENT_MISS,
    is_cached,
)
import asyncio
import pytest
import time


def test_frozen():
    cache = LRUCache(10, {i: i for i in range(8)})
    f = Frozen(cache)

    assert f.maxsize == cache.maxsize

    with pytest.raises(TypeError):
        f[0] = 0

    with pytest.raises(TypeError):
        f.pop(0)

    with pytest.raises(TypeError):
        f.popitem()

    assert len(f) == 8
    assert len(f) == len(cache)
    cache.insert(9, 9)
    assert len(f) == 9
    assert len(f) == len(cache)


def test_cached():
    obj = LRUCache(3)  # type: LRUCache[int, int]

    @cached(obj)
    def factorial(n):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        time.sleep(0.1)  # need for testing
        return fact

    perf_1 = time.perf_counter()
    factorial(15)
    perf_1 = time.perf_counter() - perf_1

    assert factorial.cache_info().length == 1
    assert factorial.cache_info().misses == 1

    perf_2 = time.perf_counter()
    factorial(15)
    perf_2 = time.perf_counter() - perf_2

    assert perf_1 > perf_2
    assert factorial.cache_info().hits == 1

    factorial.cache_clear()
    assert factorial.cache_info().hits == 0
    assert factorial.cache_info().misses == 0

    perf_3 = time.perf_counter()
    factorial(15)
    perf_3 = time.perf_counter() - perf_3
    assert perf_3 > perf_2

    # test cachebox__ignore
    factorial.cache_clear()
    assert len(factorial.cache) == 0
    factorial(15, cachebox__ignore=True)
    assert len(factorial.cache) == 0


def test_key_makers():
    @cached(LRUCache(125), key_maker=make_key)
    def func(a, b, c):
        return a, b, c

    func(1, 2, 3)
    func(1.0, 2, 3.0)
    func(3, 2, 1)

    assert len(func.cache) == 2

    @cached(LRUCache(125), key_maker=make_typed_key)
    def func(a, b, c):
        return a, b, c

    func(1, 2, 3)
    func(1.0, 2, 3.0)
    func(3, 2, 1)

    assert len(func.cache) == 3


async def _test_async_cached():
    obj = LRUCache(3)  # type: LRUCache[int, int]

    @cached(obj)
    async def factorial(n: int, _: str):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        await asyncio.sleep(0.1)  # need for testing
        return fact

    perf_1 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_1 = time.perf_counter() - perf_1

    assert factorial.cache_info().length == 1
    assert factorial.cache_info().misses == 1

    perf_2 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_2 = time.perf_counter() - perf_2

    assert perf_1 > perf_2
    assert factorial.cache_info().hits == 1

    factorial.cache_clear()
    assert factorial.cache_info().hits == 0
    assert factorial.cache_info().misses == 0

    perf_3 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_3 = time.perf_counter() - perf_3
    assert perf_3 > perf_2

    # test cachebox__ignore
    factorial.cache_clear()
    assert len(factorial.cache) == 0
    await factorial(15, "me", cachebox__ignore=True)
    assert len(factorial.cache) == 0


def test_async_cached():
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()

    loop.run_until_complete(_test_async_cached())


def test_cachedmethod():
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cachedmethod(None)
        def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert cls.method("a") == ("a" * 10)


async def _test_async_cachedmethod():
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cachedmethod(None)
        async def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert (await cls.method("a")) == ("a" * 10)


def test_async_cachedmethod():
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()

    loop.run_until_complete(_test_async_cachedmethod())


def test_callback():
    obj = LRUCache(3)

    called = list()

    @cached(
        obj,
        key_maker=lambda args, _: args[0],
        callback=lambda event, key, value: called.append((event, key, value)),
    )
    def factorial(n: int, /):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        return fact

    assert factorial(5) == 120
    assert len(called) == 1
    assert called[0] == (EVENT_MISS, 5, 120)

    assert factorial(5) == 120
    assert len(called) == 2
    assert called[1] == (EVENT_HIT, 5, 120)

    assert factorial(3) == 6
    assert len(called) == 3
    assert called[2] == (EVENT_MISS, 3, 6)

    assert is_cached(factorial)


async def _test_async_callback():
    obj = LRUCache(3)

    called = list()

    async def _callback(event, key, value):
        called.append((event, key, value))

    @cached(obj, key_maker=lambda args, _: args[0], callback=_callback)
    async def factorial(n: int, /):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        return fact

    assert await factorial(5) == 120
    assert len(called) == 1
    assert called[0] == (EVENT_MISS, 5, 120)

    assert await factorial(5) == 120
    assert len(called) == 2
    assert called[1] == (EVENT_HIT, 5, 120)

    assert await factorial(3) == 6
    assert len(called) == 3
    assert called[2] == (EVENT_MISS, 3, 6)

    assert is_cached(factorial)
    assert not is_cached(_callback)


def test_async_callback():
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()

    loop.run_until_complete(_test_async_callback())
