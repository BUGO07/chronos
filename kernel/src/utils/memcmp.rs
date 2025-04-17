pub trait Memcmp {
    fn memcmp(&self, b: &Self) -> bool;
}

impl Memcmp for [u8] {
    #[inline(always)]
    fn memcmp(&self, b: &[u8]) -> bool {
        #[allow(improper_ctypes)]
        unsafe extern "C" {
            fn memcmp(s1: *const i8, s2: *const i8, n: usize) -> i32;
        }
        self.len() == b.len()
            && unsafe {
                memcmp(
                    self.as_ptr() as *const i8,
                    b.as_ptr() as *const i8,
                    self.len(),
                ) == 0
            }
    }
}

macro_rules! memcmp_impl {
    ($int_type:ty, $bits:expr) => {
        impl Memcmp for [$int_type] {
            #[inline(always)]
            fn memcmp(&self, b: &[$int_type]) -> bool {
                let bytes = ($bits) / 8;
                let u8_len = self.len() * bytes;
                let self_ptr = self.as_ptr() as *const u8;
                let b_ptr = b.as_ptr() as *const u8;
                let self_as_bytes = unsafe { core::slice::from_raw_parts(self_ptr, u8_len) };
                let b_as_bytes = unsafe { core::slice::from_raw_parts(b_ptr, u8_len) };
                return self_as_bytes.memcmp(b_as_bytes);
            }
        }
    };
}
memcmp_impl!(u16, 16);
memcmp_impl!(u32, 32);
memcmp_impl!(u64, 64);

memcmp_impl!(i8, 8);
memcmp_impl!(i16, 16);
memcmp_impl!(i32, 32);
memcmp_impl!(i64, 64);
