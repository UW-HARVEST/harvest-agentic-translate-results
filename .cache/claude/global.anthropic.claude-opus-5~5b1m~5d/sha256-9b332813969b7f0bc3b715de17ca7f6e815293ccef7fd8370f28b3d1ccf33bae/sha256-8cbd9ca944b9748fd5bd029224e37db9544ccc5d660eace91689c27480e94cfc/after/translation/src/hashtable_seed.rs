//! Translation of `src/hashtable_seed.c`.
//!
//! Generate sizeof(uint32_t) bytes of as random data as possible to seed the
//! hash function.

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::ffi;

/// Exported (and deliberately mutable) hash seed, mirroring
/// `volatile uint32_t hashtable_seed`.
#[unsafe(no_mangle)]
pub static mut hashtable_seed: u32 = 0;

static SEED_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[inline]
pub unsafe fn seed_value() -> u32 {
    core::ptr::addr_of!(hashtable_seed).read_volatile()
}

unsafe fn buf_to_uint32(data: *const c_char) -> u32 {
    let mut result: u32 = 0;

    for i in 0..core::mem::size_of::<u32>() {
        result = (result << 8) | (*data.add(i) as u8) as u32;
    }

    result
}

/* /dev/urandom */
unsafe fn seed_from_urandom(seed: *mut u32) -> i32 {
    /* Use unbuffered I/O if we have open(), close() and read(). */
    let mut data = [0i8; 4];

    let urandom = ffi::open(b"/dev/urandom\0".as_ptr() as *const c_char, ffi::O_RDONLY);
    if urandom == -1 {
        return 1;
    }

    let ok = ffi::read(urandom, data.as_mut_ptr() as *mut c_void, 4) == 4;
    ffi::close(urandom);

    if !ok {
        return 1;
    }

    *seed = buf_to_uint32(data.as_ptr() as *const c_char);
    0
}

/* gettimeofday() and getpid() */
unsafe fn seed_from_timestamp_and_pid(seed: *mut u32) -> i32 {
    /* XOR of seconds and microseconds */
    let mut tv = ffi::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    ffi::gettimeofday(&mut tv, core::ptr::null_mut());
    *seed = (tv.tv_sec as u32) ^ (tv.tv_usec as u32);

    /* XOR with PID for more randomness */
    *seed ^= ffi::getpid() as u32;

    0
}

unsafe fn generate_seed() -> u32 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_seed(seed: usize) {
    let mut new_seed = seed as u32;

    if seed_value() == 0 {
        let atomic = AtomicU32::from_ptr(core::ptr::addr_of_mut!(hashtable_seed));
        if !SEED_INITIALIZED.swap(true, Ordering::Relaxed) {
            /* Do the seeding ourselves */
            if new_seed == 0 {
                new_seed = generate_seed();
            }

            atomic.store(new_seed, Ordering::Release);
        } else {
            /* Wait for another thread to do the seeding */
            loop {
                ffi::sched_yield();
                if atomic.load(Ordering::Acquire) != 0 {
                    break;
                }
            }
        }
    }
}
