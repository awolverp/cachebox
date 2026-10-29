"""Key hashing edge cases, for every cache type.

The tables mix Python hashes internally; these tests pin down that key
identity still follows Python semantics (equal objects share an entry, equal
hashes alone do not).
"""

import platform
import subprocess
import sys

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


# A C type whose tp_hash / tp_richcompare report an error without setting an
# exception, as a buggy extension could. CPython turns that into SystemError.
BROKEN_C_KEY = """
import ctypes
import sys

import cachebox

name, slot = sys.argv[1], sys.argv[2]
cls = getattr(cachebox, name)
cache = cls(0, 3600) if name == "TTLCache" else cls(0)

HASH = ctypes.CFUNCTYPE(ctypes.c_ssize_t, ctypes.c_void_p)
RICHCMP = ctypes.CFUNCTYPE(ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_int)
broken = slot == "hash"
tp_hash = HASH(lambda o: -1 if broken else 1)
tp_richcompare = RICHCMP(lambda a, b, op: None)  # NULL


class Slot(ctypes.Structure):
    _fields_ = [("slot", ctypes.c_int), ("pfunc", ctypes.c_void_p)]


class Spec(ctypes.Structure):
    _fields_ = [
        ("name", ctypes.c_char_p),
        ("basicsize", ctypes.c_int),
        ("itemsize", ctypes.c_int),
        ("flags", ctypes.c_uint),
        ("slots", ctypes.POINTER(Slot)),
    ]


slots = (Slot * 3)(
    Slot(59, ctypes.cast(tp_hash, ctypes.c_void_p)),  # Py_tp_hash
    Slot(67, ctypes.cast(tp_richcompare, ctypes.c_void_p)),  # Py_tp_richcompare
    Slot(0, None),
)
spec = Spec(b"broken.Key", object.__basicsize__, 0, 0, slots)
ctypes.pythonapi.PyType_FromSpec.restype = ctypes.py_object
ctypes.pythonapi.PyType_FromSpec.argtypes = [ctypes.POINTER(Spec)]
Key = ctypes.pythonapi.PyType_FromSpec(ctypes.byref(spec))

if not broken:
    cache[Key()] = 1  # same hash: the lookup below has to compare
try:
    cache.get(Key())
except SystemError:
    print("SystemError")
"""


@pytest.mark.skipif(
    platform.python_implementation() != "CPython",
    reason="builds a C type with ctypes.pythonapi",
)
@pytest.mark.parametrize("slot", ["hash", "eq"])
def test_c_key_error_without_exception(cache_cls, slot):
    # Before the fix the process aborted, hence a child process.
    done = subprocess.run(
        [sys.executable, "-c", BROKEN_C_KEY, cache_cls.__name__, slot],
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert done.stdout.strip() == "SystemError", (
        done.stderr or f"exit code {done.returncode}"
    )
