import typing

import cachebox

from . import mixins


class TestCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.Cache:
        return cachebox.Cache(maxsize, iterable, capacity=capacity, getsizeof=getsizeof)


class TestFIFOCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.FIFOCache:
        return cachebox.FIFOCache(
            maxsize, iterable, capacity=capacity, getsizeof=getsizeof
        )


class TestRRCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.RRCache:
        return cachebox.RRCache(
            maxsize, iterable, capacity=capacity, getsizeof=getsizeof
        )


class TestLRUCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.LRUCache:
        return cachebox.LRUCache(
            maxsize, iterable, capacity=capacity, getsizeof=getsizeof
        )


class TestLFUCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.LFUCache:
        return cachebox.LFUCache(
            maxsize, iterable, capacity=capacity, getsizeof=getsizeof
        )


class TestTTLCache(mixins.BenchmarkMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.TTLCache:
        return cachebox.TTLCache(
            maxsize, 10, iterable, capacity=capacity, getsizeof=getsizeof
        )
