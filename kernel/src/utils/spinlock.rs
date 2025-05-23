/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct SpinLock<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}

impl<T: Sized> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard { lock: self })
        } else {
            None
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }
}

impl<T: Sized> SpinLock<T> {
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: Sized + PartialEq> Eq for SpinLock<T> {}

impl<T: Sized + PartialEq> PartialEq for SpinLock<T> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { *self.data.get() == *other.data.get() }
    }
}

impl<T: Sized + core::cmp::Ord> PartialOrd for SpinLock<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Sized + core::cmp::Ord> Ord for SpinLock<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        unsafe { (*self.data.get()).cmp(&*other.data.get()) }
    }
}

impl<T: Sized + Debug> Debug for SpinLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SpinLock")
            .field("inner", unsafe { &*self.data.get() })
            .finish()
    }
}

pub struct SpinLockGuard<'a, T: ?Sized> {
    lock: &'a SpinLock<T>,
}

impl<'a, T: ?Sized> core::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> core::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}
