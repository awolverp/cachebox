#[macro_use]
mod base;
mod cache;
mod fifocache;

#[allow(unused_imports)]
pub use base::BaseCacheImpl;
#[allow(unused_imports)]
pub use cache::Cache;
#[allow(unused_imports)]
pub use fifocache::FIFOCache;
