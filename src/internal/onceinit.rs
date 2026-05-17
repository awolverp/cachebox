use std::cell;
use std::mem;
use std::sync::atomic;

const UNINIT: u8 = 0;
const RUNNING: u8 = 1;
const INIT: u8 = 2;

#[repr(align(64))]
pub struct OnceInit<T> {
    state: atomic::AtomicU8,
    value: cell::UnsafeCell<mem::MaybeUninit<parking_lot::Mutex<T>>>,
}

impl<T> OnceInit<T> {
    #[inline]
    pub fn uninit() -> Self {
        Self {
            state: atomic::AtomicU8::new(UNINIT),
            value: cell::UnsafeCell::new(mem::MaybeUninit::uninit()),
        }
    }

    #[inline]
    pub fn set(&self, val: T) {
        if self
            .state
            .compare_exchange(
                UNINIT,
                RUNNING,
                atomic::Ordering::Acquire,
                atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            already_init_panic();
        }
        // SAFETY: we own the RUNNING token — no other thread can write value.
        unsafe { (*self.value.get()).write(parking_lot::Mutex::new(val)) };
        self.state.store(INIT, atomic::Ordering::Release);
    }

    #[inline]
    pub fn lock(&self) -> parking_lot::MutexGuard<'_, T> {
        if std::hint::likely(self.state.load(atomic::Ordering::Acquire) == INIT) {
            // SAFETY: state == INIT guarantees `value` was fully written and is valid.
            unsafe { (*self.value.get()).assume_init_ref().lock() }
        } else {
            not_init_panic()
        }
    }
}

// SAFETY: Mutex<T> is Send+Sync when T: Send; we uphold the init invariant ourselves.
unsafe impl<T: Send> Send for OnceInit<T> {}
unsafe impl<T: Send> Sync for OnceInit<T> {}

impl<T> Drop for OnceInit<T> {
    fn drop(&mut self) {
        if *self.state.get_mut() == INIT {
            // SAFETY: state == INIT means value was written and not yet dropped.
            unsafe { (*self.value.get()).assume_init_drop() }
        }
    }
}

#[cold]
#[inline(never)]
fn not_init_panic() -> ! {
    panic!("Object not initialized (__init__ not called)")
}

#[cold]
#[inline(never)]
fn already_init_panic() -> ! {
    panic!("Object already initialized")
}
