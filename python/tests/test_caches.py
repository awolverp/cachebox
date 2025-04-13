from cachebox import (
    Cache,
    FIFOCache,
    RRCache,
    LRUCache,
    LFUCache,
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
        assert obj.first(-1) == obj.last()
        assert obj.first(-10000) is None


class TestRRCache(_TestMixin):
    CACHE = RRCache

    def test_pickle(self):
        self._test_pickle(lambda c1, c2: None)


class TestLRUCache(_TestMixin):
    CACHE = LRUCache

    def test_policy(self):
        obj = self.CACHE(3)

        obj[1] = 1
        obj[2] = 2
        obj[3] = 3

        assert (1, 1) == obj.popitem()

        obj[1] = 1
        obj[2]

        assert (3, 3) == obj.popitem()

        obj[4] = 4
        assert 1 == obj.get(1)

        obj[5] = 5
        assert 2 not in obj

    def test_ordered_iterators(self):
        obj = self.CACHE(20, **self.KWARGS, capacity=20)

        for i in range(6):
            obj[i] = i * 2

        obj[1]
        obj[5]
        obj[3] = 7

        k = [0, 2, 4, 1, 5, 3]
        v = [0, 4, 8, 2, 10, 7]
        assert k == list(obj.keys())
        assert v == list(obj.values())
        assert list(zip(k, v)) == list(obj.items())

    def test_recently_used_funcs(self):
        obj = LRUCache(10)

        for i in range(6):
            obj[i] = i * 2

        obj[1]
        obj[5]
        obj[3] = 7
        obj.peek(4)

        assert obj.most_recently_used() == 3
        assert obj.least_recently_used() == 0

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        self._test_pickle(inner)


class TestLFUCache(_TestMixin):
    CACHE = LFUCache

    def test_policy(self):
        obj = self.CACHE(5, {i: i for i in range(5)})

        for i in range(5):
            obj[i] = i

        for i in range(10):
            assert 0 == obj[0]
        for i in range(7):
            assert 1 == obj[1]
        for i in range(3):
            assert 2 == obj[2]
        for i in range(4):
            assert 3 == obj[3]
        for i in range(6):
            assert 4 == obj[4]

        assert (2, 2) == obj.popitem()
        assert (3, 3) == obj.popitem()

        for i in range(10):
            assert 4 == obj.get(4)

        assert (1, 1) == obj.popitem()

        assert 2 == len(obj)
        obj.clear()

        for i in range(5):
            obj[i] = i

        assert [0, 1, 2, 3, 4] == list(obj.keys())

        for i in range(10):
            obj[0] += 1
        for i in range(7):
            obj[1] += 1
        for i in range(3):
            obj[2] += 1
        for i in range(4):
            obj[3] += 1
        for i in range(6):
            obj[4] += 1

        obj[5] = 4
        assert [5, 3, 4, 1, 0] == list(obj.keys())

    def test_items_with_frequency(self):
        # no need to test completely items_with_frequency
        # because it's tested in test_iterators
        obj = LFUCache(10, {1:2, 3:4})
        for key, val, freq in obj.items_with_frequency():
            assert key in obj
            assert val == obj[key]
            assert isinstance(freq, int)

    def test_least_frequently_used(self):
        obj = LFUCache(10)

        for i in range(5):
            obj[i] = i * 2

        for i in range(10):
            obj[0] += 1
        for i in range(7):
            obj[1] += 1
        for i in range(3):
            obj[2] += 1
        for i in range(4):
            obj[3] += 1
        for i in range(6):
            obj[4] += 1

        assert obj.least_frequently_used() == 2
        assert obj.least_frequently_used(1) == 3
        assert obj.least_frequently_used(4) == 0
        assert obj.least_frequently_used(5) is None

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        self._test_pickle(inner)
