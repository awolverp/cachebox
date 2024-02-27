from . import Bench
import cachetools
import cachebox
import random

class CachetoolsMRUCache(Bench):
    maxsize = 1000
    rangesize = 10000

    def insert_setUp(self):
        return cachetools.MRUCache(self.maxsize)
    
    def bench_insert(self, cache):
        """
        Maxsize 1000 - Insert 10000
        """
        for i in range(self.rangesize):
            cache[i] = i

    def delete_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_delete(self, cache):
        """
        Delete 1000 items
        """
        for i in range(self.maxsize):
            del cache[i]

    def pop_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_pop(self, cache):
        """
        Pop 10000 items
        """
        for i in range(self.maxsize):
            cache.pop(i)

    def popitem_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_popitem(self, cache):
        """
        Popitem 10000 items
        """
        for i in range(self.maxsize):
            cache.popitem()
    
    def setdefault_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        return cache
    
    def bench_setdefault(self, cache):
        """
        Maxsize 1000 - setdefault 10000 random item
        """
        for i in range(self.rangesize):
            cache.setdefault(random.randint(0, 999), random.randint(0, 999))

    def update_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        return cache
    
    def bench_update(self, cache):
        """
        Maxsize 1000 - update 10000
        """
        cache.update(((i, i) for i in range(self.rangesize)))

    def clear_setUp(self):
        cache = cachetools.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_clear(self, cache):
        """
        Clear cache (1000 item)
        """
        cache.clear()


class CacheboxMRUCache(Bench):
    maxsize = 1000
    rangesize = 10000

    def insert_setUp(self):
        return cachebox.MRUCache(self.maxsize)
    
    def bench_insert(self, cache):
        """
        Maxsize 1000 - Insert 10000
        """
        for i in range(self.rangesize):
            cache[i] = i
    
    def delete_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_delete(self, cache):
        """
        Delete 1000 items
        """
        for i in range(self.maxsize):
            del cache[i]

    def pop_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_pop(self, cache):
        """
        Pop 10000 items
        """
        for i in range(self.maxsize):
            cache.pop(i)

    def popitem_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_popitem(self, cache):
        """
        Popitem 10000 items
        """
        for i in range(self.maxsize):
            cache.popitem()
    
    def setdefault_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        return cache
    
    def bench_setdefault(self, cache):
        """
        Maxsize 1000 - setdefault 10000 random item
        """
        for i in range(self.rangesize):
            cache.setdefault(random.randint(0, 999), random.randint(0, 999))

    def update_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        return cache
    
    def bench_update(self, cache):
        """
        Maxsize 1000 - update 10000
        """
        cache.update(((i, i) for i in range(self.rangesize)))

    def clear_setUp(self):
        cache = cachebox.MRUCache(self.maxsize)
        cache.update({i:i for i in range(self.maxsize)})
        return cache
    
    def bench_clear(self, cache):
        """
        Clear cache (1000 item)
        """
        cache.clear()
