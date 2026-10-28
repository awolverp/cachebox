"""Key hashing edge cases, for every cache type.

The tables mix Python hashes internally; these tests pin down that key
identity still follows Python semantics (equal objects share an entry, equal
hashes alone do not).
"""

import pytest

import cachebox


def _make(cache_cls, maxsize=0):
    if cache_cls is cachebox.TTLCache:
        return cache_cls(maxsize, 3600)
    return cache_cls(maxsize)


def test_equal_numbers_share_one_entry(cache_cls):
    c = _make(cache_cls)
    c[1] = "int"
    c[1.0] = "float"
    c[True] = "bool"
    assert len(c) == 1
    assert c[1] == "bool"


def test_equal_hash_but_unequal_keys_are_distinct(cache_cls):
    # In CPython hash(-1) == hash(-2) == -2.
    assert hash(-1) == hash(-2)
    c = _make(cache_cls)
    c[-1] = "minus one"
    c[-2] = "minus two"
    assert len(c) == 2
    assert c[-1] == "minus one"
    assert c[-2] == "minus two"


class _ConstantHash:
    def __init__(self, n):
        self.n = n

    def __hash__(self):
        return 42

    def __eq__(self, other):
        return isinstance(other, _ConstantHash) and other.n == self.n


def test_all_keys_colliding(cache_cls):
    c = _make(cache_cls)
    keys = [_ConstantHash(i) for i in range(100)]
    for k in keys:
        c[k] = k.n
    assert len(c) == 100
    for k in keys:
        assert c[_ConstantHash(k.n)] == k.n


@pytest.mark.parametrize(
    "keys",
    [
        range(2_000),
        range(-1_000, 1_000),
        [i * 2**61 for i in range(1, 1_000)],
        [i + 0.5 for i in range(1_000)],
    ],
    ids=["small-ints", "negative-ints", "huge-ints", "floats"],
)
def test_many_low_entropy_keys(cache_cls, keys):
    keys = list(keys)
    c = _make(cache_cls)
    for k in keys:
        c[k] = k
    assert len(c) == len(set(keys))
    for k in keys:
        assert c[k] == k
    for k in keys[::2]:
        del c[k]
    for i, k in enumerate(keys):
        assert (k in c) == (i % 2 == 1)
