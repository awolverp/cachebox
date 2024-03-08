#[macro_use]
mod base;
mod cache;
mod fifocache;
mod lfucache;
mod lrucache;
mod rrcache;
mod ttlcache;
mod vttlcache;

pub use self::base::BaseCacheImpl;
pub use self::cache::Cache;
pub use self::fifocache::FIFOCache;
pub use self::lfucache::LFUCache;
pub use self::lrucache::LRUCache;
pub use self::rrcache::RRCache;
pub use self::ttlcache::TTLCache;
pub use self::vttlcache::VTTLCache;
