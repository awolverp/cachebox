import asyncio
import platform
import threading
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


class TestFrozen:
    def test_init(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
        cache = random_cache_impl(10, {i: i for i in range(8)})
        f = cachebox.Frozen(cache)

        assert f.maxsize == cache.maxsize

    def test_try_to_mutate(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
        cache = random_cache_impl(10, {i: i for i in range(8)})
        f = cachebox.Frozen(cache)

        with pytest.raises(TypeError):
            f[0] = 0

        with pytest.raises(TypeError):
            f.pop(0)

        with pytest.raises(TypeError):
            f.popitem()

    def test_changing_inner(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
        cache = random_cache_impl(10, {i: i for i in range(8)})
        f = cachebox.Frozen(cache)

        assert len(f) == 8
        assert len(f) == len(cache)
        cache.insert(9, 9)
        assert len(f) == 9
        assert len(f) == len(cache)

    def test_try_to_mutate_ignore(
        self,
        random_cache_impl: type[cachebox.BaseCacheImpl],
    ):
        cache = random_cache_impl(10, {i: i for i in range(8)})
        f = cachebox.Frozen(cache, ignore=True)

        f.popitem()
        f.pop(0)
        f[0] = 0

        assert f.cache == cache


class TestCachedCache:
    def test_different_impls(
        self,
        random_cache_impl: type[cachebox.BaseCacheImpl],
    ):
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

    def test_other_than_impls(self):
        @cachebox.cached(None)
        def wrapped_1():
            pass

        assert isinstance(cachebox.get_cached_cache(wrapped_1), cachebox.LRUCache)

        @cachebox.cached({})
        def wrapped_2():
            pass

        assert isinstance(cachebox.get_cached_cache(wrapped_2), cachebox.LRUCache)

        with pytest.raises(TypeError):
            cachebox.cached(set())  # type: ignore

    @pytest.mark.asyncio
    async def test_different_impls_async_mode(
        self,
        random_cache_impl: type[cachebox.BaseCacheImpl],
    ):
        obj = random_cache_impl(3)

        @cachebox.cached(obj)
        async def factorial(n: int):
            fact = 1
            for num in range(2, n + 1):
                fact *= num

            time.sleep(0.1)
            return fact

        perf_1 = time.perf_counter()
        await factorial(15)
        perf_1 = time.perf_counter() - perf_1

        assert cachebox.get_cached_cache_info(factorial).length == 1
        assert cachebox.get_cached_cache_info(factorial).misses == 1

        perf_2 = time.perf_counter()
        await factorial(15)
        perf_2 = time.perf_counter() - perf_2

        assert perf_1 > perf_2
        assert cachebox.get_cached_cache_info(factorial).hits == 1

        cachebox.clear_cached_cache(factorial)
        assert cachebox.get_cached_cache_info(factorial).hits == 0
        assert cachebox.get_cached_cache_info(factorial).misses == 0

        perf_3 = time.perf_counter()
        await factorial(15)
        perf_3 = time.perf_counter() - perf_3
        assert perf_3 > perf_2

        # test cachebox__ignore
        cachebox.clear_cached_cache(factorial)
        assert len(cachebox.get_cached_cache(factorial)) == 0
        await factorial(15, cachebox__ignore=True)  # type: ignore
        assert len(cachebox.get_cached_cache(factorial)) == 0


class TestCachedKeyMaker:
    def test_valid(self):
        @cachebox.cached(key_maker=lambda a, b, c: a + b + c)
        def func_1(a: int, b: int, c: int):
            return a, b, c

        func_1(1, 1, 1)
        assert 3 in cachebox.get_cached_cache(func_1)

    def test_invalid(self):
        # invalid key_maker
        @cachebox.cached(key_maker=lambda a: a)
        def func_2(a: int, b: int, c: int):
            return a, b, c

        with pytest.raises(TypeError):
            func_2(1, 1, 1)

    def test_ready_to_uses(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
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

        @cachebox.cached(random_cache_impl(125), key_maker=cachebox.make_hash_key)
        def func_3(a, b, c):
            return a, b, c

        func_3(1, 2, 3)
        func_3(1.0, 2, 3.0)
        func_3(3, 2, 1)

        assert len(cachebox.get_cached_cache(func_3)) == 2


class TestCachedCallback:
    def test_sync(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
        called = list()

        @cachebox.cached(
            random_cache_impl(3),
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
    async def test_async(self, random_cache_impl: type[cachebox.BaseCacheImpl]):
        called = list()

        async def callback(event, key, value):
            called.append((event, key, value))

        # Should raise TypeError: For sync functions, you cannot use an asynchronous callback.
        with pytest.raises(TypeError):

            @cachebox.cached(callback=callback)
            def invalid_callback():
                pass

        @cachebox.cached(random_cache_impl(3), key_maker=lambda n: n, callback=callback)
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


class TestCachedPostProcess:
    def test_disabled_postprocess(self):
        @cachebox.cached(postprocess=None)
        def disabled(a: int, b: int):
            # Returns dict, which is mutable
            return {a: b}

        obj = disabled(1, 2)
        assert obj == {1: 2}

        obj[10] = 2

        newobj = disabled(1, 2)
        assert obj == newobj
        assert newobj == {1: 2, 10: 2}

    def test_ready_to_uses(self):
        @cachebox.cached(postprocess=cachebox.postprocess_copy_mutables)
        def copy_mutables_func(a: int, b: int):
            # Returns dict, which is mutable
            return {a: b}

        obj = copy_mutables_func(1, 2)
        assert obj == {1: 2}

        obj[10] = 2

        newobj = copy_mutables_func(1, 2)
        assert obj != newobj
        assert newobj == {1: 2}

        @cachebox.cached(postprocess=cachebox.postprocess_copy)
        def copy_func(a: int, b: int):
            # Returns dict, which is mutable
            return [{a: b}]

        obj = copy_func(1, 2)
        assert obj == [{1: 2}]

        obj.append({})

        newobj = copy_func(1, 2)
        assert newobj == [{1: 2}]

        obj[0][10] = 2
        assert newobj[0] == {1: 2, 10: 2}

        @cachebox.cached(postprocess=cachebox.postprocess_deepcopy)
        def deepcopy_func(a: int, b: int):
            # Returns dict, which is mutable
            return [{a: b}]

        obj = deepcopy_func(1, 2)
        assert obj == [{1: 2}]

        obj.append({})

        newobj = deepcopy_func(1, 2)
        assert newobj == [{1: 2}]

        obj[0][10] = 2
        assert newobj[0] == {1: 2}  # Should be still OK

    def test_customs(self):
        @cachebox.cached(postprocess=lambda x: "Hello, " + x)
        def sayhello(name: str):
            return name

        assert sayhello("Ali") == "Hello, Ali"
        assert sayhello("Ali") == "Hello, Ali"

        assert cachebox.get_cached_cache(sayhello).get("Ali") == "Ali"


class TestCachedMethods:
    def test_instance_methods_with_per_instance_cache(self):
        class TestCachedMethod:
            def __init__(self, num) -> None:
                self.num = num
                self._cache = cachebox.Cache(0)

            @cachebox.cached(lambda x: x._cache)
            def method(self, char: str):
                assert type(self) is TestCachedMethod
                return char * self.num

        cls = TestCachedMethod(10)
        assert cls.method("a") == ("a" * 10)

        cls = TestCachedMethod(2)
        assert cls.method("a") == ("a" * 2)

    def test_instance_methods_with_global_cache(self):
        class TestCachedMethod:
            _cache = cachebox.Cache(0)

            def __init__(self, num) -> None:
                self.num = num

            @cachebox.cached(lambda x: x._cache)
            def method(self, char: str):
                assert type(self) is TestCachedMethod
                return char * self.num

        cls = TestCachedMethod(10)
        assert cls.method("a") == ("a" * 10)

        cls = TestCachedMethod(2)
        assert cls.method("a") == ("a" * 10)

    @pytest.mark.asyncio
    async def test_async_instance_methods_with_per_instance_cache(self):
        class TestCachedMethod:
            def __init__(self, num) -> None:
                self.num = num
                self._cache = cachebox.Cache(0)

            @cachebox.cached(lambda x: x._cache)
            async def method(self, char: str):
                assert type(self) is TestCachedMethod
                return char * self.num

        cls = TestCachedMethod(10)
        assert await cls.method("a") == ("a" * 10)

        cls = TestCachedMethod(2)
        assert await cls.method("a") == ("a" * 2)

    @pytest.mark.asyncio
    async def test_async_instance_methods_with_global_cache(self):
        class TestCachedMethod:
            _cache = cachebox.Cache(0)

            def __init__(self, num) -> None:
                self.num = num

            @cachebox.cached(lambda x: x._cache)
            async def method(self, char: str):
                assert type(self) is TestCachedMethod
                return char * self.num

        cls = TestCachedMethod(10)
        assert await cls.method("a") == ("a" * 10)

        cls = TestCachedMethod(2)
        assert await cls.method("a") == ("a" * 10)

    def test_classmethod(self):
        class MyClass:
            counter = 0

            def __init__(self, num: int) -> None:
                self.num = num

            @classmethod
            @cachebox.cached(None, postprocess=cachebox.postprocess_copy)
            def new(cls, num: int):
                cls.counter += 1
                return cls(num)

        a = MyClass.new(1)
        assert isinstance(a, MyClass) and a.num == 1

        b = MyClass.new(1)
        assert a is not b and a.num == 1  # because of cachebox.postprocess_copy

        assert MyClass.counter == 1

    @pytest.mark.asyncio
    async def test_async_classmethod(self):

        class MyClass:
            counter = 0

            def __init__(self, num: int) -> None:
                self.num = num

            @classmethod
            @cachebox.cached(None, postprocess=cachebox.postprocess_copy)
            async def new(cls, num: int):
                cls.counter += 1
                return cls(num)

        a = await MyClass.new(1)
        assert isinstance(a, MyClass) and a.num == 1

        b = await MyClass.new(1)
        assert a is not b and a.num == 1  # because of cachebox.postprocess_copy

        assert MyClass.counter == 1

    def test_staticmethod(self):
        class MyClass:
            counter = 0

            @staticmethod
            @cachebox.cached(None)
            def add(a: int, b: int):
                MyClass.counter += 1
                return a + b

        a = MyClass.add(2, 3)
        assert a == 5

        b = MyClass.add(2, 3)
        assert b == 5

        assert MyClass.counter == 1

    @pytest.mark.asyncio
    async def test_async_staticmethod(self):
        class MyClass:
            counter = 0

            @staticmethod
            @cachebox.cached(None)
            async def add(a: int, b: int):
                MyClass.counter += 1
                return a + b

        a = await MyClass.add(2, 3)
        assert a == 5

        b = await MyClass.add(2, 3)
        assert b == 5

        assert MyClass.counter == 1


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


def _run_recursive_cached_func_with_thread(cached_func, key):
    result = {}

    def run():
        try:
            result["v"] = cached_func(key)
        except Exception as e:
            result["err"] = e

    t = threading.Thread(target=run, daemon=True)
    t.start()
    t.join(timeout=2.0)

    if t.is_alive():
        pytest.fail("deadlock happend - thread hung waiting on its own lock")

    assert "err" in result


@pytest.mark.skipif(
    platform.python_implementation() == "PyPy",
    reason="https://github.com/PyO3/pyo3/issues/6109",
)
def test_recursive_cached_issue_54(random_cache_impl: type[cachebox.BaseCacheImpl]):
    # https://github.com/awolverp/cachebox/issues/54

    @cachebox.cached(random_cache_impl(10), lock=None)
    def without_lock(key):
        return without_lock(key)

    _run_recursive_cached_func_with_thread(without_lock, "same-key")

    @cachebox.cached(random_cache_impl(10), lock=threading.RLock)
    def with_rlock(key):
        return with_rlock(key)

    _run_recursive_cached_func_with_thread(with_rlock, "same-key")


async def _run_recursive_cached_func_with_asyncio(cached_func, key):
    result = {}

    async def run():
        try:
            result["v"] = await cached_func(key)
        except Exception as e:
            result["err"] = e

    t = asyncio.create_task(run())

    try:
        await asyncio.wait_for(t, timeout=2.0)
    except TimeoutError:
        pytest.fail("deadlock happend - task hung waiting on its own lock")

    assert "err" in result


@pytest.mark.skipif(
    platform.python_implementation() == "PyPy",
    reason="https://github.com/PyO3/pyo3/issues/6109",
)
@pytest.mark.asyncio
async def test_async_recursive_cached_issue_54(
    random_cache_impl: type[cachebox.BaseCacheImpl],
):
    @cachebox.cached(random_cache_impl(10), lock=None)
    async def without_lock(key):
        return await without_lock(key)

    await _run_recursive_cached_func_with_asyncio(without_lock, "same-key")


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


def test_handling_pending_errors():
    # https://github.com/awolverp/cachebox/issues/57

    def key(a, b, c, exception: bool):
        return f"{a},{b},{c}"

    @cachebox.cached(cachebox.TTLCache(1024, 1), key_maker=key)
    def calc(a, b, c, exception: bool):
        if exception:
            raise ValueError("first call")

        return a + b + c

    with pytest.raises(ValueError):
        calc(1, 2, 3, exception=True)

    assert calc(1, 2, 3, exception=False) == 6


@pytest.mark.asyncio
async def test_async_handling_pending_errors():
    # https://github.com/awolverp/cachebox/issues/57

    def key(a, b, c, exception: bool):
        return f"{a},{b},{c}"

    @cachebox.cached(cachebox.TTLCache(1024, 1), key_maker=key)
    async def calc(a, b, c, exception: bool):
        if exception:
            raise ValueError("first call")

        return a + b + c

    with pytest.raises(ValueError):
        await calc(1, 2, 3, exception=True)

    assert await calc(1, 2, 3, exception=False) == 6
