#[cfg(feature = "nightly")]
pub(crate) use std::hint::likely;
#[cfg(feature = "nightly")]
pub(crate) use std::hint::unlikely;

#[cfg(not(feature = "nightly"))]
#[inline(always)]
#[cold]
fn cold_path() {}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    if b {
        true
    } else {
        cold_path();
        false
    }
}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold_path();
        true
    } else {
        false
    }
}
