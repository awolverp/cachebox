from . import _core
import typing


KT = typing.TypeVar("KT")
VT = typing.TypeVar("VT")
DT = typing.TypeVar("DT")


def _items_to_str(items, length):
    if length <= 50:
        return "{" + ", ".join(f"{k}: {v}" for k, v in items) + "}"

    c = 0
    left = []

    while c < length:
        k, v = next(items)

        if c <= 50:
            left.append(f"{k}: {v}")

        else:
            break

        c += 1

    return "{%s, ... %d more ...}" % (", ".join(left), length - c)


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

    `Cache` vs `dict`:
    - it is thread-safe and unordered, while `dict` isn't thread-safe and ordered (Python 3.6+).
    - it uses very lower memory than `dict`.
    - it supports useful and new methods for managing memory, while `dict` does not.
    - it does not support popitem, while `dict` does.
    - You can limit the size of Cache, but you cannot for `dict`.
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

    def __sizeof__(self):  # pragma: no cover
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
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Removes specified key and return the corresponding value. If the key is not found, returns the `default`.
        """
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        """
        Inserts key with a value of default if key is not in the cache. Return the value for key if key is
        in the cache, else `default`.
        """
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.NoReturn:  # pragma: no cover
        raise NotImplementedError()

    def drain(self) -> typing.NoReturn:  # pragma: no cover
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
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, Cache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, Cache):
            return False  # pragma: no cover

        return self._raw != other._raw

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


class FIFOCache(BaseCacheImpl[KT, VT]):
    def __init__(
        self,
        maxsize: int,
        iterable: typing.Union["Cache", dict, tuple, typing.Generator, None] = None,
        *,
        capacity: int = 0,
    ) -> None:
        self._raw = _core.FIFOCache(maxsize, capacity=capacity)

        if iterable is not None:
            self.update(iterable)

    @property
    def maxsize(self) -> int:
        return self._raw.maxsize()

    def capacity(self) -> int:
        return self._raw.capacity()

    def __len__(self) -> int:
        return len(self._raw)

    def __sizeof__(self):  # pragma: no cover
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
        return self._raw.insert(key, value)

    def get(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            return default

    def pop(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        try:
            return self._raw.remove(key)
        except _core.CoreKeyError:
            return default

    def setdefault(self, key: KT, default: typing.Optional[DT] = None) -> typing.Union[VT, DT]:
        return self._raw.setdefault(key, default)

    def popitem(self) -> typing.Tuple[KT, VT]:
        try:
            return self._raw.popitem()
        except _core.CoreKeyError:
            raise KeyError() from None

    def drain(self, n: int) -> int:  # pragma: no cover
        if n == 0:
            return 0

        for i in range(n):
            try:
                self._raw.popitem()
            except _core.CoreKeyError:
                return i

        return i

    def update(self, iterable: typing.Union["Cache", dict, tuple, typing.Generator]) -> None:
        if hasattr(iterable, "items"):
            iterable = iterable.items()

        self._raw.update(iterable)

    def __setitem__(self, key: KT, value: VT) -> None:
        self.insert(key, value)

    def __getitem__(self, key: KT) -> VT:
        try:
            return self._raw.get(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __delitem__(self, key: KT) -> None:
        try:
            self._raw.remove(key)
        except _core.CoreKeyError:
            raise KeyError(key) from None

    def __eq__(self, other) -> bool:
        if not isinstance(other, FIFOCache):
            return False  # pragma: no cover

        return self._raw == other._raw

    def __ne__(self, other) -> bool:
        if not isinstance(other, FIFOCache):
            return False  # pragma: no cover

        return self._raw != other._raw

    def shrink_to_fit(self) -> None:
        self._raw.shrink_to_fit()

    def clear(self, *, reuse: bool = False) -> None:
        self._raw.clear(reuse)

    def items(self) -> IteratorView[typing.Tuple[KT, VT]]:
        return IteratorView(self._raw.items(), lambda x: x)

    def keys(self) -> IteratorView[KT]:
        return IteratorView(self._raw.items(), lambda x: x[0])

    def values(self) -> IteratorView[VT]:
        return IteratorView(self._raw.items(), lambda x: x[1])

    def first(self, n: int = 0) -> typing.Optional[KT]:
        if n < 0:
            n = len(self._raw) + n

        if n < 0:
            return None

        return self._raw.get_index(n)

    def last(self) -> typing.Optional[KT]:
        return self._raw.get_index(len(self._raw) - 1)

    def __iter__(self) -> IteratorView[KT]:
        return self.keys()

    def __repr__(self) -> str:
        return "{}[{}/{}]({})".format(
            type(self).__name__,
            len(self._raw),
            self._raw.maxsize(),
            _items_to_str(self._raw.items(), len(self._raw)),
        )
