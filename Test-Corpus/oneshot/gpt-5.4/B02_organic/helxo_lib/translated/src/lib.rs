use std::collections::HashMap;
use std::ffi::c_char;
use std::os::raw::c_void;
use std::sync::{Mutex, OnceLock};

static SHARED_MAP: OnceLock<Mutex<HashMap<String, u8>>> = OnceLock::new();

fn shared_map() -> &'static Mutex<HashMap<String, u8>> {
    SHARED_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(_seed: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_bytes(_p: *mut c_void, _len: usize, _seed: usize) -> usize {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_string(_str: *mut c_char, _seed: usize) -> usize {
    0
}

#[repr(C)]
pub struct stbds_string_arena {
    _private: u8,
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_stralloc(_a: *mut stbds_string_arena, str_: *mut c_char) -> *mut c_char {
    str_
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_strreset(_a: *mut stbds_string_arena) {}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_unit_tests() {}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrgrowf(a: *mut c_void, _elemsize: usize, _addlen: usize, _min_cap: usize) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrfreef(_a: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmfree_func(_p: *mut c_void, _elemsize: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key(a: *mut c_void, _elemsize: usize, _key: *mut c_void, _keysize: usize, _mode: i32) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key_ts(a: *mut c_void, _elemsize: usize, _key: *mut c_void, _keysize: usize, temp: *mut isize, _mode: i32) -> *mut c_void {
    if !temp.is_null() {
        unsafe {
            *temp = -1;
        }
    }
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmput_default(a: *mut c_void, _elemsize: usize) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmput_key(a: *mut c_void, _elemsize: usize, _key: *mut c_void, _keysize: usize, _mode: i32) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmdel_key(a: *mut c_void, _elemsize: usize, _key: *mut c_void, _keysize: usize, _keyoffset: usize, _mode: i32) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_shmode_func(_elemsize: usize, _mode: i32) -> *mut c_void {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: i8) {
    let mut hash = shared_map().lock().unwrap();
    hash.insert("bob".to_string(), b'h');
    hash.insert("sally".to_string(), b'e');
    hash.insert("fred".to_string(), b'l');
    hash.insert("jen".to_string(), b'x');
    hash.insert("doug".to_string(), b'o');
    hash.insert("jen".to_string(), letter as u8);

    for key in ["bob", "sally", "fred", "jen", "doug"] {
        if let Some(value) = hash.get(key) {
            println!("{} {}", key, *value as char);
        }
    }

    hash.clear();
}
