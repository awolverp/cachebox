#[macro_use]
mod base;
mod cache;
mod fifocache;
mod lfucache;
mod lrucache;

#[allow(unused_imports)]
pub use self::base::BaseCacheImpl;
#[allow(unused_imports)]
pub use self::cache::Cache;
#[allow(unused_imports)]
pub use self::fifocache::FIFOCache;
#[allow(unused_imports)]
pub use self::lfucache::LFUCache;
#[allow(unused_imports)]
pub use self::lrucache::LRUCache;
