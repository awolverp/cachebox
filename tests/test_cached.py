import unittest
import cachebox
from cachebox import utils
import time


class TestCached(unittest.TestCase):
    def test_cached(self):
        obj = cachebox.LRUCache(3) # type: cachebox.LRUCache[int, int]

        @cachebox.cached(obj, info=False)
        def func(a, b, c):
            return a + b + c

        perf_1 = time.perf_counter()
        func(1, 2, 3)
        perf_1 = time.perf_counter() - perf_1

        perf_2 = time.perf_counter()
        func(1, 3, 2)
        perf_2 = time.perf_counter() - perf_2

        self.assertGreater(perf_1, perf_2)

        @cachebox.cached(obj, info=True)
        def func(a, b, c):
            return a + b + c

        self.assertIs(obj, func.cache)
        self.assertEqual(func.cache_info().length, 1)

        func.cache_clear()

        self.assertEqual(func.cache_info().length, 0)

    def test_key_makers(self):
        @cachebox.cached(cachebox.LRUCache(1), key_maker=utils.make_key, info=False)
        def func(a, b, c, d):
            return a + b + c + d

        perf_1 = time.perf_counter()
        func(1, 2, c=2, d=4)
        perf_1 = time.perf_counter() - perf_1

        perf_2 = time.perf_counter()
        func(1, 2, d=4, c=2)
        perf_2 = time.perf_counter() - perf_2

        self.assertGreater(perf_1, perf_2)

        @cachebox.cached(cachebox.FIFOCache(1), key_maker=utils.make_hash_key, info=False)
        def func(a, b, c, d):
            return a + b + c + d

        perf_1 = time.perf_counter()
        func(1, 2, c=2, d=4)
        perf_1 = time.perf_counter() - perf_1

        perf_2 = time.perf_counter()
        func(1, 2, d=4, c=2)
        perf_2 = time.perf_counter() - perf_2

        self.assertGreater(perf_1, perf_2)

        @cachebox.cached(cachebox.LFUCache(1), key_maker=utils.make_typed_key, info=False)
        def func(a, b, c, d):
            return a + b + c + d

        perf_1 = time.perf_counter()
        func(1, 2, c=2, d=4)
        perf_1 = time.perf_counter() - perf_1

        perf_2 = time.perf_counter()
        func(1, 2, d=4, c=2)
        perf_2 = time.perf_counter() - perf_2

        self.assertGreater(perf_1, perf_2)


class TestAsyncCached(unittest.IsolatedAsyncioTestCase):
    async def test_async_cached(self):
        obj = cachebox.LRUCache(3)

        @cachebox.cached(obj, info=False)
        async def func(a, b, c):
            return a + b + c

        perf_1 = time.perf_counter()
        await func(1, 2, 3)
        perf_1 = time.perf_counter() - perf_1

        perf_2 = time.perf_counter()
        await func(1, 3, 2)
        perf_2 = time.perf_counter() - perf_2

        self.assertGreater(perf_1, perf_2)

        @cachebox.cached(obj, info=True)
        async def func(a, b, c):
            return a + b + c

        self.assertIs(obj, func.cache)
        self.assertEqual(func.cache_info().length, 1)

        func.cache_clear()

        self.assertEqual(func.cache_info().length, 0)
