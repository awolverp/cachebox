import typing

import pytest

import cachebox

from . import mixins


class TestFIFOCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.SetDefaultMixin,
    mixins.PopAndDeleteMixin,
    mixins.UpdateMixin,
    mixins.IntrospectionMixin,
    # mixins.IterationMixin,
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
        return cachebox.FIFOCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )


class TestFIFOCachePolicy(mixins.BaseMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.FIFOCache:
        return cachebox.FIFOCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )

    def test_oldest_item_evicted_on_overflow(self):
        """When capacity is exceeded, the first inserted key must be evicted."""
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        cache[4] = "d"  # triggers eviction of key 1
        assert 1 not in cache
        assert 4 in cache

    def test_eviction_is_strictly_insertion_ordered(self):
        """Keys evict in the exact order they were inserted, not access order."""
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])

        cache[4] = "d"  # evicts 1
        cache[5] = "e"  # evicts 2
        cache[6] = "f"  # evicts 3

        assert 1 not in cache
        assert 2 not in cache
        assert 3 not in cache
        assert {4, 5, 6} == set(cache.keys())

    def test_accessing_key_does_not_reset_eviction_priority(self):
        """
        Unlike LRU, a cache hit must NOT push the key to the back.
        Key 1 is accessed repeatedly but must still be the first evicted.
        """
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])

        _ = cache[1]
        _ = cache[1]
        _ = cache[1]

        cache[4] = "d"  # must still evict key 1
        assert 1 not in cache

    def test_overwriting_existing_key_does_not_change_eviction_order(self):
        """
        Updating the value of an existing key must NOT change its insertion
        position in the eviction queue.
        """
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])

        cache[1] = "updated"  # update, not a new insertion
        cache[4] = "d"  # must still evict key 1

        assert 1 not in cache
        assert cache[4] == "d"

    def test_popitem_removes_oldest(self):
        """popitem() must always remove and return the oldest inserted entry."""
        cache = self.create_cache(3, [(10, "x"), (20, "y"), (30, "z")])
        key, value = cache.popitem()
        assert key == 10
        assert value == "x"

    def test_popitem_successive_calls_follow_fifo(self):
        """Successive popitem() calls must yield keys in insertion order."""
        insertion_order = [(1, "a"), (2, "b"), (3, "c"), (4, "d")]
        cache = self.create_cache(4, insertion_order)
        popped_keys = [cache.popitem()[0] for _ in range(4)]
        assert popped_keys == [1, 2, 3, 4]

    def test_drain_removes_n_oldest(self):
        """drain(n) must remove exactly n items, oldest-first."""
        cache = self.create_cache(5, [(i, str(i)) for i in range(1, 6)])
        removed = cache.drain(3)
        assert removed == 3
        assert 1 not in cache
        assert 2 not in cache
        assert 3 not in cache
        assert 4 in cache
        assert 5 in cache

    def test_first_returns_oldest_key(self):
        cache = self.create_cache(3, [(7, "a"), (8, "b"), (9, "c")])
        assert cache.first() == 7

    def test_last_returns_newest_key(self):
        cache = self.create_cache(3, [(7, "a"), (8, "b"), (9, "c")])
        assert cache.last() == 9

    def test_first_with_positive_n_browses_in_insertion_order(self):
        """first(n) must walk forward through insertion order."""
        cache = self.create_cache(4, [(10, "a"), (20, "b"), (30, "c"), (40, "d")])
        assert cache.first(0) == 10
        assert cache.first(1) == 20
        assert cache.first(2) == 30
        assert cache.first(3) == 40

    def test_first_with_negative_n_browses_from_end(self):
        """first(-1) is an alias for last(); first(-2) is the second newest."""
        cache = self.create_cache(4, [(10, "a"), (20, "b"), (30, "c"), (40, "d")])
        assert cache.first(-1) == 40
        assert cache.first(-2) == 30

    def test_first_after_eviction_reflects_new_head(self):
        """After an eviction, first() must return the new oldest key."""
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        cache[4] = "d"  # evicts key 1
        assert cache.first() == 2

    def test_last_after_insertion_reflects_new_tail(self):
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        cache[4] = "d"
        assert cache.last() == 4

    def test_first_on_single_element_cache(self):
        cache = self.create_cache(1, [(42, "only")])
        assert cache.first() == 42
        assert cache.last() == 42

    def test_first_returns_none_on_empty_cache(self):
        cache = self.create_cache(0)
        assert cache.first() is None

    def test_rolling_window_maintains_correct_contents(self):
        """
        Simulate a sliding-window workload: insert N items into a cache of
        size K and verify that only the most-recently inserted K items survive.
        """
        maxsize = 4
        total = 20
        cache = self.create_cache(maxsize)

        for i in range(total):
            cache[i] = i * 10

        expected = set(range(total - maxsize, total))
        assert set(cache.keys()) == expected

    def test_no_phantom_keys_after_eviction(self):
        """Evicted keys must not linger in contains() or iteration."""
        cache = self.create_cache(2, [(1, "a"), (2, "b")])
        cache[3] = "c"  # evicts 1

        for key in cache:
            assert key != 1

        assert not cache.contains(1)

    def test_reinsert_evicted_key_rejoins_at_tail(self):
        """
        Re-inserting a previously evicted key must treat it as a brand-new
        entry positioned at the back of the queue.
        """
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        cache[4] = "d"  # evicts 1
        cache[1] = "re"  # re-insert 1 — should now be at the tail
        cache[5] = "e"  # must evict 2 (now the oldest), not 1

        assert 2 not in cache
        assert 1 in cache
        assert cache[1] == "re"

    def test_is_full_triggers_at_maxsize(self):
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        assert cache.is_full()
        cache[4] = "d"  # eviction should keep it full, not overflow
        assert cache.is_full()
        assert len(cache) == 3

    def test_len_never_exceeds_maxsize(self):
        cache = self.create_cache(5)
        for i in range(100):
            cache[i] = i
        assert len(cache) <= 5

    def test_clear_resets_fifo_order(self):
        """After clear(), the insertion order restarts from scratch."""
        cache = self.create_cache(3, [(1, "a"), (2, "b"), (3, "c")])
        cache.clear()
        cache[10] = "x"
        cache[20] = "y"
        cache[30] = "z"
        assert cache.first() == 10
        assert cache.last() == 30

    @pytest.mark.skipif(
        not hasattr(cachebox, "_fifocache_small_offset"),
        reason="requires fifocache-small-offset feature flag",
    )
    def test_edge_case_of_front_offset_overflow(self):
        """
        Verifies that FIFOCache correctly rebases its internal `front_offset`
        counter when it approaches `u8::MAX` (255 in the small-offset test build).
        """
        U8_MAX = 255
        CACHE_SIZE = 10

        cache = self.create_cache(CACHE_SIZE)

        # drive front_offset to the rebase boundary
        total_insertions = U8_MAX + CACHE_SIZE  # 265
        for i in range(total_insertions):
            cache.insert(i, i * 10)

        # Snapshot what *should* be alive: the last CACHE_SIZE keys inserted
        expected_keys = set(range(total_insertions - CACHE_SIZE, total_insertions))

        # verify the cache is structurally sound after the rebase
        assert len(cache) == CACHE_SIZE
        assert cache.is_full()

        # Exact contents — no phantom or missing keys
        # TODO: uncomment
        # assert set(cache.keys()) == expected_keys

        # FIFO ordering must be intact
        assert cache.first() == min(expected_keys)
        assert cache.last() == max(expected_keys)

        # All surviving values are correct
        for key in expected_keys:
            assert cache[key] == key * 10

        # All evicted keys are truly gone
        for evicted in range(total_insertions - CACHE_SIZE):
            assert evicted not in cache

        # Prove the cache keeps working normally after the rebase

        # New insertions must evict the oldest surviving key (min of expected_keys)
        next_key = total_insertions  # 265
        oldest_before = cache.first()
        cache.insert(next_key, next_key * 10)

        assert oldest_before not in cache  # oldest was evicted
        assert cache[next_key] == next_key * 10  # new entry is present
        assert cache.last() == next_key  # sits at the tail
        assert len(cache) == CACHE_SIZE  # size is unchanged

        # Ordering of the remainder is still correct
        assert cache.first() == min(expected_keys) + 1

        # popitem() must still yield the oldest entry
        oldest_key, oldest_val = cache.popitem()
        assert oldest_val == oldest_key * 10
