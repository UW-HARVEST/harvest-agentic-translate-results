#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ArrPush = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arr_grow: ArrGrow,
    arr_free: ArrFree,
    rand_seed: RandSeed,
    hash_bytes: HashBytes,
    hash_string: HashString,
    hm_free: HmFree,
    hm_get: HmGet,
    hm_get_ts: HmGetTs,
    hm_put_default: HmPutDefault,
    hm_put: HmPut,
    hm_del: HmDel,
    sh_mode: ShMode,
    str_alloc: StrAlloc,
    str_reset: StrReset,
    str_key: StrKey,
    arr_push: ArrPush,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        const RTLD_NOW: c_int = 2;
        const RTLD_DEEPBIND: c_int = 8;
        let library: Library =
            libloading::os::unix::Library::open(Some(&path), RTLD_NOW | RTLD_DEEPBIND)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
                .into();
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap()
            };
        }
        Self {
            arr_grow: symbol!("stbds_arrgrowf", ArrGrow),
            arr_free: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hm_free: symbol!("stbds_hmfree_func", HmFree),
            hm_get: symbol!("stbds_hmget_key", HmGet),
            hm_get_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hm_put_default: symbol!("stbds_hmput_default", HmPutDefault),
            hm_put: symbol!("stbds_hmput_key", HmPut),
            hm_del: symbol!("stbds_hmdel_key", HmDel),
            sh_mode: symbol!("stbds_shmode_func", ShMode),
            str_alloc: symbol!("stbds_stralloc", StrAlloc),
            str_reset: symbol!("stbds_strreset", StrReset),
            str_key: symbol!("strkey", StrKey),
            arr_push: symbol!("arr_push", ArrPush),
            _library: library,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Debug)]
struct Arena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OffsetEntry {
    key: u64,
    alias: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

unsafe fn header(array: *mut c_void) -> *mut Header {
    array.cast::<Header>().sub(1)
}

unsafe fn map_raw(map: *mut c_void, element_size: usize) -> *mut c_void {
    map.cast::<u8>().sub(element_size).cast()
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut Header {
    header(map_raw(map, element_size))
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libtranslated_rust.so"),
        root.join("target/release/libarr_push_lib.so"),
    )
}

fn with_apis(test: impl FnOnce(&Api, &Api)) {
    static SERIAL: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = SERIAL.get_or_init(|| Mutex::new(())).lock().unwrap();
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe {
        let c = Api::load(c_path);
        let rust = Api::load(rust_path);
        test(&c, &rust);
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}

#[test]
fn config_rows_7_to_12_hash_functions_match() {
    with_apis(|c, rust| unsafe {
        let mut rng = Rng(0x6a09_e667_f3bc_c909);
        for &length in &[
            0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 31, 32, 63, 64, 257,
        ] {
            for _ in 0..64 {
                let seed = rng.next() as usize;
                let mut bytes = vec![0u8; length];
                for byte in &mut bytes {
                    *byte = rng.byte();
                }
                let pointer = if length == 0 {
                    null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                assert_eq!(
                    (c.hash_bytes)(pointer, length, seed),
                    (rust.hash_bytes)(pointer, length, seed),
                    "hash_bytes length={length} seed={seed:#x}"
                );
            }
        }

        for &length in &[0usize, 1, 2, 7, 31, 255] {
            for _ in 0..64 {
                let seed = rng.next() as usize;
                let mut bytes = Vec::with_capacity(length + 1);
                for _ in 0..length {
                    let mut byte = rng.byte();
                    if byte == 0 {
                        byte = 0x80;
                    }
                    bytes.push(byte);
                }
                bytes.push(0);
                let pointer = bytes.as_mut_ptr().cast::<c_char>();
                assert_eq!(
                    (c.hash_string)(pointer, seed),
                    (rust.hash_string)(pointer, seed),
                    "hash_string length={length} seed={seed:#x}"
                );
            }
        }
    });
}

unsafe fn allocation_case(
    api: &Api,
    element_size: usize,
    initial_capacity: usize,
    length: usize,
    add_length: usize,
    minimum_capacity: usize,
) -> (usize, usize, Vec<u8>, bool) {
    let mut array = if initial_capacity == 0 {
        null_mut()
    } else {
        (api.arr_grow)(null_mut(), element_size, 0, initial_capacity)
    };
    if !array.is_null() {
        (*header(array)).length = length;
        let byte_length = length * element_size;
        for index in 0..byte_length {
            *array.cast::<u8>().add(index) = (index as u8).wrapping_mul(37);
        }
    }
    let old = array;
    array = (api.arr_grow)(array, element_size, add_length, minimum_capacity);
    if array.is_null() {
        return (0, 0, Vec::new(), old == array);
    }
    let result = (
        (*header(array)).length,
        (*header(array)).capacity,
        std::slice::from_raw_parts(array.cast::<u8>(), length * element_size).to_vec(),
        old == array,
    );
    (api.arr_free)(array);
    result
}

#[test]
fn config_rows_1_to_6_dynamic_array_growth_matches() {
    with_apis(|c, rust| unsafe {
        let cases = [
            (1, 0, 0, 0, 0),
            (1, 0, 0, 1, 0),
            (4, 0, 0, 0, 3),
            (16, 0, 0, 9, 4),
            (4, 8, 3, 0, 7),
            (4, 8, 7, 1, 0),
            (4, 8, 7, 5, 0),
            (4, 8, 7, 20, 0),
            (16, 9, 9, 0, 40),
        ];
        for case in cases {
            assert_eq!(
                allocation_case(c, case.0, case.1, case.2, case.3, case.4),
                allocation_case(rust, case.0, case.1, case.2, case.3, case.4),
                "array case {case:?}"
            );
        }
    });
}

#[test]
fn config_rows_37_and_38_leaf_exports_match() {
    with_apis(|c, rust| unsafe {
        for value in [
            c_int::MIN,
            -10_000,
            -1,
            0,
            1,
            49,
            50,
            51,
            10_000,
            c_int::MAX,
        ] {
            let c_value = CStr::from_ptr((c.str_key)(value)).to_bytes().to_vec();
            let rust_value = CStr::from_ptr((rust.str_key)(value)).to_bytes().to_vec();
            assert_eq!(c_value, rust_value, "strkey({value})");
        }
        for value in [-100, -1, 0, 1, 49, 50, 51, 101, 500] {
            (c.arr_push)(value);
            (rust.arr_push)(value);
        }
    });
}

unsafe fn arena_alloc(api: &Api, arena: &mut Arena, bytes: &[u8]) -> Vec<u8> {
    let string = CString::new(bytes).unwrap();
    CStr::from_ptr((api.str_alloc)(arena, string.as_ptr().cast_mut()))
        .to_bytes()
        .to_vec()
}

#[test]
fn config_rows_32_to_36_string_arena_matches() {
    with_apis(|c, rust| unsafe {
        let mut c_arena: Arena = zeroed();
        let mut rust_arena: Arena = zeroed();
        let mut rng = Rng(0xbb67_ae85_84ca_a73b);
        for &length in &[0usize, 1, 510, 511, 512, 700, 10, 2048] {
            let bytes: Vec<u8> = (0..length)
                .map(|_| {
                    let byte = rng.byte();
                    if byte == 0 { 1 } else { byte }
                })
                .collect();
            assert_eq!(
                arena_alloc(c, &mut c_arena, &bytes),
                arena_alloc(rust, &mut rust_arena, &bytes)
            );
            assert_eq!(
                (c_arena.remaining, c_arena.block, c_arena.mode),
                (rust_arena.remaining, rust_arena.block, rust_arena.mode)
            );
        }

        for exponent in 0..24 {
            let length = 513usize << (exponent / 2).min(11);
            let bytes = vec![b'x'; length];
            assert_eq!(
                arena_alloc(c, &mut c_arena, &bytes),
                arena_alloc(rust, &mut rust_arena, &bytes)
            );
            assert_eq!(
                (c_arena.remaining, c_arena.block),
                (rust_arena.remaining, rust_arena.block)
            );
        }

        (c.str_reset)(&mut c_arena);
        (rust.str_reset)(&mut rust_arena);
        assert_eq!(
            (
                c_arena.storage,
                c_arena.remaining,
                c_arena.block,
                c_arena.mode
            ),
            (null_mut(), 0, 0, 0)
        );
        assert_eq!(
            (
                rust_arena.storage,
                rust_arena.remaining,
                rust_arena.block,
                rust_arena.mode
            ),
            (null_mut(), 0, 0, 0)
        );
        (c.str_reset)(&mut c_arena);
        (rust.str_reset)(&mut rust_arena);
    });
}

unsafe fn binary_insert(api: &Api, map: *mut c_void, key: u64, value: i64) -> *mut c_void {
    let result = (api.hm_put)(
        map,
        size_of::<Entry>(),
        (&key as *const u64).cast_mut().cast(),
        size_of::<u64>(),
        0,
    );
    let index = (*map_header(result, size_of::<Entry>())).temp;
    assert!(index >= 0);
    (*result.cast::<Entry>().add(index as usize)).value = value;
    result
}

unsafe fn binary_snapshot(map: *mut c_void) -> Vec<Entry> {
    let count = (*map_header(map, size_of::<Entry>())).length - 1;
    std::slice::from_raw_parts(map.cast::<Entry>(), count).to_vec()
}

unsafe fn binary_lookup(api: &Api, map: *mut c_void, key: u64, threaded: bool) -> (isize, i64) {
    let result;
    let index;
    if threaded {
        let mut temp = 777;
        result = (api.hm_get_ts)(
            map,
            size_of::<Entry>(),
            (&key as *const u64).cast_mut().cast(),
            size_of::<u64>(),
            &mut temp,
            0,
        );
        index = temp;
    } else {
        result = (api.hm_get)(
            map,
            size_of::<Entry>(),
            (&key as *const u64).cast_mut().cast(),
            size_of::<u64>(),
            0,
        );
        index = (*map_header(result, size_of::<Entry>())).temp;
    }
    let value = if index < 0 {
        (*result.cast::<Entry>().sub(1)).value
    } else {
        (*result.cast::<Entry>().add(index as usize)).value
    };
    (index, value)
}

unsafe fn binary_free(api: &Api, map: *mut c_void) {
    if !map.is_null() {
        (api.hm_free)(map_raw(map, size_of::<Entry>()), size_of::<Entry>());
    }
}

#[test]
fn config_rows_13_to_18_binary_map_insert_lookup_and_growth_match() {
    with_apis(|c, rust| unsafe {
        let mut rng = Rng(0x3c6e_f372_fe94_f82b);
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, rng.next() as usize] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let mut c_map = null_mut();
            let mut rust_map = null_mut();
            for index in 0..160 {
                let key = if index % 11 == 0 {
                    (index / 2) as u64
                } else {
                    rng.next()
                };
                let value = rng.next() as i64;
                c_map = binary_insert(c, c_map, key, value);
                rust_map = binary_insert(rust, rust_map, key, value);
                assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));
                if index % 17 == 0 {
                    let replacement = value ^ 0x55aa_55aa_55aa_55aa;
                    c_map = binary_insert(c, c_map, key, replacement);
                    rust_map = binary_insert(rust, rust_map, key, replacement);
                    assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));
                }
                assert_eq!(
                    binary_lookup(c, c_map, key, index % 2 == 0),
                    binary_lookup(rust, rust_map, key, index % 2 == 0)
                );
                let missing = key ^ 0xa5a5_a5a5_a5a5_a5a5;
                assert_eq!(
                    binary_lookup(c, c_map, missing, index % 2 != 0),
                    binary_lookup(rust, rust_map, missing, index % 2 != 0)
                );
            }
            binary_free(c, c_map);
            binary_free(rust, rust_map);
        }
    });
}

#[test]
fn errors_rows_2_3_and_6_force_both_missing_slot_exits() {
    with_apis(|c, rust| unsafe {
        let seed = 0x5a17_9d3busize;
        let mut colliding = Vec::new();
        let mut immediate = None;
        for key in 0..100_000u64 {
            let hash = (c.hash_bytes)(
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                seed,
            );
            let position = (if hash < 2 { hash + 2 } else { hash }) & 7;
            if position == 6 && colliding.len() < 3 {
                colliding.push(key);
            }
            if position == 2 && immediate.is_none() {
                immediate = Some(key);
            }
            if colliding.len() == 3 && immediate.is_some() {
                break;
            }
        }
        assert_eq!(colliding.len(), 3);
        let immediate = immediate.unwrap();

        (c.rand_seed)(seed);
        (rust.rand_seed)(seed);
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        for &key in &colliding[..2] {
            c_map = binary_insert(c, c_map, key, key as i64);
            rust_map = binary_insert(rust, rust_map, key, key as i64);
        }

        // Position 2 is empty in the first scan; position 6 wraps after slots 6 and 7.
        assert_eq!(
            binary_lookup(c, c_map, immediate, true),
            binary_lookup(rust, rust_map, immediate, true)
        );
        assert_eq!(
            binary_lookup(c, c_map, colliding[2], true),
            binary_lookup(rust, rust_map, colliding[2], true)
        );
        assert_eq!(binary_lookup(c, c_map, immediate, true).0, -1);
        assert_eq!(binary_lookup(c, c_map, colliding[2], true).0, -1);

        binary_free(c, c_map);
        binary_free(rust, rust_map);
    });
}

#[test]
fn config_row_16_binary_key_widths_match() {
    with_apis(|c, rust| unsafe {
        let mut rng = Rng(0xa54f_f53a_5f1d_36f1);
        for key_size in [1usize, 4, 8, 16] {
            for _ in 0..64 {
                let mut key = [0u8; 16];
                for byte in &mut key {
                    *byte = rng.byte();
                }
                let element_size = 24;
                let c_map = (c.hm_put)(
                    null_mut(),
                    element_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                let rust_map = (rust.hm_put)(
                    null_mut(),
                    element_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                let c_index = (*map_header(c_map, element_size)).temp as usize;
                let rust_index = (*map_header(rust_map, element_size)).temp as usize;
                assert_eq!(c_index, rust_index);
                assert_eq!(
                    std::slice::from_raw_parts(
                        c_map.cast::<u8>().add(c_index * element_size),
                        key_size
                    ),
                    std::slice::from_raw_parts(
                        rust_map.cast::<u8>().add(rust_index * element_size),
                        key_size
                    )
                );
                (c.hm_free)(map_raw(c_map, element_size), element_size);
                (rust.hm_free)(map_raw(rust_map, element_size), element_size);
            }
        }
    });
}

#[test]
fn config_rows_14_15_23_and_error_sentinels_match() {
    with_apis(|c, rust| unsafe {
        for threaded in [false, true] {
            let key = 123u64;
            let mut c_temp = 99isize;
            let mut rust_temp = 99isize;
            let c_map = if threaded {
                (c.hm_get_ts)(
                    null_mut(),
                    size_of::<Entry>(),
                    (&key as *const u64).cast_mut().cast(),
                    size_of::<u64>(),
                    &mut c_temp,
                    0,
                )
            } else {
                let map = (c.hm_get)(
                    null_mut(),
                    size_of::<Entry>(),
                    (&key as *const u64).cast_mut().cast(),
                    size_of::<u64>(),
                    0,
                );
                c_temp = (*map_header(map, size_of::<Entry>())).temp;
                map
            };
            let rust_map = if threaded {
                (rust.hm_get_ts)(
                    null_mut(),
                    size_of::<Entry>(),
                    (&key as *const u64).cast_mut().cast(),
                    size_of::<u64>(),
                    &mut rust_temp,
                    0,
                )
            } else {
                let map = (rust.hm_get)(
                    null_mut(),
                    size_of::<Entry>(),
                    (&key as *const u64).cast_mut().cast(),
                    size_of::<u64>(),
                    0,
                );
                rust_temp = (*map_header(map, size_of::<Entry>())).temp;
                map
            };
            assert_eq!(c_temp, -1);
            assert_eq!(c_temp, rust_temp);
            assert_eq!((*map_header(c_map, size_of::<Entry>())).length, 1);
            assert_eq!(
                (*map_header(c_map, size_of::<Entry>())).length,
                (*map_header(rust_map, size_of::<Entry>())).length
            );
            assert_eq!(
                std::slice::from_raw_parts(
                    map_raw(c_map, size_of::<Entry>()).cast::<u8>(),
                    size_of::<Entry>()
                ),
                std::slice::from_raw_parts(
                    map_raw(rust_map, size_of::<Entry>()).cast::<u8>(),
                    size_of::<Entry>()
                )
            );

            let mut c_missing = 99;
            let mut rust_missing = 99;
            let c_same = (c.hm_get_ts)(
                c_map,
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                &mut c_missing,
                0,
            );
            let rust_same = (rust.hm_get_ts)(
                rust_map,
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                &mut rust_missing,
                0,
            );
            assert_eq!(c_same, c_map);
            assert_eq!(rust_same, rust_map);
            assert_eq!(c_missing, -1);
            assert_eq!(c_missing, rust_missing);

            let c_same = (c.hm_put_default)(c_map, size_of::<Entry>());
            let rust_same = (rust.hm_put_default)(rust_map, size_of::<Entry>());
            assert_eq!(c_same, c_map);
            assert_eq!(rust_same, rust_map);
            assert_eq!((*map_header(c_same, size_of::<Entry>())).length, 1);
            assert_eq!(
                (*map_header(c_same, size_of::<Entry>())).length,
                (*map_header(rust_same, size_of::<Entry>())).length
            );

            let c_deleted = (c.hm_del)(
                c_same,
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                0,
                0,
            );
            let rust_deleted = (rust.hm_del)(
                rust_same,
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                0,
                0,
            );
            assert_eq!(c_deleted, c_same);
            assert_eq!(rust_deleted, rust_same);
            assert_eq!(
                (*map_header(c_deleted, size_of::<Entry>())).temp,
                (*map_header(rust_deleted, size_of::<Entry>())).temp
            );
            binary_free(c, c_deleted);
            binary_free(rust, rust_deleted);
        }

        let c_default = (c.hm_put_default)(null_mut(), size_of::<Entry>());
        let rust_default = (rust.hm_put_default)(null_mut(), size_of::<Entry>());
        assert_eq!(
            std::slice::from_raw_parts(
                map_raw(c_default, size_of::<Entry>()).cast::<u8>(),
                size_of::<Entry>()
            ),
            std::slice::from_raw_parts(
                map_raw(rust_default, size_of::<Entry>()).cast::<u8>(),
                size_of::<Entry>()
            )
        );
        binary_free(c, c_default);
        binary_free(rust, rust_default);

        assert!(
            (c.hm_del)(
                null_mut(),
                size_of::<Entry>(),
                null_mut(),
                size_of::<u64>(),
                0,
                0
            )
            .is_null()
        );
        assert!(
            (rust.hm_del)(
                null_mut(),
                size_of::<Entry>(),
                null_mut(),
                size_of::<u64>(),
                0,
                0
            )
            .is_null()
        );
        (c.hm_free)(null_mut(), size_of::<Entry>());
        (rust.hm_free)(null_mut(), size_of::<Entry>());
    });
}

unsafe fn binary_delete(api: &Api, map: *mut c_void, key: u64) -> (*mut c_void, isize) {
    let result = (api.hm_del)(
        map,
        size_of::<Entry>(),
        (&key as *const u64).cast_mut().cast(),
        size_of::<u64>(),
        0,
        0,
    );
    let removed = (*map_header(result, size_of::<Entry>())).temp;
    (result, removed)
}

#[test]
fn config_rows_24_to_28_binary_delete_rebuild_and_shrink_match() {
    with_apis(|c, rust| unsafe {
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        for key in 0..160u64 {
            c_map = binary_insert(c, c_map, key, (key as i64).wrapping_mul(-17));
            rust_map = binary_insert(rust, rust_map, key, (key as i64).wrapping_mul(-17));
        }
        assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));

        let mut deletes = vec![999_999, 0, 79, 159, 17, 18, 19, 20];
        deletes.extend((1..150).step_by(2));
        deletes.extend((2..150).step_by(3));
        for key in deletes {
            let (next_c, removed_c) = binary_delete(c, c_map, key);
            let (next_rust, removed_rust) = binary_delete(rust, rust_map, key);
            c_map = next_c;
            rust_map = next_rust;
            assert_eq!(removed_c, removed_rust, "delete key {key}");
            assert_eq!(
                binary_snapshot(c_map),
                binary_snapshot(rust_map),
                "delete key {key}"
            );
            assert_eq!(
                binary_lookup(c, c_map, key, true),
                binary_lookup(rust, rust_map, key, true)
            );
        }

        for key in 1000..1080u64 {
            c_map = binary_insert(c, c_map, key, key as i64);
            rust_map = binary_insert(rust, rust_map, key, key as i64);
            assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));
        }
        binary_free(c, c_map);
        binary_free(rust, rust_map);
    });
}

#[test]
fn config_row_30_nonzero_delete_key_offset_matches() {
    with_apis(|c, rust| unsafe {
        let element_size = size_of::<OffsetEntry>();
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        for key in 10..40u64 {
            c_map = (c.hm_put)(
                c_map,
                element_size,
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                0,
            );
            rust_map = (rust.hm_put)(
                rust_map,
                element_size,
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                0,
            );
            let c_index = (*map_header(c_map, element_size)).temp as usize;
            let rust_index = (*map_header(rust_map, element_size)).temp as usize;
            (*c_map.cast::<OffsetEntry>().add(c_index)).alias = key;
            (*c_map.cast::<OffsetEntry>().add(c_index)).value = -(key as i64);
            (*rust_map.cast::<OffsetEntry>().add(rust_index)).alias = key;
            (*rust_map.cast::<OffsetEntry>().add(rust_index)).value = -(key as i64);
        }
        for key in [10u64, 25, 39, 777] {
            c_map = (c.hm_del)(
                c_map,
                element_size,
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                size_of::<u64>(),
                0,
            );
            rust_map = (rust.hm_del)(
                rust_map,
                element_size,
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                size_of::<u64>(),
                0,
            );
            assert_eq!(
                (*map_header(c_map, element_size)).temp,
                (*map_header(rust_map, element_size)).temp
            );
            let c_count = (*map_header(c_map, element_size)).length - 1;
            let rust_count = (*map_header(rust_map, element_size)).length - 1;
            assert_eq!(c_count, rust_count);
            assert_eq!(
                std::slice::from_raw_parts(c_map.cast::<OffsetEntry>(), c_count),
                std::slice::from_raw_parts(rust_map.cast::<OffsetEntry>(), rust_count)
            );
        }
        (c.hm_free)(map_raw(c_map, element_size), element_size);
        (rust.hm_free)(map_raw(rust_map, element_size), element_size);
    });
}

unsafe fn string_insert(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    value: i64,
    mode: c_int,
) -> *mut c_void {
    let result = (api.hm_put)(
        map,
        size_of::<StringEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        mode,
    );
    let index = (*map_header(result, size_of::<StringEntry>())).temp;
    assert!(index >= 0);
    (*result.cast::<StringEntry>().add(index as usize)).value = value;
    result
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, i64)> {
    let count = (*map_header(map, size_of::<StringEntry>())).length - 1;
    (0..count)
        .map(|index| {
            let entry = *map.cast::<StringEntry>().add(index);
            (CStr::from_ptr(entry.key).to_bytes().to_vec(), entry.value)
        })
        .collect()
}

unsafe fn string_lookup(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    mode: c_int,
) -> (isize, i64) {
    let mut temp = 333;
    let result = (api.hm_get_ts)(
        map,
        size_of::<StringEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        &mut temp,
        mode,
    );
    let value = if temp < 0 {
        (*result.cast::<StringEntry>().sub(1)).value
    } else {
        (*result.cast::<StringEntry>().add(temp as usize)).value
    };
    (temp, value)
}

unsafe fn string_free(api: &Api, map: *mut c_void) {
    (api.hm_free)(
        map_raw(map, size_of::<StringEntry>()),
        size_of::<StringEntry>(),
    );
}

#[test]
fn config_rows_19_to_22_29_and_31_string_modes_match() {
    with_apis(|c, rust| unsafe {
        for table_mode in [1, 2, 3] {
            (c.rand_seed)(0x9e37_79b9);
            (rust.rand_seed)(0x9e37_79b9);
            let mut c_map = (c.sh_mode)(size_of::<StringEntry>(), table_mode);
            let mut rust_map = (rust.sh_mode)(size_of::<StringEntry>(), table_mode);
            let call_mode = if table_mode == 4 { 4 } else { 1 };
            let mut keys = Vec::new();
            for index in 0..48 {
                let text = match index {
                    0 => String::new(),
                    1 => "x".repeat(700),
                    _ => format!("key_{index:03}_{}", "q".repeat(index % 13)),
                };
                keys.push(CString::new(text).unwrap());
                let pointer = keys.last().unwrap().as_ptr().cast_mut();
                let value = (index as i64) * -31;
                c_map = string_insert(c, c_map, pointer, value, call_mode);
                rust_map = string_insert(rust, rust_map, pointer, value, call_mode);
                assert_eq!(string_snapshot(c_map), string_snapshot(rust_map));
                assert_eq!(
                    string_lookup(c, c_map, pointer, call_mode),
                    string_lookup(rust, rust_map, pointer, call_mode)
                );
            }

            let duplicate = CString::new("key_017_qqqq").unwrap();
            c_map = string_insert(c, c_map, duplicate.as_ptr().cast_mut(), 55_555, call_mode);
            rust_map = string_insert(
                rust,
                rust_map,
                duplicate.as_ptr().cast_mut(),
                55_555,
                call_mode,
            );
            assert_eq!(string_snapshot(c_map), string_snapshot(rust_map));

            let missing = CString::new("definitely_missing").unwrap();
            assert_eq!(
                string_lookup(c, c_map, missing.as_ptr().cast_mut(), call_mode),
                string_lookup(rust, rust_map, missing.as_ptr().cast_mut(), call_mode)
            );

            for index in [0usize, 17, 47, 9, 33] {
                let pointer = keys[index].as_ptr().cast_mut();
                c_map = (c.hm_del)(
                    c_map,
                    size_of::<StringEntry>(),
                    pointer.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    call_mode,
                );
                rust_map = (rust.hm_del)(
                    rust_map,
                    size_of::<StringEntry>(),
                    pointer.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    call_mode,
                );
                assert_eq!(string_snapshot(c_map), string_snapshot(rust_map));
                assert_eq!(
                    (*map_header(c_map, size_of::<StringEntry>())).temp,
                    (*map_header(rust_map, size_of::<StringEntry>())).temp
                );
            }
            string_free(c, c_map);
            string_free(rust, rust_map);
        }

        for mode in [-7, -1, 0] {
            let key = 0x8877_6655_4433_2211u64;
            let c_map = (c.hm_put)(
                null_mut(),
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                mode,
            );
            let rust_map = (rust.hm_put)(
                null_mut(),
                size_of::<Entry>(),
                (&key as *const u64).cast_mut().cast(),
                size_of::<u64>(),
                mode,
            );
            assert_eq!(
                std::slice::from_raw_parts(c_map.cast::<u8>(), size_of::<u64>()),
                std::slice::from_raw_parts(rust_map.cast::<u8>(), size_of::<u64>())
            );
            binary_free(c, c_map);
            binary_free(rust, rust_map);
        }

        for mode in [2, 4, c_int::MAX] {
            let key = CString::new(format!("out_of_range_{mode}")).unwrap();
            let c_map = string_insert(c, null_mut(), key.as_ptr().cast_mut(), 91, mode);
            let rust_map = string_insert(rust, null_mut(), key.as_ptr().cast_mut(), 91, mode);
            assert_eq!(string_snapshot(c_map), string_snapshot(rust_map));
            assert_eq!(
                string_lookup(c, c_map, key.as_ptr().cast_mut(), mode),
                string_lookup(rust, rust_map, key.as_ptr().cast_mut(), mode)
            );
            string_free(c, c_map);
            string_free(rust, rust_map);
        }
    });
}

#[test]
fn errors_assertion_sites_are_exercised_without_violation() {
    with_apis(|c, rust| unsafe {
        // Public operations below reach all externally reachable invariant checks.
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        for key in 0..80u64 {
            c_map = binary_insert(c, c_map, key, key as i64);
            rust_map = binary_insert(rust, rust_map, key, key as i64);
        }
        for key in [3u64, 17, 41, 79] {
            (c_map, _) = binary_delete(c, c_map, key);
            (rust_map, _) = binary_delete(rust, rust_map, key);
        }
        assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));
        binary_free(c, c_map);
        binary_free(rust, rust_map);

        let text = CString::new("assertion-path").unwrap();
        let mut c_arena: Arena = zeroed();
        let mut rust_arena: Arena = zeroed();
        assert_eq!(
            CStr::from_ptr((c.str_alloc)(&mut c_arena, text.as_ptr().cast_mut())).to_bytes(),
            CStr::from_ptr((rust.str_alloc)(&mut rust_arena, text.as_ptr().cast_mut())).to_bytes()
        );
        (c.str_reset)(&mut c_arena);
        (rust.str_reset)(&mut rust_arena);
        (c.arr_push)(101);
        (rust.arr_push)(101);
    });
}
