use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::ptr;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ArrIns = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_bytes: HashBytes,
    hash_string: HashString,
    hmfree: HmFree,
    hmget_ts: HmGetTs,
    hmget: HmGet,
    hmput_default: HmPutDefault,
    hmput: HmPut,
    hmdel: HmDel,
    shmode: ShMode,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    arr_ins: ArrIns,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(path).unwrap() };
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let symbol: libloading::Symbol<$ty> =
                    unsafe { library.get(concat!($name, "\0").as_bytes()).unwrap() };
                *symbol
            }};
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            shmode: symbol!("stbds_shmode_func", ShMode),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            arr_ins: symbol!("arr_ins", ArrIns),
            _library: library,
        }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        (
            Api::load(root.join("c_src/build/libtranslated_rust.so")),
            Api::load(root.join("target/release/libarr_ins_lib.so")),
        )
    }
}

fn one_library(kind: &str) -> Api {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = match kind {
        "c" => root.join("c_src/build/libtranslated_rust.so"),
        "rust" => root.join("target/release/libarr_ins_lib.so"),
        other => panic!("unknown library kind {other}"),
    };
    unsafe { Api::load(path) }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct HashIndexPrefix {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StringArena,
    storage: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct HashBucket {
    hash: [usize; 8],
    index: [isize; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BinaryEntry {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct MapMeta {
    length: usize,
    capacity: usize,
    temp: isize,
    slots: Option<usize>,
    used: Option<usize>,
    tombstones: Option<usize>,
    seed: Option<usize>,
    string_remaining: Option<usize>,
    string_block: Option<u8>,
    string_mode: Option<u8>,
}

unsafe fn array_header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe {
        array
            .cast::<u8>()
            .sub(size_of::<ArrayHeader>())
            .cast::<ArrayHeader>()
    }
}

unsafe fn map_header(entries: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe { array_header(entries.cast::<u8>().sub(element_size).cast()) }
}

unsafe fn map_meta(entries: *mut c_void, element_size: usize) -> MapMeta {
    let header = unsafe { &*map_header(entries, element_size) };
    let table = header.hash_table.cast::<HashIndexPrefix>();
    let hash = unsafe { table.as_ref() };
    MapMeta {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        slots: hash.map(|h| h.slot_count),
        used: hash.map(|h| h.used_count),
        tombstones: hash.map(|h| h.tombstone_count),
        seed: hash.map(|h| h.seed),
        string_remaining: hash.map(|h| h.string.remaining),
        string_block: hash.map(|h| h.string.block),
        string_mode: hash.map(|h| h.string.mode),
    }
}

unsafe fn binary_entries(entries: *mut c_void) -> Vec<BinaryEntry> {
    let count = unsafe { (*map_header(entries, size_of::<BinaryEntry>())).length - 1 };
    unsafe { std::slice::from_raw_parts(entries.cast::<BinaryEntry>(), count).to_vec() }
}

unsafe fn string_entries(entries: *mut c_void) -> Vec<(Vec<u8>, i64)> {
    let count = unsafe { (*map_header(entries, size_of::<StringEntry>())).length - 1 };
    unsafe { std::slice::from_raw_parts(entries.cast::<StringEntry>(), count) }
        .iter()
        .map(|entry| {
            (
                unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
                entry.value,
            )
        })
        .collect()
}

unsafe fn put_binary(api: &Api, entries: &mut *mut c_void, key: u64, value: i64, mode: c_int) {
    let mut key = key;
    *entries = unsafe {
        (api.hmput)(
            *entries,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            mode,
        )
    };
    let index = unsafe { (*map_header(*entries, size_of::<BinaryEntry>())).temp as usize };
    unsafe { (*entries.cast::<BinaryEntry>().add(index)).value = value };
}

unsafe fn put_string(
    api: &Api,
    entries: &mut *mut c_void,
    key: &mut [u8],
    value: i64,
    mode: c_int,
) {
    *entries = unsafe {
        (api.hmput)(
            *entries,
            size_of::<StringEntry>(),
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { (*map_header(*entries, size_of::<StringEntry>())).temp as usize };
    unsafe { (*entries.cast::<StringEntry>().add(index)).value = value };
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

unsafe fn binary_hash(api: &Api, key: &mut u64, seed: usize) -> usize {
    let hash = unsafe { (api.hash_bytes)(ptr::addr_of_mut!(*key).cast(), size_of::<u64>(), seed) };
    if hash < 2 { hash + 2 } else { hash }
}

fn run_boundary_child(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env("STBDS_CHILD_LIBRARY", library)
        .env("STBDS_CHILD_CASE", case)
        .status()
        .unwrap()
}

fn assert_same_failure(case: &str) {
    let c = run_boundary_child("c", case);
    let rust = run_boundary_child("rust", case);
    assert!(!c.success(), "C unexpectedly accepted boundary case {case}");
    assert!(
        !rust.success(),
        "Rust unexpectedly accepted boundary case {case}"
    );
    #[cfg(unix)]
    assert_eq!(
        (c.code(), c.signal()),
        (rust.code(), rust.signal()),
        "different process result for boundary case {case}: C={c:?}, Rust={rust:?}"
    );
    #[cfg(not(unix))]
    assert_eq!(c.code(), rust.code());
}

#[test]
fn ffi_boundary_child() {
    let Ok(kind) = std::env::var("STBDS_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("STBDS_CHILD_CASE").unwrap();
    let api = one_library(&kind);
    unsafe {
        match case.as_str() {
            "arrfree_null" => (api.arrfree)(ptr::null_mut()),
            "hash_bytes_null" => {
                (api.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hash_bytes_null_oversized" => {
                (api.hash_bytes)(ptr::null_mut(), usize::MAX, 0);
            }
            "hash_string_null" => {
                (api.hash_string)(ptr::null_mut(), 0);
            }
            "hmget_ts_null_temp" => {
                (api.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    0,
                );
            }
            "hmget_null_key" => {
                let mut map = ptr::null_mut();
                put_binary(&api, &mut map, 1, 2, 0);
                (api.hmget)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    0,
                );
            }
            "hmput_null_string" => {
                (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    1,
                );
            }
            "stralloc_null_arena" => {
                let mut string = b"x\0".to_vec();
                (api.stralloc)(ptr::null_mut(), string.as_mut_ptr().cast());
            }
            "stralloc_null_string" => {
                let mut arena: StringArena = zeroed();
                (api.stralloc)(&mut arena, ptr::null_mut());
            }
            "strreset_null" => (api.strreset)(ptr::null_mut()),
            "assert_hash_threshold" => {
                let mut map = ptr::null_mut();
                put_binary(&api, &mut map, 1, 1, 0);
                let table = (*map_header(map, size_of::<BinaryEntry>()))
                    .hash_table
                    .cast::<HashIndexPrefix>();
                (*table).slot_count = 1;
                (*table).used_count = (*table).used_count_threshold;
                put_binary(&api, &mut map, 2, 2, 0);
            }
            "assert_moved_key_missing" => {
                let mut map = ptr::null_mut();
                put_binary(&api, &mut map, 10, 10, 0);
                put_binary(&api, &mut map, 20, 20, 0);
                (*map.cast::<BinaryEntry>().add(1)).key = 999;
                let mut key = 10u64;
                (api.hmdel)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                );
            }
            "assert_moved_index_mismatch" => {
                let mut map = ptr::null_mut();
                for key in [10u64, 20, 30] {
                    put_binary(&api, &mut map, key, key as i64, 0);
                }
                (*map.cast::<BinaryEntry>().add(1)).key = 30;
                let table = (*map_header(map, size_of::<BinaryEntry>()))
                    .hash_table
                    .cast::<HashIndexPrefix>();
                let hash = binary_hash(&api, &mut 30, (*table).seed);
                let buckets = (*table).storage.cast::<HashBucket>();
                let mut position = hash & ((*table).slot_count - 1);
                let mut step = 8;
                'found: loop {
                    let bucket = buckets.add(position >> 3);
                    for slot in (position & 7)..8 {
                        if (*bucket).hash[slot] == hash && (*bucket).index[slot] == 2 {
                            (*bucket).index[slot] = 1;
                            break 'found;
                        }
                    }
                    for slot in 0..(position & 7) {
                        if (*bucket).hash[slot] == hash && (*bucket).index[slot] == 2 {
                            (*bucket).index[slot] = 1;
                            break 'found;
                        }
                    }
                    position = (position + step) & ((*table).slot_count - 1);
                    step += 8;
                }
                let mut key = 10u64;
                (api.hmdel)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                );
            }
            other => panic!("unknown child case {other}"),
        }
    }
}

#[test]
fn defined_zero_null_and_oversized_boundaries_match() {
    let (c, rust) = libraries();
    unsafe {
        let c_array = (c.arrgrow)(ptr::null_mut(), 0, 0, usize::MAX);
        let r_array = (rust.arrgrow)(ptr::null_mut(), 0, 0, usize::MAX);
        assert_eq!((*array_header(c_array)).length, 0);
        assert_eq!((*array_header(r_array)).length, 0);
        assert_eq!((*array_header(c_array)).capacity, usize::MAX);
        assert_eq!((*array_header(r_array)).capacity, usize::MAX);
        (c.arrfree)(c_array);
        (rust.arrfree)(r_array);

        let mut ct = 7;
        let mut rt = 7;
        let cm = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::null_mut(),
            usize::MAX,
            &mut ct,
            c_int::MAX,
        );
        let rm = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::null_mut(),
            usize::MAX,
            &mut rt,
            c_int::MAX,
        );
        assert_eq!((ct, rt), (-1, -1));
        assert_eq!(
            map_meta(cm, size_of::<BinaryEntry>()),
            map_meta(rm, size_of::<BinaryEntry>())
        );
        (c.hmfree)(
            cm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
            size_of::<BinaryEntry>(),
        );
        (rust.hmfree)(
            rm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
            size_of::<BinaryEntry>(),
        );
    }
}

#[test]
fn invalid_pointer_boundaries_fail_identically() {
    for case in [
        "arrfree_null",
        "hash_bytes_null",
        "hash_bytes_null_oversized",
        "hash_string_null",
        "hmget_ts_null_temp",
        "hmget_null_key",
        "hmput_null_string",
        "stralloc_null_arena",
        "stralloc_null_string",
        "strreset_null",
    ] {
        assert_same_failure(case);
    }
}

#[test]
fn reachable_assertion_guards_abort_identically() {
    for case in [
        "assert_hash_threshold",
        "assert_moved_key_missing",
        "assert_moved_index_mismatch",
    ] {
        assert_same_failure(case);
    }
}

#[test]
fn hash_functions_match_all_length_and_value_branches() {
    let (c, rust) = libraries();
    let mut state = 0x6a09_e667_f3bc_c909;
    unsafe {
        assert_eq!(
            (c.hash_bytes)(ptr::null_mut(), 0, 0),
            (rust.hash_bytes)(ptr::null_mut(), 0, 0)
        );
        for length in 0..=135usize {
            for _ in 0..64 {
                let seed = next_random(&mut state) as usize;
                let mut data = vec![0u8; length];
                for byte in &mut data {
                    *byte = next_random(&mut state) as u8;
                }
                assert_eq!(
                    (c.hash_bytes)(data.as_mut_ptr().cast(), length, seed),
                    (rust.hash_bytes)(data.as_mut_ptr().cast(), length, seed),
                    "byte hash mismatch for length {length}, seed {seed:#x}, data {data:02x?}"
                );
            }
        }

        for length in 0..=96usize {
            for _ in 0..64 {
                let seed = next_random(&mut state) as usize;
                let mut string = Vec::with_capacity(length + 1);
                for _ in 0..length {
                    let mut byte = next_random(&mut state) as u8;
                    if byte == 0 {
                        byte = 0x80;
                    }
                    string.push(byte);
                }
                string.push(0);
                assert_eq!(
                    (c.hash_string)(string.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(string.as_mut_ptr().cast(), seed),
                    "string hash mismatch for seed {seed:#x}, string {string:02x?}"
                );
            }
        }
    }
}

#[test]
fn array_growth_and_metadata_match() {
    let (c, rust) = libraries();
    unsafe {
        assert!((c.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        assert!((rust.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());

        for request in [1usize, 2, 3, 4, 5, 17, 257] {
            let c_array = (c.arrgrow)(ptr::null_mut(), 4, 0, request);
            let r_array = (rust.arrgrow)(ptr::null_mut(), 4, 0, request);
            let c_header = *array_header(c_array);
            let r_header = *array_header(r_array);
            assert_eq!(c_header.length, r_header.length);
            assert_eq!(c_header.capacity, r_header.capacity);
            assert_eq!(c_header.temp, r_header.temp);
            assert_eq!(c_header.hash_table.is_null(), r_header.hash_table.is_null());
            (c.arrfree)(c_array);
            (rust.arrfree)(r_array);
        }

        let mut c_array = (c.arrgrow)(ptr::null_mut(), 1, 0, 5);
        let mut r_array = (rust.arrgrow)(ptr::null_mut(), 1, 0, 5);
        (*array_header(c_array)).length = 5;
        (*array_header(r_array)).length = 5;
        for index in 0..5 {
            *c_array.cast::<u8>().add(index) = index as u8 + 10;
            *r_array.cast::<u8>().add(index) = index as u8 + 10;
        }
        let c_same = (c.arrgrow)(c_array, 1, 0, 5);
        let r_same = (rust.arrgrow)(r_array, 1, 0, 5);
        assert_eq!(c_same, c_array);
        assert_eq!(r_same, r_array);

        c_array = (c.arrgrow)(c_same, 1, 1, 0);
        r_array = (rust.arrgrow)(r_same, 1, 1, 0);
        assert_eq!((*array_header(c_array)).capacity, 10);
        assert_eq!((*array_header(r_array)).capacity, 10);
        assert_eq!(
            std::slice::from_raw_parts(c_array.cast::<u8>(), 5),
            std::slice::from_raw_parts(r_array.cast::<u8>(), 5)
        );

        c_array = (c.arrgrow)(c_array, 1, 0, 40);
        r_array = (rust.arrgrow)(r_array, 1, 0, 40);
        assert_eq!((*array_header(c_array)).capacity, 40);
        assert_eq!((*array_header(r_array)).capacity, 40);
        assert_eq!(
            std::slice::from_raw_parts(c_array.cast::<u8>(), 5),
            std::slice::from_raw_parts(r_array.cast::<u8>(), 5)
        );
        (c.arrfree)(c_array);
        (rust.arrfree)(r_array);
    }
}

#[test]
fn binary_map_randomized_operations_match() {
    let (c, rust) = libraries();
    let mut state = 0xbb67_ae85_84ca_a73b;
    unsafe {
        for seed in [0usize, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            for mode in [-17, 0] {
                let mut cm = ptr::null_mut();
                let mut rm = ptr::null_mut();

                cm = (c.hmput_default)(cm, size_of::<BinaryEntry>());
                rm = (rust.hmput_default)(rm, size_of::<BinaryEntry>());
                assert_eq!(
                    map_meta(cm, size_of::<BinaryEntry>()),
                    map_meta(rm, size_of::<BinaryEntry>())
                );
                assert_eq!(binary_entries(cm), binary_entries(rm));
                let c_before = cm;
                let r_before = rm;
                cm = (c.hmput_default)(cm, size_of::<BinaryEntry>());
                rm = (rust.hmput_default)(rm, size_of::<BinaryEntry>());
                assert_eq!(cm, c_before);
                assert_eq!(rm, r_before);

                for step in 0..400 {
                    let key = next_random(&mut state) % 73;
                    match next_random(&mut state) % 4 {
                        0 | 1 => {
                            let value = next_random(&mut state) as i64;
                            put_binary(&c, &mut cm, key, value, mode);
                            put_binary(&rust, &mut rm, key, value, mode);
                        }
                        2 => {
                            cm = (c.hmdel)(
                                cm,
                                size_of::<BinaryEntry>(),
                                ptr::addr_of!(key).cast_mut().cast(),
                                size_of::<u64>(),
                                0,
                                mode,
                            );
                            rm = (rust.hmdel)(
                                rm,
                                size_of::<BinaryEntry>(),
                                ptr::addr_of!(key).cast_mut().cast(),
                                size_of::<u64>(),
                                0,
                                mode,
                            );
                        }
                        _ => {
                            let mut ct = 999;
                            let mut rt = 999;
                            cm = (c.hmget_ts)(
                                cm,
                                size_of::<BinaryEntry>(),
                                ptr::addr_of!(key).cast_mut().cast(),
                                size_of::<u64>(),
                                &mut ct,
                                mode,
                            );
                            rm = (rust.hmget_ts)(
                                rm,
                                size_of::<BinaryEntry>(),
                                ptr::addr_of!(key).cast_mut().cast(),
                                size_of::<u64>(),
                                &mut rt,
                                mode,
                            );
                            assert_eq!(ct, rt);
                        }
                    }
                    assert_eq!(
                        map_meta(cm, size_of::<BinaryEntry>()),
                        map_meta(rm, size_of::<BinaryEntry>()),
                        "metadata mismatch at seed {seed:#x}, mode {mode}, step {step}"
                    );
                    assert_eq!(
                        binary_entries(cm),
                        binary_entries(rm),
                        "entries mismatch at seed {seed:#x}, mode {mode}, step {step}"
                    );
                }
                (c.hmfree)(
                    cm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
                    size_of::<BinaryEntry>(),
                );
                (rust.hmfree)(
                    rm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
                    size_of::<BinaryEntry>(),
                );
            }
        }
    }
}

#[test]
fn hash_table_maintenance_transitions_match() {
    let (c, rust) = libraries();
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (rust.rand_seed)(0x3141_5926);
        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();

        for key in 0..6u64 {
            put_binary(&c, &mut cm, key, key as i64, 0);
            put_binary(&rust, &mut rm, key, key as i64, 0);
            assert_eq!(map_meta(cm, 16).slots, Some(8));
            assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        }
        put_binary(&c, &mut cm, 6, 6, 0);
        put_binary(&rust, &mut rm, 6, 6, 0);
        assert_eq!(map_meta(cm, 16).slots, Some(16));
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));

        for key in [0u64, 1, 2, 3] {
            cm = (c.hmdel)(cm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 0, 0);
            rm = (rust.hmdel)(rm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 0, 0);
            assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        }
        assert_eq!(map_meta(cm, 16).tombstones, Some(0));
        let key = 4u64;
        cm = (c.hmdel)(cm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 0, 0);
        rm = (rust.hmdel)(rm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 0, 0);
        assert_eq!(map_meta(cm, 16).slots, Some(8));
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));

        (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
        (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);

        for delete_final in [false, true] {
            let mut cm = ptr::null_mut();
            let mut rm = ptr::null_mut();
            for key in [101u64, 202, 303] {
                put_binary(&c, &mut cm, key, key as i64, 0);
                put_binary(&rust, &mut rm, key, key as i64, 0);
            }
            let mut key = if delete_final { 303 } else { 101 };
            cm = (c.hmdel)(cm, 16, ptr::addr_of_mut!(key).cast(), 8, 0, 0);
            rm = (rust.hmdel)(rm, 16, ptr::addr_of_mut!(key).cast(), 8, 0, 0);
            assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
            assert_eq!(binary_entries(cm), binary_entries(rm));
            (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
            (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
        }

        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        put_binary(&c, &mut cm, 10, 10, 0);
        put_binary(&rust, &mut rm, 10, 10, 0);
        let c_seed = map_meta(cm, 16).seed.unwrap();
        let r_seed = map_meta(rm, 16).seed.unwrap();
        assert_eq!(c_seed, r_seed);
        let mut deleted = 10u64;
        let bucket = binary_hash(&c, &mut deleted, c_seed) & 7;
        let mut replacement = 11u64;
        while binary_hash(&c, &mut replacement, c_seed) & 7 != bucket {
            replacement += 1;
        }
        cm = (c.hmdel)(cm, 16, ptr::addr_of_mut!(deleted).cast(), 8, 0, 0);
        rm = (rust.hmdel)(rm, 16, ptr::addr_of_mut!(deleted).cast(), 8, 0, 0);
        assert_eq!(map_meta(cm, 16).tombstones, Some(1));
        put_binary(&c, &mut cm, replacement, 99, 0);
        put_binary(&rust, &mut rm, replacement, 99, 0);
        assert_eq!(map_meta(cm, 16).tombstones, Some(0));
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        assert_eq!(binary_entries(cm), binary_entries(rm));
        (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
        (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
    }
}

#[test]
fn wrapped_probe_missing_key_sentinel_matches() {
    let (c, rust) = libraries();
    unsafe {
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        for key in 0..5u64 {
            put_binary(&c, &mut cm, key, key as i64, 0);
            put_binary(&rust, &mut rm, key, key as i64, 0);
        }

        let c_table = (*map_header(cm, 16)).hash_table.cast::<HashIndexPrefix>();
        let r_table = (*map_header(rm, 16)).hash_table.cast::<HashIndexPrefix>();
        let c_bucket = (*c_table).storage.cast::<HashBucket>();
        let r_bucket = (*r_table).storage.cast::<HashBucket>();
        assert_eq!((*c_bucket).hash, (*r_bucket).hash);
        let seed = (*c_table).seed;

        let mut missing = 100u64;
        loop {
            let position = binary_hash(&c, &mut missing, seed) & 7;
            let suffix_full = (position..8).all(|slot| (*c_bucket).hash[slot] != 0);
            let prefix_empty = (0..position).any(|slot| (*c_bucket).hash[slot] == 0);
            let is_present = binary_entries(cm).iter().any(|entry| entry.key == missing);
            if suffix_full && prefix_empty && !is_present {
                break;
            }
            missing += 1;
            assert!(missing < 1_000_000);
        }

        let mut ct = 88;
        let mut rt = 88;
        cm = (c.hmget_ts)(cm, 16, ptr::addr_of_mut!(missing).cast(), 8, &mut ct, 0);
        rm = (rust.hmget_ts)(rm, 16, ptr::addr_of_mut!(missing).cast(), 8, &mut rt, 0);
        assert_eq!((ct, rt), (-1, -1));
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
        (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
    }
}

#[test]
fn map_lookup_rejection_sentinels_match() {
    let (c, rust) = libraries();
    unsafe {
        let key = 44u64;
        let mut ct = 123;
        let mut rt = 123;
        let mut cm = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            &mut ct,
            0,
        );
        let mut rm = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            &mut rt,
            0,
        );
        assert_eq!((ct, rt), (-1, -1));
        assert_eq!(
            map_meta(cm, size_of::<BinaryEntry>()),
            map_meta(rm, size_of::<BinaryEntry>())
        );

        cm = (c.hmget)(
            cm,
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            0,
        );
        rm = (rust.hmget)(
            rm,
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            0,
        );
        assert_eq!(map_meta(cm, size_of::<BinaryEntry>()).temp, -1);
        assert_eq!(map_meta(rm, size_of::<BinaryEntry>()).temp, -1);

        let c_before = cm;
        let r_before = rm;
        cm = (c.hmdel)(
            cm,
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            0,
            0,
        );
        rm = (rust.hmdel)(
            rm,
            size_of::<BinaryEntry>(),
            ptr::addr_of!(key).cast_mut().cast(),
            size_of::<u64>(),
            0,
            0,
        );
        assert_eq!(cm, c_before);
        assert_eq!(rm, r_before);
        assert_eq!(map_meta(cm, size_of::<BinaryEntry>()).temp, 0);
        assert_eq!(map_meta(rm, size_of::<BinaryEntry>()).temp, 0);
        assert!((c.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 8, 0, 0).is_null());
        assert!((rust.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 8, 0, 0).is_null());
        (c.hmfree)(ptr::null_mut(), 16);
        (rust.hmfree)(ptr::null_mut(), 16);
        (c.hmfree)(
            cm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
            size_of::<BinaryEntry>(),
        );
        (rust.hmfree)(
            rm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
            size_of::<BinaryEntry>(),
        );
    }
}

#[test]
fn binary_key_sizes_and_nonzero_offsets_match() {
    let (c, rust) = libraries();
    unsafe {
        for key_size in [1usize, 3, 4, 7, 8] {
            let mut cm = ptr::null_mut();
            let mut rm = ptr::null_mut();
            let mut key = 0x8070_6050_4030_2010u64.to_ne_bytes();
            cm = (c.hmput)(cm, 24, key.as_mut_ptr().cast(), key_size, 0);
            rm = (rust.hmput)(rm, 24, key.as_mut_ptr().cast(), key_size, 0);
            assert_eq!(map_meta(cm, 24), map_meta(rm, 24));
            assert_eq!(
                std::slice::from_raw_parts(cm.cast::<u8>(), key_size),
                std::slice::from_raw_parts(rm.cast::<u8>(), key_size)
            );
            let mut ct = -9;
            let mut rt = -9;
            cm = (c.hmget_ts)(cm, 24, key.as_mut_ptr().cast(), key_size, &mut ct, 0);
            rm = (rust.hmget_ts)(rm, 24, key.as_mut_ptr().cast(), key_size, &mut rt, 0);
            assert_eq!((ct, rt), (0, 0));
            cm = (c.hmget)(cm, 24, key.as_mut_ptr().cast(), key_size, 0);
            rm = (rust.hmget)(rm, 24, key.as_mut_ptr().cast(), key_size, 0);
            assert_eq!(map_meta(cm, 24).temp, 0);
            assert_eq!(map_meta(rm, 24).temp, 0);
            key[0] ^= 0xff;
            ct = 4;
            rt = 4;
            cm = (c.hmget_ts)(cm, 24, key.as_mut_ptr().cast(), key_size, &mut ct, 0);
            rm = (rust.hmget_ts)(rm, 24, key.as_mut_ptr().cast(), key_size, &mut rt, 0);
            assert_eq!((ct, rt), (-1, -1));
            (c.hmfree)(cm.cast::<u8>().sub(24).cast(), 24);
            (rust.hmfree)(rm.cast::<u8>().sub(24).cast(), 24);
        }

        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        for key in [11u64, 22, 33] {
            put_binary(&c, &mut cm, key, key as i64, 0);
            put_binary(&rust, &mut rm, key, key as i64, 0);
            *cm.cast::<BinaryEntry>().add(map_meta(cm, 16).temp as usize) = BinaryEntry {
                key,
                value: key as i64,
            };
            *rm.cast::<BinaryEntry>().add(map_meta(rm, 16).temp as usize) = BinaryEntry {
                key,
                value: key as i64,
            };
        }
        let key = 22u64;
        cm = (c.hmdel)(cm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 8, 0);
        rm = (rust.hmdel)(rm, 16, ptr::addr_of!(key).cast_mut().cast(), 8, 8, 0);
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        assert_eq!(binary_entries(cm), binary_entries(rm));
        (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
        (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
    }
}

#[test]
fn string_map_modes_and_deletions_match() {
    let (c, rust) = libraries();
    unsafe {
        for storage_mode in [1, 2, 3] {
            for seed in [0usize, usize::MAX] {
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
                let mut cm = (c.shmode)(size_of::<StringEntry>(), storage_mode);
                let mut rm = (rust.shmode)(size_of::<StringEntry>(), storage_mode);
                let mut c_keys = Vec::new();
                let mut r_keys = Vec::new();
                for key in [String::new(), "L".repeat(700)] {
                    let mut ck = key.into_bytes();
                    let mut rk = ck.clone();
                    ck.push(0);
                    rk.push(0);
                    put_string(&c, &mut cm, &mut ck, -1, 1);
                    put_string(&rust, &mut rm, &mut rk, -1, 1);
                    c_keys.push(ck);
                    r_keys.push(rk);
                }
                for index in 0..80 {
                    let key = format!("key_{index:03}_{}", "x".repeat(index % 17));
                    let mut ck = key.as_bytes().to_vec();
                    let mut rk = ck.clone();
                    ck.push(0);
                    rk.push(0);
                    put_string(&c, &mut cm, &mut ck, (index * 17) as i64, 1);
                    put_string(&rust, &mut rm, &mut rk, (index * 17) as i64, 1);
                    c_keys.push(ck);
                    r_keys.push(rk);
                    assert_eq!(string_entries(cm), string_entries(rm));
                    assert_eq!(
                        map_meta(cm, size_of::<StringEntry>()),
                        map_meta(rm, size_of::<StringEntry>())
                    );
                }
                if storage_mode == 2 {
                    c_keys[7][0] = b'X';
                    r_keys[7][0] = b'X';
                    assert_eq!(string_entries(cm), string_entries(rm));
                    assert_eq!(string_entries(cm)[7].0[0], b'k');
                }
                let mut final_key = format!("key_{:03}_{}", 79, "x".repeat(79 % 17)).into_bytes();
                final_key.push(0);
                cm = (c.hmdel)(
                    cm,
                    size_of::<StringEntry>(),
                    final_key.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                rm = (rust.hmdel)(
                    rm,
                    size_of::<StringEntry>(),
                    final_key.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                assert_eq!(string_entries(cm), string_entries(rm));
                for index in (0..80).step_by(3) {
                    let mut key = format!("key_{index:03}_{}", "x".repeat(index % 17)).into_bytes();
                    key.push(0);
                    cm = (c.hmdel)(
                        cm,
                        size_of::<StringEntry>(),
                        key.as_mut_ptr().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        1,
                    );
                    rm = (rust.hmdel)(
                        rm,
                        size_of::<StringEntry>(),
                        key.as_mut_ptr().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        1,
                    );
                    assert_eq!(string_entries(cm), string_entries(rm));
                    assert_eq!(
                        map_meta(cm, size_of::<StringEntry>()),
                        map_meta(rm, size_of::<StringEntry>())
                    );
                }
                (c.hmfree)(
                    cm.cast::<u8>().sub(size_of::<StringEntry>()).cast(),
                    size_of::<StringEntry>(),
                );
                (rust.hmfree)(
                    rm.cast::<u8>().sub(size_of::<StringEntry>()).cast(),
                    size_of::<StringEntry>(),
                );
            }
        }
    }
}

#[test]
fn direct_string_mode_and_noncanonical_delete_modes_match() {
    let (c, rust) = libraries();
    unsafe {
        for put_mode in [1, 7] {
            let mut cm = ptr::null_mut();
            let mut rm = ptr::null_mut();
            let mut c_keys = Vec::new();
            let mut r_keys = Vec::new();
            for index in 0..24 {
                let mut ck = format!("direct_{put_mode}_{index}").into_bytes();
                ck.push(0);
                let mut rk = ck.clone();
                put_string(&c, &mut cm, &mut ck, index, put_mode);
                put_string(&rust, &mut rm, &mut rk, index, put_mode);
                c_keys.push(ck);
                r_keys.push(rk);
            }
            assert_eq!(string_entries(cm), string_entries(rm));
            assert_eq!(
                map_meta(cm, size_of::<StringEntry>()),
                map_meta(rm, size_of::<StringEntry>())
            );

            let mut absent = b"not_present\0".to_vec();
            for delete_mode in [1, 2, 99] {
                cm = (c.hmdel)(
                    cm,
                    size_of::<StringEntry>(),
                    absent.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    delete_mode,
                );
                rm = (rust.hmdel)(
                    rm,
                    size_of::<StringEntry>(),
                    absent.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    delete_mode,
                );
                assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
            }
            let mut present = format!("direct_{put_mode}_23").into_bytes();
            present.push(0);
            cm = (c.hmdel)(
                cm,
                size_of::<StringEntry>(),
                present.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                2,
            );
            rm = (rust.hmdel)(
                rm,
                size_of::<StringEntry>(),
                present.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                2,
            );
            assert_eq!(string_entries(cm), string_entries(rm));
            assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
            (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
            (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
        }

        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        for key in [5u64, 9, 13] {
            put_binary(&c, &mut cm, key, key as i64, -9);
            put_binary(&rust, &mut rm, key, key as i64, -9);
        }
        let mut key = 9u64;
        cm = (c.hmdel)(cm, 16, ptr::addr_of_mut!(key).cast(), 8, 0, -9);
        rm = (rust.hmdel)(rm, 16, ptr::addr_of_mut!(key).cast(), 8, 0, -9);
        assert_eq!(map_meta(cm, 16), map_meta(rm, 16));
        assert_eq!(binary_entries(cm), binary_entries(rm));
        (c.hmfree)(cm.cast::<u8>().sub(16).cast(), 16);
        (rust.hmfree)(rm.cast::<u8>().sub(16).cast(), 16);
    }
}

#[test]
fn out_of_range_modes_match_c_integer_semantics() {
    let (c, rust) = libraries();
    unsafe {
        for mode in [-1, 0, 4, 255, 256, 257] {
            let mut cm = (c.shmode)(size_of::<BinaryEntry>(), mode);
            let mut rm = (rust.shmode)(size_of::<BinaryEntry>(), mode);
            assert_eq!(
                map_meta(cm, size_of::<BinaryEntry>()),
                map_meta(rm, size_of::<BinaryEntry>())
            );
            let mut key = 0x0102_0304_0506_0708u64;
            cm = (c.hmput)(
                cm,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                0,
            );
            rm = (rust.hmput)(
                rm,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                0,
            );
            let c_index = (*map_header(cm, size_of::<BinaryEntry>())).temp as usize;
            let r_index = (*map_header(rm, size_of::<BinaryEntry>())).temp as usize;
            (*cm.cast::<BinaryEntry>().add(c_index)).value = 0x1234_5678;
            (*rm.cast::<BinaryEntry>().add(r_index)).value = 0x1234_5678;
            assert_eq!(
                map_meta(cm, size_of::<BinaryEntry>()),
                map_meta(rm, size_of::<BinaryEntry>())
            );
            assert_eq!(binary_entries(cm), binary_entries(rm));
            (c.hmfree)(
                cm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
                size_of::<BinaryEntry>(),
            );
            (rust.hmfree)(
                rm.cast::<u8>().sub(size_of::<BinaryEntry>()).cast(),
                size_of::<BinaryEntry>(),
            );
        }
    }
}

#[test]
fn string_arena_allocation_classes_and_reset_match() {
    let (c, rust) = libraries();
    unsafe {
        let mut ca: StringArena = zeroed();
        let mut ra: StringArena = zeroed();
        let lengths = [
            0usize, 1, 7, 127, 510, 511, 512, 513, 900, 2048, 17, 600_000,
        ];
        for (index, length) in lengths.into_iter().enumerate() {
            let mut cs = vec![b'a' + index as u8 % 23; length];
            let mut rs = cs.clone();
            cs.push(0);
            rs.push(0);
            let cp = (c.stralloc)(&mut ca, cs.as_mut_ptr().cast());
            let rp = (rust.stralloc)(&mut ra, rs.as_mut_ptr().cast());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(ca.remaining, ra.remaining);
            assert_eq!(ca.block, ra.block);
            assert_eq!(ca.mode, ra.mode);
            assert_eq!(ca.storage.is_null(), ra.storage.is_null());
        }
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
        assert_eq!(
            (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
            (true, 0, 0, 0)
        );
        assert_eq!(
            (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
            (true, 0, 0, 0)
        );
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
    }
}

#[test]
fn string_arena_growth_cap_and_fresh_large_block_match() {
    let (c, rust) = libraries();
    unsafe {
        let mut ca: StringArena = zeroed();
        let mut ra: StringArena = zeroed();
        for _ in 0..26 {
            let block_size = 512usize << ((ca.block >> 1) as usize);
            let content_length = block_size + 1;
            let mut cs = vec![b'q'; content_length];
            let mut rs = cs.clone();
            cs.push(0);
            rs.push(0);
            let cp = (c.stralloc)(&mut ca, cs.as_mut_ptr().cast());
            let rp = (rust.stralloc)(&mut ra, rs.as_mut_ptr().cast());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        }
        assert_eq!(ca.block, 22);
        assert_eq!(ra.block, 22);
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);

        let mut ca: StringArena = zeroed();
        let mut ra: StringArena = zeroed();
        let mut cs = vec![b'z'; 513];
        let mut rs = cs.clone();
        cs.push(0);
        rs.push(0);
        let cp = (c.stralloc)(&mut ca, cs.as_mut_ptr().cast());
        let rp = (rust.stralloc)(&mut ra, rs.as_mut_ptr().cast());
        assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
    }
}

#[test]
fn strkey_and_arr_ins_exports_match() {
    let (c, rust) = libraries();
    unsafe {
        for value in [c_int::MIN, -1_000_000, -1, 0, 1, 1_000_000, c_int::MAX] {
            let c_value = CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec();
            let rust_value = CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec();
            assert_eq!(c_value, rust_value);
        }
        let mut state = 0x3c6e_f372_fe94_f82b;
        for value in [c_int::MIN, 0, c_int::MAX] {
            (c.arr_ins)(value);
            (rust.arr_ins)(value);
        }
        for _ in 0..256 {
            let value = next_random(&mut state) as c_int;
            (c.arr_ins)(value);
            (rust.arr_ins)(value);
        }
    }
}
