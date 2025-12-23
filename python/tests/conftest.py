import cachebox
import pytest
import typing


@pytest.fixture(
    scope="function",
    params=[
        cachebox.Cache,
        cachebox.FIFOCache,
        cachebox.LFUCache,
        cachebox.LRUCache,
        cachebox.TTLCache,
        cachebox.RRCache,
        cachebox.VTTLCache,
    ],
)
def random_cache_impl(request):
    typ: typing.Type[cachebox.BaseCacheImpl] = request.param

    def inner(maxsize, iterable=None):
        if typ is cachebox.TTLCache:
            return typ(maxsize, ttl=10, iterable=iterable)

        if typ is cachebox.VTTLCache:
            return typ(maxsize, ttl=10, iterable=iterable)

        return typ(maxsize, iterable=iterable)

    return inner
