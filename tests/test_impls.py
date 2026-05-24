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


class TestFIFOCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.PopitemMixin,
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

    def test_first_raise_indexerror_on_empty_cache(self):
        cache = self.create_cache(0)

        with pytest.raises(IndexError):
            cache.first()

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
        reason="requires small-offset feature flag",
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
        assert set(cache.keys()) == expected_keys

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


class TestRRCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.PopitemMixin,
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
    ) -> cachebox.RRCache:
        return cachebox.RRCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )

    def test_random_key_method(self):
        cache = self.create_cache(10)

        with pytest.raises(KeyError):
            cache.random_key()

        cache["a"] = 1
        assert cache.random_key() == "a"

        cache["b"] = 2
        cache["c"] = 3
        cache["d"] = 4
        assert cache.random_key() in ("a", "b", "c", "d")


class TestLRUCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.PopitemMixin,
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
    ) -> cachebox.LRUCache:
        return cachebox.LRUCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )


class TestLRUCachePolicy(mixins.BaseMixin):
    def create_cache(
        self,
        maxsize: int = 10,
        iterable: typing.Any = None,
        capacity: int = 0,
        getsizeof: typing.Any = None,
    ) -> cachebox.LRUCache:
        return cachebox.LRUCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )

    def test_evicts_lru_when_full(self):
        c = self.create_cache(3, {"a": 1, "b": 2, "c": 3})
        c.insert("d", 4)
        assert "a" not in c
        assert "d" in c

        c = self.create_cache(3, {"a": 1, "b": 2, "c": 3})
        c.insert("a", 1)
        c.insert("b", 2)
        c.insert("c", 3)
        c.insert("d", 4)
        assert "a" not in c
        assert "d" in c

    def test_does_not_evict_recently_read_key(self):
        c = self.create_cache(3)
        c.insert("a", 1)
        c.insert("b", 2)
        c.insert("c", 3)
        _ = c["a"]  # promote "a" → "b" becomes LRU
        c.insert("d", 4)
        assert "b" not in c
        assert "a" in c

    def test_reinserting_existing_key_promotes_it(self):
        c = self.create_cache(3, [("a", 1), ("b", 2), ("c", 3)])
        c.insert("a", 99)  # "a" was LRU, now MRU
        c.insert("d", 4)  # should evict "b", not "a"
        assert "a" in c
        assert "b" not in c

    def test_cache_never_exceeds_maxsize(self):
        c = self.create_cache(5)
        for i in range(20):
            c.insert(i, i)
            assert len(c) <= 5

    def test_sequential_inserts_keep_only_latest(self):
        c = self.create_cache(3)
        for i in range(6):
            c.insert(i, i)

        for k in range(3):
            assert k not in c

        for k in range(3, 6):
            assert k in c

    def test_update_evicts_lru_to_make_room(self):
        c = self.create_cache(3)
        c.insert("a", 1)
        c.insert("b", 2)
        c.insert("c", 3)
        c.update({"d": 4})
        assert "a" not in c

    def test_update_existing_key_promotes_it(self):
        c = self.create_cache(3, [("a", 1), ("b", 2), ("c", 3)])
        c.update({"a": 99})  # "a" was LRU, now MRU
        c.update({"d": 4})  # should evict "b"
        assert "a" in c
        assert "b" not in c

    def test_lru_and_mru_key_methods(self):
        c = self.create_cache(3)
        c.insert("a", 1)

        assert c.least_recently_used() == "a"
        assert c.most_recently_used() == "a"

        c.insert("b", 2)
        c.insert("c", 3)

        assert c.least_recently_used() == "a"
        assert c.most_recently_used() == "c"

        _ = c["a"]  # promote "a"

        assert c.least_recently_used() == "b"
        assert c.most_recently_used() == "a"

        assert "b" in c  # promote "b"

        assert c.least_recently_used() == "c"
        assert c.most_recently_used() == "b"

    def test_setdefault_on_existing_key_promotes_it(self):
        c = self.create_cache(0, [("a", 1), ("b", 2), ("c", 3)])
        c.setdefault("a", 0)
        assert c.most_recently_used() == "a"

    def test_lru_mru_empty_raises(self):
        with pytest.raises(KeyError):
            self.create_cache(5).least_recently_used()

        with pytest.raises(KeyError):
            self.create_cache(5).most_recently_used()

    def test_removes_least_recently_used(self):
        c = self.create_cache(0, [("a", 1), ("b", 2), ("c", 3)])
        key, val = c.popitem()
        assert key == "a"
        assert val == 1
        assert "a" not in c

    def test_order_after_read(self):
        c = self.create_cache(0, [("a", 1), ("b", 2), ("c", 3)])
        _ = c["a"]  # "a" now MRU → "b" is LRU
        key, _ = c.popitem()
        assert key == "b"

    def test_order_after_reinsert(self):
        c = self.create_cache(0, [("a", 1), ("b", 2), ("c", 3)])
        c.insert("a", 99)  # "a" now MRU → "b" is LRU
        key, _ = c.popitem()
        assert key == "b"

    def test_repeated_popitem_respects_lru_order(self):
        c = self.create_cache(5)
        for i in range(5):
            c.insert(i, i * 10)

        for expected in range(5):
            key, _ = c.popitem()
            assert key == expected

    def test_empty_raises(self):
        with pytest.raises(KeyError):
            self.create_cache(5).popitem()

    def test_hot_key_never_evicted(self):
        c = self.create_cache(3)
        c.insert("hot", 0)
        for i in range(20):
            _ = c.get("hot")
            c.insert(f"cold_{i}", i)

        assert "hot" in c

    def test_mixed_reads_and_writes_evict_correctly(self):
        c = self.create_cache(4)
        c.insert("a", 1)
        c.insert("b", 2)
        c.insert("c", 3)
        c.insert("d", 4)
        _ = c["a"]  # order: b, c, d, a
        _ = c["c"]  # order: b, d, a, c
        c.insert("e", 5)  # evicts "b"
        assert "b" not in c
        c.insert("f", 6)  # evicts "d"
        assert "d" not in c

    def test_peek_existing_key(self):
        cache = self.create_cache()

        cache.insert("k", 42)
        assert cache.peek("k") == 42

    def test_peek_missing_key_returns_none(self):
        cache = self.create_cache()

        assert cache.peek("nope") is None

    def test_peek_missing_key_returns_custom_default(self):
        cache = self.create_cache()

        assert cache.peek("nope", "fallback") == "fallback"

    def test_peek_no_promote_key(self):
        c = self.create_cache(3)
        c.insert("a", 1)
        c.insert("b", 2)
        c.insert("c", 3)

        assert c.least_recently_used() == "a"
        assert c.most_recently_used() == "c"

        c.peek("a")

        assert c.least_recently_used() == "a"
        assert c.most_recently_used() == "c"


class TestLFUCache(
    mixins.InitializeMixin,
    mixins.InsertAndGetMixin,
    mixins.PopitemMixin,
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
    ) -> cachebox.LFUCache:
        return cachebox.LFUCache(
            maxsize,
            iterable,
            capacity=capacity,
            getsizeof=getsizeof,
        )

    @staticmethod
    def _hit(cache: cachebox.LFUCache, key, times: int = 1) -> None:
        """Access a key `times` times to accumulate frequency."""
        for _ in range(times):
            cache[key]

    def test_evicts_least_frequent_on_insert(self):
        c = self.create_cache(3)
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        self._hit(c, "a", 5)
        self._hit(c, "b", 3)
        # "c" has frequency 1 — should be evicted
        c["d"] = 4
        assert "c" not in c
        assert "a" in c
        assert "b" in c
        assert "d" in c

    def test_evicts_lowest_frequency_not_oldest(self):
        """LFU must evict by count, not by insertion order."""
        c = self.create_cache(3)
        c["old"] = 0  # inserted first
        c["mid"] = 0
        c["new"] = 0  # inserted last
        self._hit(c, "old", 10)
        self._hit(c, "mid", 10)
        # "new" has lowest frequency even though "old" is oldest
        c["x"] = 99
        assert "new" not in c
        assert "old" in c
        assert "mid" in c

    def test_frequency_survives_value_update(self):
        """Re-inserting a key should update value but preserve (and increment) frequency."""
        c = self.create_cache(2)
        c["a"] = 1
        c["b"] = 1
        self._hit(c, "a", 5)  # a.freq = 6 (5 reads + 1 insert)
        c["a"] = 99  # update — should NOT reset frequency to 1
        # b has freq=1, a has freq>=6; inserting "c" must evict "b"
        c["c"] = 3
        assert "b" not in c
        assert "a" in c

    def test_popitem_removes_lfu_item(self):
        c = self.create_cache(3)
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        self._hit(c, "a", 5)
        self._hit(c, "b", 2)
        # c has lowest frequency
        key, val = c.popitem()
        assert key == "c"
        assert val == 3
        assert "c" not in c

    def test_tie_broken_by_recency_oldest_evicted(self):
        """When frequencies are equal, the oldest-inserted key is evicted."""
        c = self.create_cache(3)
        c["first"] = 1  # inserted first → evicted on tie
        c["second"] = 2
        c["third"] = 3
        # All have freq=1; "first" is oldest
        c["fourth"] = 4
        assert "first" not in c

    def test_single_item_cache_evicts_on_second_insert(self):
        c = self.create_cache(1)
        c["only"] = 42
        self._hit(c, "only", 100)
        c["new"] = 7
        assert "only" not in c
        assert c["new"] == 7

    def test_get_increments_frequency(self):
        c = self.create_cache(2)
        c["a"] = 1
        c["b"] = 2
        self._hit(c, "a", 3)  # a.freq = 4, b.freq = 1
        c["c"] = 3  # evicts b
        assert "b" not in c
        assert "a" in c

    def test_setdefault_increments_frequency_on_hit(self):
        c = self.create_cache(2)
        c["a"] = 1
        c["b"] = 2
        # setdefault on existing key should count as an access
        for _ in range(5):
            c.setdefault("a", 999)
        c["c"] = 3  # should evict "b", not "a"
        assert "b" not in c
        assert "a" in c

    def test_peek_does_not_increment_frequency(self):
        c = self.create_cache(2)
        c["a"] = 1
        c["b"] = 2

        # Peek "a" many times — frequency must NOT change
        for _ in range(100):
            c.peek("a")

        # hit b once so it has freq=2 vs a's freq=1
        self._hit(c, "b", 1)
        c["c"] = 3  # must evict "a" (lower freq due to peek not counting)
        assert "a" not in c
        assert "b" in c

    def test_least_frequently_used_reflects_access_counts(self):
        c = self.create_cache(4)
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        c["d"] = 4
        self._hit(c, "a", 10)
        self._hit(c, "b", 5)
        self._hit(c, "c", 2)
        # d has freq=1, c has freq=3, b has freq=6, a has freq=11
        assert c.least_frequently_used(0) == "d"
        assert c.least_frequently_used(1) == "c"
        assert c.least_frequently_used(2) == "b"
        assert c.least_frequently_used(3) == "a"

    def test_frequency_not_reset_after_pop_and_reinsert(self):
        """A key that is popped and re-added starts fresh at frequency 1."""
        c = self.create_cache(2)
        c["a"] = 1
        c["b"] = 2
        self._hit(c, "a", 10)
        c.pop("a")
        c["a"] = 1  # fresh insert — freq resets to 1
        # now b also has freq=1; tie broken by insertion order — a is newer
        c["c"] = 3  # should evict b (older with same freq=1)
        assert "b" not in c
        assert "a" in c

    def test_cache_never_exceeds_maxsize(self):
        c = self.create_cache(5)
        for i in range(20):
            c[i] = i
        assert len(c) <= 5

    def test_update_triggers_eviction(self):
        c = self.create_cache(3)
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        self._hit(c, "a", 5)
        self._hit(c, "b", 3)
        c.update({"d": 4, "e": 5})
        assert len(c) == 3

    def test_drain_removes_lfu_items_in_order(self):
        c = self.create_cache(4)
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        c["d"] = 4
        self._hit(c, "d", 10)
        self._hit(c, "c", 5)
        self._hit(c, "b", 2)
        # a has freq=1 → evicted first; b next; etc.
        removed = c.drain(2)
        assert removed == 2
        assert "a" not in c
        assert "b" not in c
        assert "c" in c
        assert "d" in c

    def test_single_entry_popitem(self):
        c = self.create_cache(10)
        c["solo"] = 99
        k, v = c.popitem()
        assert k == "solo" and v == 99
        assert len(c) == 0

    def test_popitem_empty_raises(self):
        c = self.create_cache(5)
        with pytest.raises(KeyError):
            c.popitem()

    def test_least_frequently_used_empty_raises(self):
        c = self.create_cache(5)
        with pytest.raises(IndexError):
            c.least_frequently_used()

    def test_least_frequently_used_out_of_range_raises(self):
        c = self.create_cache(5)
        c["a"] = 1
        with pytest.raises(IndexError):
            c.least_frequently_used(5)

    def test_clear_resets_all_frequencies(self):
        c = self.create_cache(3)
        c["a"] = 1
        self._hit(c, "a", 50)
        c.clear()
        assert len(c) == 0
        # After clearing, re-inserted keys start at frequency 1
        c["a"] = 1
        c["b"] = 2
        c["c"] = 3
        # All freq=1; tie → oldest ("a") evicted
        c["d"] = 4
        assert "a" not in c

    def test_insert_returns_none_for_new_key(self):
        c = self.create_cache(5)
        result = c.insert("x", 42)
        assert result is None

    def test_insert_returns_old_value_for_existing_key(self):
        c = self.create_cache(5)
        c["x"] = 1
        old = c.insert("x", 99)
        assert old == 1
        assert c["x"] == 99
