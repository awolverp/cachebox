import asyncio
import time
import typing

import pytest

import cachebox


@pytest.fixture(
    scope="function",
    params=[
        cachebox.Cache,
        cachebox.FIFOCache,
        cachebox.LFUCache,
        cachebox.LRUCache,
        cachebox.TTLCache,
        cachebox.RRCache,
        cachebox.VTTLCache,
    ],
)
def random_cache_impl(request):
    typ: typing.Type[cachebox.BaseCacheImpl] = request.param

    def inner(maxsize, iterable=None):
        if typ is cachebox.TTLCache:
            return typ(maxsize, global_ttl=10, iterable=iterable)

        if typ is cachebox.VTTLCache:
            return typ(maxsize, ttl=10, iterable=iterable)

        return typ(maxsize, iterable=iterable)

    return inner


def test_frozen(random_cache_impl: type[cachebox.BaseCacheImpl]):
    cache = random_cache_impl(10, {i: i for i in range(8)})
    f = cachebox.Frozen(cache)

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

    f = cachebox.Frozen(cache, ignore=True)
    f.popitem()


def test_cached(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(3)

    @cachebox.cached(obj)
    def factorial(n: int):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        time.sleep(0.1)
        return fact

    perf_1 = time.perf_counter()
    factorial(15)
    perf_1 = time.perf_counter() - perf_1

    assert cachebox.get_cached_cache_info(factorial).length == 1
    assert cachebox.get_cached_cache_info(factorial).misses == 1

    perf_2 = time.perf_counter()
    factorial(15)
    perf_2 = time.perf_counter() - perf_2

    assert perf_1 > perf_2
    assert cachebox.get_cached_cache_info(factorial).hits == 1

    cachebox.clear_cached_cache(factorial)
    assert cachebox.get_cached_cache_info(factorial).hits == 0
    assert cachebox.get_cached_cache_info(factorial).misses == 0

    perf_3 = time.perf_counter()
    factorial(15)
    perf_3 = time.perf_counter() - perf_3
    assert perf_3 > perf_2

    # test cachebox__ignore
    cachebox.clear_cached_cache(factorial)
    assert len(cachebox.get_cached_cache(factorial)) == 0
    factorial(15, cachebox__ignore=True)  # type: ignore
    assert len(cachebox.get_cached_cache(factorial)) == 0


def test_key_makers(random_cache_impl: type[cachebox.BaseCacheImpl]):
    @cachebox.cached(random_cache_impl(125), key_maker=cachebox.make_key)
    def func_1(a, b, c):
        return a, b, c

    func_1(1, 2, 3)
    func_1(1.0, 2, 3.0)
    func_1(3, 2, 1)

    assert len(cachebox.get_cached_cache(func_1)) == 2

    @cachebox.cached(random_cache_impl(125), key_maker=cachebox.make_typed_key)
    def func_2(a, b, c):
        return a, b, c

    func_2(1, 2, 3)
    func_2(1.0, 2, 3.0)
    func_2(3, 2, 1)

    assert len(cachebox.get_cached_cache(func_2)) == 3


@pytest.mark.asyncio
async def test_async_cached(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(3)

    @cachebox.cached(obj)
    async def factorial(n: int, _: str):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        await asyncio.sleep(0.1)  # need for testing
        return fact

    perf_1 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_1 = time.perf_counter() - perf_1

    assert cachebox.get_cached_cache_info(factorial).length == 1
    assert cachebox.get_cached_cache_info(factorial).misses == 1

    perf_2 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_2 = time.perf_counter() - perf_2

    assert perf_1 > perf_2
    assert cachebox.get_cached_cache_info(factorial).hits == 1

    cachebox.clear_cached_cache(factorial)
    assert cachebox.get_cached_cache_info(factorial).hits == 0
    assert cachebox.get_cached_cache_info(factorial).misses == 0

    perf_3 = time.perf_counter()
    await factorial(15, "cachebox")
    perf_3 = time.perf_counter() - perf_3
    assert perf_3 > perf_2

    # test cachebox__ignore
    cachebox.clear_cached_cache(factorial)
    assert len(cachebox.get_cached_cache(factorial)) == 0
    await factorial(15, "me", cachebox__ignore=True)  # type: ignore
    assert len(cachebox.get_cached_cache(factorial)) == 0


def test_cachedmethod():
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cachebox.cached(None)
        def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert cls.method("a") == ("a" * 10)

    cls = TestCachedMethod(2)
    assert cls.method("a") == ("a" * 2)


def test_callback(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(3)

    called = list()

    @cachebox.cached(
        obj,
        key_maker=lambda n: n,
        callback=lambda event, key, value: called.append((event, key, value)),
    )
    def factorial(n: int, /):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        return fact

    assert factorial(5) == 120
    assert len(called) == 1
    assert called[0] == (cachebox.EVENT_MISS, 5, 120)

    assert factorial(5) == 120
    assert len(called) == 2
    assert called[1] == (cachebox.EVENT_HIT, 5, 120)

    assert factorial(3) == 6
    assert len(called) == 3
    assert called[2] == (cachebox.EVENT_MISS, 3, 6)

    assert cachebox.is_cached(factorial)


@pytest.mark.asyncio
async def test_async_cachedmethod(random_cache_impl: type[cachebox.BaseCacheImpl]):
    class TestCachedMethod:
        def __init__(self, num) -> None:
            self.num = num

        @cachebox.cached(random_cache_impl(0))
        async def method(self, char: str):
            assert type(self) is TestCachedMethod
            return char * self.num

    cls = TestCachedMethod(10)
    assert (await cls.method("a")) == ("a" * 10)


@pytest.mark.asyncio
async def test_async_callback(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(3)

    called = list()

    async def _callback(event, key, value):
        called.append((event, key, value))

    @cachebox.cached(obj, key_maker=lambda n: n, callback=_callback)
    async def factorial(n: int, /):
        fact = 1
        for num in range(2, n + 1):
            fact *= num

        return fact

    assert await factorial(5) == 120
    assert len(called) == 1
    assert called[0] == (cachebox.EVENT_MISS, 5, 120)

    assert await factorial(5) == 120
    assert len(called) == 2
    assert called[1] == (cachebox.EVENT_HIT, 5, 120)

    assert await factorial(3) == 6
    assert len(called) == 3
    assert called[2] == (cachebox.EVENT_MISS, 3, 6)

    assert cachebox.is_cached(factorial)
    assert not cachebox.is_cached(_callback)


def test_classmethod():
    class MyClass:
        def __init__(self, num: int) -> None:
            self.num = num

        @classmethod
        @cachebox.cached(None, postprocess=cachebox.postprocess_copy)
        def new(cls, num: int):
            return cls(num)

    a = MyClass.new(1)
    assert isinstance(a, MyClass) and a.num == 1


def test_staticmethod():
    class MyClass:
        def __init__(self, num: int) -> None:
            self.num = num

        @staticmethod
        @cachebox.cached(None, postprocess=cachebox.postprocess_copy)
        def new(num: int):
            return num

    a = MyClass.new(1)
    assert isinstance(a, int) and a == 1


def test_cached_method(random_cache_impl: type[cachebox.BaseCacheImpl]):
    class Test:
        def __init__(self, num) -> None:
            self.num = num
            self._cache = random_cache_impl(20)

        @cachebox.cached(lambda self: self._cache)
        def method(self, char: str):
            assert type(self) is Test
            return char * self.num

    for i in range(10):
        cls = Test(i)
        assert cls.method("a") == ("a" * i)


def test_nested_cached_shared_cache(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(10)

    @cachebox.cached(obj, key_maker=cachebox.make_typed_key)
    def func_inner(a: int, b: int):
        return a + b

    @cachebox.cached(
        obj,
        # `key_maker`s should be different
        key_maker=cachebox.make_key,
    )
    def func_outer(a: int, b: int):
        return f"{a} + {b} = {func_inner(a, b)}"

    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(1, 2) == "1 + 2 = 3"
    assert func_outer(2, 3) == "2 + 3 = 5"
    assert func_outer(a=2, b=3) == "2 + 3 = 5"


def test_recursive_cached(random_cache_impl: type[cachebox.BaseCacheImpl]):
    obj = random_cache_impl(10)

    @cachebox.cached(obj)
    def factorial(n):
        if n < 0:
            raise ValueError
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

    obj = cachebox.LRUCache(10)

    @cachebox.cached(obj)
    def factorial(n):
        if n < 0:
            raise ValueError
        if n == 0 or n == 1:
            return 1
        else:
            return n * factorial(n - 1)

    threads = list(
        map(
            lambda x: x.start() or x,
            (
                threading.Thread(target=factorial, args=(10,), name=str(i))
                for i in range(10)
            ),
        )
    )
    for t in threads:
        t.join(timeout=60)


@pytest.mark.asyncio
async def test_recursive_asyncio_cached():
    obj = cachebox.LRUCache(10)

    @cachebox.cached(obj)
    async def factorial(n) -> int:
        if n < 0:
            raise ValueError
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
