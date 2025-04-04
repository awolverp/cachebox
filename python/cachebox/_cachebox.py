from . import _core
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")

_sential = object()


def _items_to_str(items, length, max_len=50):
    if length <= max_len:
        return "{" + ", ".join(f"{k}: {v}" for k, v in items) + "}"

    c = 0
    left = []
    right = []

    while c < length:
        k, v = next(items)

        if c <= 20:
            left.append(f"{k}: {v}")

        elif (length - c) <= 20:
            right.append(f"{k}: {v}")

        c += 1

    return "{" + ", ".join(left) + " ... truncated ... " + ", ".join(right) + "}"


class BaseCacheImpl(typing.Generic[KT, VT]):
    """
    This is the base class of all cache classes such as Cache, FIFOCache, ...
    """
    pass


class IteratorView(typing.Generic[VT]):
    __slots__ = ("iterator", "func")

    def __init__(self, iterator, func: typing.Callable[[tuple], typing.Any]):
        self.iterator = iterator
        self.func = func

    def __iter__(self):
        self.iterator = self.iterator.__iter__()
        return self

    def __next__(self) -> VT:
        return self.func(self.iterator.__next__())


class Cache(BaseCacheImpl[KT, VT]):
    """
    A simple cache that has no algorithm; this is only a hashmap.

    Cache vs dict:

    it is thread-safe and unordered, while dict isn't thread-safe and ordered (Python 3.6+).
    it uses very lower memory than dict.
    it supports useful and new methods for managing memory, while dict does not.
    it does not support popitem, while dict does.
    You can limit the size of Cache, but you cannot for dict.
    """

    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union["Cache", dict, tuple, typing.Generator, None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        """
        A simple cache that has no algorithm; this is only a hashmap.

        :param maxsize: you can specify the limit size of the cache ( zero means infinity ); this is unchangable.

        :param iterable: you can create cache from a dict or an iterable.

        :param capacity: If `capacity` param is given, cache attempts to allocate a new hash table with at
        least enough capacity for inserting the given number of elements without reallocating.
        """
        self._raw = _core.Cache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        """Returns the number of elements the map can hold without reallocating."""
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):
        return self._raw.__sizeof__()

    def __contains__(self, key: KT) -> bool:
        return key in self._raw

    def __bool__(self) -> bool:
        return not self.is_empty()

    def is_empty(self) -> bool:
        return self._raw.is_empty()

    def is_full(self) -> bool:
        return self._raw.is_full()

    def insert(self, key: KT, value: VT) -> typing.Optional[VT]:
        """
        Equals to `self[key] = value`, but returns a value:

        - If the cache did not have this key present, None is returned.
        - If the cache did have this key present, the value is updated,
          and the old value is returned. The key is not updated, though;

        Note: raises `OverflowError` if the cache reached the maxsize limit,
        because this class does not have any algorithm.
        """
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """Equals to `self[key]`, but returns `default` if the cache don't have this key present."""
        return self._raw.get(key, default)

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        return self._raw.pop(key, default)

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache. Return the value for key if key is
        in the cache, else `default`.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.NoReturn:
        raise NotImplementedError()

    def drain(self) -> typing.NoReturn:
        raise NotImplementedError()

    def update(self, iterable: typing.Union["Cache", dict, tuple, typing.Generator]) -> None:
        """
        Updates the cache with elements from a dictionary or an iterable object of key/value pairs.

        Note: raises `OverflowError` if the cache reached the maxsize limit.
        """
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        val = self.get(key, _sential)
        if val is _sential:
            raise KeyError(key)

        return val

    def __delitem__(self, key: KT) -> None:
        self._raw.remove(key)

    def __eq__(self, other) -> bool:
        return self._raw == other

    def __ne__(self, other) -> bool:
        return self._raw != other

    def shrink_to_fit(self) -> None:
        """Shrinks the cache to fit len(self) elements."""
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        """
        Removes all items from cache.

        If reuse is True, will not free the memory for reusing in the future.
        """
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        """
        Returns an iterable object of the cache's items (key-value pairs).

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Items are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        """
        Returns an iterable object of the cache's keys.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Keys are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        """
        Returns an iterable object of the cache's values.

        Notes:
        - You should not make any changes in cache while using this iterable object.
        - Values are not ordered.
        """
        return IteratorView(self._raw.items(), lambda x: x[1])

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        return "{}[{}/{}]({})".format(
            type(self).__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )
