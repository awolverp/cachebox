//! Rust cache implemetations, these will be bridged to python in `bridge/` path.

pub(super) const MAX_N_SHIFT: usize = usize::MAX - (isize::MAX as usize);

mod fifo;
mod lru;
mod nopolicy;
mod ttl;

pub use fifo::{FIFOIterator, FIFOPolicy};
pub use lru::LRUPolicy;
pub use nopolicy::NoPolicy;
pub use ttl::{TTLElement, TTLPolicy, TTLVecPtr};
