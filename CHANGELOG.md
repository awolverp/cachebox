# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
