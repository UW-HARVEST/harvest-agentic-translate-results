//! Translation of `src/hashtable_seed.c`.
//!
//! The C build has both HAVE_ATOMIC_BUILTINS and HAVE_SCHED_YIELD, so the
//! `__atomic` variant of `json_object_seed()` is the active one.

use crate::cffi;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

fn buf_to_uint32(data: &[u8; 4]) -> u32 {
    let mut result: u32 = 0;
    for i in 0..4 {
        result = (result << 8) | data[i] as u32;
    }
    result
}

/* /dev/urandom */
unsafe fn seed_from_urandom(seed: &mut u32) -> core::ffi::c_int {
    unsafe {
        let mut data = [0u8; 4];

        let urandom = cffi::open(c"/dev/urandom".as_ptr(), cffi::O_RDONLY);
        if urandom == -1 {
            return 1;
        }

        let ok = cffi::read(urandom, data.as_mut_ptr() as *mut c_void, 4) == 4;
        cffi::close(urandom);

        if !ok {
            return 1;
        }

        *seed = buf_to_uint32(&data);
        0
    }
}

/* gettimeofday() and getpid() */
unsafe fn seed_from_timestamp_and_pid(seed: &mut u32) -> core::ffi::c_int {
    unsafe {
        /* XOR of seconds and microseconds */
        let mut tv = cffi::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        cffi::gettimeofday(&mut tv, core::ptr::null_mut());
        *seed = (tv.tv_sec as u32) ^ (tv.tv_usec as u32);

        /* XOR with PID for more randomness */
        *seed ^= cffi::getpid() as u32;

        0
    }
}

unsafe fn generate_seed() -> u32 {
    unsafe {
        let mut seed: u32 = 0;
        let mut done = 0;

        if seed_from_urandom(&mut seed) == 0 {
            done = 1;
        }

        if done == 0 {
            /* Fall back to timestamp and PID if no better randomness is
            available */
            seed_from_timestamp_and_pid(&mut seed);
        }

        /* Make sure the seed is never zero */
        if seed == 0 {
            seed = 1;
        }

        seed
    }
}

/// `volatile uint32_t hashtable_seed = 0;` - an exported global.
#[unsafe(no_mangle)]
pub static hashtable_seed: AtomicU32 = AtomicU32::new(0);

static seed_initialized: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn hashtable_seed_value() -> u32 {
    hashtable_seed.load(Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_seed(seed: usize) {
    unsafe {
        let mut new_seed = seed as u32;

        if hashtable_seed.load(Ordering::Relaxed) == 0 {
            /* __atomic_test_and_set(&seed_initialized, __ATOMIC_RELAXED) */
            if seed_initialized.swap(1, Ordering::Relaxed) == 0 {
                /* Do the seeding ourselves */
                if new_seed == 0 {
                    new_seed = generate_seed();
                }

                hashtable_seed.store(new_seed, Ordering::Release);
            } else {
                /* Wait for another thread to do the seeding */
                while hashtable_seed.load(Ordering::Acquire) == 0 {
                    cffi::sched_yield();
                }
            }
        }
    }
}
