/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::mem::MaybeUninit;

pub struct HeaplessVec<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Default for HeaplessVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> HeaplessVec<T, N> {
    pub const fn new() -> Self {
        Self {
            buf: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len >= N {
            return Err(item);
        }

        self.buf[self.len].write(item);
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        Some(unsafe { self.buf[self.len].assume_init_read() })
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        Some(unsafe { &*self.buf[index].as_ptr() })
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }

        Some(unsafe { &mut *self.buf[index].as_mut_ptr() })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == N
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&T, &T) -> core::cmp::Ordering,
    {
        let initialized_slice =
            unsafe { core::slice::from_raw_parts_mut(self.buf.as_mut_ptr() as *mut T, self.len) };
        initialized_slice.sort_by(compare);
    }

    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr() as *const T, self.len).iter() }
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        unsafe {
            core::slice::from_raw_parts_mut(self.buf.as_mut_ptr() as *mut T, self.len).iter_mut()
        }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr() as *const T, self.len) }
    }

    pub fn into_inner(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr() as *const T, N) }
    }
}

impl<T, const N: usize> Drop for HeaplessVec<T, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { self.buf[i].assume_init_drop() };
        }
    }
}
