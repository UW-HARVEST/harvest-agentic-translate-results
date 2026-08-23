//! Translation of c_src/src/hashtable_seed.c
use crate::libc;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/* Exported (non-static) in the C library. */
#[unsafe(no_mangle)]
pub static mut hashtable_seed: u32 = 0;

static mut seed_initialized: u8 = 0;

#[inline]
pub fn seed_atomic() -> &'static AtomicU32 {
    unsafe { &*(std::ptr::addr_of!(hashtable_seed) as *const AtomicU32) }
}

#[inline]
fn seed_initialized_atomic() -> &'static AtomicU8 {
    unsafe { &*(std::ptr::addr_of!(seed_initialized) as *const AtomicU8) }
}

#[inline]
pub fn get_hashtable_seed() -> u32 {
    /* plain (volatile) read of hashtable_seed */
    unsafe { std::ptr::read_volatile(std::ptr::addr_of!(hashtable_seed)) }
}

unsafe fn buf_to_uint32(data: *const c_char) -> u32 {
    let mut i: usize = 0;
    let mut result: u32 = 0;

    while i < std::mem::size_of::<u32>() {
        result = (result << 8) | (*data.add(i) as u8 as u32);
        i += 1;
    }

    result
}

/* /dev/urandom */
unsafe fn seed_from_urandom(seed: *mut u32) -> i32 {
    let mut data: [c_char; 4] = [0; 4];
    let ok: bool;

    let urandom = libc::open(b"/dev/urandom\0".as_ptr() as *const c_char, libc::O_RDONLY);
    if urandom == -1 {
        return 1;
    }

    ok = libc::read(urandom, data.as_mut_ptr() as *mut c_void, 4) == 4;
    libc::close(urandom);

    if !ok {
        return 1;
    }

    *seed = buf_to_uint32(data.as_ptr());
    0
}

/* gettimeofday() and getpid() */
unsafe fn seed_from_timestamp_and_pid(seed: *mut u32) -> i32 {
    let mut tv = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    libc::gettimeofday(&mut tv, std::ptr::null_mut());
    *seed = (tv.tv_sec as u32) ^ (tv.tv_usec as u32);

    *seed ^= libc::getpid() as u32;

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

    if get_hashtable_seed() == 0 {
        if seed_initialized_atomic().swap(1, Ordering::Relaxed) == 0 {
            /* Do the seeding ourselves */
            if new_seed == 0 {
                new_seed = generate_seed();
            }

            seed_atomic().store(new_seed, Ordering::Release);
        } else {
            /* Wait for another thread to do the seeding */
            loop {
                libc::sched_yield();
                if seed_atomic().load(Ordering::Acquire) != 0 {
                    break;
                }
            }
        }
    }
}
