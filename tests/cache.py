import typing

import pytest

import cachebox

from . import mixins


class TestCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.SetDefaultMixin,
    mixins.PopAndDeleteMixin,
    mixins.UpdateMixin,
    mixins.IntrospectionMixin,
    mixins.IterationMixin,
    mixins.DrainClearShrinkMixin,
    mixins.CopyMixin,
    mixins.GetSizeOfMixin,
    mixins.EdgeCasesMixin,
    mixins.IssuesMixin,
    mixins.FuzzyMixin,
):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.BaseCacheImpl:
        return cachebox.Cache(maxsize, iterable, capacity=capacity, getsizeof=getsizeof)

    def test_popitem_overflow_error(self):
        cache = self.create_cache()

        # cachebox.Cache does not have any algorithm to use
        with pytest.raises(OverflowError):
            cache.popitem()

    def test_insert_overflow_error(self):
        cache = self.create_cache(5)

        for i in range(5):
            cache.insert(i, i)

        with pytest.raises(OverflowError):
            cache.insert(6, 6)

        cache.insert(4, "A")  # <- Replacing should be OK

        # Try again with custom getsizeof
        cache = self.create_cache(5, getsizeof=lambda k, v: len(k))
        cache.insert("AA", 1)
        cache.insert("BBB", 1)  # <- Now is full

        assert cache.is_full()

        with pytest.raises(OverflowError):
            cache.insert("NEW", 1)

        cache.insert("AA", "A")  # <- Replacing should be OK

    def test_update_overflow_error(self):
        with pytest.raises(OverflowError):
            self.create_cache(5, {i: i for i in range(6)})

        cache = self.create_cache(5)
        cache.update({i: i for i in range(5)})  # <- Now is full

        with pytest.raises(OverflowError):
            cache.insert(6, 6)

        with pytest.raises(OverflowError):
            cache.update({10: 10})

        # Replacing should be OK
        cache.update({i: i for i in range(5)})
