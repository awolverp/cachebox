# Caching libraries Benchmarks
**Qualification criteria is:**
- Needs to support minimum 2 alghoritms
- Runs on Python3.8

If you know other library, tell me to add to this page.

> [!IMPORTANT]\
> The system on which the benchmarks are done: **Linux x86_64, 8G, Intel i3-1115G4**

## Benchmarks:

**Versions**:
- cachebox version: 1.0.0
- cachetools version: 5.3.2

### Cache
| operation\class         | cachetools.Cache(1000)  | cachebox.Cache(1000)    |
| ----------------------- | ------------------ | ------------------ |
| clear                   | 0.986ms / 0.728KB  | 0.033ms / 0.0KB    |
| delete                  | 0.488ms / 0.144KB  | 0.252ms / 0.128KB  |
| insert (1000 items)     | 1.033ms / 69.048KB | 0.247ms / 23.824KB |
| pop                     | 0.641ms / 0.144KB  | 0.264ms / 0.128KB  |
| popitem                 | 1.149ms / 0.184KB  | not implemented    |
| setdefault (1000 items) | 5.595ms / 49.348KB | 5.150ms / 29.364KB |
| update (1000 items)     | 1.112ms / 69.408KB | 0.272ms / 24.184KB |


### FIFOCache
| operation\class         | cachetools.FIFOCache(1000)| cachebox.FIFOCache(1000)|
| ----------------------- | -------------------- | ------------------ |
| clear                   | 0.909ms / 0.803KB    | 0.031ms / 0.0KB    |
| delete                  | 0.602ms / 0.144KB    | 0.248ms / 0.128KB  |
| insert (10000 items)    | 37.510ms / 318.4KB   | 3.281ms / 32.08KB  |
| pop                     | 0.751ms / 0.144KB    | 0.320ms / 0.128KB  |
| popitem                 | 1.076ms / 0.192KB    | 0.296ms / 0.128KB  |
| setdefault (10000 items)| 52.665ms / 170.4KB   | 49.751ms / 48.048KB|
| update (10000 items)    | 39.526ms / 318.76KB  | 5.714ms / 760.184KB|


### LFUCache
| operation\class         | cachetools.LFUCache(1000) | cachebox.LFUCache(1000) |
| ----------------------- | -------------------- | ------------------ |
| clear                   | 28.280ms / 0.898KB   | 0.031ms / 0.0KB    |
| delete                  | 1.180ms / 0.216KB    | 0.273ms / 0.128KB  |
| insert (10000 items)    | 509.691ms / 253.312KB| 35.511ms / 23.856KB|
| pop                     | 1.648ms / 0.216KB    | 0.282ms / 0.128KB  |
| popitem                 | 29.073ms / 0.4KB     | 3.056ms / 0.128KB  |
| setdefault (10000 items)| 60.283ms / 152.192KB | 50.914ms / 47.408KB|
| update (10000 items)    | 500.150ms / 253.672KB| 38.764ms / 760.184KB|


### LRUCache
| operation\class         | cachetools.LRUCache(1000) | cachebox.LRUCache(1000) |
| ----------------------- | -------------------- | ------------------ |
| clear                   | 1.090ms / 0.802KB    | 0.030ms / 0.0KB    |
| delete                  | 0.609ms / 0.144KB    | 0.268ms / 0.128KB  |
| insert (10000 items)    | 37.679ms / 318.376KB | 3.451ms / 32.08KB  |
| pop                     | 0.948ms / 0.144KB    | 0.450ms / 0.128KB  |
| popitem                 | 1.261ms / 0.192KB    | 0.459ms / 0.128KB  |
| setdefault (10000 items)| 54.138ms / 169.96KB  | 51.069ms / 47.984KB|
| update (10000 items)    | 38.840ms / 318.736KB | 5.992ms / 760.184KB|


### MRUCache
| operation\class         | cachetools.MRUCache(1000) | cachebox.MRUCache(1000) |
| ----------------------- | -------------------- | ------------------ |
| clear                   | 1.105ms / 0.802KB    | 0.031ms / 0.0KB    |
| delete                  | 0.630ms / 0.144KB    | 0.488ms / 0.128KB  |
| insert (10000 items)    | 39.594ms / 318.376KB | 3.408ms / 23.856KB |
| pop                     | 0.992ms / 0.144KB    | 0.505ms / 0.128KB  |
| popitem                 | 1.897ms / 0.192KB    | 0.308ms / 0.128KB  |
| setdefault (10000 items)| 55.141ms / 170.184KB | 51.317ms / 47.504KB|
| update (10000 items)    | 41.258ms / 318.736KB | 6.031ms / 760.184KB|


### RRCache
| operation\class         | cachetools.RRCache(1000) | cachebox.RRCache(1000) |
| ----------------------- | -------------------- | ------------------ |
| clear                   | 4.879ms / 8.216KB    | 0.031ms / 0.0KB    |
| delete                  | 0.480ms / 0.144KB    | 0.253ms / 0.128KB  |
| insert (10000 items)    | 83.215ms / 179.568KB | 8.653ms / 32.08KB  |
| pop                     | 0.635ms / 0.144KB    | 0.262ms / 0.128KB  |
| popitem                 | 5.102ms / 8.264KB    | 0.625ms / 0.128KB  |
| setdefault (10000 items)| 51.305ms / 87.896KB  | 49.541ms / 48.464KB|
| update (10000 items)    | 83.815ms / 179.928KB | 10.763ms / 760.18KB|


### TTLCache
| operation\class         | cachetools.TTLCache(1000) | cachebox.TTLCache(1000) |
| ----------------------- | -------------------- | ------------------ |
| clear                   | 4.111ms / 0.882KB    | 0.030ms / 0.0KB    |
| delete                  | 1.035ms / 0.144KB    | 0.289ms / 0.128KB  |
| expire (1000 items)     | 1.727ms / 0.064KB    | 0.332ms / 0.0KB    |
| insert (10000 items)    | 98.520ms / 406.656KB | 4.162ms / 32.08KB  |
| pop                     | 3.185ms / 0.24KB     | 0.294ms / 0.128KB  |
| popitem                 | 4.390ms / 0.304KB    | 0.340ms / 0.128KB  |
| setdefault (10000 items)| 79.153ms / 258.312KB | 50.547ms / 47.472KB|
| update (10000 items)    | 99.515ms / 407.016KB | 8.109ms / 760.184KB|


### TTLCacheNoDefault
| operation\class         | cachebox.TTLCacheNoDefault  |
| ----------------------- | --------------------------- |
| clear                   | 0.030ms / 0.0KB             |
| delete                  | 0.313ms / 0.128KB           |
| expire (1000 items)     | 0.224ms / 0.0KB             |
| insert (10000 items)    | 263.789ms / 32.152KB        |
| pop                     | 0.323ms / 0.128KB           |
| popitem                 | 0.343ms / 0.128KB           |
| setdefault (10000 items)| 60.743ms / 48.336KB         |
| update (10000 items)    | 5.667ms / 760.184KB         |

> [!TIP]\
> If you want to insert several values in same time, use `update` instead of `insert` or `__setitem__`.

## Run yourself
1. Download source from here.
```sh
git clone https://github.com/awolverp/cachebox
cd cachebox
```

2. Install/Upgrade requirements:
```sh
pip3 install -U cachetools cachebox
```

3. Run
```sh
python3 benchmarks # or `make bench`
```
