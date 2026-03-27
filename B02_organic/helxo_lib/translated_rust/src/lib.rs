use std::ffi::{c_char, c_int, CStr};
use std::ptr;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

/// A simple ordered string→char map that mimics stb_ds shput/shlen iteration behavior:
/// - New keys are appended in insertion order.
/// - Putting an existing key updates its value in place.
/// - Iteration is over entries in insertion order.
struct ShMap {
    entries: Vec<(&'static [u8], c_char)>,
}

impl ShMap {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn put(&mut self, key: &'static [u8], value: c_char) {
        for e in self.entries.iter_mut() {
            // Compare as C strings (up to and not including the NUL)
            if e.0 == key
                || CStr::from_bytes_with_nul(e.0).ok()
                    == CStr::from_bytes_with_nul(key).ok()
            {
                e.1 = value;
                return;
            }
        }
        self.entries.push((key, value));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash = ShMap::new();
    hash.put(b"bob\0", b'h' as c_char);
    hash.put(b"sally\0", b'e' as c_char);
    hash.put(b"fred\0", b'l' as c_char);
    hash.put(b"jen\0", b'x' as c_char);
    hash.put(b"doug\0", b'o' as c_char);
    hash.put(b"jen\0", letter);

    let fmt = b"%s %c\n\0";
    for e in &hash.entries {
        unsafe {
            printf(
                fmt.as_ptr() as *const c_char,
                e.0.as_ptr() as *const c_char,
                e.1 as c_int,
            );
        }
    }
}

// --- strkey: matches C static buffer behavior ---
static mut BUFFER: [u8; 256] = [0u8; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let fmt = b"test_%d\0";
        sprintf(
            BUFFER.as_mut_ptr() as *mut c_char,
            fmt.as_ptr() as *const c_char,
            n,
        );
        BUFFER.as_mut_ptr() as *mut c_char
    }
}

// --- stb_ds symbol stubs (exported to match C .so) ---
use libc::{c_void, ptrdiff_t, size_t};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    _a: *mut c_void, _elemsize: size_t, _addlen: size_t, _min_cap: size_t,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(_a: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(_seed: size_t) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(_str: *mut c_char, _seed: size_t) -> size_t { 0 }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(
    _p: *mut c_void, _len: size_t, _seed: size_t,
) -> size_t { 0 }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    _a: *mut c_void, _elemsize: size_t, _key: *mut c_void, _keysize: size_t,
    _temp: *mut ptrdiff_t, _mode: c_int,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    _a: *mut c_void, _elemsize: size_t, _key: *mut c_void, _keysize: size_t, _mode: c_int,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    _a: *mut c_void, _elemsize: size_t,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    _a: *mut c_void, _elemsize: size_t, _key: *mut c_void, _keysize: size_t, _mode: c_int,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(
    _elemsize: size_t, _mode: c_int,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    _a: *mut c_void, _elemsize: size_t, _key: *mut c_void, _keysize: size_t,
    _keyoffset: size_t, _mode: c_int,
) -> *mut c_void { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    _a: *mut c_void, _str: *mut c_char,
) -> *mut c_char { ptr::null_mut() }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(_a: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(_p: *mut c_void, _elemsize: size_t) {}
