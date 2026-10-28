"""Differential tests: LFUCache and VTTLCache against small pure-Python models.

Hypothesis drives random sequences of operations and checks eviction order,
``popitem``, ``least_frequently_used`` and ordered iteration after every step.
LFU membership is checked with ``peek`` because ``key in LFUCache`` counts as
an access.
"""

import time

from hypothesis import assume, settings
from hypothesis import strategies as st
from hypothesis.stateful import (
    RuleBasedStateMachine,
    invariant,
    precondition,
    rule,
)

import cachebox

KEYS = st.integers(min_value=0, max_value=15)
_MISSING = object()


class LFUModel(RuleBasedStateMachine):
    """Reference: evict the minimal (frequency, insertion order).

    Inserting a new key starts at frequency 0, every read adds 1, and
    replacing a value resets the frequency to 1 while keeping insertion order.
    """

    MAXSIZE = 6

    def __init__(self):
        super().__init__()
        self.cache = cachebox.LFUCache(self.MAXSIZE)
        self.model = {}  # key -> [frequency, seq, value]
        self.seq = 0

    def _order(self):
        return sorted(
            self.model, key=lambda k: (self.model[k][0], self.model[k][1])
        )

    @rule(key=KEYS, value=st.integers())
    def set(self, key, value):
        if key in self.model:
            self.model[key][0] = 1
            self.model[key][2] = value
        else:
            if len(self.model) >= self.MAXSIZE:
                del self.model[self._order()[0]]
            self.model[key] = [0, self.seq, value]
            self.seq += 1
        self.cache[key] = value

    @rule(key=KEYS)
    def get(self, key):
        got = self.cache.get(key, _MISSING)
        if key in self.model:
            self.model[key][0] += 1
            assert got == self.model[key][2]
        else:
            assert got is _MISSING

    @precondition(lambda self: self.model)
    @rule()
    def popitem(self):
        expected = self._order()[0]
        key, value = self.cache.popitem()
        assert key == expected
        assert value == self.model.pop(key)[2]

    @rule(key=KEYS)
    def delete(self, key):
        if key in self.model:
            del self.model[key]
            del self.cache[key]

    @precondition(lambda self: self.model)
    @rule(data=st.data())
    def least_frequently_used(self, data):
        n = data.draw(st.integers(min_value=0, max_value=len(self.model) - 1))
        assert self.cache.least_frequently_used(n) == self._order()[n]

    @rule()
    def iterate(self):
        assert list(self.cache.keys()) == self._order()

    @invariant()
    def same_contents(self):
        assert len(self.cache) == len(self.model)
        for key, (_, _, value) in self.model.items():
            assert self.cache.peek(key, _MISSING) == value


class VTTLModel(RuleBasedStateMachine):
    """Reference: evict the earliest expiry; entries without ttl last.

    TTLs are whole seconds far in the future so nothing expires during the test.
    Live entries never share a TTL, so their expiries differ by at least a
    second and clock resolution cannot reorder them; entries without ttl are
    ordered by insertion.
    """

    MAXSIZE = 6

    def __init__(self):
        super().__init__()
        self.cache = cachebox.VTTLCache(self.MAXSIZE)
        self.model = {}  # key -> [priority, seq]
        self.ttls = {}  # key -> ttl, to keep live TTLs distinct
        self.seq = 0

    def _order(self):
        return sorted(
            self.model, key=lambda k: (self.model[k][0], self.model[k][1])
        )

    @rule(
        key=KEYS,
        ttl=st.none() | st.integers(min_value=1_000, max_value=1_000_000),
    )
    def insert(self, key, ttl):
        if ttl is not None:
            assume(all(t != ttl for k, t in self.ttls.items() if k != key))
        if key not in self.model and len(self.model) >= self.MAXSIZE:
            evicted = self._order()[0]
            del self.model[evicted]
            self.ttls.pop(evicted, None)
        self.cache.insert(key, key, ttl)
        priority = (1, 0.0) if ttl is None else (0, time.time() + ttl)
        self.ttls[key] = ttl
        if key in self.model:
            self.model[key][0] = priority
        else:
            self.model[key] = [priority, self.seq]
            self.seq += 1

    @precondition(lambda self: self.model)
    @rule()
    def popitem(self):
        expected = self._order()[0]
        key, _ = self.cache.popitem()
        assert key == expected
        del self.model[key]
        self.ttls.pop(key, None)

    @rule(key=KEYS)
    def delete(self, key):
        if key in self.model:
            del self.model[key]
            del self.cache[key]
            self.ttls.pop(key, None)

    @rule()
    def iterate(self):
        assert list(self.cache.keys()) == self._order()

    @invariant()
    def same_contents(self):
        assert len(self.cache) == len(self.model)
        for key in self.model:
            # VTTLCache has no peek(); membership does not affect its order.
            assert key in self.cache


_settings = settings(max_examples=200, stateful_step_count=60, deadline=None)

TestLFUModel = LFUModel.TestCase
TestLFUModel.settings = _settings

TestVTTLModel = VTTLModel.TestCase
TestVTTLModel.settings = _settings
