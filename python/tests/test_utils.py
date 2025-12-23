from cachebox import (
    Frozen,
    LRUCache,
    BaseCacheImpl,
    cached,
    make_typed_key,
    make_key,
    EVENT_HIT,
    EVENT_MISS,
    is_cached,
)
import asyncio
import pytest
import time


def test_frozen(random_cache_impl: type[BaseCacheImpl]):
    cache = random_cache_impl(10, {i: i for i in range(8)})
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

    f = Frozen(cache, ignore=True)
    f.popitem()


def test_cached(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(3)

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


def test_key_makers(random_cache_impl: type[BaseCacheImpl]):
    @cached(random_cache_impl(125), key_maker=make_key)
    def func(a, b, c):
        return a, b, c

    func(1, 2, 3)
    func(1.0, 2, 3.0)
    func(3, 2, 1)

    assert len(func.cache) == 2

    @cached(random_cache_impl(125), key_maker=make_typed_key)
    def func(a, b, c):
        return a, b, c

    func(1, 2, 3)
    func(1.0, 2, 3.0)
    func(3, 2, 1)

    assert len(func.cache) == 3


@pytest.mark.asyncio
async def test_async_cached(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(3)

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


def test_cachedmethod():
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cached(None)
        def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert cls.method("a") == ("a" * 10)

    cls = TestCachedMethod(2)
    assert cls.method("a") == ("a" * 2)


@pytest.mark.asyncio
async def test_async_cachedmethod(random_cache_impl: type[BaseCacheImpl]):
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cached(random_cache_impl(0))
        async def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert (await cls.method("a")) == ("a" * 10)


def test_callback(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(3)

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


async def _test_async_callback(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(3)

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


def test_async_callback(random_cache_impl: type[BaseCacheImpl]):
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()

    loop.run_until_complete(_test_async_callback(random_cache_impl))


def test_copy_level(random_cache_impl: type[BaseCacheImpl]):
    class A:
        def __init__(self, c: int) -> None:
            self.c = c

    @cached(random_cache_impl(0))
    def func(c: int) -> A:
        return A(c)

    result = func(1)
    assert result.c == 1
    result.c = 2

    result = func(1)
    assert result.c == 2  # !!!

    @cached(random_cache_impl(0), copy_level=2)
    def func(c: int) -> A:
        return A(c)

    result = func(1)
    assert result.c == 1
    result.c = 2

    result = func(1)
    assert result.c == 1  # :)


def test_classmethod():
    class MyClass:
        def __init__(self, num: int) -> None:
            self.num = num

        @classmethod
        @cached(None, copy_level=2)
        def new(cls, num: int):
            return cls(num)

    a = MyClass.new(1)
    assert isinstance(a, MyClass) and a.num == 1


def test_staticmethod():
    class MyClass:
        def __init__(self, num: int) -> None:
            self.num = num

        @staticmethod
        @cached(None, copy_level=2)
        def new(num: int):
            return num

    a = MyClass.new(1)
    assert isinstance(a, int) and a == 1


def test_new_cached_method(random_cache_impl: type[BaseCacheImpl]):
    class Test:
        def __init__(self, num) -> None:
            self.num = num
            self._cache = random_cache_impl(20)

        @cached(lambda self: self._cache)
        def method(self, char: str):
            assert type(self) is Test
            return char * self.num

    for i in range(10):
        cls = Test(i)
        assert cls.method("a") == ("a" * i)


def test_nested_cached_shared_cache(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(10)

    @cached(obj, key_maker=make_typed_key)
    def func_inner(a: int, b: int):
        return a + b

    @cached(obj, key_maker=make_key)
    def func_outer(a: int, b: int):
        return f"{a} + {b} = {func_inner(a, b)}"

    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(2, 3) == "2 + 3 = 5"
    assert func_outer(a=2, b=3) == "2 + 3 = 5"


def test_recursive_cached(random_cache_impl: type[BaseCacheImpl]):
    obj = random_cache_impl(10)

    @cached(obj)
    def factorial(n):
        if n < 0:
            raise ValueError("فاکتوریل برای اعداد منفی تعریف نشده است.")
        if n == 0 or n == 1:
            return 1
        else:
            return n * factorial(n - 1)

    assert factorial(10) == 3628800
    assert factorial(5) == 120
    assert factorial(10) == 3628800
    assert factorial(5) == 120
    assert factorial(10) == 3628800
    assert factorial(2) == 2


def test_recursive_threading_cached():
    import threading

    obj = LRUCache(10)

    @cached(obj)
    def factorial(n):
        if n < 0:
            raise ValueError("فاکتوریل برای اعداد منفی تعریف نشده است.")
        if n == 0 or n == 1:
            return 1
        else:
            return n * factorial(n - 1)

    threads = list(
        map(
            lambda x: x.start() or x,
            (threading.Thread(target=factorial, args=(10,), name=str(i)) for i in range(10)),
        )
    )
    for t in threads:
        t.join(timeout=60)


@pytest.mark.asyncio
async def test_recursive_asyncio_cached():
    obj = LRUCache(10)

    @cached(obj)
    async def factorial(n) -> int:
        if n < 0:
            raise ValueError("فاکتوریل برای اعداد منفی تعریف نشده است.")
        if n == 0 or n == 1:
            return 1
        else:
            return n * (await factorial(n - 1))

    result = await asyncio.wait_for(
        asyncio.gather(
            factorial(10),
            factorial(10),
            factorial(10),
            factorial(10),
            factorial(10),
            factorial(10),
            factorial(10),
            factorial(10),
        ),
        10,
    )
    assert result == ([3628800] * 8)
