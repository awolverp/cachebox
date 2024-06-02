# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2024-06-02

### Changed
- `__repr__` changed to `__str__`
- Maxsize system changed; when you pass `0` as maxsize, the value of `sys.maxsize` is automatically used.
- \_\_eq\_\_ and \_\_ne\_\_ behaviors changed
- Iterators mechanisms changed:
    - Now uses pointer to hashmap and iterate it
    - Additional spaces are removed
    - Caches cannot change while using iterators
- hashing machanism changed; now we cache hashes for elements to improve speed
- `Cache` was rewritten:
    - Now we use low-level API of hashbrown hashmap.
    - We removed additional layers to improve speed and performance.
- `FIFOCache` was rewritten:
    - The additional memory space removed
    - Keeping items order system changed
    - popitem, last, and first methods optimized
- `LFUCache` was rewritten:
    - optimized and additional spaces removed
    - now just uses one hashmap instead of two
    - algorithm optimized
- `RRCache` was rewritten:
    - Uses low-level API of hashbrown hashmap
- `LRUCache` was rewritten; Doesn't have any special changes.
- `TTLCache` was rewritten:
    - Time-To-Live checking system has a little changes
    - iterators do not return expired items now
- `VTTLCache` was rewritten:
    - Now keeps expire times of each element in vector; this may improves speed in some operations

### Fixed
- Fix generic error: type ... is not subscriptable

### Added
- Added new methods `is_empty` and `is_full`

## [2.2.4] - 2024-05-09

### Fixed
- Document fixed

### Internal
- Dependecies updated

## [2.2.3] - 2024-04-26

### Changed
- Improve code stablity
- Reduce memory usages and allocations.
- Optimize `VTTLCache.__delitem__` method for more speed.
- Improve performance of all caches.

### Internal
- Use `hashbrown` instead of standard hashmap.
- Increase `unsafe` blocks in safe situations to optimize performance

## [2.2.2] - 2024-04-13

### Changed
- The behavior of the `__repr__` function has been changed and improved.
- Improve `RRCache` performance.

### Internal
- `pyo3` updated and features changed.
- Use `fastrand` instead of `rand`.

## [2.2.1] - 2024-04-05

### Fixed
- Fix `RuntimeError` when you passing a cache implemetation to its own methods.

### Internal
- Update Rust dependecies
- Optimize code for threading

## [2.2.0] - 2024-03-31

### Changed
- Change and improve sorting strategy (VTTLCache)

### Removed
- Remove deprecated methods (getmaxsize, getttl, and delete)
- Remove dependecies

## [2.1.1] - 2024-03-14

### Added
- New decorator `cachedmethod` for class methods.

### Changed
- Now `cached` accept `None` as cache.

### Fixed
- Fix some bug

## [2.0.1] - 2024-03-09

### Added
- README.md updated and added new examples
- Stub-file updated and added new examples

### Fixed
- README.md mistakes fixed

### Internal
- `strip` value changed.
- Use `AHashMap` instead of standard `HashMap`; that's very faster.

## [2.0.0] - 2024-03-09
In this release, I rewritten all implemetations, documentation, and stub-file.

### Added
- New `.drain(n)` method: According to cache algorithm, deletes and returns `n` items from cache.
- New `.shrink_to_fit()` method: Shrinks the capacity of the cache as much as possible.

### Removed
- The `MRUCache` removed.

### Changed
- `__new__` methods changed; Now you can insert items to caches when creating those.
- `TTLCacheNoDefault` name changed to `VTTLCache`.
- `__iter__`, `keys`, `values` and `items` methods now are iterable.
- `LFUCache` sorting algorithm changed to improve speed.
- `__eq__` and `__ne__` methods changed.
- `cached` decorator parameter `clear_reuse` default value from `True` changed to `False`.

### Deprecated
- `.delete()` methods are deprecated; use `del cache[key]` instead.
- `.getmaxsize()` methods are deprecated; use `.maxsize` property instead.
- `TTLCache.getttl()` method is deprecated; use `.ttl` property instead.

### Fixed
- `make_typed_key` function bug fixed.

### Internal
- Link-time optimization value changed.
- `codegen-units` value changed.
- `strip` value changed to reduce binary file size.
- New dependency: `typing_extensions`

## [1.0.21] - 2024-03-01

### Fixed

- Improve code stability
- Fix `__module__` attribute for `TTLCache` and `TTLCacheNoDefault`

### Changed

- Benchmarks moved to another repository (https://github.com/awolverp/cachebox-benchmark)

## [1.0.19] - 2024-02-29

### Added

- CHANGELOG file added to show you changes

### Fixed

- Improve code stability
- README.md file examples fixed
- Add versions information to BENCHMARK.md file
- `__version__` and `__author__` variables fixed

### Changed

- Makefile test commands changed
