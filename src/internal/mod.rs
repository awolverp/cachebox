//! Rust cache implemetations, these will be bridged to python in `bridge/` path.

mod fifo;
mod nopolicy;

pub use fifo::FIFOPolicy;
pub use fifo::FIFOVecPtr;
pub use nopolicy::NoPolicy;
