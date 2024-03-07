mod cache;
mod fifocache;
mod lfucache;

#[allow(unused_imports)]
pub use self::cache::Cache;
#[allow(unused_imports)]
pub use self::fifocache::FIFOCache;
#[allow(unused_imports)]
pub use self::lfucache::LFUCache;
