//! Translation of `src/hashtable_seed.c`.
//!
//! `HAVE_ATOMIC_BUILTINS`, `HAVE_SCHED_YIELD`, `HAVE_GETTIMEOFDAY`,
//! `HAVE_GETPID`, `HAVE_OPEN`/`HAVE_CLOSE`/`HAVE_READ` and `USE_URANDOM` are
//! all defined by `jansson_private_config.h`, so this is the `/dev/urandom`
//! plus `__atomic` variant.

use crate::types::*;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// `volatile uint32_t hashtable_seed = 0;`
#[unsafe(no_mangle)]
pub static hashtable_seed: AtomicU32 = AtomicU32::new(0);

static SEED_INITIALIZED: AtomicBool = AtomicBool::new(false);

const O_RDONLY: core::ffi::c_int = 0;

unsafe fn buf_to_uint32(data: *const c_char) -> u32 {
    let mut result: u32 = 0;

    for i in 0..core::mem::size_of::<u32>() {
        result = (result << 8) | (*data.add(i) as u8) as u32;
    }

    result
}

/* /dev/urandom */
unsafe fn seed_from_urandom(seed: *mut u32) -> core::ffi::c_int {
    let mut data = [0i8; 4];
    let ok: bool;

    let urandom = open(b"/dev/urandom\0".as_ptr() as *const c_char, O_RDONLY);
    if urandom == -1 {
        return 1;
    }

    ok = read(urandom, data.as_mut_ptr() as *mut c_void, 4) == 4;
    close(urandom);

    if !ok {
        return 1;
    }

    *seed = buf_to_uint32(data.as_ptr());
    0
}

/* gettimeofday() and getpid() */
unsafe fn seed_from_timestamp_and_pid(seed: *mut u32) -> core::ffi::c_int {
    let mut tv = Timeval {
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
        /* Fall back to timestamp and PID if no better randomness is available */
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

    if hashtable_seed.load(Ordering::Relaxed) == 0 {
        if !SEED_INITIALIZED.swap(true, Ordering::Relaxed) {
            /* Do the seeding ourselves */
            if new_seed == 0 {
                new_seed = generate_seed();
            }

            hashtable_seed.store(new_seed, Ordering::Release);
        } else {
            /* Wait for another thread to do the seeding */
            while hashtable_seed.load(Ordering::Acquire) == 0 {
                sched_yield();
            }
        }
    }
}
