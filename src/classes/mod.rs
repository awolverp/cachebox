#[macro_use]
mod base;
mod cache;
mod fifocache;
mod lfucache;

#[allow(unused_imports)]
pub use base::BaseCacheImpl;
#[allow(unused_imports)]
pub use cache::Cache;
#[allow(unused_imports)]
pub use fifocache::FIFOCache;
#[allow(unused_imports)]
pub use lfucache::LFUCache;
