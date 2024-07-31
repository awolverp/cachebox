from cachebox import BaseCacheImpl, Cache
import pytest

from .mixin import _TestMixin


def test___new__():
    with pytest.raises(NotImplementedError):
        BaseCacheImpl()

def test_subclass():
    class _TestSubclass(BaseCacheImpl):
        def __init__(self) -> None:
            self.a = 1
        
        def inc(self, x: int):
            self.a += x
    
    t = _TestSubclass()
    t.inc(10)
    assert t.a == 11


class TestCache(_TestMixin):
    CACHE = Cache
    NO_POLICY = True

    def test_pickle(self):
        self._test_pickle(False)
