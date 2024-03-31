import cachebox
import unittest
import typing


class CacheTestSuiteMixin:
    cache: typing.Type[cachebox.BaseCacheImpl]
    fixed_size = False
    has_popitem = True
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
        obj = self.cache(10, **self.kwargs)

        for i in range(6):
            obj[i] = i * 2

        k = list(range(6))
        v = list(i * 2 for i in range(6))
        self.assertEqual(k, sorted(obj.keys()))
        self.assertEqual(v, sorted(obj.values()))
        self.assertEqual(list(zip(k, v)), sorted(obj.items()))

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
        self.assertGreater(obj.__sizeof__(), cap)

        obj[1] = 1
        obj[2] = 2
        self.assertEqual(2, len(obj))

        cap = self.__sizeof__()
        obj.clear(reuse=False)
        self.assertEqual(0, len(obj))
        self.assertGreater(cap, obj.__sizeof__())

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

    def test_references_count(self):
        import sys

        key = "Key1"
        value = "Value1"
        keyref = sys.getrefcount(key)
        valueref = sys.getrefcount(value)

        obj = self.cache(0, **self.kwargs)

        obj[key] = value
        self.assertEqual(keyref + 1, sys.getrefcount(key))
        self.assertEqual(valueref + 1, sys.getrefcount(value))

        del obj[key]
        self.assertEqual(keyref, sys.getrefcount(key))
        self.assertEqual(valueref, sys.getrefcount(value))

        obj.setdefault(key, value)
        self.assertEqual(keyref + 1, sys.getrefcount(key))
        self.assertEqual(valueref + 1, sys.getrefcount(value))

        obj.pop(key)
        self.assertEqual(keyref, sys.getrefcount(key))
        self.assertEqual(valueref, sys.getrefcount(value))

        obj[key] = value
        obj.clear()
        self.assertEqual(keyref, sys.getrefcount(key))
        self.assertEqual(valueref, sys.getrefcount(value))

        obj.update({key: value})
        self.assertEqual(keyref + 1, sys.getrefcount(key))
        self.assertEqual(valueref + 1, sys.getrefcount(value))

        obj.update(
            [(key, value)]
        )  # this updates old value, so should not increase reference counts
        self.assertEqual(keyref + 1, sys.getrefcount(key))
        self.assertEqual(valueref + 1, sys.getrefcount(value))

        if self.has_popitem:
            obj[key] = value
            obj.popitem()
            self.assertEqual(keyref, sys.getrefcount(key))
            self.assertEqual(valueref, sys.getrefcount(value))

    def test_subclass(self):
        self.assertIsInstance(self.cache(0, **self.kwargs), cachebox.BaseCacheImpl)

        class CustomClass(self.cache):
            pass

        self.assertIsInstance(CustomClass(0, **self.kwargs), cachebox.BaseCacheImpl)


class TestBaseCacheImpl(unittest.TestCase):
    def test_new(self):
        with self.assertRaises(NotImplementedError):
            cachebox.BaseCacheImpl(0)


class TestCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.Cache
    fixed_size = True
    has_popitem = False


class TestFIFOCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.FIFOCache

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


class TestLFUCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.LFUCache

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


class TestLRUCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.LRUCache

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


class TestRRCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.RRCache

    def test_policy(self):
        obj = self.cache(2)

        obj["name"] = 1
        obj["age"] = 2

        self.assertIn(obj.popitem(), [("name", 1), ("age", 2)])


class TestVTTLCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.VTTLCache

    def test_policy(self):
        import time

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
        import time

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


class TestTTLCache(unittest.TestCase, CacheTestSuiteMixin):
    cache = cachebox.TTLCache
    kwargs = {"ttl": 120}

    def test_policy(self):
        import time

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
        import time

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
