#![allow(dead_code)]

pub mod alloc;
pub mod control;
pub mod raw;
pub mod scopeguard;
pub mod util;

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,

    /// The memory allocator returned an error
    AllocError {
        /// The layout of the allocation request that failed.
        layout: std::alloc::Layout,
    },
}

// matches stdalloc::collections::TryReserveError
impl core::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("memory allocation failed")?;
        let reason = match self {
            TryReserveError::CapacityOverflow => {
                " because the computed capacity exceeded the collection's maximum"
            }
            TryReserveError::AllocError { .. } => " because the memory allocator returned an error",
        };
        f.write_str(reason)
    }
}

impl core::error::Error for TryReserveError {}
