pub(crate) use std::hint::likely;
pub(crate) use std::hint::unlikely;

// FIXME: use strict provenance functions once they are stable.
// Implement it with a transmute for now.
#[inline(always)]
pub(crate) fn invalid_mut<T>(addr: usize) -> *mut T {
    unsafe { core::mem::transmute(addr) }
}
