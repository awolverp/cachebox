use std::sync::atomic;
use std::sync::Arc;

/// Generation version implementation
///
/// Very useful for checking changes while iteration, like what CPython does;
/// because we can't use lifetimes.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct GenerationVersion(Arc<atomic::AtomicU32>);

impl GenerationVersion {
    #[inline]
    pub fn new() -> Self {
        Self(Default::default())
    }

    #[inline]
    pub fn increment(&self) -> u32 {
        self.0.fetch_add(1, atomic::Ordering::SeqCst)
    }

    #[inline]
    pub fn get(&self) -> u32 {
        self.0.load(atomic::Ordering::Relaxed)
    }
}
