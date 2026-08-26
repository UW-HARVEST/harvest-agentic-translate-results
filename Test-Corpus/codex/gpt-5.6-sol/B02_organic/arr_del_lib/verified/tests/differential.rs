use libloading::Library;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::{self, null_mut};

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ArrDel = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_string: HashString,
    hash_bytes: HashBytes,
    hmget_ts: HmGetTs,
    hmget: HmGet,
    hmput_default: HmPutDefault,
    hmput: HmPut,
    shmode: ShMode,
    hmdel: HmDel,
    hmfree: HmFree,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    arr_del: ArrDel,
}

impl Api {
    unsafe fn open(path: PathBuf) -> Self {
        let library = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let value = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *value
            }};
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_string: symbol!("stbds_hash_string", HashString),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            shmode: symbol!("stbds_shmode_func", ShMode),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            arr_del: symbol!("arr_del", ArrDel),
            _library: library,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pair {
    key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringPair {
    key: *mut c_char,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HashBucket {
    hash: [usize; 8],
    index: [isize; 8],
}

#[repr(C)]
struct HashIndex {
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
    storage: *mut HashBucket,
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, element_size)) }
}

unsafe fn map_len(map: *mut c_void, element_size: usize) -> usize {
    unsafe { (*map_header(map, element_size)).length - 1 }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let profile_library = root.join(format!("target/{profile}/libarr_del_lib.so"));
    let rust_library = if profile_library.is_file() {
        profile_library
    } else {
        root.join("target/release/libarr_del_lib.so")
    };
    (root.join("c_src/build/libtranslated_rust.so"), rust_library)
}

unsafe fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "build the C shared library first");
    assert!(
        rust_path.is_file(),
        "Rust cdylib missing at {}",
        rust_path.display()
    );
    unsafe { (Api::open(c_path), Api::open(rust_path)) }
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_map(map, element_size), element_size) };
    }
}

unsafe fn set_pair_value(map: *mut c_void, value: u64) {
    let index = unsafe { (*map_header(map, size_of::<Pair>())).temp };
    assert!(index >= 0);
    unsafe { (*map.cast::<Pair>().add(index as usize)).value = value };
}

unsafe fn pair_snapshot(map: *mut c_void) -> Vec<Pair> {
    let length = unsafe { map_len(map, size_of::<Pair>()) };
    unsafe { std::slice::from_raw_parts(map.cast::<Pair>(), length).to_vec() }
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, i64)> {
    let length = unsafe { map_len(map, size_of::<StringPair>()) };
    (0..length)
        .map(|index| unsafe {
            let pair = *map.cast::<StringPair>().add(index);
            (CStr::from_ptr(pair.key).to_bytes().to_vec(), pair.value)
        })
        .collect()
}

unsafe fn set_string_value(map: *mut c_void, value: i64) {
    let index = unsafe { (*map_header(map, size_of::<StringPair>())).temp };
    assert!(index >= 0);
    unsafe { (*map.cast::<StringPair>().add(index as usize)).value = value };
}

unsafe fn hash_state(
    map: *mut c_void,
    element_size: usize,
) -> (Vec<usize>, Vec<isize>, [usize; 7]) {
    let table = unsafe {
        (*map_header(map, element_size))
            .hash_table
            .cast::<HashIndex>()
    };
    assert!(!table.is_null());
    let mut hashes = Vec::with_capacity(unsafe { (*table).slot_count });
    let mut indexes = Vec::with_capacity(unsafe { (*table).slot_count });
    for bucket_index in 0..unsafe { (*table).slot_count / 8 } {
        let bucket = unsafe { &*(*table).storage.add(bucket_index) };
        hashes.extend_from_slice(&bucket.hash);
        indexes.extend_from_slice(&bucket.index);
    }
    (hashes, indexes, unsafe {
        [
            (*table).slot_count,
            (*table).used_count,
            (*table).used_count_threshold,
            (*table).used_count_shrink_threshold,
            (*table).tombstone_count,
            (*table).tombstone_count_threshold,
            (*table).seed,
        ]
    })
}

#[test]
fn differential_surface() {
    unsafe {
        let (c, rust) = load_apis();
        arrays(&c, &rust);
        hashes(&c, &rust);
        default_maps_and_sentinels(&c, &rust);
        binary_maps(&c, &rust);
        string_maps(&c, &rust);
        string_arenas(&c, &rust);
        utility_exports(&c, &rust);
    }
}

#[test]
fn invalid_pointer_boundaries_match() {
    const CASES: &[&str] = &[
        "arrfree_null",
        "hash_string_null",
        "hash_bytes_null_nonzero",
        "hash_bytes_oversized",
        "hmget_ts_null_temp",
        "hmget_null_key",
        "hmput_null_string",
        "hmdel_null_string",
        "stralloc_null_arena",
        "stralloc_null_string",
        "strreset_null",
        "assert_hash_threshold",
        "assert_slot_range",
        "assert_moved_key_missing",
        "assert_moved_index_mismatch",
    ];
    let executable = std::env::current_exe().unwrap();
    for case in CASES {
        let run = |library: &str| {
            Command::new(&executable)
                .arg("--exact")
                .arg("ffi_crash_case_worker")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env("HARVEST_CRASH_CASE", case)
                .env("HARVEST_CRASH_LIBRARY", library)
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let r_status = run("rust");
        assert!(!c_status.success(), "C unexpectedly accepted {case}");
        assert!(!r_status.success(), "Rust unexpectedly accepted {case}");
        assert_eq!(
            c_status.signal(),
            r_status.signal(),
            "termination mismatch for {case}: C={c_status:?}, Rust={r_status:?}"
        );
    }
}

#[test]
fn ffi_crash_case_worker() {
    let Ok(case) = std::env::var("HARVEST_CRASH_CASE") else {
        return;
    };
    unsafe {
        let (c_path, rust_path) = library_paths();
        let api = match std::env::var("HARVEST_CRASH_LIBRARY").as_deref() {
            Ok("c") => Api::open(c_path),
            Ok("rust") => Api::open(rust_path),
            other => panic!("invalid HARVEST_CRASH_LIBRARY: {other:?}"),
        };
        run_crash_case(&api, &case);
    }
    panic!("crash case {case} returned");
}

unsafe fn run_crash_case(api: &Api, case: &str) {
    match case {
        "arrfree_null" => unsafe { (api.arrfree)(null_mut()) },
        "hash_string_null" => {
            let _ = unsafe { (api.hash_string)(null_mut(), 0) };
        }
        "hash_bytes_null_nonzero" => {
            let _ = unsafe { (api.hash_bytes)(null_mut(), 1, 0) };
        }
        "hash_bytes_oversized" => {
            let mut byte = 0u8;
            let _ = unsafe { (api.hash_bytes)(ptr::addr_of_mut!(byte).cast(), usize::MAX, 0) };
        }
        "hmget_ts_null_temp" => {
            let mut key = 0u64;
            let _ = unsafe {
                (api.hmget_ts)(
                    null_mut(),
                    size_of::<Pair>(),
                    ptr::addr_of_mut!(key).cast(),
                    8,
                    null_mut(),
                    0,
                )
            };
        }
        "hmget_null_key" => {
            let map = unsafe { put_pair(api, null_mut(), 1, 2, 0) };
            let _ = unsafe { (api.hmget)(map, size_of::<Pair>(), null_mut(), 8, 0) };
        }
        "hmput_null_string" => {
            let map = unsafe { (api.shmode)(size_of::<StringPair>(), 1) };
            let _ = unsafe {
                (api.hmput)(
                    map,
                    size_of::<StringPair>(),
                    null_mut(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
        }
        "hmdel_null_string" => {
            let key = CString::new("key").unwrap();
            let mut map = unsafe { (api.shmode)(size_of::<StringPair>(), 1) };
            map = unsafe {
                (api.hmput)(
                    map,
                    size_of::<StringPair>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
            let _ = unsafe {
                (api.hmdel)(
                    map,
                    size_of::<StringPair>(),
                    null_mut(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
        }
        "stralloc_null_arena" => {
            let string = CString::new("key").unwrap();
            let _ = unsafe { (api.stralloc)(null_mut(), string.as_ptr().cast_mut()) };
        }
        "stralloc_null_string" => {
            let mut arena: StringArena = unsafe { zeroed() };
            let _ = unsafe { (api.stralloc)(&mut arena, null_mut()) };
        }
        "strreset_null" => unsafe { (api.strreset)(null_mut()) },
        "assert_hash_threshold" => {
            let map = unsafe { put_pair(api, null_mut(), 1, 2, 0) };
            let table = unsafe {
                (*map_header(map, size_of::<Pair>()))
                    .hash_table
                    .cast::<HashIndex>()
            };
            unsafe {
                (*table).slot_count = 0;
                (*table).used_count = (*table).used_count_threshold;
            }
            let _ = unsafe { put_pair(api, map, 2, 3, 0) };
        }
        "assert_slot_range" => {
            let mut map = unsafe { put_pair(api, null_mut(), 1, 10, 0) };
            let table = unsafe {
                (*map_header(map, size_of::<Pair>()))
                    .hash_table
                    .cast::<HashIndex>()
            };
            let mut key = 100u64;
            loop {
                let mut hash = unsafe {
                    (api.hash_bytes)(
                        ptr::addr_of_mut!(key).cast(),
                        size_of::<u64>(),
                        (*table).seed,
                    )
                };
                if hash < 2 {
                    hash += 2;
                }
                if hash & 4 == 4 {
                    break;
                }
                key += 1;
            }
            map = unsafe { put_pair(api, map, key, 20, 0) };
            let table = unsafe {
                (*map_header(map, size_of::<Pair>()))
                    .hash_table
                    .cast::<HashIndex>()
            };
            let bucket = unsafe { &mut *(*table).storage };
            let source = bucket.index.iter().position(|index| *index == 1).unwrap();
            let hash = bucket.hash[source];
            bucket.hash[source] = 0;
            bucket.index[source] = -1;
            for slot in 4..=5 {
                bucket.hash[slot] = 1;
                bucket.index[slot] = -2;
            }
            bucket.hash[6] = hash;
            bucket.index[6] = 1;
            unsafe { (*table).slot_count = 5 };
            let _ = unsafe { delete_pair(api, map, key) };
        }
        "assert_moved_key_missing" => {
            let mut map = unsafe { put_pair(api, null_mut(), 1, 10, 0) };
            map = unsafe { put_pair(api, map, 2, 20, 0) };
            unsafe { corrupt_bucket_index(map, 1, -2, 1) };
            let _ = unsafe { delete_pair(api, map, 1) };
        }
        "assert_moved_index_mismatch" => {
            let mut map = unsafe { put_pair(api, null_mut(), 1, 10, 0) };
            map = unsafe { put_pair(api, map, 2, 20, 0) };
            map = unsafe { put_pair(api, map, 3, 30, 0) };
            unsafe {
                corrupt_bucket_index(map, 2, 1, usize::MAX);
                (*map.cast::<Pair>().add(1)).key = 3;
            }
            let _ = unsafe { delete_pair(api, map, 1) };
        }
        _ => panic!("unknown crash case: {case}"),
    }
}

unsafe fn corrupt_bucket_index(
    map: *mut c_void,
    old_index: isize,
    new_index: isize,
    replacement_hash: usize,
) {
    let table = unsafe {
        (*map_header(map, size_of::<Pair>()))
            .hash_table
            .cast::<HashIndex>()
    };
    for bucket_index in 0..unsafe { (*table).slot_count / 8 } {
        let bucket = unsafe { &mut *(*table).storage.add(bucket_index) };
        for slot in 0..8 {
            if bucket.index[slot] == old_index {
                bucket.index[slot] = new_index;
                if replacement_hash != usize::MAX {
                    bucket.hash[slot] = replacement_hash;
                }
                return;
            }
        }
    }
    panic!("bucket index {old_index} not found");
}

unsafe fn arrays(c: &Api, rust: &Api) {
    for minimum in 0..=3 {
        let c_array = unsafe { (c.arrgrow)(null_mut(), 4, 0, minimum) };
        let r_array = unsafe { (rust.arrgrow)(null_mut(), 4, 0, minimum) };
        if minimum == 0 {
            assert!(c_array.is_null() && r_array.is_null());
            continue;
        }
        assert!(!c_array.is_null() && !r_array.is_null());
        let ch = unsafe { *header(c_array) };
        let rh = unsafe { *header(r_array) };
        assert_eq!((ch.length, ch.capacity, ch.temp), (0, 4, 0));
        assert_eq!(
            (ch.length, ch.capacity, ch.temp),
            (rh.length, rh.capacity, rh.temp)
        );
        unsafe {
            (c.arrfree)(c_array);
            (rust.arrfree)(r_array);
        }
    }

    let mut rng = Rng(0xd1ff_e4e5_1234_5678);
    for _ in 0..128 {
        let element_size = (rng.next() as usize % 32) + 1;
        let add_length = (rng.next() as usize % 40) + 4;
        let c_array = unsafe { (c.arrgrow)(null_mut(), element_size, add_length, 0) };
        let r_array = unsafe { (rust.arrgrow)(null_mut(), element_size, add_length, 0) };
        assert_eq!(unsafe { (*header(c_array)).capacity }, unsafe {
            (*header(r_array)).capacity
        });
        assert_eq!(unsafe { (*header(c_array)).capacity }, add_length.max(4));
        unsafe {
            (c.arrfree)(c_array);
            (rust.arrfree)(r_array);
        }
    }

    let c_array = unsafe { (c.arrgrow)(null_mut(), 1, 0, 4) };
    let r_array = unsafe { (rust.arrgrow)(null_mut(), 1, 0, 4) };
    unsafe {
        (*header(c_array)).length = 3;
        (*header(r_array)).length = 3;
        ptr::copy_nonoverlapping(b"abc".as_ptr(), c_array.cast(), 3);
        ptr::copy_nonoverlapping(b"abc".as_ptr(), r_array.cast(), 3);
    }
    let c_same = unsafe { (c.arrgrow)(c_array, 1, 0, 4) };
    let r_same = unsafe { (rust.arrgrow)(r_array, 1, 0, 4) };
    assert_eq!(c_same, c_array);
    assert_eq!(r_same, r_array);
    assert_eq!(
        unsafe { std::slice::from_raw_parts(c_same.cast::<u8>(), 3) },
        unsafe { std::slice::from_raw_parts(r_same.cast::<u8>(), 3) }
    );

    unsafe {
        (*header(c_same)).length = 4;
        (*header(r_same)).length = 4;
    }
    let c_doubled = unsafe { (c.arrgrow)(c_same, 1, 1, 0) };
    let r_doubled = unsafe { (rust.arrgrow)(r_same, 1, 1, 0) };
    assert_eq!(unsafe { (*header(c_doubled)).capacity }, 8);
    assert_eq!(unsafe { (*header(c_doubled)).capacity }, unsafe {
        (*header(r_doubled)).capacity
    });

    let c_exact = unsafe { (c.arrgrow)(c_doubled, 1, 0, 20) };
    let r_exact = unsafe { (rust.arrgrow)(r_doubled, 1, 0, 20) };
    assert_eq!(unsafe { (*header(c_exact)).capacity }, 20);
    assert_eq!(unsafe { (*header(c_exact)).capacity }, unsafe {
        (*header(r_exact)).capacity
    });
    assert_eq!(
        unsafe { std::slice::from_raw_parts(c_exact.cast::<u8>(), 3) },
        unsafe { std::slice::from_raw_parts(r_exact.cast::<u8>(), 3) }
    );
    unsafe {
        (c.arrfree)(c_exact);
        (rust.arrfree)(r_exact);
    }
}

unsafe fn hashes(c: &Api, rust: &Api) {
    let seeds = [0, 1, 0x3141_5926, 1usize << (usize::BITS - 1), usize::MAX];
    let strings = [
        Vec::new(),
        b"a".to_vec(),
        b"the quick brown fox".to_vec(),
        vec![0x80, 0xfe, 0xff],
    ];
    for seed in seeds {
        for bytes in &strings {
            let string = CString::new(bytes.clone()).unwrap();
            let c_hash = unsafe { (c.hash_string)(string.as_ptr().cast_mut(), seed) };
            let r_hash = unsafe { (rust.hash_string)(string.as_ptr().cast_mut(), seed) };
            assert_eq!(c_hash, r_hash, "hash_string seed={seed:#x} bytes={bytes:?}");
        }

        unsafe {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
        }
        let c_map = unsafe { put_pair(c, null_mut(), 11, 22, 0) };
        let r_map = unsafe { put_pair(rust, null_mut(), 11, 22, 0) };
        assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()).2[6] }, seed);
        assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()) }, unsafe {
            hash_state(r_map, size_of::<Pair>())
        });
        unsafe {
            free_map(c, c_map, size_of::<Pair>());
            free_map(rust, r_map, size_of::<Pair>());
        }
    }

    let c_empty = unsafe { (c.hash_bytes)(null_mut(), 0, 0x1234) };
    let r_empty = unsafe { (rust.hash_bytes)(null_mut(), 0, 0x1234) };
    assert_eq!(c_empty, r_empty);

    let mut rng = Rng(0x5eed_f00d_dead_beef);
    for length in 0..=160usize {
        for _ in 0..32 {
            let seed = rng.next() as usize;
            let mut bytes = vec![0u8; length.max(1)];
            for byte in bytes.iter_mut().take(length) {
                *byte = rng.next() as u8;
            }
            let pointer = bytes.as_mut_ptr().cast();
            let c_hash = unsafe { (c.hash_bytes)(pointer, length, seed) };
            let r_hash = unsafe { (rust.hash_bytes)(pointer, length, seed) };
            assert_eq!(c_hash, r_hash, "hash_bytes length={length} seed={seed:#x}");
        }
    }
}

unsafe fn default_maps_and_sentinels(c: &Api, rust: &Api) {
    let element_size = size_of::<Pair>();
    let c_default = unsafe { (c.hmput_default)(null_mut(), element_size) };
    let r_default = unsafe { (rust.hmput_default)(null_mut(), element_size) };
    assert_eq!(unsafe { (*map_header(c_default, element_size)).length }, 1);
    assert_eq!(
        unsafe { (*map_header(c_default, element_size)).length },
        unsafe { (*map_header(r_default, element_size)).length }
    );
    assert_eq!(
        unsafe {
            std::slice::from_raw_parts(raw_map(c_default, element_size).cast::<u8>(), element_size)
        },
        unsafe {
            std::slice::from_raw_parts(raw_map(r_default, element_size).cast::<u8>(), element_size)
        }
    );
    let c_again = unsafe { (c.hmput_default)(c_default, element_size) };
    let r_again = unsafe { (rust.hmput_default)(r_default, element_size) };
    assert_eq!(c_again, c_default);
    assert_eq!(r_again, r_default);

    let mut key = 17u64;
    let mut c_temp = 99isize;
    let mut r_temp = 99isize;
    let c_get = unsafe {
        (c.hmget_ts)(
            c_again,
            element_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        )
    };
    let r_get = unsafe {
        (rust.hmget_ts)(
            r_again,
            element_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            &mut r_temp,
            0,
        )
    };
    assert_eq!((c_get == c_again, c_temp), (true, -1));
    assert_eq!((c_get == c_again, c_temp), (r_get == r_again, r_temp));

    let c_get = unsafe { (c.hmget)(c_get, element_size, ptr::addr_of_mut!(key).cast(), 8, 0) };
    let r_get = unsafe { (rust.hmget)(r_get, element_size, ptr::addr_of_mut!(key).cast(), 8, 0) };
    assert_eq!(unsafe { (*map_header(c_get, element_size)).temp }, unsafe {
        (*map_header(r_get, element_size)).temp
    });
    assert_eq!(unsafe { (*map_header(c_get, element_size)).temp }, -1);

    let c_del = unsafe { (c.hmdel)(c_get, element_size, ptr::addr_of_mut!(key).cast(), 8, 0, 0) };
    let r_del =
        unsafe { (rust.hmdel)(r_get, element_size, ptr::addr_of_mut!(key).cast(), 8, 0, 0) };
    assert_eq!(unsafe { (*map_header(c_del, element_size)).temp }, 0);
    assert_eq!(unsafe { (*map_header(c_del, element_size)).temp }, unsafe {
        (*map_header(r_del, element_size)).temp
    });
    unsafe {
        free_map(c, c_del, element_size);
        free_map(rust, r_del, element_size);
    }

    let mut c_temp = 123isize;
    let mut r_temp = 123isize;
    let c_null_get = unsafe {
        (c.hmget_ts)(
            null_mut(),
            element_size,
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut c_temp,
            0,
        )
    };
    let r_null_get = unsafe {
        (rust.hmget_ts)(
            null_mut(),
            element_size,
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut r_temp,
            0,
        )
    };
    assert_eq!((c_temp, r_temp), (-1, -1));
    assert_eq!(
        unsafe { (*map_header(c_null_get, element_size)).length },
        unsafe { (*map_header(r_null_get, element_size)).length }
    );
    unsafe {
        free_map(c, c_null_get, element_size);
        free_map(rust, r_null_get, element_size);
    }

    assert!(unsafe { (c.hmdel)(null_mut(), element_size, null_mut(), 0, 0, 0).is_null() });
    assert!(unsafe { (rust.hmdel)(null_mut(), element_size, null_mut(), 0, 0, 0).is_null() });
    unsafe {
        (c.hmfree)(null_mut(), element_size);
        (rust.hmfree)(null_mut(), element_size);
    }
}

unsafe fn binary_maps(c: &Api, rust: &Api) {
    let mut rng = Rng(0x0ddc_0ffe_e15e_beef);
    for key_size in [1usize, 4, 8, 16] {
        let element_size = key_size + 8;
        unsafe {
            (c.rand_seed)(0x1020_3040);
            (rust.rand_seed)(0x1020_3040);
        }
        let mut c_map = null_mut();
        let mut r_map = null_mut();
        for _ in 0..96 {
            let mut key = vec![0u8; key_size];
            for byte in &mut key {
                *byte = rng.next() as u8;
            }
            c_map = unsafe { (c.hmput)(c_map, element_size, key.as_mut_ptr().cast(), key_size, 0) };
            r_map =
                unsafe { (rust.hmput)(r_map, element_size, key.as_mut_ptr().cast(), key_size, 0) };
            let c_index = unsafe { (*map_header(c_map, element_size)).temp };
            let r_index = unsafe { (*map_header(r_map, element_size)).temp };
            assert_eq!(c_index, r_index);
            assert_eq!(
                unsafe {
                    std::slice::from_raw_parts(
                        c_map.cast::<u8>().add(c_index as usize * element_size),
                        key_size,
                    )
                },
                unsafe {
                    std::slice::from_raw_parts(
                        r_map.cast::<u8>().add(r_index as usize * element_size),
                        key_size,
                    )
                }
            );

            let marker = rng.next().to_ne_bytes();
            unsafe {
                ptr::copy_nonoverlapping(
                    marker.as_ptr(),
                    c_map
                        .cast::<u8>()
                        .add(c_index as usize * element_size + key_size),
                    marker.len(),
                );
                ptr::copy_nonoverlapping(
                    marker.as_ptr(),
                    r_map
                        .cast::<u8>()
                        .add(r_index as usize * element_size + key_size),
                    marker.len(),
                );
            }

            let mut c_temp = -99isize;
            let mut r_temp = -99isize;
            c_map = unsafe {
                (c.hmget_ts)(
                    c_map,
                    element_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    &mut c_temp,
                    0,
                )
            };
            r_map = unsafe {
                (rust.hmget_ts)(
                    r_map,
                    element_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    &mut r_temp,
                    0,
                )
            };
            assert_eq!(c_temp, r_temp);
            assert!(c_temp >= 0);
        }
        assert_eq!(unsafe { map_len(c_map, element_size) }, unsafe {
            map_len(r_map, element_size)
        });
        assert_eq!(unsafe { hash_state(c_map, element_size) }, unsafe {
            hash_state(r_map, element_size)
        });
        unsafe {
            free_map(c, c_map, element_size);
            free_map(rust, r_map, element_size);
        }
    }

    unsafe {
        (c.rand_seed)(0x5566_7788_99aa_bbcc);
        (rust.rand_seed)(0x5566_7788_99aa_bbcc);
    }
    let mut c_map = null_mut();
    let mut r_map = null_mut();
    let mut model = BTreeMap::new();
    for key in 0..80u64 {
        let value = key.wrapping_mul(0x9e37_79b9).wrapping_add(7);
        c_map = unsafe { put_pair(c, c_map, key, value, if key == 0 { -1 } else { 0 }) };
        r_map = unsafe { put_pair(rust, r_map, key, value, if key == 0 { -1 } else { 0 }) };
        model.insert(key, value);
        unsafe { assert_pair_maps(c_map, r_map, &model) };
    }

    for key in [0u64, 1, 17, 79] {
        let mut c_key = key;
        let mut r_key = key;
        c_map = unsafe {
            (c.hmget)(
                c_map,
                size_of::<Pair>(),
                ptr::addr_of_mut!(c_key).cast(),
                8,
                0,
            )
        };
        r_map = unsafe {
            (rust.hmget)(
                r_map,
                size_of::<Pair>(),
                ptr::addr_of_mut!(r_key).cast(),
                8,
                0,
            )
        };
        let c_index = unsafe { (*map_header(c_map, size_of::<Pair>())).temp };
        let r_index = unsafe { (*map_header(r_map, size_of::<Pair>())).temp };
        assert_eq!(c_index, r_index);
        assert_eq!(
            unsafe { (*c_map.cast::<Pair>().add(c_index as usize)).value },
            model[&key]
        );
    }

    let updated = 0xfeed_face_cafe_beefu64;
    c_map = unsafe { put_pair(c, c_map, 17, updated, 0) };
    r_map = unsafe { put_pair(rust, r_map, 17, updated, 0) };
    model.insert(17, updated);
    unsafe { assert_pair_maps(c_map, r_map, &model) };

    c_map = unsafe { delete_pair(c, c_map, 79) };
    r_map = unsafe { delete_pair(rust, r_map, 79) };
    model.remove(&79);
    assert_eq!(unsafe { (*map_header(c_map, size_of::<Pair>())).temp }, 1);
    unsafe { assert_pair_maps(c_map, r_map, &model) };
    c_map = unsafe { put_pair(c, c_map, 79, 79 * 0x9e37_79b9 + 7, 0) };
    r_map = unsafe { put_pair(rust, r_map, 79, 79 * 0x9e37_79b9 + 7, 0) };
    model.insert(79, 79 * 0x9e37_79b9 + 7);
    unsafe { assert_pair_maps(c_map, r_map, &model) };

    let mut missing = u64::MAX;
    let c_before = unsafe { pair_snapshot(c_map) };
    c_map = unsafe {
        (c.hmdel)(
            c_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(missing).cast(),
            8,
            0,
            0,
        )
    };
    r_map = unsafe {
        (rust.hmdel)(
            r_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(missing).cast(),
            8,
            0,
            0,
        )
    };
    assert_eq!(unsafe { (*map_header(c_map, size_of::<Pair>())).temp }, 0);
    assert_eq!(unsafe { pair_snapshot(c_map) }, c_before);
    unsafe { assert_pair_maps(c_map, r_map, &model) };

    unsafe { exercise_wrapped_miss(c, rust, c_map, r_map) };

    c_map = unsafe { delete_pair(c, c_map, 0) };
    r_map = unsafe { delete_pair(rust, r_map, 0) };
    model.remove(&0);
    let (hashes, indexes, state) = unsafe { hash_state(c_map, size_of::<Pair>()) };
    let tombstone_slot = indexes.iter().position(|index| *index == -2).unwrap();
    assert_eq!(hashes[tombstone_slot], 1);
    let mut tombstone_key = 10_000u64;
    loop {
        let mut hash = unsafe {
            (c.hash_bytes)(
                ptr::addr_of_mut!(tombstone_key).cast(),
                size_of::<u64>(),
                state[6],
            )
        };
        if hash < 2 {
            hash += 2;
        }
        if hash & (state[0] - 1) == tombstone_slot {
            break;
        }
        tombstone_key += 1;
    }
    c_map = unsafe { put_pair(c, c_map, tombstone_key, 0x1234, 0) };
    r_map = unsafe { put_pair(rust, r_map, tombstone_key, 0x1234, 0) };
    model.insert(tombstone_key, 0x1234);
    assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()).2[4] }, 0);
    unsafe { assert_pair_maps(c_map, r_map, &model) };

    for key in 1..=25u64 {
        c_map = unsafe { delete_pair(c, c_map, key) };
        r_map = unsafe { delete_pair(rust, r_map, key) };
        model.remove(&key);
        unsafe { assert_pair_maps(c_map, r_map, &model) };
    }
    assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()).2[0] }, 128);
    assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()).2[4] }, 0);

    c_map = unsafe { put_pair(c, c_map, 1000, 2000, 0) };
    r_map = unsafe { put_pair(rust, r_map, 1000, 2000, 0) };
    model.insert(1000, 2000);
    unsafe { assert_pair_maps(c_map, r_map, &model) };

    let mut key = 25u64;
    while unsafe { hash_state(c_map, size_of::<Pair>()).2[0] } == 128 {
        c_map = unsafe { delete_pair(c, c_map, key) };
        r_map = unsafe { delete_pair(rust, r_map, key) };
        model.remove(&key);
        key += 1;
        unsafe { assert_pair_maps(c_map, r_map, &model) };
    }
    assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()).2[0] }, 64);

    let mut operation_rng = Rng(0xabc0_1234_9876_5def);
    for _ in 0..512 {
        let key = operation_rng.next() % 128;
        match operation_rng.next() % 3 {
            0 => {
                let value = operation_rng.next();
                c_map = unsafe { put_pair(c, c_map, key, value, 0) };
                r_map = unsafe { put_pair(rust, r_map, key, value, 0) };
                model.insert(key, value);
            }
            1 => {
                c_map = unsafe { delete_pair(c, c_map, key) };
                r_map = unsafe { delete_pair(rust, r_map, key) };
                model.remove(&key);
            }
            _ => {
                let mut c_key = key;
                let mut r_key = key;
                let mut c_temp = -7isize;
                let mut r_temp = -7isize;
                c_map = unsafe {
                    (c.hmget_ts)(
                        c_map,
                        size_of::<Pair>(),
                        ptr::addr_of_mut!(c_key).cast(),
                        8,
                        &mut c_temp,
                        0,
                    )
                };
                r_map = unsafe {
                    (rust.hmget_ts)(
                        r_map,
                        size_of::<Pair>(),
                        ptr::addr_of_mut!(r_key).cast(),
                        8,
                        &mut r_temp,
                        0,
                    )
                };
                assert_eq!(c_temp, r_temp);
                assert_eq!(c_temp >= 0, model.contains_key(&key));
            }
        }
        unsafe { assert_pair_maps(c_map, r_map, &model) };
    }
    unsafe {
        free_map(c, c_map, size_of::<Pair>());
        free_map(rust, r_map, size_of::<Pair>());
    }
}

unsafe fn put_pair(
    api: &Api,
    map: *mut c_void,
    mut key: u64,
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            mode,
        )
    };
    unsafe { set_pair_value(map, value) };
    map
}

unsafe fn delete_pair(api: &Api, map: *mut c_void, mut key: u64) -> *mut c_void {
    unsafe {
        (api.hmdel)(
            map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    }
}

unsafe fn assert_pair_maps(c_map: *mut c_void, r_map: *mut c_void, model: &BTreeMap<u64, u64>) {
    let c_pairs = unsafe { pair_snapshot(c_map) };
    let r_pairs = unsafe { pair_snapshot(r_map) };
    assert_eq!(c_pairs, r_pairs);
    assert_eq!(c_pairs.len(), model.len());
    for pair in c_pairs {
        assert_eq!(model.get(&pair.key), Some(&pair.value));
    }
    assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()) }, unsafe {
        hash_state(r_map, size_of::<Pair>())
    });
}

unsafe fn exercise_wrapped_miss(c: &Api, rust: &Api, c_map: *mut c_void, r_map: *mut c_void) {
    let (_, _, state) = unsafe { hash_state(c_map, size_of::<Pair>()) };
    let table = unsafe {
        (*map_header(c_map, size_of::<Pair>()))
            .hash_table
            .cast::<HashIndex>()
    };
    let seed = state[6];
    let mut candidate = 10_000u64;
    let found = loop {
        let mut hash =
            unsafe { (c.hash_bytes)(ptr::addr_of_mut!(candidate).cast(), size_of::<u64>(), seed) };
        if hash < 2 {
            hash += 2;
        }
        let position = hash & (state[0] - 1);
        let bucket = unsafe { &*(*table).storage.add(position >> 3) };
        let start = position & 7;
        if start > 0
            && bucket.hash[start..].iter().all(|value| *value != 0)
            && bucket.hash[..start].contains(&0)
        {
            break candidate;
        }
        candidate += 1;
        assert!(candidate < 2_000_000, "failed to construct wrapped miss");
    };

    let mut c_key = found;
    let mut r_key = found;
    let mut c_temp = 123;
    let mut r_temp = 123;
    let c_result = unsafe {
        (c.hmget_ts)(
            c_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(c_key).cast(),
            8,
            &mut c_temp,
            0,
        )
    };
    let r_result = unsafe {
        (rust.hmget_ts)(
            r_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(r_key).cast(),
            8,
            &mut r_temp,
            0,
        )
    };
    assert_eq!((c_result == c_map, c_temp), (true, -1));
    assert_eq!((c_result == c_map, c_temp), (r_result == r_map, r_temp));
}

unsafe fn string_maps(c: &Api, rust: &Api) {
    for ownership_mode in [1, 2, 3] {
        unsafe {
            (c.rand_seed)(0x1357_9bdf);
            (rust.rand_seed)(0x1357_9bdf);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<StringPair>(), ownership_mode) };
        let mut r_map = unsafe { (rust.shmode)(size_of::<StringPair>(), ownership_mode) };
        let byte_keys = [
            Vec::new(),
            b"a".to_vec(),
            b"short-key".to_vec(),
            vec![b'x'; 700],
            vec![0x80, 0xfe, b'z'],
        ];
        let mut c_keys = Vec::new();
        let mut r_keys = Vec::new();
        for (index, bytes) in byte_keys.into_iter().enumerate() {
            c_keys.push(CString::new(bytes.clone()).unwrap());
            r_keys.push(CString::new(bytes).unwrap());
            let hash_mode = if ownership_mode == 1 && index == 2 {
                2
            } else {
                1
            };
            c_map = unsafe {
                (c.hmput)(
                    c_map,
                    size_of::<StringPair>(),
                    c_keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    hash_mode,
                )
            };
            r_map = unsafe {
                (rust.hmput)(
                    r_map,
                    size_of::<StringPair>(),
                    r_keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    hash_mode,
                )
            };
            unsafe {
                set_string_value(c_map, index as i64 * 31 - 7);
                set_string_value(r_map, index as i64 * 31 - 7);
            }
            assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
                string_snapshot(r_map)
            });
            assert_eq!(
                unsafe { hash_state(c_map, size_of::<StringPair>()) },
                unsafe { hash_state(r_map, size_of::<StringPair>()) }
            );
        }

        let duplicate_c = CString::new("short-key").unwrap();
        let duplicate_r = CString::new("short-key").unwrap();
        c_map = unsafe {
            (c.hmput)(
                c_map,
                size_of::<StringPair>(),
                duplicate_c.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        r_map = unsafe {
            (rust.hmput)(
                r_map,
                size_of::<StringPair>(),
                duplicate_r.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        unsafe {
            set_string_value(c_map, 999);
            set_string_value(r_map, 999);
        }
        assert_eq!(unsafe { map_len(c_map, size_of::<StringPair>()) }, 5);
        assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
            string_snapshot(r_map)
        });

        for lookup in ["", "a", "short-key", "missing"] {
            let c_lookup = CString::new(lookup).unwrap();
            let r_lookup = CString::new(lookup).unwrap();
            let mut c_temp = -77isize;
            let mut r_temp = -77isize;
            c_map = unsafe {
                (c.hmget_ts)(
                    c_map,
                    size_of::<StringPair>(),
                    c_lookup.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    1,
                )
            };
            r_map = unsafe {
                (rust.hmget_ts)(
                    r_map,
                    size_of::<StringPair>(),
                    r_lookup.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut r_temp,
                    1,
                )
            };
            assert_eq!(c_temp, r_temp);
            assert_eq!(c_temp >= 0, lookup != "missing");
        }

        for deletion in ["a", "short-key"] {
            let c_key = CString::new(deletion).unwrap();
            let r_key = CString::new(deletion).unwrap();
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    size_of::<StringPair>(),
                    c_key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            r_map = unsafe {
                (rust.hmdel)(
                    r_map,
                    size_of::<StringPair>(),
                    r_key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            assert_eq!(
                unsafe { (*map_header(c_map, size_of::<StringPair>())).temp },
                1
            );
            assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
                string_snapshot(r_map)
            });
            assert_eq!(
                unsafe { hash_state(c_map, size_of::<StringPair>()) },
                unsafe { hash_state(r_map, size_of::<StringPair>()) }
            );
        }
        unsafe {
            free_map(c, c_map, size_of::<StringPair>());
            free_map(rust, r_map, size_of::<StringPair>());
        }
    }

    for mode in [0, 4, 260] {
        unsafe {
            (c.rand_seed)(0x2468_ace0);
            (rust.rand_seed)(0x2468_ace0);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<Pair>(), mode) };
        let mut r_map = unsafe { (rust.shmode)(size_of::<Pair>(), mode) };
        for key in 10..40u64 {
            c_map = unsafe { put_pair(c, c_map, key, key * 3, 0) };
            r_map = unsafe { put_pair(rust, r_map, key, key * 3, 0) };
        }
        assert_eq!(unsafe { pair_snapshot(c_map) }, unsafe {
            pair_snapshot(r_map)
        });
        assert_eq!(unsafe { hash_state(c_map, size_of::<Pair>()) }, unsafe {
            hash_state(r_map, size_of::<Pair>())
        });
        unsafe {
            free_map(c, c_map, size_of::<Pair>());
            free_map(rust, r_map, size_of::<Pair>());
        }
    }
}

unsafe fn string_arenas(c: &Api, rust: &Api) {
    let mut c_arena: StringArena = unsafe { zeroed() };
    let mut r_arena: StringArena = unsafe { zeroed() };
    let mut rng = Rng(0x7777_3333_aaaa_5555);

    for length in [0usize, 1, 7, 63, 127, 255] {
        let mut bytes = vec![b'a'; length];
        for byte in &mut bytes {
            *byte = b'a' + (rng.next() % 26) as u8;
        }
        let c_string = CString::new(bytes.clone()).unwrap();
        let r_string = CString::new(bytes.clone()).unwrap();
        let c_output = unsafe { (c.stralloc)(&mut c_arena, c_string.as_ptr().cast_mut()) };
        let r_output = unsafe { (rust.stralloc)(&mut r_arena, r_string.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(c_output).to_bytes() }, bytes);
        assert_eq!(unsafe { CStr::from_ptr(c_output).to_bytes() }, unsafe {
            CStr::from_ptr(r_output).to_bytes()
        });
        assert_eq!(
            (c_arena.remaining, c_arena.block, c_arena.mode),
            (r_arena.remaining, r_arena.block, r_arena.mode)
        );
    }

    let oversized = vec![b'q'; 700];
    let c_string = CString::new(oversized.clone()).unwrap();
    let r_string = CString::new(oversized.clone()).unwrap();
    let c_output = unsafe { (c.stralloc)(&mut c_arena, c_string.as_ptr().cast_mut()) };
    let r_output = unsafe { (rust.stralloc)(&mut r_arena, r_string.as_ptr().cast_mut()) };
    assert_eq!(unsafe { CStr::from_ptr(c_output).to_bytes() }, unsafe {
        CStr::from_ptr(r_output).to_bytes()
    });
    assert_eq!(
        (c_arena.remaining, c_arena.block),
        (r_arena.remaining, r_arena.block)
    );
    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
    }
    assert_eq!(
        (
            c_arena.storage.is_null(),
            c_arena.remaining,
            c_arena.block,
            c_arena.mode
        ),
        (true, 0, 0, 0)
    );
    assert_eq!(
        (
            c_arena.storage.is_null(),
            c_arena.remaining,
            c_arena.block,
            c_arena.mode
        ),
        (
            r_arena.storage.is_null(),
            r_arena.remaining,
            r_arena.block,
            r_arena.mode
        )
    );

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
    }

    let dedicated = CString::new(vec![b'z'; 700]).unwrap();
    let c_output = unsafe { (c.stralloc)(&mut c_arena, dedicated.as_ptr().cast_mut()) };
    let r_output = unsafe { (rust.stralloc)(&mut r_arena, dedicated.as_ptr().cast_mut()) };
    assert_eq!(unsafe { CStr::from_ptr(c_output).to_bytes() }, unsafe {
        CStr::from_ptr(r_output).to_bytes()
    });
    assert_eq!((c_arena.remaining, c_arena.block), (0, 1));
    assert_eq!(
        (c_arena.remaining, c_arena.block),
        (r_arena.remaining, r_arena.block)
    );

    for _ in 0..24 {
        let block_size = 512usize << (c_arena.block >> 1);
        let allocation = block_size.min(1 << 20);
        let string = CString::new(vec![b'm'; allocation - 1]).unwrap();
        let c_output = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let r_output = unsafe { (rust.stralloc)(&mut r_arena, string.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_output).to_bytes().len() },
            unsafe { CStr::from_ptr(r_output).to_bytes().len() }
        );
        assert_eq!(
            (c_arena.remaining, c_arena.block),
            (r_arena.remaining, r_arena.block)
        );
        if c_arena.block >= 22 {
            break;
        }
    }
    assert!(c_arena.block >= 22);
    let capped = CString::new(vec![b'c'; (1 << 20) - 1]).unwrap();
    let c_block_before = c_arena.block;
    let r_block_before = r_arena.block;
    let c_output = unsafe { (c.stralloc)(&mut c_arena, capped.as_ptr().cast_mut()) };
    let r_output = unsafe { (rust.stralloc)(&mut r_arena, capped.as_ptr().cast_mut()) };
    assert_eq!(
        unsafe { CStr::from_ptr(c_output).to_bytes().len() },
        unsafe { CStr::from_ptr(r_output).to_bytes().len() }
    );
    assert_eq!(c_arena.block, c_block_before);
    assert_eq!(r_arena.block, r_block_before);
    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
    }
}

unsafe fn utility_exports(c: &Api, rust: &Api) {
    for value in [c_int::MIN, -1, 0, 1, 42, c_int::MAX] {
        let c_output = unsafe { CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec() };
        let r_output = unsafe { CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec() };
        assert_eq!(c_output, r_output);
        assert_eq!(c_output, format!("test_{value}").as_bytes());
    }

    for value in [c_int::MIN, -1, 0, 1, 42, c_int::MAX] {
        unsafe {
            (c.arr_del)(value);
            (rust.arr_del)(value);
        }
    }
}
