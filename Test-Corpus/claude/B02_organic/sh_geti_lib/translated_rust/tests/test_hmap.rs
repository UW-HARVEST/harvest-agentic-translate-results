//! Mid-level tests: stbds_shmode_func, stbds_hmput_key, stbds_hmget_key,
//! stbds_hmdel_key, stbds_hmfree_func driving the full hashmap path.

mod common;

use common::{c_lib_path, ensure_libs_built, rust_lib_path};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

#[repr(C)]
#[derive(Copy, Clone)]
struct Entry {
    key: *mut c_char,
    value: c_int,
    _pad: c_int,
}

const ELEMSIZE: usize = std::mem::size_of::<Entry>();
const KEYSIZE: usize = std::mem::size_of::<*mut c_char>();

struct Lib {
    _lib: Library,
    shmode: unsafe extern "C" fn(usize, c_int) -> *mut c_void,
    hmput: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    hmget: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
    hmdel: unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void,
    hmfree: unsafe extern "C" fn(*mut c_void, usize),
    rand_seed: unsafe extern "C" fn(usize),
}

impl Lib {
    fn load(p: &std::path::Path) -> Self {
        unsafe {
            let lib = Library::new(p).expect("load lib");
            let shmode: Symbol<unsafe extern "C" fn(usize, c_int) -> *mut c_void> =
                lib.get(b"stbds_shmode_func").unwrap();
            let hmput: Symbol<
                unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
            > = lib.get(b"stbds_hmput_key").unwrap();
            let hmget: Symbol<
                unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void,
            > = lib.get(b"stbds_hmget_key").unwrap();
            let hmdel: Symbol<
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    usize,
                    c_int,
                ) -> *mut c_void,
            > = lib.get(b"stbds_hmdel_key").unwrap();
            let hmfree: Symbol<unsafe extern "C" fn(*mut c_void, usize)> =
                lib.get(b"stbds_hmfree_func").unwrap();
            let rand_seed: Symbol<unsafe extern "C" fn(usize)> =
                lib.get(b"stbds_rand_seed").unwrap();
            Self {
                shmode: *shmode,
                hmput: *hmput,
                hmget: *hmget,
                hmdel: *hmdel,
                hmfree: *hmfree,
                rand_seed: *rand_seed,
                _lib: lib,
            }
        }
    }
}

unsafe fn hash_to_array(p: *mut c_void) -> *mut c_void {
    unsafe { (p as *mut u8).sub(ELEMSIZE) as *mut c_void }
}
#[repr(C)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}
unsafe fn header(p: *mut c_void) -> *mut Header {
    unsafe { (p as *mut Header).sub(1) }
}

fn run_strdup(lib: &Lib) -> Vec<(String, c_int)> {
    unsafe {
        // Reset seed so both libs allocate identically.
        (lib.rand_seed)(0x31415926);

        // sh_new_strdup: stbds_shmode_func(elemsize, STBDS_SH_STRDUP).
        // shmode_func already returns the "hash side" pointer (t[-1] valid).
        let p = (lib.shmode)(ELEMSIZE, STBDS_SH_STRDUP);
        let mut arr = p as *mut Entry;

        // Insert ten keys.
        let mut owners: Vec<Vec<u8>> = (0..10)
            .map(|i| format!("key_{i}\0").into_bytes())
            .collect();
        for (i, owner) in owners.iter_mut().enumerate() {
            let kp = owner.as_mut_ptr() as *mut c_void;
            let new_p = (lib.hmput)(arr as *mut c_void, ELEMSIZE, kp, KEYSIZE, STBDS_HM_STRING);
            arr = new_p as *mut Entry;
            // Get insertion slot from header->temp.
            let raw = hash_to_array(arr as *mut c_void);
            let temp = (*header(raw)).temp;
            (*arr.offset(temp)).value = (i as c_int) * 100;
        }

        // Read all entries in insertion order.
        let raw = hash_to_array(arr as *mut c_void);
        let length = (*header(raw)).length;
        let mut out = Vec::new();
        // length includes the [-1] default slot, so length-1 entries from
        // index 0..length-1 in arr (which already points to arr+elemsize).
        for z in 0..(length - 1) {
            let entry = arr.add(z);
            let key = std::ffi::CStr::from_ptr((*entry).key)
                .to_str()
                .unwrap()
                .to_string();
            out.push((key, (*entry).value));
        }

        // Now delete every other and then read remaining.
        for owner in owners.iter_mut().step_by(2) {
            let kp = owner.as_mut_ptr() as *mut c_void;
            let new_p = (lib.hmdel)(
                arr as *mut c_void,
                ELEMSIZE,
                kp,
                KEYSIZE,
                0,
                STBDS_HM_STRING,
            );
            arr = new_p as *mut Entry;
        }

        // Read remaining via hmget.
        let mut remaining = Vec::new();
        for owner in owners.iter_mut() {
            let kp = owner.as_mut_ptr() as *mut c_void;
            let new_p = (lib.hmget)(arr as *mut c_void, ELEMSIZE, kp, KEYSIZE, STBDS_HM_STRING);
            arr = new_p as *mut Entry;
            let raw = hash_to_array(arr as *mut c_void);
            let temp = (*header(raw)).temp;
            // Convert key in owner to string.
            let key = String::from_utf8_lossy(&owner[..owner.len() - 1]).to_string();
            if temp == -1 {
                remaining.push((key, -1));
            } else {
                remaining.push((key, (*arr.offset(temp)).value));
            }
        }
        out.extend(remaining);

        // Free.
        let raw = hash_to_array(arr as *mut c_void);
        (lib.hmfree)(raw, ELEMSIZE);

        out
    }
}

#[test]
fn test_hashmap_strdup_path() {
    ensure_libs_built();
    let c_lib = Lib::load(&c_lib_path());
    let r_lib = Lib::load(&rust_lib_path());

    let c_out = run_strdup(&c_lib);
    let r_out = run_strdup(&r_lib);
    assert_eq!(c_out, r_out);
}

fn run_arena(lib: &Lib) -> Vec<(String, c_int)> {
    unsafe {
        (lib.rand_seed)(0x31415926);

        let p = (lib.shmode)(ELEMSIZE, STBDS_SH_ARENA);
        let mut arr = p as *mut Entry;

        let mut owners: Vec<Vec<u8>> = (0..16)
            .map(|i| format!("arena_key_{i:03}\0").into_bytes())
            .collect();

        for (i, owner) in owners.iter_mut().enumerate() {
            let kp = owner.as_mut_ptr() as *mut c_void;
            let new_p = (lib.hmput)(arr as *mut c_void, ELEMSIZE, kp, KEYSIZE, STBDS_HM_STRING);
            arr = new_p as *mut Entry;
            let raw = hash_to_array(arr as *mut c_void);
            let temp = (*header(raw)).temp;
            (*arr.offset(temp)).value = (i as c_int) + 1000;
        }

        let raw = hash_to_array(arr as *mut c_void);
        let length = (*header(raw)).length;
        let mut out = Vec::new();
        for z in 0..(length - 1) {
            let entry = arr.add(z);
            let key = std::ffi::CStr::from_ptr((*entry).key)
                .to_str()
                .unwrap()
                .to_string();
            out.push((key, (*entry).value));
        }

        let raw = hash_to_array(arr as *mut c_void);
        (lib.hmfree)(raw, ELEMSIZE);

        out
    }
}

#[test]
fn test_hashmap_arena_path() {
    ensure_libs_built();
    let c_lib = Lib::load(&c_lib_path());
    let r_lib = Lib::load(&rust_lib_path());

    let c_out = run_arena(&c_lib);
    let r_out = run_arena(&r_lib);
    assert_eq!(c_out, r_out);
}

fn run_binary(lib: &Lib) -> Vec<(u64, c_int)> {
    unsafe {
        (lib.rand_seed)(0x31415926);

        // Binary key map: key is u64. Entry layout: { u64 key; int value; int pad; } = 16 bytes.
        #[repr(C)]
        struct U64Entry {
            key: u64,
            value: c_int,
            _pad: c_int,
        }
        let elem = std::mem::size_of::<U64Entry>();
        let key_size = std::mem::size_of::<u64>();

        let mut arr: *mut U64Entry = std::ptr::null_mut();

        let mut keys: Vec<u64> = (0..32u64).map(|i| i.wrapping_mul(1234567)).collect();
        for (i, k) in keys.iter_mut().enumerate() {
            let new_p = (lib.hmput)(
                arr as *mut c_void,
                elem,
                k as *mut u64 as *mut c_void,
                key_size,
                STBDS_HM_BINARY,
            );
            arr = new_p as *mut U64Entry;
            let raw = (arr as *mut u8).sub(elem) as *mut c_void;
            let temp = (*header(raw)).temp;
            (*arr.offset(temp)).value = (i as c_int).wrapping_mul(7);
        }

        let raw = (arr as *mut u8).sub(elem) as *mut c_void;
        let length = (*header(raw)).length;
        let mut out = Vec::new();
        for z in 0..(length - 1) {
            let entry = arr.add(z);
            out.push(((*entry).key, (*entry).value));
        }

        // Free.
        (lib.hmfree)(raw, elem);
        out
    }
}

#[test]
fn test_hashmap_binary_path() {
    ensure_libs_built();
    let c_lib = Lib::load(&c_lib_path());
    let r_lib = Lib::load(&rust_lib_path());

    let c_out = run_binary(&c_lib);
    let r_out = run_binary(&r_lib);
    assert_eq!(c_out, r_out);
}
