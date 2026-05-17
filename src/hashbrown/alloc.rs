use core::ptr::NonNull;

#[cfg(test)]
pub(crate) use std::alloc::AllocError;
use std::alloc::Layout;
pub(crate) use std::alloc::{Allocator, Global};

pub(crate) fn do_alloc<A: Allocator>(alloc: &A, layout: Layout) -> Result<NonNull<[u8]>, ()> {
    match alloc.allocate(layout) {
        Ok(ptr) => Ok(ptr),
        Err(_) => Err(()),
    }
}
