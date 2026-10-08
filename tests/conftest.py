import pytest
from hypothesis import HealthCheck, settings

import cachebox

# Register a custom profile that suppresses the health check
settings.register_profile(
    "global_fuzz_settings",
    suppress_health_check=[HealthCheck.differing_executors],
)

# Load the profile globally for the entire test run
settings.load_profile("global_fuzz_settings")


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
