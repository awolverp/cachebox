pub(crate) use self::inner::do_alloc;
#[cfg(test)]
pub(crate) use self::inner::AllocError;
pub(crate) use self::inner::Allocator;
pub(crate) use self::inner::Global;

#[cfg(feature = "nightly")]
mod inner {
    use core::alloc::Layout;
    use core::ptr::NonNull;
    #[cfg(test)]
    pub(crate) use std::alloc::AllocError;
    pub(crate) use std::alloc::Allocator;
    pub(crate) use std::alloc::Global;

    pub(crate) fn do_alloc<A: Allocator>(alloc: &A, layout: Layout) -> Result<NonNull<[u8]>, ()> {
        match alloc.allocate(layout) {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(()),
        }
    }
}

#[cfg(not(feature = "nightly"))]
mod inner {
    #[cfg(test)]
    pub(crate) use allocator_api2::alloc::AllocError;
    pub(crate) use allocator_api2::alloc::Allocator;
    pub(crate) use allocator_api2::alloc::Global;
    use core::alloc::Layout;
    use core::ptr::NonNull;

    pub(crate) fn do_alloc<A: Allocator>(alloc: &A, layout: Layout) -> Result<NonNull<[u8]>, ()> {
        match alloc.allocate(layout) {
            Ok(ptr) => Ok(ptr),
            Err(_) => Err(()),
        }
    }
}
