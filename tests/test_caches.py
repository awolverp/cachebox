import cachebox
import unittest
import typing
import time
import pickle
import tempfile
import dataclasses


class CacheTestSuiteMixin:
    cache: typing.Type[cachebox.BaseCacheImpl]
    fixed_size = False
    has_popitem = True
    can_pickle = False
    kwargs = dict()

    def test_creation(self):
        obj = self.cache(1, **self.kwargs)
        self.assertEqual(0, len(obj))
        self.assertEqual(1, obj.maxsize)
        cap1 = obj.__sizeof__()

        obj = self.cache(maxsize=10, capacity=20, **self.kwargs)
        self.assertEqual(0, len(obj))
        self.assertEqual(10, obj.maxsize)
        cap2 = obj.__sizeof__()

        obj = self.cache(maxsize=20, capacity=20, **self.kwargs)
        self.assertEqual(0, len(obj))
        self.assertEqual(20, obj.maxsize)
        cap3 = obj.__sizeof__()

        self.assertGreater(cap2, cap1)
        self.assertGreater(cap3, cap2)

    def test_setitem(self):
        obj = self.cache(2, **self.kwargs)

        obj[0] = 1
        obj["name"] = "nick"
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[0])
        self.assertEqual("nick", obj["name"])

        try:
            obj[3] = 4
        except OverflowError as e:
            if not self.fixed_size:
                raise e from None

            self.assertEqual(2, len(obj))
            self.assertEqual(1, obj[0])
            self.assertEqual("nick", obj["name"])

        else:
            self.assertEqual(2, len(obj))
            self.assertEqual(4, obj[3])
            self.assertTrue("name" in obj or 0 in obj)

    def test_insert(self):
        obj = self.cache(2, **self.kwargs)

        obj.insert(0, 1)
        obj.insert("name", "nick")
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[0])
        self.assertEqual("nick", obj["name"])

        try:
            obj.insert(3, 4)
        except OverflowError as e:
            if not self.fixed_size:
                raise e from None

            self.assertEqual(2, len(obj))
            self.assertEqual(1, obj[0])
            self.assertEqual("nick", obj["name"])

        else:
            self.assertEqual(2, len(obj))
            self.assertEqual(4, obj[3])
            self.assertTrue("name" in obj or 0 in obj)

    def test_hashable(self):
        obj = self.cache(0, **self.kwargs)

        with self.assertRaises(TypeError):
            obj[set()] = set()

    def test_runtime_error_on_mutable(self):
        obj = self.cache(0, **self.kwargs)
        obj.update(obj)

    def test_update(self):
        obj = self.cache(2, **self.kwargs)

        obj.update({1: 1, 2: 2})
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[1])
        self.assertEqual(2, obj[2])

        obj.update({1: 1, 2: 2})
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[1])
        self.assertEqual(2, obj[2])

        obj.update([(1, "a"), (2, "b")])
        self.assertEqual(2, len(obj))
        self.assertEqual("a", obj[1])
        self.assertEqual("b", obj[2])

    def test_delitem(self):
        obj = self.cache(2, **self.kwargs)

        obj[0] = 1
        obj["name"] = "nick"
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[0])
        self.assertEqual("nick", obj["name"])

        del obj[0]
        self.assertEqual(1, len(obj))
        self.assertEqual("nick", obj["name"])
        self.assertNotIn(0, obj)

        del obj["name"]
        self.assertEqual(0, len(obj))
        self.assertNotIn(0, obj)
        self.assertNotIn("name", obj)

        with self.assertRaises(KeyError):
            del obj["name"]

    def test_delete(self):
        obj = self.cache(2, **self.kwargs)

        obj[0] = 1
        obj["name"] = "nick"
        self.assertEqual(2, len(obj))
        self.assertEqual(1, obj[0])
        self.assertEqual("nick", obj["name"])

        del obj[0]
        self.assertEqual(1, len(obj))
        self.assertEqual("nick", obj["name"])
        self.assertNotIn(0, obj)

        del obj["name"]
        self.assertEqual(0, len(obj))
        self.assertNotIn(0, obj)
        self.assertNotIn("name", obj)

        with self.assertRaises(KeyError):
            del obj["name"]

    def test_pop(self):
        obj = self.cache(2, **self.kwargs)

        obj[1] = 1
        obj[2] = 2
        self.assertEqual(2, obj.pop(2))
        self.assertEqual(1, len(obj))
        self.assertEqual(1, obj.pop(1))
        self.assertEqual(0, len(obj))

        self.assertEqual(None, obj.pop(2, None))
        self.assertEqual(None, obj.pop(1, None))
        self.assertEqual(None, obj.pop(0, None))
        self.assertEqual(None, obj.pop(2))
        self.assertEqual(None, obj.pop(1))
        self.assertEqual(None, obj.pop(0))

    def test_setdefault(self):
        obj = self.cache(2, **self.kwargs)

        obj.setdefault("name", "nick")
        obj["age"] = 18
        self.assertEqual(18, obj.setdefault("age", 1000))
        self.assertEqual(18, obj["age"])
        self.assertEqual("nick", obj["name"])

        if self.fixed_size:
            with self.assertRaises(OverflowError):
                obj.setdefault("newkey", 0)

    def test_keys_values_items(self):
        obj = self.cache(100, **self.kwargs)

        for i in range(6):
            obj[i] = i * 2

        k = list(range(6))
        v = list(i * 2 for i in range(6))
        self.assertEqual(k, sorted(obj.keys()))
        self.assertEqual(v, sorted(obj.values()))
        self.assertEqual(list(zip(k, v)), sorted(obj.items()))

        with self.assertRaises(RuntimeError):
            for i in obj:
                del obj[i]

        for i in range(100):
            obj[i] = i * 2

        for i in range(50):
            del obj[i]

        p = iter(obj)
        next(p)

        obj.shrink_to_fit()

        with self.assertRaises(RuntimeError):
            next(p)

    def test_get(self):
        obj = self.cache(2, **self.kwargs)

        obj[1] = 1
        obj[2] = 2
        self.assertEqual(2, obj.get(2))
        self.assertEqual(None, obj.get(3))
        self.assertEqual(7, obj.get(3, 7))
        self.assertEqual(2, len(obj))

    def test_clear(self):
        obj = self.cache(2, **self.kwargs)

        obj[1] = 1
        obj[2] = 2
        self.assertEqual(2, len(obj))

        cap = self.__sizeof__()
        obj.clear(reuse=True)
        self.assertEqual(0, len(obj))
        self.assertGreaterEqual(obj.__sizeof__(), cap)

        obj[1] = 1
        obj[2] = 2
        self.assertEqual(2, len(obj))

        cap = self.__sizeof__()
        obj.clear(reuse=False)
        self.assertEqual(0, len(obj))
        # this is not stable and
        # may increases the capacity!
        self.assertNotEqual(cap, obj.__sizeof__())

    def test_popitem(self):
        obj = self.cache(maxsize=2, **self.kwargs)

        if not self.has_popitem:
            with self.assertRaises(NotImplementedError):
                obj.popitem()

            return

        obj[1] = 1
        obj[2] = 2
        self.assertIsInstance(obj.popitem(), tuple)
        self.assertEqual(1, len(obj))
        self.assertIsInstance(obj.popitem(), tuple)
        self.assertEqual(0, len(obj))

        with self.assertRaises(KeyError):
            obj.popitem()

        obj["age"] = 19

        self.assertEqual(obj.popitem(), ("age", 19))

    def test_subclass(self):
        self.assertIsInstance(self.cache(0, **self.kwargs), cachebox.BaseCacheImpl)

        class CustomClass(self.cache):
            pass

        self.assertIsInstance(CustomClass(0, **self.kwargs), cachebox.BaseCacheImpl)

    def test_limit(self):
        obj = self.cache(maxsize=10, **self.kwargs)

        if self.has_popitem:
            obj.update({i: i for i in range(20)})

        else:
            with self.assertRaises(OverflowError):
                obj.update({i: i for i in range(20)})

    def test_generic(self):
        obj: self.cache[int, int] = self.cache(maxsize=0, **self.kwargs)
        _ = obj

    def test_pickle(self, co=True):
        if not self.can_pickle:
            return

        def check_order(cache1, cache2):
            while not cache1.is_empty():
                if hasattr(cache1, "popitem_with_expire"):
                    (k1, v1, r1) = cache1.popitem_with_expire()
                    (k2, v2, r2) = cache2.popitem_with_expire()
                    assert (k1, v1) == (
                        k2,
                        v2,
                    ), "invalid order: ({}, {}) != ({}, {}) | [{}] and [{}]".format(
                        k1, v1, k2, v2, r1, r2
                    )
                else:
                    (k1, v1) = cache1.popitem()
                    (k2, v2) = cache2.popitem()
                    assert (k1, v1) == (k2, v2), "invalid order: ({}, {}) != ({}, {})".format(
                        k1, v1, k2, v2
                    )

        # empty cache
        c1 = self.cache(maxsize=0, **self.kwargs)
        c2 = pickle.loads(pickle.dumps(c1))
        assert c1 == c2
        assert c1.capacity() == c2.capacity()

        # not empty
        c1 = self.cache(maxsize=100, **self.kwargs)
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
        assert c1 == c2
        assert c1.capacity() == c2.capacity(), "{} != {}".format(c1.capacity(), c2.capacity())
        if co:
            check_order(c1, c2)

        # pickle in file
        with tempfile.TemporaryFile("w+b") as fd:
            c1 = self.cache(maxsize=100, **self.kwargs)
            c1.update({i: i for i in range(100)})
            pickle.dump(c1, fd)
            fd.seek(0)
            c2 = pickle.load(fd)
            assert c1 == c2
            assert c1.capacity() == c2.capacity()
            if co:
                check_order(c1, c2)

    def test_eq_implemetation(self):
        # see https://github.com/awolverp/cachebox/issues/5

        @dataclasses.dataclass
        class EQ:
            def __init__(self, val: int) -> None:
                self.val = val

            def __eq__(self, other: "EQ") -> bool:
                return id(self) == id(other)

            def __hash__(self) -> int:
                return self.val


        @dataclasses.dataclass
        class NoEQ:
            def __init__(self, val: int) -> None:
                self.val = val

            def __hash__(self) -> int:
                return self.val

        size = 1000
        cache = self.cache(size, **self.kwargs)

        for i in range(size):
            cache.insert(NoEQ(val=i), i)
            cache.get(NoEQ(val=i))
        
        cache = self.cache(size, **self.kwargs)

        for i in range(size):
            cache.insert(EQ(val=i), i)
            cache.get(EQ(val=i))


class TestBaseCacheImpl(unittest.TestCase):
    def test_new(self):
        with self.assertRaises(NotImplementedError):
            cachebox.BaseCacheImpl(0)


class TestCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.Cache
    fixed_size = True
    has_popitem = False
    can_pickle = True

    def test_pickle(self):
        return super().test_pickle(co=False)


class TestFIFOCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.FIFOCache
    can_pickle = True

    def test_policy(self):
        obj = self.cache(2)

        obj["name"] = 1
        obj["age"] = 2

        obj["key3"] = "value3"
        self.assertEqual(2, len(obj))
        self.assertNotIn("name", obj)

        obj["key4"] = "value4"
        self.assertEqual(2, len(obj))
        self.assertNotIn("age", obj)

    def test_ordered_iter(self):
        keys = [0, 1, 2]
        values = [10, 5, 7]

        obj: cachebox.FIFOCache = self.cache(3, zip(keys, values))

        arr = [obj.first(i) for i in range(len(obj))]
        assert arr == keys

        arr = [obj[obj.first(i)] for i in range(len(obj))]
        assert arr == values

        arr = [(obj.first(i), obj[obj.first(i)]) for i in range(len(obj))]
        assert arr == list(zip(keys, values))


class TestLFUCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.LFUCache
    can_pickle = True

    def test_policy(self):
        obj = self.cache(5)

        for i in range(5):
            obj[i] = i

        for i in range(10):
            self.assertEqual(0, obj[0])
        for i in range(7):
            self.assertEqual(1, obj[1])
        for i in range(3):
            self.assertEqual(2, obj[2])
        for i in range(4):
            self.assertEqual(3, obj[3])
        for i in range(6):
            self.assertEqual(4, obj[4])

        self.assertEqual((2, 2), obj.popitem())
        self.assertEqual((3, 3), obj.popitem())

        for i in range(10):
            self.assertEqual(4, obj.get(4))

        self.assertEqual((1, 1), obj.popitem())

        self.assertEqual(2, len(obj))
        obj.clear()

        for i in range(5):
            obj[i] = i
        self.assertEqual([0, 1, 2, 3, 4], sorted(obj.keys()))

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
        self.assertEqual([0, 1, 3, 4, 5], sorted(obj.keys()))

    def test_ordered_iter(self):
        keys = [0, 1, 2]
        values = [10, 5, 7]

        obj: cachebox.LFUCache = self.cache(3, zip(keys, values))

        obj[1]
        obj[0]
        obj[0]
        obj[2]
        obj[2]
        obj[2]

        # sort again keys values
        keys = [1, 0, 2]
        values = [5, 10, 7]

        arr = [obj.least_frequently_used(i) for i in range(len(obj))]
        assert arr == keys, "{} != {}".format(arr, keys)

        arr = [obj.peek(obj.least_frequently_used(i)) for i in range(len(obj))]
        assert arr == values, "{} != {}".format(arr, values)

        arr = [
            (obj.least_frequently_used(i), obj.peek(obj.least_frequently_used(i)))
            for i in range(len(obj))
        ]
        assert arr == list(zip(keys, values)), "{} != {}".format(arr, list(zip(keys, values)))


class TestLRUCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.LRUCache
    can_pickle = True

    def test_policy(self):
        obj = self.cache(3)

        obj[1] = 1
        obj[2] = 2
        obj[3] = 3

        self.assertEqual((1, 1), obj.popitem())

        obj[1] = 1
        obj[2]

        self.assertEqual((3, 3), obj.popitem())

        obj[4] = 4
        self.assertEqual(1, obj.get(1))

        obj[5] = 5
        self.assertNotIn(2, obj)

    def test_ordered_iter(self):
        keys = [0, 1, 2]
        values = [10, 5, 7]

        obj: cachebox.LRUCache = self.cache(3, zip(keys, values))

        obj[1]
        obj[0]

        # sort again keys values
        keys = [2, 1, 0]
        values = [7, 5, 10]

        arr = [obj.least_recently_used(i) for i in range(len(obj))]
        assert arr == keys, "{} != {}".format(arr, keys)

        arr = [obj.peek(obj.least_recently_used(i)) for i in range(len(obj))]
        assert arr == values, "{} != {}".format(arr, values)

        arr = [
            (obj.least_recently_used(i), obj.peek(obj.least_recently_used(i)))
            for i in range(len(obj))
        ]
        assert arr == list(zip(keys, values)), "{} != {}".format(arr, list(zip(keys, values)))


class TestRRCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.RRCache
    can_pickle = True

    def test_policy(self):
        obj = self.cache(2)

        obj["name"] = 1
        obj["age"] = 2

        self.assertIn(obj.popitem(), [("name", 1), ("age", 2)])

    def test_pickle(self):
        return super().test_pickle(co=False)


class TestVTTLCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.VTTLCache
    can_pickle = True

    def test_policy(self):
        obj = self.cache(2)

        obj.insert(0, 1, 0.5)
        time.sleep(0.5)

        with self.assertRaises(KeyError):
            obj[0]

        obj.insert("name", "nick", 0.3)
        obj.insert("age", 18, None)
        time.sleep(0.3)

        with self.assertRaises(KeyError):
            obj["name"]

        del obj["age"]

        obj.insert(0, 0, 70)
        obj.insert(1, 1, 60)
        obj.insert(2, 2, 90)

        self.assertNotIn(1, obj)
        self.assertTupleEqual((0, 0), obj.popitem())

    def test_update_with_ttl(self):
        obj = self.cache(2)

        obj.update({1: 1, 2: 2, 3: 3}, 0.5)
        time.sleep(0.5)

        with self.assertRaises(KeyError):
            obj[1]

        with self.assertRaises(KeyError):
            obj[2]

        with self.assertRaises(KeyError):
            obj[3]

    def test_get_with_expire(self):
        obj = self.cache(2)

        obj.insert(1, 1, 10)

        value, dur = obj.get_with_expire(1)
        self.assertEqual(1, value)
        self.assertTrue(10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur)

        obj.insert(1, 1, None)
        value, dur = obj.get_with_expire(1)
        self.assertEqual(1, value)
        self.assertEqual(0, dur)

        value, dur = obj.get_with_expire("no-exists")
        self.assertIs(None, value)
        self.assertEqual(0, dur)

        value, dur = obj.get_with_expire("no-exists", "value")
        self.assertEqual("value", value)
        self.assertEqual(0, dur)

    def test_pickle(self, co=True):
        super().test_pickle(co)

        # test expire
        c1 = cachebox.VTTLCache(10)
        c1.update({i: i for i in range(5)}, 3)
        time.sleep(1)
        c1.update({i + 5: i + 5 for i in range(5)}, 5)

        byt = pickle.dumps(c1)
        time.sleep(2)
        c2 = pickle.loads(byt)

        assert len(c2) == 5, "{}".format(len(c2))


class TestTTLCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.TTLCache
    kwargs = {"ttl": 120}
    can_pickle = True

    def test_policy(self):
        obj = self.cache(2, 0.5)
        self.assertEqual(obj.ttl, 0.5)

        obj.insert(0, 1)
        time.sleep(0.5)

        with self.assertRaises(KeyError):
            obj[0]

        obj = self.cache(2, 20)

        obj.insert(0, 0)
        obj.insert(1, 1)
        obj.insert(2, 2)

        self.assertNotIn(0, obj)
        self.assertTupleEqual((1, 1), obj.popitem())

    def test_update_with_ttl(self):
        obj = self.cache(2, 0.5)

        obj.update({1: 1, 2: 2, 3: 3})
        time.sleep(0.5)

        with self.assertRaises(KeyError):
            obj[1]

        with self.assertRaises(KeyError):
            obj[2]

        with self.assertRaises(KeyError):
            obj[3]

    def test_get_with_expire(self):
        obj = self.cache(2, 10)

        obj.insert(1, 1)
        value, dur = obj.get_with_expire(1)
        self.assertEqual(1, value)
        self.assertTrue(10 > dur > 9, "10 > dur > 9 failed [dur: %f]" % dur)

        value, dur = obj.get_with_expire("no-exists")
        self.assertIs(None, value)
        self.assertEqual(0, dur)

        value, dur = obj.get_with_expire("no-exists", "value")
        self.assertEqual("value", value)
        self.assertEqual(0, dur)

    def test_pickle(self, co=True):
        super().test_pickle(co)

        # test expire
        c1 = cachebox.TTLCache(10, 3)
        c1.update({i: i for i in range(5)})
        time.sleep(1)
        c1.update({i + 5: i + 5 for i in range(5)})

        byt = pickle.dumps(c1)
        time.sleep(2)
        c2 = pickle.loads(byt)

        assert len(c2) == 5

    def test_ordered_iter(self):
        keys = [0, 1, 2]
        values = [10, 5, 7]

        obj: cachebox.TTLCache = self.cache(3, 10, zip(keys, values))

        arr = [obj.first(i) for i in range(len(obj))]
        assert arr == keys

        arr = [obj[obj.first(i)] for i in range(len(obj))]
        assert arr == values

        arr = [(obj.first(i), obj[obj.first(i)]) for i in range(len(obj))]
        assert arr == list(zip(keys, values))
