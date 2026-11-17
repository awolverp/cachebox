import pytest
from hypothesis import HealthCheck, settings

import cachebox

settings.register_profile(
    "default",
    max_examples=10,
    suppress_health_check=[HealthCheck.differing_executors],
)
settings.register_profile(
    "slow",
    max_examples=100,
    suppress_health_check=[HealthCheck.differing_executors],
)

settings.load_profile("default")


ALL_CACHE_TYPES = [
    cachebox.Cache,
    cachebox.FIFOCache,
    cachebox.RRCache,
    cachebox.LRUCache,
    cachebox.LFUCache,
    cachebox.TTLCache,
    cachebox.VTTLCache,
]


@pytest.fixture(params=ALL_CACHE_TYPES)
def cache_cls(request: pytest.FixtureRequest) -> type[cachebox.BaseCacheImpl]:
    return request.param
