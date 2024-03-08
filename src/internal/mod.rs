mod cache;
mod fifocache;
mod lfucache;
mod lrucache;
mod rrcache;

pub use self::cache::Cache;
pub use self::fifocache::FIFOCache;
pub use self::lfucache::LFUCache;
pub use self::lrucache::LRUCache;
pub use self::rrcache::RRCache;
