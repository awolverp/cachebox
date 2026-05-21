from hypothesis import HealthCheck, settings

# Register a custom profile that suppresses the health check
settings.register_profile(
    "global_fuzz_settings", suppress_health_check=[HealthCheck.differing_executors]
)

# Load the profile globally for the entire test run
settings.load_profile("global_fuzz_settings")
