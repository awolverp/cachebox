from benchmarks import (
    _cache,
    _fifocache,
    _lfucache,
    _lrucache,
    _mrucache,
    _rrcache,
    _ttlcache,
    Bench
)

for i in Bench.classes:
    b = i()
    b.start()
    print()
