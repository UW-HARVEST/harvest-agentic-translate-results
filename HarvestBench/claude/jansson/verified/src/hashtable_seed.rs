//! Translation of hashtable_seed.c
//! Config: USE_URANDOM, HAVE_OPEN/CLOSE/READ, HAVE_GETTIMEOFDAY, HAVE_GETPID,
//! HAVE_ATOMIC_BUILTINS, HAVE_SCHED_YIELD => the __atomic path is used.
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn gettimeofday(tv: *mut Timeval, tz: *mut c_void) -> c_int;
    fn getpid() -> c_int;
    fn sched_yield() -> c_int;
}

#[repr(C)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

const O_RDONLY: c_int = 0;

unsafe fn buf_to_uint32(data: *const c_char) -> u32 {
    let mut result: u32 = 0;
    for i in 0..core::mem::size_of::<u32>() {
        result = (result << 8) | (*data.add(i) as u8) as u32;
    }
    result
}

// /dev/urandom
unsafe fn seed_from_urandom(seed: *mut u32) -> c_int {
    let mut data = [0 as c_char; 4];

    let urandom = open(b"/dev/urandom\0".as_ptr() as *const c_char, O_RDONLY);
    if urandom == -1 {
        return 1;
    }

    let ok = read(
        urandom,
        data.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<u32>(),
    ) == core::mem::size_of::<u32>() as isize;
    close(urandom);

    if !ok {
        return 1;
    }

    *seed = buf_to_uint32(data.as_ptr());
    0
}

// gettimeofday() and getpid()
unsafe fn seed_from_timestamp_and_pid(seed: *mut u32) -> c_int {
    let mut tv = Timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    gettimeofday(&mut tv, core::ptr::null_mut());
    *seed = (tv.tv_sec as u32) ^ (tv.tv_usec as u32);

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
        seed_from_timestamp_and_pid(&mut seed);
    }

    /* Make sure the seed is never zero */
    if seed == 0 {
        seed = 1;
    }

    seed
}

// volatile uint32_t hashtable_seed = 0;
#[unsafe(no_mangle)]
pub static hashtable_seed: AtomicU32 = AtomicU32::new(0);

static seed_initialized: AtomicU8 = AtomicU8::new(0);

// __atomic path (HAVE_ATOMIC_BUILTINS && HAVE_SCHED_YIELD)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_object_seed(seed: usize) {
    let mut new_seed = seed as u32;

    if hashtable_seed.load(Ordering::Relaxed) == 0 {
        // __atomic_test_and_set(&seed_initialized, __ATOMIC_RELAXED) == 0
        if seed_initialized.swap(1, Ordering::Relaxed) == 0 {
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
