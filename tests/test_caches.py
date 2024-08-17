from cachebox import (
    BaseCacheImpl,
    Cache,
    FIFOCache,
    RRCache,
    TTLCache,
    LRUCache,
    LFUCache,
    VTTLCache,
    cache_iterator,
    fifocache_iterator,
    ttlcache_iterator,
    lrucache_iterator,
    lfucache_iterator,
)

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
    ITERATOR_CLASS = cache_iterator

    def test_pickle(self):
        self._test_pickle(lambda c1, c2: None)


class TestFIFOCache(_TestMixin):
    CACHE = FIFOCache
    ITERATOR_CLASS = fifocache_iterator

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


class TestRRCache(_TestMixin):
    CACHE = RRCache
    ITERATOR_CLASS = cache_iterator

    def test_pickle(self):
        self._test_pickle(lambda c1, c2: None)


class TestTTLCache(_TestMixin):
    CACHE = TTLCache
    KWARGS = {"ttl": 10}
    ITERATOR_CLASS = ttlcache_iterator

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
        obj.update((i + 1, i + 1) for i in range(3))

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
            obj[i] = i * 2

        assert obj.first() == 0
        assert obj.last() == 4

        obj[10] = 20

        assert obj.first() == 1
        assert obj.last() == 10

    def test_get_with_expire(self):
        obj = TTLCache(2, 10)

        obj.insert(1, 1)
        time.sleep(0.1)
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
        time.sleep(0.1)
        value, dur = obj.pop_with_expire(1)
        assert 1 == value
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        value, dur = obj.pop_with_expire("no-exists")
        assert value is None
        assert 0 == dur

        value, dur = obj.pop_with_expire("no-exists", "value")
        assert "value" == value
        assert 0 == dur

    def test_popitem_with_expire(self):
        obj = TTLCache(2, 10)

        obj.insert(1, 1)
        obj.insert(2, 2)
        time.sleep(0.1)
        key, value, dur = obj.popitem_with_expire()
        assert (1, 1) == (key, value)
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        key, value, dur = obj.popitem_with_expire()
        assert (2, 2) == (key, value)
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        with pytest.raises(KeyError):
            obj.popitem_with_expire()


class TestLRUCache(_TestMixin):
    CACHE = LRUCache
    ITERATOR_CLASS = lrucache_iterator

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
        assert obj.least_recently_used(1) == 2
        assert obj.least_recently_used(5) == 3
        assert obj.least_recently_used(6) is None

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        self._test_pickle(inner)


class TestLFUCache(_TestMixin):
    CACHE = LFUCache
    ITERATOR_CLASS = lfucache_iterator

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


class TestVTTLCache(_TestMixin):
    CACHE = VTTLCache

    def test_policy(self):
        obj = VTTLCache(2)

        obj.insert(0, 1, 0.5)
        time.sleep(0.501)

        with pytest.raises(KeyError):
            obj[0]

        obj.insert("name", "nick", 0.3)
        obj.insert("age", 18, None)
        time.sleep(0.301)

        with pytest.raises(KeyError):
            obj["name"]

        del obj["age"]

        obj.insert(0, 0, 70)
        obj.insert(1, 1, 60)
        obj.insert(2, 2, 90)

        assert 1 not in obj
        assert (0, 0) == obj.popitem()

    def test_update_with_ttl(self):
        obj = VTTLCache(3)

        obj.update({1: 1, 2: 2, 3: 3}, 0.5)
        time.sleep(0.501)

        with pytest.raises(KeyError):
            obj[1]

        with pytest.raises(KeyError):
            obj[2]

        with pytest.raises(KeyError):
            obj[3]

    def test_get_with_expire(self):
        obj = VTTLCache(2)

        obj.insert(1, 1, 10)
        time.sleep(0.1)
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
        obj = VTTLCache(2)

        obj.insert(1, 1, 10)
        time.sleep(0.1)
        value, dur = obj.pop_with_expire(1)
        assert 1 == value
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        value, dur = obj.pop_with_expire("no-exists")
        assert value is None
        assert 0 == dur

        value, dur = obj.pop_with_expire("no-exists", "value")
        assert "value" == value
        assert 0 == dur

    def test_popitem_with_expire(self):
        obj = VTTLCache(2)

        obj.insert(1, 1, 10)
        obj.insert(2, 2, 6)
        time.sleep(0.1)
        key, value, dur = obj.popitem_with_expire()
        assert (2, 2) == (key, value)
        assert 6 > dur > 5, "6 > dur > 5 failed [dur: %f]" % dur

        key, value, dur = obj.popitem_with_expire()
        assert (1, 1) == (key, value)
        assert 10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur

        with pytest.raises(KeyError):
            obj.popitem_with_expire()

    def test_pickle(self):
        def inner(c1, c2):
            assert list(c1.items()) == list(c2.items())

        import pickle
        import tempfile

        c1 = self.CACHE(maxsize=0, **self.KWARGS)
        c2 = pickle.loads(pickle.dumps(c1))
        assert c1 == c2
        assert c1.capacity() == c2.capacity()

        c1 = self.CACHE(maxsize=100, **self.KWARGS)

        for i in range(10):
            c1.insert(i, i * 2, i + 2)

        c2 = pickle.loads(pickle.dumps(c1))
        assert c1 == c2
        assert c1.capacity() == c2.capacity()
        inner(c1, c2)

        with tempfile.TemporaryFile("w+b") as fd:
            c1 = self.CACHE(maxsize=100, **self.KWARGS)
            c1.update({i: i for i in range(10)})

            for i in range(10):
                c1.insert(i, i * 2, i + 2)

            pickle.dump(c1, fd)
            fd.seek(0)
            c2 = pickle.load(fd)
            assert c1 == c2
            assert c1.capacity() == c2.capacity()
            inner(c1, c2)

        c1 = self.CACHE(maxsize=100, **self.KWARGS)

        for i in range(10):
            c1.insert(i, i * 2, i + 0.5)

        time.sleep(0.51)

        c2 = pickle.loads(pickle.dumps(c1))

        assert len(c2) == len(c1)
        assert c1.capacity() == c2.capacity()
        inner(c1, c2)
