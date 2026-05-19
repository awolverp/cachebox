//! According to PyO3 updates, we can write `__init__` methods inside the Rust, which allows developers
//! to use classes as subclass in Python.
//!
//! All of classes must implement `__new__` and `__init__` methods.
//! - In `__new__` methods, we should allocate memory for the type;
//! - And in `__init__` methods, we should initialize and constrcut the type, according to parameters.
//!
//! There are types that help us to create these methods completely thread-safe.

use std::cell;
use std::mem;
use std::sync::atomic;
use std::sync::Arc;

const UNINIT: u8 = 0;
const RUNNING: u8 = 1;
const INIT: u8 = 2;

pub struct OnceInitInner<T> {
    /// Tracks the lifecycle of the inner value:
    /// `UNINIT` → `RUNNING` (mid-write) → `INIT` (ready).
    state: atomic::AtomicU8,
    /// Heap-allocated storage that is uninitialized until [`set`](OnceInit::set) completes.
    /// Wrapped in a [`std::sync::Mutex`] so that post-init access is safe across threads.
    value: cell::UnsafeCell<mem::MaybeUninit<T>>,
}

/// A thread-safe, write-once container for PyO3 `__new__` / `__init__` two-phase construction.
///
/// PyO3 splits Python object creation into two steps:
/// - `__new__` allocates the Rust-side storage (calls [`OnceInit::uninit`]),
/// - `__init__` fills it in exactly once (calls [`OnceInit::set`]).
///
/// After initialisation the inner value is accessible through a [`std::sync::MutexGuard`]
/// via [`OnceInit::lock`], which is safe to call from multiple threads simultaneously.
#[repr(transparent)]
pub struct OnceInit<T>(Arc<OnceInitInner<T>>);

impl<T> OnceInit<T> {
    /// Creates a new, **uninitialized** [`OnceInit`].
    ///
    /// Intended to be called from the PyO3 `__new__` handler to allocate the
    /// object slot before Python passes arguments to `__init__`.
    ///
    /// The returned value must not be accessed via [`lock`](Self::lock)
    /// until [`set`](Self::set) has been called.
    #[inline]
    pub fn uninit() -> Self {
        OnceInitInner {
            state: atomic::AtomicU8::new(UNINIT),
            value: cell::UnsafeCell::new(mem::MaybeUninit::uninit()),
        }
        .into()
    }

    /// Creates a new **initialized** [`OnceInit`].
    #[inline]
    pub fn new(val: T) -> Self {
        OnceInitInner {
            state: atomic::AtomicU8::new(INIT),
            value: cell::UnsafeCell::new(mem::MaybeUninit::new(val)),
        }
        .into()
    }

    /// Initializes the container with `val`, transitioning state from `UNINIT` to `INIT`.
    ///
    /// Intended to be called from the PyO3 `__init__` handler once the Python-side
    /// arguments have been validated and the Rust value can be constructed.
    ///
    /// # Panics
    ///
    /// Panics if `set` has already been called on this instance.
    #[inline]
    pub fn set(&self, val: T) {
        if self
            .0
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
        unsafe { (*self.0.value.get()).write(val) };
        self.0.state.store(INIT, atomic::Ordering::Release);
    }

    /// Returns an immutable reference to initialized value.
    ///
    /// # Panics
    ///
    /// Panics if called before [`set`](Self::set) has completed.
    #[inline]
    pub fn get(&self) -> &T {
        if std::hint::likely(self.0.state.load(atomic::Ordering::Acquire) == INIT) {
            // SAFETY: state == INIT guarantees `value` was fully written and is valid.
            unsafe { (*self.0.value.get()).assume_init_ref() }
        } else {
            not_init_panic()
        }
    }
}

impl<T> Clone for OnceInit<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> From<OnceInitInner<T>> for OnceInit<T> {
    fn from(value: OnceInitInner<T>) -> Self {
        Self(Arc::new(value))
    }
}

// SAFETY: Mutex<T> is Send+Sync when T: Send; we uphold the init invariant ourselves.
unsafe impl<T: Send> Send for OnceInit<T> {}
unsafe impl<T: Sync> Sync for OnceInit<T> {}

impl<T> Drop for OnceInit<T> {
    /// Drops the inner value if and only if [`set`](OnceInit::set) was called.
    ///
    /// Checks the state flag without any atomic synchronisation since `drop`
    /// requires `&mut self`, guaranteeing exclusive access.
    fn drop(&mut self) {
        if unsafe { *self.0.state.as_ptr() == INIT } {
            // SAFETY: state == INIT means value was written and not yet dropped.
            unsafe { (*self.0.value.get()).assume_init_drop() }
        }
    }
}

/// Marked `#[cold]` and `#[inline(never)]` so it is compiled as a separate,
/// rarely-executed stub and does not bloat the hot path of [`lock`](OnceInit::lock).
#[cold]
#[inline(never)]
fn not_init_panic() -> ! {
    panic!("Object not initialized (__init__ not called)")
}

/// Marked `#[cold]` and `#[inline(never)]` so it is compiled as a separate,
/// rarely-executed stub and does not bloat the hot path of [`set`](OnceInit::set).
#[cold]
#[inline(never)]
fn already_init_panic() -> ! {
    panic!("Object already initialized")
}
