# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v5.0.1 - 2025-04-25
### Changed
- The issue [#25](https://github.com/awolverp/cachebox/issues/25) fixed - thanks to @Techcable
- Type-hint improved
- `FIFOCache.get` docstring fixed

## v5.0.0 - 2025-04-18
### Added
- A new method named `random_key` added to `RRCache`.
- A new method named `expire` added to `TTLCache`.
- Some new methods added to `VTTLCache`: `expire`, `items_with_expire`.
- `TTLCache` now supports `timedelta` as ttl.
- `VTTLCache` now supports `timedelta` and `datetime` as ttl.
- A new method `copy` added to all caches.

### Changed
- The core codes (rust code) renamed from `_cachebox` to `_core`. Instead of that, all of classes
  implemented in Python which are using the core's classes. This change can help to customize the alghoritms.
- Now the errors which occurred while doing `__eq__` operations will not be ignored.
- Docstrings is now more complete.
- The strictness in `__eq__` methods was reduced.
- Add more strictness for loading pickle objects.
- `LFUCache` now uses `VecDeque` instead of `Vec` (improves performance).
- The `CacheInfo.cachememory` renamed to `CacheInfo.memory`.
- *`isize` to `u64` strategy* changed in Rust.
- `__repr__` methods refactored.

### Removed
- The `n` parameter of the `LRUCache.least_recently_used` method has been removed.
- The deprecated `always_copy` parameter of the `cached` and `cachedmethod` decorators has been removed.

## 4.5.3 - 2025-03-31
### Changed
- The `cached` and `cachedmethods` decorators cached the exceptions regardless of the number of waiters. This issue has now been resolved. Thanks to @pyfreyr for the issue [#23](https://github.com/awolverp/cachebox/issues/23).

## 4.5.2 - 2025-03-14
### Changed
- In previous version, `clear_cache`, does not clear the exceptions dictionary. Thanks to @dada-engineer for the fix [#22](https://github.com/awolverp/cachebox/pull/22).

## 4.5.1 - 2025-02-01
### Changed
- In previous version, the `cached` and `cachedmethod` functions caught a `KeyError` from the callback function, which led to the cached function being called again. Thanks to @AlePiccin for the issue [#20](https://github.com/awolverp/cachebox/issues/20).

## 4.5.0 - 2025-01-31
### Updated
- `cached` and `cachedmethod` improved:
    we used `threading.Lock` for sync functions, and `asyncio.Lock` for async functions to avoid [`cache stampede`](https://en.wikipedia.org/wiki/Cache_stampede). This changes fix [#15](https://github.com/awolverp/cachebox/issues/15) and [#20](https://github.com/awolverp/cachebox/issues/20) issues. Special thanks to [@AlePiccin](https://github.com/AlePiccin).

## 4.4.2 - 2024-12-19
### Updated
- Update `pyo3` to v0.23.4
- Remove `Box` layout from Rust code

## 4.4.1 - 2024-12-19
### Updated
- Update `pyo3` to v.0.23.3

## 4.4.0 - 2024-11-28
### Added
- New `copy_level` parameter added to `cachedmethod` and `cached`.

### Deprecated
- The `always_copy` parameter marked as deprecated.

### Changed
- The `cached` and `cachedmethod` structures changed due to some issues.

### Removed
- `typing_extensions` dependency removed.
- `info` parameter removed from `cachedmethod` and `cached`.

## 4.3.3 - 2024-11-25
### Fixed
- cachedmethod type-hint fixed

## 4.3.2 - 2024-11-25
### Added
- New dependency is added: `typing_extensions`. We used this library to use `ParamSpec` that makes `cachebox` type-hint
  better, and easier than ever for you.

### Changed
- the behaviour of the iteratores changed. Previously, iterators used capacity and length of the cache
  to know "is cache have changes?". But now, each cache has a number called "state" that increments
  with each change; iterators now uses this "state" number.

## 4.3.1 - 2024-11-18
### Changed
- `__str__` changed to `__repr__`
- Free-threading is supported now
- Dependencies updated

## 4.3.0 - 2024-11-08
### Added
- Add `always_copy` parameter to `cached` and `cachedmethod` decorators

## 4.2.3 - 2024-10-18
### Fixed
- Fix https://github.com/awolverp/cachebox/issues/13

## 4.2.2 - 2024-10-16
### Fixed
- Fix `TTLCache.__str__`
- Dependencies updated

## 4.2.1 - 2024-10-10
### Fixed
- Fix `BaseCacheImpl.__class_getitem__`
- Fix `cached` and `cachedmethod` typehint

## 4.2.0 - 2024-10-04
### Added
- New function `is_cached` is added.
- New parameter `callback` is added to `cached` and `cachedmethod`: now you can set a callback for cached functions.

### Changed
- Some docstrings are changed.

### Fixed
- [manual_inspect](https://rust-lang.github.io/rust-clippy/master/index.html#manual_inspect) warning fixed

## 4.1.3 - 2024-09-13
### Fixed
- `cached` and `cachedmethod` type hint fixed

## 4.1.2 - 2024-09-07
### Fixed
- fix https://github.com/awolverp/cachebox/issues/11

## 4.1.1 - 2024-09-04
### Fixed
- fix https://github.com/awolverp/cachebox/issues/10

## 4.1.0 - 2024-08-27
### Improved
- `cached` type-hint is better now.

### Added
- there's a new feature, see that in [`cached` documentation](README.md#function-cached).

## 4.0.1 - 2024-08-21
### Fixed
- Fix `unexpected_cfg` warnings on compile time

## 4.0.0 - 2024-08-16
The Big Update ...

### Added
- There's a new class: `Frozen`

### Fixed
- using `BaseCacheImpl` as subclass was cause NotImplementedError, but now fixed.
- `make_*_key` functions error fixed

### Improved
All of caches improved and their perfomance improved a lot.

### Changed
- All of caches' algorthims changed:
    - `Cache` performance improved about 5%.
    - `FIFOCache` performance improved about 20%.
    - `LFUCache` performance improved about 46%.
    - `TTLCache` performance improved about 30%.
    - `VTTLCache` performance improved about 20%.
    - `LRUCache` performance improved about 40%.
    - See [benchmarks](https://github.com/awolverp/cachebox-benchmark)

- All of `.items()` and `.keys()` and `.values()` methods are now ordered.
- `.insert()` method changed: If the cache did have this key present, the value is updated, and the old value is returned.

### Deprecated
- `utils.items_in_order` function is deprecated and no longer available.

## 3.4.0 - 2024-07-15
### Added
- `items_in_order` function added - helps you to iterate caches items in order

### Changed
- `LRUCache` alghoritm changed and optimized (4x faster)
- The `ttl` parameter of `VTTLCache.insert` now has a default value (`None`).
- Testing improved

### Updated
- Dependecies updated

## 3.3.1 - 2024-07-13
### Fixed
- Change comparing alghoritm
- Fix [#5](https://github.com/awolverp/cachebox/issues/5)

## 3.3.0 - 2024-07-04
### Added
- `FIFOCache`:
    - Added new parameter `n` to `.first()` method.

- `LFUCache`:
    - New `.peek()` method is added.
    - New parameter `n` to `.least_frequently_used()` method is added.

- `LRUCache`:
    - New `.peek()` method is added.
    - New parameter `n` to `.least_recently_used()` method is added.

- `TTLCache`:
    - New `.first()` and `.last()` methods are added.

### Updated
- dependecies updated

## 3.2.1 - 2024-06-17
### Changed
- `VTTLCache` sorting alghrotim changed; its speed improved.
- Compile-time flags changed and optimized

## 3.2.0 - 2024-06-09
### Added
- Add `version_info` variable

### Fixed
- pyproject.toml classifiers changed and fixed
- Documentation markdown fixed

## 3.1.1 - 2024-06-08
### Changed
- `cached` and `cachedmethod` will use `FIFOCache` on default (previously it used `Cache`).

### Fixed
- Fix undefined behavior on iterators when cache's capacity changed

## 3.1.0 - 2024-06-06
### Added
- Now supports `pickle`
- a little document added to Rust code

### Changed
- `VTTLCache`: uses `time::SystemTime` instead of `time::Instant` ( doesn't effect python codes, don't care )
- `TTLCache`: uses `time::SystemTime` instead of `time::Instant` ( doesn't effect python codes, don't care )

## 3.0.0 - 2024-06-02

### Changed
- `__repr__` changed to `__str__`
- Maxsize system changed; when you pass `0` as maxsize, the value of `sys.maxsize` is automatically used.
- `__eq__` and `__ne__` behaviors changed
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

## 2.2.4 - 2024-05-09

### Fixed
- Document fixed

### Internal
- Dependecies updated

## 2.2.3 - 2024-04-26

### Changed
- Improve code stablity
- Reduce memory usages and allocations.
- Optimize `VTTLCache.__delitem__` method for more speed.
- Improve performance of all caches.

### Internal
- Use `hashbrown` instead of standard hashmap.
- Increase `unsafe` blocks in safe situations to optimize performance

## 2.2.2 - 2024-04-13

### Changed
- The behavior of the `__repr__` function has been changed and improved.
- Improve `RRCache` performance.

### Internal
- `pyo3` updated and features changed.
- Use `fastrand` instead of `rand`.

## 2.2.1 - 2024-04-05

### Fixed
- Fix `RuntimeError` when you passing a cache implemetation to its own methods.

### Internal
- Update Rust dependecies
- Optimize code for threading

## 2.2.0 - 2024-03-31

### Changed
- Change and improve sorting strategy (VTTLCache)

### Removed
- Remove deprecated methods (getmaxsize, getttl, and delete)
- Remove dependecies

## 2.1.1 - 2024-03-14

### Added
- New decorator `cachedmethod` for class methods.

### Changed
- Now `cached` accept `None` as cache.

### Fixed
- Fix some bug

## 2.0.1 - 2024-03-09

### Added
- README.md updated and added new examples
- Stub-file updated and added new examples

### Fixed
- README.md mistakes fixed

### Internal
- `strip` value changed.
- Use `AHashMap` instead of standard `HashMap`; that's very faster.

## 2.0.0 - 2024-03-09
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

## 1.0.21 - 2024-03-01

### Fixed

- Improve code stability
- Fix `__module__` attribute for `TTLCache` and `TTLCacheNoDefault`

### Changed

- Benchmarks moved to another repository (https://github.com/awolverp/cachebox-benchmark)

## 1.0.19 - 2024-02-29

### Added

- CHANGELOG file added to show you changes

### Fixed

- Improve code stability
- README.md file examples fixed
- Add versions information to BENCHMARK.md file
- `__version__` and `__author__` variables fixed

### Changed

- Makefile test commands changed
