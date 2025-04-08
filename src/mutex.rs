//! Mutex lock
//!
//! Thanks to `Amanieu d'Antras` for this beautiful implementation.

use core::sync::atomic::{AtomicU8, Ordering};
use parking_lot_core::deadlock;
use parking_lot_core::{self, ParkResult, SpinWait, UnparkResult, UnparkToken, DEFAULT_PARK_TOKEN};
use std::time::Instant;

const TOKEN_NORMAL: UnparkToken = UnparkToken(0);
const TOKEN_HANDOFF: UnparkToken = UnparkToken(1);

const LOCKED_BIT: u8 = 0b01;
const PARKED_BIT: u8 = 0b10;

pub struct RawMutex {
    state: AtomicU8,
}

unsafe impl lock_api::RawMutex for RawMutex {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: RawMutex = RawMutex {
        state: AtomicU8::new(0),
    };

    type GuardMarker = lock_api::GuardSend;

    #[inline]
    fn lock(&self) {
        if self
            .state
            .compare_exchange_weak(0, LOCKED_BIT, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.lock_slow(None);
        }
        unsafe { deadlock::acquire_resource(self as *const _ as usize) };
    }

    #[inline]
    fn try_lock(&self) -> bool {
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            if state & LOCKED_BIT != 0 {
                return false;
            }
            match self.state.compare_exchange_weak(
                state,
                state | LOCKED_BIT,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe { deadlock::acquire_resource(self as *const _ as usize) };
                    return true;
                }
                Err(x) => state = x,
            }
        }
    }

    #[inline]
    unsafe fn unlock(&self) {
        deadlock::release_resource(self as *const _ as usize);
        if self
            .state
            .compare_exchange(LOCKED_BIT, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            return;
        }
        self.unlock_slow(false);
    }

    #[inline]
    fn is_locked(&self) -> bool {
        let state = self.state.load(Ordering::Relaxed);
        state & LOCKED_BIT != 0
    }
}

impl RawMutex {
    #[cold]
    fn lock_slow(&self, timeout: Option<Instant>) -> bool {
        let mut spinwait = SpinWait::new();
        let mut state = self.state.load(Ordering::Relaxed);
        loop {
            if state & LOCKED_BIT == 0 {
                match self.state.compare_exchange_weak(
                    state,
                    state | LOCKED_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return true,
                    Err(x) => state = x,
                }
                continue;
            }

            if state & PARKED_BIT == 0 && spinwait.spin() {
                state = self.state.load(Ordering::Relaxed);
                continue;
            }

            if state & PARKED_BIT == 0 {
                if let Err(x) = self.state.compare_exchange_weak(
                    state,
                    state | PARKED_BIT,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    state = x;
                    continue;
                }
            }

            let addr = self as *const _ as usize;
            let validate = || self.state.load(Ordering::Relaxed) == LOCKED_BIT | PARKED_BIT;
            let before_sleep = || {};
            let timed_out = |_, was_last_thread| {
                if was_last_thread {
                    self.state.fetch_and(!PARKED_BIT, Ordering::Relaxed);
                }
            };

            match unsafe {
                parking_lot_core::park(
                    addr,
                    validate,
                    before_sleep,
                    timed_out,
                    DEFAULT_PARK_TOKEN,
                    timeout,
                )
            } {
                ParkResult::Unparked(TOKEN_HANDOFF) => return true,
                ParkResult::Unparked(_) => (),
                ParkResult::Invalid => (),
                ParkResult::TimedOut => return false,
            }

            spinwait.reset();
            state = self.state.load(Ordering::Relaxed);
        }
    }

    #[cold]
    fn unlock_slow(&self, force_fair: bool) {
        let addr = self as *const _ as usize;
        let callback = |result: UnparkResult| {
            if result.unparked_threads != 0 && (force_fair || result.be_fair) {
                if !result.have_more_threads {
                    self.state.store(LOCKED_BIT, Ordering::Relaxed);
                }
                return TOKEN_HANDOFF;
            }

            if result.have_more_threads {
                self.state.store(PARKED_BIT, Ordering::Release);
            } else {
                self.state.store(0, Ordering::Release);
            }
            TOKEN_NORMAL
        };

        unsafe {
            parking_lot_core::unpark_one(addr, callback);
        }
    }
}

pub type Mutex<T> = lock_api::Mutex<RawMutex, T>;
