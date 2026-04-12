/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        #[cfg(target_arch = "x86_64")]
        let mut backoff = 0u32;
        loop {
            if self
                .lock
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return MutexGuard {
                    mutex: self,
                    data: self.data.get(),
                };
            }

            #[cfg(target_arch = "x86_64")]
            if crate::scheduler::is_initialized() {
                if backoff < 10 {
                    crate::scheduler::thread::yield_();
                } else {
                    crate::scheduler::thread::sleep((1u64 << (backoff - 10)).min(1_000_000));
                }
                backoff += 1;
            } else {
                spin_loop();
            }
            #[cfg(target_arch = "aarch64")]
            spin_loop();
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(MutexGuard {
                mutex: self,
                data: self.data.get(),
            })
        } else {
            None
        }
    }

    pub fn lock_no_guard(&self) {
        #[cfg(target_arch = "x86_64")]
        let mut backoff = 0u32;
        loop {
            if self
                .lock
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }

            #[cfg(target_arch = "x86_64")]
            if crate::scheduler::is_initialized() {
                if backoff < 10 {
                    crate::scheduler::thread::yield_();
                } else {
                    crate::scheduler::thread::sleep((1u64 << (backoff - 10)).min(1_000_000));
                }
                backoff += 1;
            } else {
                spin_loop();
            }
            #[cfg(target_arch = "aarch64")]
            spin_loop();
        }
    }

    pub fn try_lock_no_guard(&self) -> bool {
        self.lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    data: *mut T,
}

unsafe impl<T: Send> Send for MutexGuard<'_, T> {}
unsafe impl<T: Send + Sync> Sync for MutexGuard<'_, T> {}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.lock.store(false, Ordering::Release);
    }
}
