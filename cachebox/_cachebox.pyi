import typing

__version_: str
__author__: str

KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    This is only a base class for all other caches; It's not implemented.

    You can use it for type hint::

        cache = create_a_cache() # type: BaseCacheImpl[int, str]

    Or use for isinstance::

        cache = Cache(10)
        assert isinstance(cache, BaseCacheImpl) # OK
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None: ...
    @property
    def maxsize(self) -> int:
        """
        Returns the maxsize.

        Example::

            >>> cache = Cache(10)
            >>> cache.maxsize
            10
        """
        ...

    def __len__(self) -> int: ...
    def __sizeof__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __setitem__(self, key: KT, value: VT) -> None: ...
    def __getitem__(self, key: KT) -> VT: ...
    def __delitem__(self, key: KT) -> None: ...
    def insert(self, key: KT, value: VT) -> None:
        """
        Works like `cache[key] = value`
        """
        ...

    def get(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns value of specified key; returns `default` if key not found.
        """
        ...

    def __contains__(self, key: KT) -> bool: ...
    def __eq__(self, other: typing.Self) -> bool: ...
    def __ne__(self, other: typing.Self) -> bool: ...
    def __iter__(self) -> typing.Iterator[KT]: ...
    def keys(self) -> typing.Iterator[KT]: ...
    def values(self) -> typing.Iterator[VT]: ...
    def items(self) -> typing.Iterator[typing.Tuple[KT, VT]]: ...
    def __repr__(self) -> str: ...
    def capacity(self) -> int:
        """
        Returns the number of elements the map can hold without reallocating.

        This number is a lower bound; the cache might be able to hold more,
        but is guaranteed to be able to hold at least this many.

        Example::

            >>> cache = Cache(100)
            >>> cache.capacity()
            0
            >>> cache[1] = 1
            >>> cache.capacity()
            3
        """
        ...

    def clear(self, *, reuse: bool = False) -> None:
        """
        Clears the cache, removing all key-value pairs.
        If `reuse` True, keeps the allocated memory for reuse.

        Example::

            >>> cache = Cache(0, {i:i for i in range(1000)})
            >>> len(cache), cache.capacity()
            1000, 1798
            >>> cache.clear(reuse=True)
            >>> len(cache), cache.capacity()
            0, 1798
            >>> cache.clear(reuse=False)
            >>> len(cache), cache.capacity()
            0, 0
        """
        ...

    def pop(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Deletes and returns the stored key-value from cache; returns `default` if key not found.
        """
        ...

    def setdefault(self, key: KT, default: DT = None) -> typing.Union[VT, DT]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.
        """
        ...

    def popitem(self) -> typing.Tuple[KT, VT]:
        """
        According to cache algorithm, deletes and returns an item from cache.
        """
        ...

    def drain(self, n: int) -> int:
        """
        According to cache algorithm, deletes and returns how many items removed from cache.

        Example::

            >>> cache = LFUCache(10, {i:i for i in range(10)})
            >>> cache.drain(8)
            8
            >>> len(cache)
            2
            >>> cache.drain(10)
            2
            >>> len(cache)
            0
        """
        ...

    def update(self, iterable: typing.Iterable) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Example::

            >>> cache = FIFOCache(10)
            >>> cache.update({i:i for i in range(12)})
            >>> len(cache)
            10
        """
        ...

    def shrink_to_fit(self) -> None:
        """
        Shrinks the capacity of the cache as much as possible.
        It will drop down as much as possible while maintaining the internal rules and possibly
        leaving some space in accordance with the resize policy.

        Example::

            >>> cache = LRUCache(0, {i:i for i in range(10)})
            >>> cache.capacity()
            27
            >>> cache.shrink_to_fit()
            >>> cache.capacity()
            11
        """
        ...

class Cache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        Fixed-size (or can be not) cache implementation without any policy,
        So only can be fixed-size, or unlimited size cache.

        Example::

            >>> cache = Cache(100) # fixed-size cache
            >>> cache = Cache(0) # unlimited-size cache
            >>> cache = Cache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
            >>> cache = Cache(2, {i:i for i in range(10)})
            ...
            OverflowError: maximum size limit reached
        """
        ...

    def popitem(self) -> typing.NoReturn:
        """
        It is not implemented for this cache;
        """
        ...

    def drain(self, n: int) -> typing.NoReturn:
        """
        It is not implemented for this cache;
        """
        ...

class FIFOCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        FIFO Cache implementation (First-In First-Out policy, very useful).

        In simple terms, the FIFO cache will remove the element that has been in the cache the longest;
        It behaves like a Python dictionary.

        Example::

            >>> cache = FIFOCache(100) # fixed-size cache
            >>> cache = FIFOCache(0) # unlimited-size cache
            >>> cache = FIFOCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
        """
        ...

    def first(self) -> typing.Optional[KT]: ...
    def last(self) -> typing.Optional[KT]: ...

class LFUCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        LFU Cache implementation (Least frequantly used policy).

        In simple terms, the LFU cache will remove the element in the cache that has been accessed the least,
        regardless of time.

        Example::

            >>> cache = LFUCache(100) # fixed-size cache
            >>> cache = LFUCache(0) # unlimited-size cache
            >>> cache = LFUCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
        """
        ...

    def least_frequently_used(self) -> typing.Optional[KT]: ...

class RRCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        RRCache implementation (Random Replacement policy).

        In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.

        Example::

            >>> cache = RRCache(100) # fixed-size cache
            >>> cache = RRCache(0) # unlimited-size cache
            >>> cache = RRCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
        """
        ...

class LRUCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        LRU Cache implementation (Least recently used policy).

        In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.

        Example::

            >>> cache = LRUCache(100) # fixed-size cache
            >>> cache = LRUCache(0) # unlimited-size cache
            >>> cache = LRUCache(100, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
        """
        ...

    def least_recently_used(self) -> typing.Optional[KT]: ...
    def most_recently_used(self) -> typing.Optional[KT]: ...

class TTLCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        ttl: float,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        TTL Cache implementation (Time-to-live policy).

        In simple terms, The TTL cache is one that evicts items that are older than a time-to-live.

        Example::

            >>> cache = TTLCache(100, 2) # fixed-size cache, 2 ttl value
            >>> cache = TTLCache(0, 10) # unlimited-size cache, 10 ttl value
            >>> cache = TTLCache(100, 5, {"key1": "value1", "key2": "value2"}) # initialize from dict or any iterable object
        """
        ...

    @property
    def ttl(self) -> float: ...
    
    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2})
            >>> cache.get_with_expire(1)
            (1, 1.23445675)
            >>> cache.get_with_expire("no-exists")
            (None, 0.0)
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2})
            >>> cache.pop_with_expire(1)
            (1, 1.23445675)
            >>> cache.pop_with_expire(1)
            (None, 0.0)
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float]:
        """
        Works like `.popitem()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2})
            >>> cache.popitem_with_expire()
            (1, 1, 1.23445675)
            >>> cache.popitem_with_expire()
            (2, 2, 1.94389545)
            >>> cache.popitem_with_expire()
            ...
            KeyError
        """
        ...

class VTTLCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union[typing.Iterable[typing.Tuple[KT, VT]], typing.Dict[KT, VT]] = ...,
        ttl: typing.Optional[float] = ...,
        *,
        capacity: int = ...,
    ) -> None:
        """
        VTTL Cache implementation (Time-to-live per-key policy)

        Works like TTLCache, with this different that each key has own time-to-live value.

        Example::

            >>> cache = VTTLCache(100) # fixed-size cache
            >>> cache = VTTLCache(0) # unlimited-size cache

            # initialize from dict or any iterable object;
            # also these items will expire after 5 seconds
            >>> cache = VTTLCache(100, {"key1": "value1", "key2": "value2"}, 5)

            # initialize from dict or any iterable object;
            # but these items never expire, because we pass None as them ttl value
            >>> cache = VTTLCache(100, {"key1": "value1", "key2": "value2"}, None)
        """
        ...

    def insert(self, key: KT, value: VT, ttl: typing.Optional[float]) -> None:
        """
        `.insert()` is different here. if you use `cache[key] = value` way, you cannot set ttl value for those item,
        but here you can.

        Example::

            >>> cache.insert("key", "value", 10) # this item will expire after 10 seconds
            >>> cache.insert("key", "value", None) # but this item never expire.
        """
        ...

    def setdefault(
        self, key: KT, default: DT = None, ttl: typing.Optional[float] = None
    ) -> typing.Union[VT, DT]:
        """
        Returns the value of the specified key.

        If the key does not exist, insert the key, with the specified value.
        """
        ...

    def update(self, iterable: typing.Iterable, ttl: typing.Optional[float] = None) -> None:
        """
        inserts the specified items to the cache. The `iterable` can be a dictionary,
        or an iterable object with key value pairs.

        Example::

            >>> cache = VTTLCache(20)
            >>> cache.insert("key", "value", 10)
            >>> cache.update({i:i for i in range(12)}, 2)
            >>> len(cache)
            13
            >>> time.sleep(2)
            >>> len(cache)
            1
        """
        ...

    def get_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.get()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2}, 2)
            >>> cache.get_with_expire(1)
            (1, 1.9934)
            >>> cache.get_with_expire("no-exists")
            (None, 0.0)
        """
        ...

    def pop_with_expire(
        self, key: KT, default: DT = None
    ) -> typing.Tuple[typing.Union[VT, DT], float]:
        """
        Works like `.pop()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2}, 2)
            >>> cache.pop_with_expire(1)
            (1, 1.99954)
            >>> cache.pop_with_expire(1)
            (None, 0.0)
        """
        ...

    def popitem_with_expire(self) -> typing.Tuple[VT, DT, float]:
        """
        Works like `.popitem()`, but also returns the remaining expiration.

        Example::

            >>> cache.update({1: 1, 2: 2}, 2)
            >>> cache.popitem_with_expire()
            (1, 1, 1.9786564)
            >>> cache.popitem_with_expire()
            (2, 2, 1.97389545)
            >>> cache.popitem_with_expire()
            ...
            KeyError
        """
        ...
