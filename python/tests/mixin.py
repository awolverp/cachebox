from cachebox import BaseCacheImpl, TTLCache
import dataclasses
import pytest
import typing
import sys


@dataclasses.dataclass
class EQ:
    def __init__(self, val: int) -> None:
        self.val = val

    def __eq__(self, other: "EQ") -> bool:
        return self.val == other.val

    def __hash__(self) -> int:
        return self.val


@dataclasses.dataclass
class NoEQ:
    def __init__(self, val: int) -> None:
        self.val = val

    def __hash__(self) -> int:
        return self.val


def getsizeof(obj, use_sys=True):  # pragma: no cover
    try:
        if use_sys:
            return sys.getsizeof(obj)
        else:
            return obj.__sizeof__()
    except TypeError:  # PyPy doesn't implement getsizeof or __sizeof__
        return len(obj)


class _TestMixin:  # pragma: no cover
    CACHE: typing.Type[BaseCacheImpl]

    KWARGS: dict = {}
    NO_POLICY: bool = False

    def test__new__(self):
        cache = self.CACHE(10, **self.KWARGS, capacity=8)
        assert cache.maxsize == 10
        assert 20 > cache.capacity() >= 8, "capacity: {}".format(cache.capacity())

        cache = self.CACHE(20, **self.KWARGS, capacity=0)
        assert cache.maxsize == 20
        assert 2 >= cache.capacity() >= 0  # This is depends on platform

        cache = self.CACHE(20, **self.KWARGS, capacity=100)
        assert cache.maxsize == 20
        assert 30 > cache.capacity() >= 20

        cache = self.CACHE(0, **self.KWARGS, capacity=8)
        assert cache.maxsize == sys.maxsize
        assert 20 > cache.capacity() >= 8

    def test_overflow(self):
        if not self.NO_POLICY:
            return

        cache = self.CACHE(10, **self.KWARGS, capacity=10)

        for i in range(10):
            cache[i] = i

        with pytest.raises(OverflowError):
            cache["new-key"] = "new-value"

    def test___len__(self):
        cache = self.CACHE(10, **self.KWARGS, capacity=10)

        assert len(cache) == 0
        assert cache.is_empty() ^ bool(cache)

        cache[0] = 0
        assert len(cache) == 1

        cache[1] = 1
        cache[2] = 2
        cache[3] = 3
        assert len(cache) == 4

        cache[0] = 10
        cache[1] = 5
        assert len(cache) == 4

        for i in range(1000, 1000 + (10 - len(cache))):
            cache[i] = i

        assert len(cache) == 10
        assert cache.is_full()

    def test___contains__(self):
        cache = self.CACHE(1, **self.KWARGS, capacity=1)

        assert 1 not in cache
        cache[1] = 1
        assert 1 in cache

    def test___setitem__(self):
        cache = self.CACHE(10, **self.KWARGS, capacity=10)

        with pytest.raises(KeyError):
            cache[1]

        cache[1] = 1
        cache[1]
        cache[0] = 0
        cache[0]
        cache[2] = 2
        cache[3] = 3

        with pytest.raises(KeyError):
            cache[4]

        del cache[1]
        del cache[2]
        del cache[3]

        with pytest.raises(KeyError):
            del cache["error"]

        cache[0]

        with pytest.raises(KeyError):
            cache[2]

    def test___repr__(self):
        cache = self.CACHE(1000, **self.KWARGS, capacity=2)
        assert repr(cache).startswith(self.CACHE.__module__ + "." + self.CACHE.__name__)

        cache.update((i, i) for i in range(1000))
        assert str(cache) == repr(cache)

    def test_insert(self):
        cache = self.CACHE(5, **self.KWARGS, capacity=5)

        assert cache.insert(1, 1) is None
        assert cache.insert(1, 1) == 1
        assert cache.insert(1, 10) == 1
        assert cache.insert(1, 2) == 10

        cache[5] = 5

        assert cache.insert(5, "value") == 5
        assert cache.insert(5, 5) == "value"

        del cache[5]

        assert cache.insert(5, 5) is None

    def test_get(self):
        cache = self.CACHE(5, **self.KWARGS, capacity=5)

        for i in range(5):
            cache[i] = i

        assert cache.get(0, None) == 0
        assert cache.get(1, None) == 1
        assert cache.get("no-exists") is None
        assert cache.get("no-exists", None) is None
        assert cache.get("no-exists", 111) == 111

    def test_pop(self):
        cache = self.CACHE(5, **self.KWARGS, capacity=5)

        for i in range(5):
            cache[i] = i * 2

        assert cache.pop(1, None) == 2
        assert cache.get(1, None) is None
        assert cache.pop(2, None) == 4
        assert cache.get(2, None) is None

        assert cache.pop(10, None) is None
        assert cache.pop(10, 2) == 2

    def test_setdefault(self):
        obj = self.CACHE(2, **self.KWARGS, capacity=2)

        obj.setdefault("name", "nick")
        obj["age"] = 18
        assert 18 == obj.setdefault("age", 1000)
        assert 18 == obj["age"]
        assert "nick" == obj["name"]

        if self.NO_POLICY:
            with pytest.raises(OverflowError):
                obj.setdefault("newkey", 0)

    def test_clear(self):
        obj = self.CACHE(2, **self.KWARGS, capacity=2)

        obj[1] = 1
        obj[2] = 2
        assert 2 == len(obj)

        cap = getsizeof(obj, False)
        obj.clear(reuse=True)
        assert 0 == len(obj)
        try:
            assert getsizeof(obj, False) >= cap
        except AssertionError as e:
            # if not isinstance(obj, (LRUCache, LFUCache)):
            raise e

        obj[1] = 1
        obj[2] = 2
        assert 2 == len(obj)

        cap = getsizeof(obj, False)
        obj.clear(reuse=False)
        assert 0 == len(obj)
        # this is not stable and
        # may increases the capacity!
        try:
            assert cap != getsizeof(obj, False)
        except AssertionError as e:
            # if not isinstance(obj, (LRUCache, LFUCache)):
            raise e

    def test_update(self):
        obj = self.CACHE(2, **self.KWARGS, capacity=2)

        obj.update({1: 1, 2: 2})
        assert 2 == len(obj)
        assert 1 == obj[1]
        assert 2 == obj[2]

        obj.update({1: 1, 2: 2})
        assert 2 == len(obj)
        assert 1 == obj[1]
        assert 2 == obj[2]

        obj.update([(1, "a"), (2, "b")])
        assert 2 == len(obj)
        assert "a" == obj[1]
        assert "b" == obj[2]

        if self.NO_POLICY:
            with pytest.raises(OverflowError):
                obj.update([(3, "a"), (4, "b")])
        else:
            obj.update([(3, "a"), (4, "b")])

        kw = self.KWARGS.copy()
        kw["iterable"] = {1: 1, 2: 2}
        obj = self.CACHE(2, **kw, capacity=2)
        assert 2 == len(obj)
        assert 1 == obj[1]
        assert 2 == obj[2]

        kw["iterable"] = [(1, "a"), (2, "b")]
        obj = self.CACHE(2, **kw, capacity=2)
        assert 2 == len(obj)
        assert "a" == obj[1]
        assert "b" == obj[2]

    def test_eq_implemetation(self):
        # see https://github.com/awolverp/cachebox/issues/5

        size = 1000
        cache = self.CACHE(size, **self.KWARGS, capacity=size)

        for i in range(size):
            cache.insert(NoEQ(val=i), i)
            cache.get(NoEQ(val=i))

        cache = self.CACHE(size, **self.KWARGS, capacity=size)

        for i in range(size):
            cache.insert(EQ(val=i), i)
            cache.get(EQ(val=i))

    def test_iterators(self):
        obj = self.CACHE(100, **self.KWARGS, capacity=100)

        for i in range(6):
            obj[i] = i * 2

        k = list(range(6))
        v = list(i * 2 for i in range(6))
        assert k == sorted(obj.keys())
        assert v == sorted(obj.values())
        assert list(zip(k, v)) == sorted(obj.items())

        with pytest.raises(RuntimeError):
            for i in obj:
                del obj[i]

        for i in range(100):
            obj[i] = i * 2

        for i in range(50):
            del obj[i]

        p = iter(obj)
        next(p)

        obj.shrink_to_fit()

        with pytest.raises(RuntimeError):
            next(p)

        obj = self.CACHE(0, **self.KWARGS)
        obj.update({i: i for i in range(20)})

        for key, value in obj.items():
            assert obj[key] == value

        try:
            for key, value in obj.items():
                obj[key] = value * 2
        except RuntimeError:
            if not isinstance(obj, TTLCache):
                raise

        with pytest.raises(RuntimeError):
            for key, value in obj.items():
                obj[str(key)] = value

    def test___eq__(self):
        cache = self.CACHE(100, **self.KWARGS, capacity=100)

        with pytest.raises(TypeError):
            cache > cache

        with pytest.raises(TypeError):
            cache < cache

        with pytest.raises(TypeError):
            cache >= cache

        with pytest.raises(TypeError):
            cache <= cache

        assert cache == cache
        assert not cache != cache

        for i in range(90):
            cache[i] = i

        assert cache == cache
        assert not cache != cache

        c2 = self.CACHE(100, **self.KWARGS, capacity=100)
        for i in range(90):
            c2[i] = i

        assert cache == c2
        assert not c2 != cache

        c2 = self.CACHE(1000, **self.KWARGS, capacity=100)
        for i in range(90):
            c2[i] = i

        assert not cache == c2
        assert c2 != cache

    def _test_pickle(self, check_order: typing.Callable):
        import pickle
        import tempfile

        c1 = self.CACHE(maxsize=0, **self.KWARGS)
        c2 = pickle.loads(pickle.dumps(c1))
        assert c1 == c2
        assert c1.capacity() == c2.capacity()

        c1 = self.CACHE(maxsize=100, **self.KWARGS)
        c1.update({i: i for i in range(10)})

        for _ in range(10):
            c1[0]
        for _ in range(9):
            c1[1]
        for _ in range(8):
            c1[2]
        for _ in range(7):
            c1[3]
        for _ in range(6):
            c1[4]
        for _ in range(5):
            c1[5]
        for _ in range(4):
            c1[6]
        for _ in range(3):
            c1[7]
        for _ in range(2):
            c1[8]
        for _ in range(1):
            c1[9]

        c2 = pickle.loads(pickle.dumps(c1))
        assert c1 == c2, f"{c1} - {c2}"
        assert c1.capacity() == c2.capacity()
        check_order(c1, c2)

        with tempfile.TemporaryFile("w+b") as fd:
            c1 = self.CACHE(maxsize=100, **self.KWARGS)
            c1.update({i: i for i in range(10)})

            for _ in range(10):
                c1[1]
            for _ in range(9):
                c1[2]
            for _ in range(8):
                c1[0]
            for _ in range(7):
                c1[3]
            for _ in range(6):
                c1[5]
            for _ in range(5):
                c1[4]
            for _ in range(4):
                c1[6]
            for _ in range(3):
                c1[7]
            for _ in range(2):
                c1[9]
            for _ in range(1):
                c1[8]

            pickle.dump(c1, fd)
            fd.seek(0)
            c2 = pickle.load(fd)
            assert c1 == c2
            assert c1.capacity() == c2.capacity()
            check_order(c1, c2)

    def test_copy(self):
        import copy

        # shallow copy
        c1 = self.CACHE(maxsize=0, **self.KWARGS)
        c1.insert("dict", {})
        c2 = c1.copy()

        assert c2 == c1
        c2["dict"][1] = 1

        assert c1["dict"][1] == 1

        c2.insert(1, 1)
        assert 1 not in c1

        # deepcopy
        c1 = self.CACHE(maxsize=0, **self.KWARGS)
        c1.insert("dict", {})
        c2 = copy.deepcopy(c1)

        assert c2 == c1
        c2["dict"][1] = 1

        assert 1 not in c1["dict"]

        c2.insert(1, 1)
        assert 1 not in c1
