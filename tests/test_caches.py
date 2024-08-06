from cachebox import BaseCacheImpl, Cache, FIFOCache, RRCache, TTLCache
import pytest
import time

from .mixin import _TestMixin


def test___new__():
    with pytest.raises(NotImplementedError):
        BaseCacheImpl()

def test_subclass():
    class _TestSubclass(BaseCacheImpl):
        def __init__(self) -> None:
            self.a = 1
        
        def inc(self, x: int):
            self.a += x
    
    t = _TestSubclass()
    t.inc(10)
    assert t.a == 11


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
            obj[i] = i*2
        
        assert obj.first() == 0
        assert obj.last() == 4

        obj[10] = 20

        assert obj.first() == 1
        assert obj.last() == 10

class TestRRCache(_TestMixin):
    CACHE = RRCache

    def test_pickle(self):
        self._test_pickle(lambda c1, c2: None)


class TestTTLCache(_TestMixin):
    CACHE = TTLCache
    KWARGS = {"ttl": 10}

    def test_policy(self):
        obj = self.CACHE(2, 0.5)
        assert obj.ttl == 0.5

        obj.insert(0, 1)
        time.sleep(0.8)

        with pytest.raises(KeyError):
            obj[0]

        obj = self.CACHE(2, 20)

        obj.insert(0, 0)
        obj.insert(1, 1)
        obj.insert(2, 2)

        assert 0 not in obj
        assert (1, 1) == obj.popitem()

    def test_update_with_ttl(self):
        obj = self.CACHE(2, 0.5)

        # obj.update({1: 1, 2: 2, 3: 3})
        obj.update((i+1, i+1) for i in range(3))

        with pytest.raises(KeyError):
            obj[1]

        time.sleep(0.8)

        with pytest.raises(KeyError):
            obj[2]

        with pytest.raises(KeyError):
            obj[3]

    def test_policy_ttl_no_care(self):
        cache = TTLCache(5, 10)

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

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        self._test_pickle(inner)

    def test_first_last(self):
        obj = self.CACHE(5, **self.KWARGS, capacity=5)

        for i in range(5):
            obj[i] = i*2
        
        assert obj.first() == 0
        assert obj.last() == 4

        obj[10] = 20

        assert obj.first() == 1
        assert obj.last() == 10

    def test_get_with_expire(self):
        obj = TTLCache(2, 10)

        obj.insert(1, 1)
        value, dur = obj.get_with_expire(1)
        assert 1 == value
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        value, dur = obj.get_with_expire("no-exists")
        assert value is None
        assert 0 == dur

        value, dur = obj.get_with_expire("no-exists", "value")
        assert "value" == value
        assert 0 == dur

    def test_pop_with_expire(self):
        obj = TTLCache(2, 10)

        obj.insert(1, 1)
        value, dur = obj.pop_with_expire(1)
        assert 1 == value
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        value, dur = obj.pop_with_expire("no-exists")
        assert value is None
        assert 0 == dur

        value, dur = obj.pop_with_expire("no-exists", "value")
        assert "value" == value
        assert 0 == dur
