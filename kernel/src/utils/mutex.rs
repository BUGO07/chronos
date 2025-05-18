/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{cell::UnsafeCell, hint::spin_loop};

#[cfg(target_arch = "x86_64")]
use crate::scheduler::{self, thread};

use super::spinlock::SpinLock;

pub struct Mutex<T> {
    inner: SpinLock<MutexInner<T>>,
}

struct MutexInner<T> {
    locked: bool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: SpinLock::new(MutexInner {
                locked: false,
                data: UnsafeCell::new(data),
            }),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        #[cfg(target_arch = "x86_64")]
        let mut backoff = 0;
        loop {
            let mut guard = self.inner.lock();
            if !guard.locked {
                guard.locked = true;
                return MutexGuard { mutex: self };
            }
            drop(guard);

            #[cfg(target_arch = "x86_64")]
            if scheduler::is_initialized() {
                if backoff < 10 {
                    thread::yld();
                } else {
                    thread::sleep((1 << (backoff - 10)).min(1_000_000));
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
        let mut guard = self.inner.lock();
        if !guard.locked {
            guard.locked = true;
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }

    pub unsafe fn force_unlock(&self) {
        let mut guard = self.inner.lock();
        guard.locked = false;
    }

    pub fn is_locked(&self) -> bool {
        let guard = self.inner.lock();
        guard.locked
    }

    pub fn into_inner(self) -> T {
        let inner = self.inner.into_inner();

        inner.data.into_inner()
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.inner.lock().data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.inner.lock().data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { self.mutex.force_unlock() };
    }
}
