#[cfg(feature = "nightly")]
pub(crate) use std::hint::likely;
#[cfg(feature = "nightly")]
pub(crate) use std::hint::unlikely;

#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    b
}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    b
}
