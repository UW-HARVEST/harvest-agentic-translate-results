//! Translation of `src/hashtable_seed.c`.
//!
//! The active configuration is `USE_URANDOM`, `HAVE_ATOMIC_BUILTINS` and
//! `HAVE_SCHED_YIELD`, i.e. the first `json_object_seed` variant.

use crate::types::*;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

#[unsafe(no_mangle)]
pub static mut hashtable_seed: u32 = 0;

static SEED_INITIALIZED: AtomicU8 = AtomicU8::new(0);

fn buf_to_uint32(data: &[u8; 4]) -> u32 {
    let mut result: u32 = 0;
    for i in 0..4 {
        result = (result << 8) | data[i] as u32;
    }
    result
}

/* /dev/urandom */
unsafe fn seed_from_urandom(seed: &mut u32) -> i32 {
    /* Use unbuffered I/O since we have open(), close() and read() */
    let mut data = [0u8; 4];

    let urandom = open(b"/dev/urandom\0".as_ptr() as *const c_char, O_RDONLY);
    if urandom == -1 {
        return 1;
    }

    let ok = read(urandom, data.as_mut_ptr() as *mut c_void, 4) == 4;
    close(urandom);

    if !ok {
        return 1;
    }

    *seed = buf_to_uint32(&data);
    0
}

/* gettimeofday() and getpid() */
unsafe fn seed_from_timestamp_and_pid(seed: &mut u32) -> i32 {
    /* XOR of seconds and microseconds */
    let mut tv = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    gettimeofday(&mut tv, core::ptr::null_mut());
    *seed = (tv.tv_sec as u32) ^ (tv.tv_usec as u32);

    /* XOR with PID for more randomness */
    *seed ^= getpid() as u32;

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

    let cell = AtomicU32::from_ptr(core::ptr::addr_of_mut!(hashtable_seed));

    if cell.load(Ordering::Relaxed) == 0 {
        if SEED_INITIALIZED.swap(1, Ordering::Relaxed) == 0 {
            /* Do the seeding ourselves */
            if new_seed == 0 {
                new_seed = generate_seed();
            }

            cell.store(new_seed, Ordering::Release);
        } else {
            /* Wait for another thread to do the seeding */
            loop {
                sched_yield();
                if cell.load(Ordering::Acquire) != 0 {
                    break;
                }
            }
        }
    }
}
