import dataclasses
import sys
import typing

import pytest
from hypothesis import assume, given
from hypothesis import strategies as st

import cachebox

# Strategy for keys that are hashable (str, int, tuple of ints)
hashable_keys = st.one_of(
    st.text(),
    st.integers(),
    st.floats(allow_nan=False),
    st.decimals(allow_nan=False),
    st.tuples(st.integers(), st.integers()),
)

# Strategy for arbitrary values
any_value = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(),
    st.floats(allow_nan=False),
    st.text(),
    st.binary(),
    st.lists(st.integers(), max_size=5),
)


class BaseMixin:
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.BaseCacheImpl:
        raise NotImplementedError


class InitializeMixin(BaseMixin):
    def test_empty_on_creation(self):
        cache = self.create_cache()
        assert len(cache) == 0

    def test_maxsize_stored(self):
        cache = self.create_cache()
        assert cache.maxsize == 10

    def test_maxsize_zero_means_unlimited(self):
        cache = self.create_cache(0)
        assert cache.maxsize == sys.maxsize

    def test_init_from_dict(self):
        c = self.create_cache(maxsize=10, iterable={"a": 1, "b": 2})
        assert c.get("a") == 1
        assert c.get("b") == 2
        assert len(c) == 2

    def test_init_from_list_of_tuples(self):
        c = self.create_cache(maxsize=10, iterable=[("x", 10), ("y", 20)])
        assert c.get("x") == 10
        assert c.get("y") == 20

    def test_init_from_other_cache(self):
        iterable = self.create_cache(maxsize=10, iterable=[("x", 10), ("y", 20)])

        c = self.create_cache(maxsize=10, iterable=iterable)
        assert c.get("x") == 10
        assert c.get("y") == 20

    def test_capacity_param(self):
        c = self.create_cache(maxsize=10, capacity=10)
        assert c.capacity() >= 10

    def test_getsizeof_stored(self):
        sizer = lambda k, v: len(v)  # noqa: E731

        c = self.create_cache(maxsize=100, getsizeof=sizer)
        assert c.getsizeof is sizer


class InsertAndGetMixin(BaseMixin):
    def test_insert_returns_none_on_new_key(self):
        cache = self.create_cache()

        result = cache.insert("k", "v")
        assert result is None

    def test_insert_returns_old_value_on_update(self):
        cache = self.create_cache()

        cache.insert("k", "v1")
        result = cache.insert("k", "v2")
        assert result == "v1"

    def test_get_existing_key(self):
        cache = self.create_cache()

        cache.insert("k", 42)
        assert cache.get("k") == 42

    def test_get_missing_key_returns_none(self):
        cache = self.create_cache()

        assert cache.get("nope") is None

    def test_get_missing_key_returns_custom_default(self):
        cache = self.create_cache()

        assert cache.get("nope", "fallback") == "fallback"

    def test_setitem_getitem(self):
        cache = self.create_cache()

        cache["k"] = "v"
        assert cache["k"] == "v"

    def test_getitem_missing_raises_keyerror(self):
        cache = self.create_cache()

        with pytest.raises(KeyError):
            _ = cache["ghost"]

    def test_none_value_stored_correctly(self):
        cache = self.create_cache()

        cache.insert("k", None)
        # None value is present — default should NOT be returned
        assert cache.get("k", "MISS") is None

    def test_overwrite_keeps_len_unchanged(self):
        cache = self.create_cache()

        cache.insert("k", 1)
        cache.insert("k", 2)
        assert len(cache) == 1

    def test_insert_get_raw_type(self):
        class AType:
            pass

        cache = self.create_cache()
        cache[AType] = AType
        assert cache[AType] is AType


class PopitemMixin(BaseMixin):
    def test_popitem_raises_keyerror(self):
        cache = self.create_cache()

        with pytest.raises(KeyError):
            cache.popitem()

    def test_popitem_updates_currsize(self):
        cache = self.create_cache(10, {i: i for i in range(20)})

        assert cache.is_full()
        assert cache.remaining_size() == 0
        assert cache.current_size() == 10
        assert len(cache) == 10

        cache.popitem()

        assert not cache.is_full()
        assert cache.remaining_size() == 1
        assert cache.current_size() == 9
        assert len(cache) == 9


class SetDefaultMixin(BaseMixin):
    def test_setdefault_inserts_when_absent(self):
        cache = self.create_cache()

        result = cache.setdefault("k", "default")
        assert result == "default"
        assert cache.get("k") == "default"

    def test_setdefault_returns_existing_value(self):
        cache = self.create_cache()

        cache.insert("k", "existing")
        result = cache.setdefault("k", "default")
        assert result == "existing"
        assert cache.get("k") == "existing"


class PopAndDeleteMixin(BaseMixin):
    def test_pop_existing_key(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        result = cache.pop("k")
        assert result == "v"
        assert cache.get("k") is None

    def test_pop_missing_key_with_default(self):
        cache = self.create_cache()

        assert cache.pop("ghost", "default") == "default"

    def test_pop_missing_key_raises_keyerror(self):
        cache = self.create_cache()

        with pytest.raises(KeyError):
            cache.pop("ghost")

    def test_delitem_existing_key(self):
        cache = self.create_cache()

        cache["k"] = "v"
        del cache["k"]
        assert cache.get("k") is None

    def test_delitem_missing_key_raises_keyerror(self):
        cache = self.create_cache()

        with pytest.raises(KeyError):
            del cache["ghost"]


class UpdateMixin(BaseMixin):
    def test_update_from_dict(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2})
        assert cache.get("a") == 1
        assert cache.get("b") == 2

    def test_update_from_other(self):
        iterable = self.create_cache(10, ((str(i), i) for i in range(10)))
        cache = self.create_cache()

        cache.update(iterable)
        for i in range(10):
            assert cache.get(str(i)) == i

    def test_update_from_list_of_tuples(self):
        cache = self.create_cache()

        cache.update([("x", 10), ("y", 20)])
        assert cache.get("x") == 10
        assert cache.get("y") == 20

    def test_update_overwrites_existing(self):
        cache = self.create_cache()

        cache.insert("a", 1)
        cache.update({"a": 99})
        assert cache.get("a") == 99

    def test_update_invalid_argument(self):
        cache = self.create_cache()

        with pytest.raises(TypeError):
            cache.update("abc")  # type: ignore

        with pytest.raises(TypeError):
            cache.update({1, 2, 3})  # type: ignore

        class _invalid_items:
            def items(self):
                return [1, 2, 3]

        with pytest.raises(TypeError):
            cache.update(_invalid_items())  # type: ignore


class IntrospectionMixin(BaseMixin):
    def test_len_reflects_insertions(self):
        cache = self.create_cache()

        assert len(cache) == 0
        cache.insert("a", 1)
        assert len(cache) == 1
        cache.insert("b", 2)
        assert len(cache) == 2

    def test_current_size_equals_len_without_getsizeof(self):
        cache = self.create_cache()

        cache.insert("a", 1)
        cache.insert("b", 2)
        assert cache.current_size() == len(cache)

    def test_remaining_size(self):
        cache = self.create_cache()

        cache.insert("a", 1)
        assert cache.remaining_size() == cache.maxsize - cache.current_size()

    def test_is_empty_on_new_cache(self):
        cache = self.create_cache()

        assert cache.is_empty()

    def test_is_not_empty_after_insert(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        assert not cache.is_empty()

    def test_bool_false_when_empty(self):
        cache = self.create_cache()

        assert not bool(cache)

    def test_bool_true_when_not_empty(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        assert bool(cache)

    def test_contains_operator(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        assert "k" in cache
        assert "ghost" not in cache

    def test_contains_method(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        assert cache.contains("k")
        assert not cache.contains("ghost")

    def test_repr_string(self):
        cache = self.create_cache()

        cache.insert("k", "v")
        out = repr(cache)

        assert isinstance(out, str)
        assert type(cache).__name__ in out

    def test_eq_same_contents(self):
        c1 = self.create_cache(maxsize=10, iterable={"a": 1})
        c2 = self.create_cache(maxsize=10, iterable={"a": 1})
        assert c1 == c2

    def test_ne_different_contents(self):
        c1 = self.create_cache(maxsize=10, iterable={"a": 1})
        c2 = self.create_cache(maxsize=10, iterable={"b": 2})
        assert c1 != c2


class IterationMixin(BaseMixin):
    def test_keys_returns_all_keys(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2, "c": 3})
        assert set(cache.keys()) == {"a", "b", "c"}

    def test_values_returns_all_values(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2, "c": 3})
        assert set(cache.values()) == {1, 2, 3}

    def test_items_returns_all_pairs(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2})
        assert set(cache.items()) == {("a", 1), ("b", 2)}

    def test_iter_yields_keys(self):
        cache = self.create_cache()

        cache.update({"x": 10, "y": 20})
        assert set(iter(cache)) == {"x", "y"}

    # TODO: test generation version


class DrainClearShrinkMixin(BaseMixin):
    def test_clear_removes_all_items(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2})
        cache.clear()
        assert len(cache) == 0
        assert cache.is_empty()
        assert cache.current_size() == 0

    def test_clear_with_reuse(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2})
        cache.clear(reuse=True)
        assert len(cache) == 0

    def test_items_accessible_after_clear_and_reinsert(self):
        cache = self.create_cache()

        cache.insert("a", 1)
        cache.clear()
        cache.insert("b", 2)
        assert cache.get("b") == 2
        assert cache.get("a") is None

    def test_shrink_to_fit_does_not_lose_data(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2, "c": 3})
        cache.shrink_to_fit()
        assert cache.get("a") == 1
        assert cache.get("b") == 2
        assert cache.get("c") == 3


class CopyMixin(BaseMixin):
    def test_copy_has_same_items(self):
        cache = self.create_cache()

        cache.update({"a": 1, "b": 2})
        c2 = cache.copy()
        assert set(c2.items()) == set(cache.items())

    def test_copy_is_independent(self):
        cache = self.create_cache()

        cache.insert("a", 1)
        c2 = cache.copy()
        c2.insert("b", 2)
        assert not cache.contains("b")

    def test_copy_preserves_maxsize(self):
        cache = self.create_cache()

        c2 = cache.copy()
        assert c2.maxsize == cache.maxsize


@dataclasses.dataclass
class Sized:
    size: int
    key: typing.Any

    def __hash__(self) -> int:
        return hash(self.key)

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Sized) and self.key == other.key


class GetSizeOfMixin(BaseMixin):
    def test_current_size_uses_getsizeof(self):
        # Each value is a list; size = len(value)
        sizer = lambda k, v: len(v)  # noqa: E731

        c = self.create_cache(maxsize=10, getsizeof=sizer)
        c.insert("a", [1, 2, 3])  # size 3
        c.insert("b", [1])  # size 1
        assert c.current_size() == 4

    def test_overflow_based_on_weighted_size(self):
        # maxsize=5; each entry costs its value
        sizer = lambda k, v: v  # noqa: E731

        c = self.create_cache(maxsize=5, getsizeof=sizer)
        c.insert("a", 3)  # size now 3
        c.insert("b", 2)  # size now 5 — full

        if isinstance(c, cachebox.Cache):
            with pytest.raises(OverflowError):
                c.insert("c", 1)  # would push to 6

    def test_getsizeof_invalid_handle_size(self):
        c = self.create_cache(maxsize=5, getsizeof=lambda x, _: len(x))

        with pytest.raises(OverflowError):
            c["more than 5"] = 1

        with pytest.raises(OverflowError):
            c.update({"more than 5": 1})

        with pytest.raises(OverflowError):
            c.update({"5": 1, "more than 5": 2})

        assert "5" in c

    def test_getsizeof_insert_enforced(self):
        c = self.create_cache(maxsize=100, getsizeof=lambda x, v: x.size + v.size)

        k1 = Sized(10, 1)
        v1 = Sized(80, 101)
        c[k1] = v1

        k2 = Sized(10, 2)
        v2 = Sized(80, 102)

        if isinstance(c, cachebox.Cache):
            with pytest.raises(OverflowError):
                c[k2] = v2

            assert k1 in c

        else:
            c[k2] = v2
            assert k1 not in c
            assert k2 in c
            assert c.current_size() <= c.maxsize

    def test_getsizeof_insert_existing_key_enforced(self):
        c = self.create_cache(maxsize=100, getsizeof=lambda x, _: x.size)

        a_size_10 = Sized(10, "A")
        a_size_100 = Sized(100, "A")

        b_size_10 = Sized(10, "B")

        c[a_size_10] = 1
        c[b_size_10] = 2

        # A(10) -> currsize=10
        # B(10) -> currsize=20
        #
        # A(100) -> currsize=110 - exceeded maxsize, should call evict
        if isinstance(c, cachebox.Cache):
            with pytest.raises(OverflowError):
                c[a_size_100] = "new"

            return

        c[a_size_100] = "new"


class EdgeCasesMixin(BaseMixin):
    def test_integer_keys(self):
        cache = self.create_cache()

        cache.insert(1, "one")
        assert cache.get(1) == "one"

    def test_tuple_keys(self):
        cache = self.create_cache()

        cache.insert((1, 2), "tuple")
        assert cache.get((1, 2)) == "tuple"

    def test_empty_string_key_and_value(self):
        cache = self.create_cache()

        cache.insert("", "")
        assert cache.get("") == ""

    def test_large_value(self):
        unlimited = self.create_cache(0)

        big = "x" * 100_000
        unlimited.insert("big", big)
        assert unlimited.get("big") == big

    def test_multiple_types_as_values(self):
        cache = self.create_cache()

        cache.insert("int", 1)
        cache.insert("list", [1, 2])
        cache.insert("dict", {"a": 1})
        assert cache.get("int") == 1
        assert cache.get("list") == [1, 2]
        assert cache.get("dict") == {"a": 1}

    def test_bad_hash_key(self):

        @dataclasses.dataclass
        class BadHash:
            val: int

            def __hash__(self) -> int:
                return 1

        size = 1000
        cache = self.create_cache(size, capacity=size)

        for i in range(size):
            cache.insert(BadHash(val=i), i)
            cache.get(BadHash(val=i))


class IssuesMixin(BaseMixin):
    def test_issue_5(self):
        # https://github.com/awolverp/cachebox/issues/5

        @dataclasses.dataclass
        class EQ:
            val: int

            def __hash__(self) -> int:
                return self.val

        @dataclasses.dataclass
        class NoEQ:
            val: int

            def __hash__(self) -> int:
                return self.val

        size = 1000
        cache = self.create_cache(size, capacity=size)

        for i in range(size):
            cache.insert(NoEQ(val=i), i)
            cache.get(NoEQ(val=i))

        cache = self.create_cache(size, capacity=size)

        for i in range(size):
            cache.insert(EQ(val=i), i)
            cache.get(EQ(val=i))


class FuzzyMixin(BaseMixin):
    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_insert_then_get_returns_same_value(self, key, value):
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        assert c.get(key) == value

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_insert_new_key_returns_none(self, key, value):
        c = self.create_cache(maxsize=0)
        result = c.insert(key, value)
        assert result is None

    @given(key=hashable_keys, v1=any_value, v2=any_value)
    def test_fuzzy_insert_existing_key_returns_old_value(self, key, v1, v2):
        c = self.create_cache(maxsize=0)
        c.insert(key, v1)
        old = c.insert(key, v2)
        assert old == v1

    @given(pairs=st.lists(st.tuples(hashable_keys, any_value), max_size=20))
    def test_fuzzy_len_never_exceeds_unique_keys(self, pairs):
        c = self.create_cache(maxsize=0)
        expected = {}
        for k, v in pairs:
            c.insert(k, v)
            expected[k] = v
        assert len(c) == len(expected)

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_len_increases_by_one_on_new_key(self, key, value):
        c = self.create_cache(maxsize=0)
        before = len(c)
        c.insert(key, value)
        assert len(c) == before + 1

    @given(key=hashable_keys, v1=any_value, v2=any_value)
    def test_fuzzy_len_unchanged_on_overwrite(self, key, v1, v2):
        c = self.create_cache(maxsize=0)
        c.insert(key, v1)
        before = len(c)
        c.insert(key, v2)
        assert len(c) == before

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_contains_true_after_insert(self, key, value):
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        assert key in c
        assert c.contains(key)

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_contains_false_after_delete(self, key, value):
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        del c[key]
        assert key not in c

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_pop_returns_inserted_value(self, key, value):
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        assert c.pop(key) == value

    @given(key=hashable_keys, value=any_value)
    def test_fuzzy_pop_removes_key(self, key, value):
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        c.pop(key)
        assert key not in c

    @given(
        maxsize=st.integers(min_value=1, max_value=50),
        pairs=st.lists(st.tuples(hashable_keys, any_value), max_size=50),
    )
    def test_fuzzy_current_size_plus_remaining_equals_maxsize(self, maxsize, pairs):
        c = self.create_cache(maxsize=maxsize)
        for k, v in pairs:
            if c.is_full():
                break
            c.insert(k, v)
        assert c.current_size() + c.remaining_size() == maxsize

    @given(pairs=st.lists(st.tuples(hashable_keys, any_value), max_size=20))
    def test_fuzzy_clear_always_leaves_cache_empty(self, pairs):
        c = self.create_cache(maxsize=0)
        for k, v in pairs:
            c.insert(k, v)
        c.clear()
        assert len(c) == 0
        assert c.is_empty()

    @given(pairs=st.lists(st.tuples(hashable_keys, any_value), max_size=20))
    def test_fuzzy_keys_values_items_are_consistent(self, pairs):
        c = self.create_cache(maxsize=0)
        truth = {}
        for k, v in pairs:
            c.insert(k, v)
            truth[k] = v

        cache_items = dict(c.items())
        assert cache_items == truth
        assert set(c.keys()) == set(truth.keys())
        assert sorted(str(v) for v in c.values()) == sorted(
            str(v) for v in truth.values()
        )

    @given(key=hashable_keys, existing=any_value, default=any_value)
    def test_fuzzy_setdefault_never_overwrites_existing(self, key, existing, default):
        c = self.create_cache(maxsize=0)
        c.insert(key, existing)
        c.setdefault(key, default)
        assert c.get(key) == existing

    @given(key=hashable_keys, default=any_value)
    def test_fuzzy_setdefault_inserts_when_missing(self, key, default):
        c = self.create_cache(maxsize=0)
        c.setdefault(key, default)
        assert c.get(key) == default

    @given(pairs=st.lists(st.tuples(hashable_keys, any_value), max_size=20))
    def test_fuzzy_copy_equals_original(self, pairs):
        c = self.create_cache(maxsize=0)
        for k, v in pairs:
            c.insert(k, v)
        assert c.copy() == c

    @given(
        key=hashable_keys, value=any_value, new_key=hashable_keys, new_value=any_value
    )
    def test_fuzzy_copy_is_independent_of_original(
        self, key, value, new_key, new_value
    ):
        assume(new_key != key)
        c = self.create_cache(maxsize=0)
        c.insert(key, value)
        c2 = c.copy()
        c2.insert(new_key, new_value)
        assert not c.contains(new_key)
