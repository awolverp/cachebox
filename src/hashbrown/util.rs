#[inline(always)]
#[cold]
fn cold_path() {}

#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    if b {
        true
    } else {
        cold_path();
        false
    }
}

#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold_path();
        true
    } else {
        false
    }
}
