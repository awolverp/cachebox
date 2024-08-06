//! Rust cache implemetations, these will be bridged to python in `bridge/` path.

pub(super) const MAX_N_SHIFT: usize = usize::MAX - (isize::MAX as usize);

mod fifo;
mod nopolicy;
mod ttl;

pub use fifo::{FIFOPolicy, FIFOVecPtr};
pub use nopolicy::NoPolicy;
pub use ttl::{TTLElement, TTLPolicy, TTLVecPtr};
