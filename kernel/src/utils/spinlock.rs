/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Spin<T: ?Sized> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Spin<T> {}
unsafe impl<T: ?Sized + Send> Send for Spin<T> {}

impl<T: Sized> Spin<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Spin<T> {
    pub fn lock(&self) -> SpinGuard<'_, T> {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinGuard { spin: self }
    }

    pub fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinGuard { spin: self })
        } else {
            None
        }
    }

    pub fn lock_no_guard(&self) {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }
}

impl<T: Sized> Spin<T> {
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: Sized + Debug> Debug for Spin<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_lock() {
            Some(guard) => f.debug_struct("Spin").field("inner", &*guard).finish(),
            None => f.debug_struct("Spin").field("inner", &"<locked>").finish(),
        }
    }
}

pub struct SpinGuard<'a, T: ?Sized> {
    spin: &'a Spin<T>,
}

impl<'a, T: ?Sized> core::ops::Deref for SpinGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.spin.data.get() }
    }
}

impl<'a, T: ?Sized> core::ops::DerefMut for SpinGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.spin.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        self.spin.lock.store(false, Ordering::Release);
    }
}
