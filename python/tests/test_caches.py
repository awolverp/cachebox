from cachebox import (
    Cache,
    FIFOCache,
)
import pytest
from .mixin import _TestMixin


class TestCache(_TestMixin):
    CACHE = Cache
    NO_POLICY = True

    def test_pickle(self):
        self._test_pickle(lambda c1, c2: None)


class TestFIFOCache(_TestMixin):
    CACHE = FIFOCache

    def test_policy(self):
        cache = FIFOCache(5)

        cache[0] = 0
        cache[1] = 1
        cache[2] = 2

        assert cache[0] == 0
        assert cache[1] == 1

        assert cache.popitem() == (0, 0)

        cache[3] = 3

        assert cache.popitem() == (1, 1)
        assert cache.popitem() == (2, 2)
        assert cache.popitem() == (3, 3)

        with pytest.raises(KeyError):
            cache.popitem()

        for i in range(5):
            cache[i] = i

        for i in range(5):
            assert i in cache

        cache[10] = 10

        assert 0 not in cache
        assert 10 in cache

        assert cache.popitem() == (1, 1)

        del cache[2]
        del cache[3]
        del cache[4]

        assert cache.popitem() == (10, 10)

    def test_ordered_iterators(self):
        obj = self.CACHE(100, **self.KWARGS, capacity=100)

        for i in range(6):
            obj[i] = i * 2

        k = list(range(6))
        v = list(i * 2 for i in range(6))
        assert k == list(obj.keys())
        assert v == list(obj.values())
        assert list(zip(k, v)) == list(obj.items())

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        self._test_pickle(inner)

    def test_first_last(self):
        obj = self.CACHE(5, **self.KWARGS, capacity=5)

        for i in range(5):
            obj[i] = i * 2

        assert obj.first() == 0
        assert obj.last() == 4

        obj[10] = 20

        assert obj.first() == 1
        assert obj.last() == 10
