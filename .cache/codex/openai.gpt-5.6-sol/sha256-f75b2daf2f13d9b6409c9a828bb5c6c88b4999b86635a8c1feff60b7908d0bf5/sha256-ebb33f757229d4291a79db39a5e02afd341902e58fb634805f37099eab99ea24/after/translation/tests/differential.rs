use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt::Debug;
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmGetKeyTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGetKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPutKey = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDelKey =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
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
    hmget_ts: HmGetKeyTs,
    hmget: HmGetKey,
    hmput_default: HmPutDefault,
    hmput: HmPutKey,
    shmode: ShMode,
    hmdel: HmDelKey,
    hmfree: HmFree,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    arr_ins: ArrIns,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(&path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let loaded = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *loaded
            }};
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetKeyTs),
            hmget: symbol!("stbds_hmget_key", HmGetKey),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPutKey),
            shmode: symbol!("stbds_shmode_func", ShMode),
            hmdel: symbol!("stbds_hmdel_key", HmDelKey),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            arr_ins: symbol!("arr_ins", ArrIns),
            _library: library,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl StringArena {
    fn empty() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BinaryEntry {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct HeaderSnapshot {
    length: usize,
    capacity: usize,
    has_hash_table: bool,
    temp: isize,
}

#[derive(Debug, PartialEq, Eq)]
struct TableSnapshot {
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string_remaining: usize,
    string_block: u8,
    string_mode: u8,
    hashes: Vec<usize>,
    indices: Vec<isize>,
}

#[derive(Debug, PartialEq, Eq)]
struct MapSnapshot<T> {
    header: HeaderSnapshot,
    table: Option<TableSnapshot>,
    entries: Vec<T>,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

unsafe fn header(data: *mut c_void) -> *mut ArrayHeader {
    unsafe { (data as *mut ArrayHeader).sub(1) }
}

unsafe fn raw_map(map: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(elemsize).cast() }
}

unsafe fn header_snapshot(data: *mut c_void) -> HeaderSnapshot {
    let value = unsafe { *header(data) };
    HeaderSnapshot {
        length: value.length,
        capacity: value.capacity,
        has_hash_table: !value.hash_table.is_null(),
        temp: value.temp,
    }
}

unsafe fn table_snapshot(raw: *mut c_void) -> Option<TableSnapshot> {
    let table = unsafe { (*header(raw)).hash_table.cast::<HashIndex>() };
    if table.is_null() {
        return None;
    }
    let table_ref = unsafe { &*table };
    let mut hashes = Vec::with_capacity(table_ref.slot_count);
    let mut indices = Vec::with_capacity(table_ref.slot_count);
    for bucket_index in 0..(table_ref.slot_count / 8) {
        let bucket = unsafe { &*table_ref.storage.add(bucket_index) };
        hashes.extend_from_slice(&bucket.hash);
        indices.extend_from_slice(&bucket.index);
    }
    Some(TableSnapshot {
        slot_count: table_ref.slot_count,
        used_count: table_ref.used_count,
        used_count_threshold: table_ref.used_count_threshold,
        used_count_shrink_threshold: table_ref.used_count_shrink_threshold,
        tombstone_count: table_ref.tombstone_count,
        tombstone_count_threshold: table_ref.tombstone_count_threshold,
        seed: table_ref.seed,
        slot_count_log2: table_ref.slot_count_log2,
        string_remaining: table_ref.string.remaining,
        string_block: table_ref.string.block,
        string_mode: table_ref.string.mode,
        hashes,
        indices,
    })
}

unsafe fn binary_snapshot(map: *mut c_void) -> MapSnapshot<BinaryEntry> {
    let elemsize = size_of::<BinaryEntry>();
    let raw = unsafe { raw_map(map, elemsize) };
    let header_value = unsafe { *header(raw) };
    let mut entries = Vec::new();
    for index in 0..header_value.length.saturating_sub(1) {
        entries.push(unsafe { *(map as *const BinaryEntry).add(index) });
    }
    MapSnapshot {
        header: unsafe { header_snapshot(raw) },
        table: unsafe { table_snapshot(raw) },
        entries,
    }
}

unsafe fn string_snapshot(map: *mut c_void) -> MapSnapshot<Vec<u8>> {
    let elemsize = size_of::<StringEntry>();
    let raw = unsafe { raw_map(map, elemsize) };
    let header_value = unsafe { *header(raw) };
    let mut entries = Vec::new();
    for index in 0..header_value.length.saturating_sub(1) {
        let entry = unsafe { &*(map as *const StringEntry).add(index) };
        let mut bytes = unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec();
        bytes.extend_from_slice(&entry.value.to_ne_bytes());
        entries.push(bytes);
    }
    MapSnapshot {
        header: unsafe { header_snapshot(raw) },
        table: unsafe { table_snapshot(raw) },
        entries,
    }
}

fn assert_same<T: Debug + PartialEq>(label: &str, c: T, rust: T) {
    assert_eq!(c, rust, "{label}");
}

unsafe fn set_binary_value(map: *mut c_void, value: i64) {
    let raw = unsafe { raw_map(map, size_of::<BinaryEntry>()) };
    let index = unsafe { (*header(raw)).temp as usize };
    unsafe { (*(map as *mut BinaryEntry).add(index)).value = value };
}

unsafe fn set_string_value(map: *mut c_void, value: i64) {
    let raw = unsafe { raw_map(map, size_of::<StringEntry>()) };
    let index = unsafe { (*header(raw)).temp as usize };
    unsafe { (*(map as *mut StringEntry).add(index)).value = value };
}

unsafe fn free_binary(api: &Api, map: *mut c_void) {
    if !map.is_null() {
        unsafe {
            (api.hmfree)(
                raw_map(map, size_of::<BinaryEntry>()),
                size_of::<BinaryEntry>(),
            )
        };
    }
}

unsafe fn free_string(api: &Api, map: *mut c_void) {
    if !map.is_null() {
        unsafe {
            (api.hmfree)(
                raw_map(map, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            )
        };
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-EZOeop.so"),
        root.join("target/release/libarr_ins_lib.so"),
    )
}

unsafe fn verify_hashes(c: &Api, rust: &Api) {
    // CONFIGS 3-10; ERRORS 18.
    let seeds = [0, 1, 0x3141_5926, usize::MAX, 0xfeed_face_dead_beef];
    for seed in seeds {
        let mut empty = [0u8; 1];
        assert_same(
            "zero-length hash with non-null pointer",
            unsafe { (c.hash_bytes)(empty.as_mut_ptr().cast(), 0, seed) },
            unsafe { (rust.hash_bytes)(empty.as_mut_ptr().cast(), 0, seed) },
        );
        assert_same(
            "zero-length hash with null pointer",
            unsafe { (c.hash_bytes)(ptr::null_mut(), 0, seed) },
            unsafe { (rust.hash_bytes)(ptr::null_mut(), 0, seed) },
        );

        let empty_string = CString::new("").unwrap();
        assert_same(
            "empty string hash",
            unsafe { (c.hash_string)(empty_string.as_ptr().cast_mut(), seed) },
            unsafe { (rust.hash_string)(empty_string.as_ptr().cast_mut(), seed) },
        );
    }

    let mut rng = Rng::new(0x65f0_4a11_7c2d_99e3);
    for len in 0..=95 {
        for _ in 0..64 {
            let mut bytes = vec![0u8; len.max(1)];
            rng.fill(&mut bytes[..len]);
            let seed = rng.next_u64() as usize;
            let c_hash = unsafe { (c.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed) };
            let rust_hash = unsafe { (rust.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed) };
            assert_same(&format!("byte hash len={len}"), c_hash, rust_hash);
        }
    }

    for len in 1..=80 {
        for _ in 0..32 {
            let mut bytes = Vec::with_capacity(len);
            while bytes.len() < len {
                let byte = (rng.next_u64() as u8).max(1);
                bytes.push(byte);
            }
            let string = CString::new(bytes).unwrap();
            let seed = rng.next_u64() as usize;
            assert_same(
                &format!("string hash len={len}"),
                unsafe { (c.hash_string)(string.as_ptr().cast_mut(), seed) },
                unsafe { (rust.hash_string)(string.as_ptr().cast_mut(), seed) },
            );
        }
    }
}

unsafe fn verify_arrays(c: &Api, rust: &Api) {
    // CONFIGS 11-18; ERRORS 20.
    assert!((unsafe { (c.arrgrow)(ptr::null_mut(), 4, 0, 0) }).is_null());
    assert!((unsafe { (rust.arrgrow)(ptr::null_mut(), 4, 0, 0) }).is_null());

    let mut rng = Rng::new(0x91da_22cb_12f8_0031);
    for elemsize in [1usize, 4, 16] {
        for _case in 0..80 {
            let initial_min = match rng.usize(5) {
                0 => 1,
                1 => 2,
                2 => 3,
                3 => 4,
                _ => 4 + rng.usize(24),
            };
            let mut c_data = unsafe { (c.arrgrow)(ptr::null_mut(), elemsize, 0, initial_min) };
            let mut rust_data =
                unsafe { (rust.arrgrow)(ptr::null_mut(), elemsize, 0, initial_min) };
            assert_eq!(c_data.is_null(), rust_data.is_null());
            assert_same(
                "initial array header",
                unsafe { header_snapshot(c_data) },
                unsafe { header_snapshot(rust_data) },
            );

            let initial_capacity = unsafe { (*header(c_data)).capacity };
            let length = rng.usize(initial_capacity + 1);
            unsafe {
                (*header(c_data)).length = length;
                (*header(rust_data)).length = length;
                (*header(c_data)).temp = rng.next_u64() as isize;
                (*header(rust_data)).temp = (*header(c_data)).temp;
            }
            let mut bytes = vec![0u8; length * elemsize];
            rng.fill(&mut bytes);
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), c_data.cast(), bytes.len());
                ptr::copy_nonoverlapping(bytes.as_ptr(), rust_data.cast(), bytes.len());
            }

            for step in 0..20 {
                let capacity = unsafe { (*header(c_data)).capacity };
                let (addlen, min_cap) = match step % 4 {
                    0 => (0, capacity.saturating_sub(rng.usize(capacity + 1))),
                    1 => (capacity.saturating_sub(length) + 1, 0),
                    2 => (0, capacity.saturating_mul(2).saturating_add(rng.usize(9))),
                    _ => (rng.usize(8), rng.usize(capacity.saturating_mul(3) + 9)),
                };
                c_data = unsafe { (c.arrgrow)(c_data, elemsize, addlen, min_cap) };
                rust_data = unsafe { (rust.arrgrow)(rust_data, elemsize, addlen, min_cap) };
                assert_same(
                    "grown array header",
                    unsafe { header_snapshot(c_data) },
                    unsafe { header_snapshot(rust_data) },
                );
                let c_bytes =
                    unsafe { std::slice::from_raw_parts(c_data.cast::<u8>(), bytes.len()) };
                let rust_bytes =
                    unsafe { std::slice::from_raw_parts(rust_data.cast::<u8>(), bytes.len()) };
                assert_eq!(c_bytes, rust_bytes, "array payload preservation");
            }
            unsafe {
                (c.arrfree)(c_data);
                (rust.arrfree)(rust_data);
            }
        }
    }
}

unsafe fn verify_default_and_sentinel_paths(c: &Api, rust: &Api) {
    // CONFIGS 19-22, 34; ERRORS 1-6, 19.
    let elemsize = size_of::<BinaryEntry>();
    let mut key = 0x1122_3344_5566_7788u64;

    let mut c_temp = 99isize;
    let mut rust_temp = 99isize;
    let c_get_ts = unsafe {
        (c.hmget_ts)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        )
    };
    let rust_get_ts = unsafe {
        (rust.hmget_ts)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut rust_temp,
            0,
        )
    };
    assert_eq!(c_temp, -1);
    assert_same("null hmget_ts temp", c_temp, rust_temp);
    assert_same(
        "null hmget_ts state",
        unsafe { binary_snapshot(c_get_ts) },
        unsafe { binary_snapshot(rust_get_ts) },
    );
    unsafe {
        free_binary(c, c_get_ts);
        free_binary(rust, rust_get_ts);
    }

    let c_get = unsafe {
        (c.hmget)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    let rust_get = unsafe {
        (rust.hmget)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    assert_same(
        "null hmget state",
        unsafe { binary_snapshot(c_get) },
        unsafe { binary_snapshot(rust_get) },
    );
    unsafe {
        free_binary(c, c_get);
        free_binary(rust, rust_get);
    }

    let mut c_default = unsafe { (c.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>()) };
    let mut rust_default =
        unsafe { (rust.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>()) };
    assert_same(
        "new default-only map",
        unsafe { binary_snapshot(c_default) },
        unsafe { binary_snapshot(rust_default) },
    );
    let c_before = c_default;
    let rust_before = rust_default;
    c_default = unsafe { (c.hmput_default)(c_default, elemsize) };
    rust_default = unsafe { (rust.hmput_default)(rust_default, elemsize) };
    assert_eq!(c_default == c_before, rust_default == rust_before);

    c_temp = 88;
    rust_temp = 88;
    c_default = unsafe {
        (c.hmget_ts)(
            c_default,
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        )
    };
    rust_default = unsafe {
        (rust.hmget_ts)(
            rust_default,
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut rust_temp,
            0,
        )
    };
    assert_same("default-only lookup sentinel", c_temp, rust_temp);
    assert_eq!(c_temp, -1);

    let c_pointer = c_default;
    let rust_pointer = rust_default;
    c_default = unsafe {
        (c.hmdel)(
            c_default,
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    rust_default = unsafe {
        (rust.hmdel)(
            rust_default,
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    assert_eq!(c_default == c_pointer, rust_default == rust_pointer);
    assert_same(
        "default-only delete no-op",
        unsafe { binary_snapshot(c_default) },
        unsafe { binary_snapshot(rust_default) },
    );

    let c_null = unsafe {
        (c.hmdel)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    let rust_null = unsafe {
        (rust.hmdel)(
            ptr::null_mut(),
            elemsize,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    assert!(c_null.is_null() && rust_null.is_null());
    unsafe {
        free_binary(c, c_default);
        free_binary(rust, rust_default);
        (c.hmfree)(ptr::null_mut(), elemsize);
        (rust.hmfree)(ptr::null_mut(), elemsize);
    }
}

unsafe fn verify_binary_maps(c: &Api, rust: &Api) {
    // CONFIGS 1, 23-27; ERRORS 3, 6-12.
    let elemsize = size_of::<BinaryEntry>();
    let mut rng = Rng::new(0xe910_a52d_718b_f007);
    for round in 0..12usize {
        let seed = rng.next_u64() as usize;
        unsafe {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
        }
        let mut c_map = if round % 2 == 0 {
            unsafe { (c.shmode)(elemsize, 0) }
        } else {
            ptr::null_mut()
        };
        let mut rust_map = if round % 2 == 0 {
            unsafe { (rust.shmode)(elemsize, 0) }
        } else {
            ptr::null_mut()
        };

        for operation in 0..700 {
            let mut key = rng.next_u64() % 180;
            match rng.usize(5) {
                0 | 1 => {
                    let value = rng.next_u64() as i64;
                    c_map = unsafe {
                        (c.hmput)(
                            c_map,
                            elemsize,
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                        )
                    };
                    rust_map = unsafe {
                        (rust.hmput)(
                            rust_map,
                            elemsize,
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                        )
                    };
                    unsafe {
                        set_binary_value(c_map, value);
                        set_binary_value(rust_map, value);
                    }
                }
                2 | 3 => {
                    if c_map.is_null() {
                        continue;
                    }
                    if operation % 2 == 0 {
                        let mut c_temp = 123;
                        let mut rust_temp = 123;
                        c_map = unsafe {
                            (c.hmget_ts)(
                                c_map,
                                elemsize,
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                &mut c_temp,
                                0,
                            )
                        };
                        rust_map = unsafe {
                            (rust.hmget_ts)(
                                rust_map,
                                elemsize,
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                &mut rust_temp,
                                0,
                            )
                        };
                        assert_same("binary hmget_ts temp", c_temp, rust_temp);
                    } else {
                        c_map = unsafe {
                            (c.hmget)(
                                c_map,
                                elemsize,
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                0,
                            )
                        };
                        rust_map = unsafe {
                            (rust.hmget)(
                                rust_map,
                                elemsize,
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                0,
                            )
                        };
                    }
                }
                _ => {
                    c_map = unsafe {
                        (c.hmdel)(
                            c_map,
                            elemsize,
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                            0,
                        )
                    };
                    rust_map = unsafe {
                        (rust.hmdel)(
                            rust_map,
                            elemsize,
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                            0,
                        )
                    };
                }
            }
            if !c_map.is_null() {
                assert_same(
                    &format!("binary map round={round} operation={operation}"),
                    unsafe { binary_snapshot(c_map) },
                    unsafe { binary_snapshot(rust_map) },
                );
            } else {
                assert!(rust_map.is_null());
            }
        }
        unsafe {
            free_binary(c, c_map);
            free_binary(rust, rust_map);
        }
    }
}

unsafe fn verify_string_mode(c: &Api, rust: &Api, mode: c_int, seed: usize) {
    let elemsize = size_of::<StringEntry>();
    unsafe {
        (c.rand_seed)(seed);
        (rust.rand_seed)(seed);
    }
    let mut c_map = unsafe { (c.shmode)(elemsize, mode) };
    let mut rust_map = unsafe { (rust.shmode)(elemsize, mode) };
    assert_same(
        "new string map",
        unsafe { string_snapshot(c_map) },
        unsafe { string_snapshot(rust_map) },
    );

    let mut owned_keys = Vec::new();
    for index in 0..96 {
        let key = CString::new(format!("key_{mode}_{index:03}_{}", index * 17)).unwrap();
        owned_keys.push(key);
        let key_ptr = owned_keys.last().unwrap().as_ptr().cast_mut();
        c_map = unsafe { (c.hmput)(c_map, elemsize, key_ptr.cast(), size_of::<*mut c_char>(), 1) };
        rust_map = unsafe {
            (rust.hmput)(
                rust_map,
                elemsize,
                key_ptr.cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        unsafe {
            set_string_value(c_map, index as i64 * -31);
            set_string_value(rust_map, index as i64 * -31);
        }
        assert_same(
            &format!("string insert mode={mode} index={index}"),
            unsafe { string_snapshot(c_map) },
            unsafe { string_snapshot(rust_map) },
        );
    }

    for index in (0..96).step_by(3) {
        let key_ptr = owned_keys[index].as_ptr().cast_mut();
        c_map = unsafe { (c.hmput)(c_map, elemsize, key_ptr.cast(), size_of::<*mut c_char>(), 1) };
        rust_map = unsafe {
            (rust.hmput)(
                rust_map,
                elemsize,
                key_ptr.cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        unsafe {
            set_string_value(c_map, 100_000 + index as i64);
            set_string_value(rust_map, 100_000 + index as i64);
        }
    }

    for index in 0..120 {
        let lookup = if index < 96 {
            CString::new(owned_keys[index].to_bytes()).unwrap()
        } else {
            CString::new(format!("absent_{mode}_{index}")).unwrap()
        };
        let key_ptr = lookup.as_ptr().cast_mut();
        c_map = unsafe { (c.hmget)(c_map, elemsize, key_ptr.cast(), size_of::<*mut c_char>(), 1) };
        rust_map = unsafe {
            (rust.hmget)(
                rust_map,
                elemsize,
                key_ptr.cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        assert_same(
            &format!("string lookup mode={mode} index={index}"),
            unsafe { string_snapshot(c_map) },
            unsafe { string_snapshot(rust_map) },
        );
    }

    for index in (0..96).step_by(2) {
        let key_ptr = owned_keys[index].as_ptr().cast_mut();
        c_map = unsafe {
            (c.hmdel)(
                c_map,
                elemsize,
                key_ptr.cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            )
        };
        rust_map = unsafe {
            (rust.hmdel)(
                rust_map,
                elemsize,
                key_ptr.cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            )
        };
        assert_same(
            &format!("string delete mode={mode} index={index}"),
            unsafe { string_snapshot(c_map) },
            unsafe { string_snapshot(rust_map) },
        );
    }

    let missing = CString::new("definitely_missing").unwrap();
    c_map = unsafe {
        (c.hmdel)(
            c_map,
            elemsize,
            missing.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            1,
        )
    };
    rust_map = unsafe {
        (rust.hmdel)(
            rust_map,
            elemsize,
            missing.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            1,
        )
    };
    assert_same(
        "absent string delete",
        unsafe { string_snapshot(c_map) },
        unsafe { string_snapshot(rust_map) },
    );
    unsafe {
        free_string(c, c_map);
        free_string(rust, rust_map);
    }
}

unsafe fn verify_string_maps(c: &Api, rust: &Api) {
    // CONFIGS 2, 28-33.
    for (mode, seed) in [(1, 0x1111), (2, 0x2222), (3, 0x3333)] {
        unsafe { verify_string_mode(c, rust, mode, seed) };
    }

    let elemsize = size_of::<StringEntry>();
    let key = CString::new("implicit_default_mode").unwrap();
    unsafe {
        (c.rand_seed)(0x8765_4321);
        (rust.rand_seed)(0x8765_4321);
    }
    let c_map = unsafe {
        (c.hmput)(
            ptr::null_mut(),
            elemsize,
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            1,
        )
    };
    let rust_map = unsafe {
        (rust.hmput)(
            ptr::null_mut(),
            elemsize,
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            1,
        )
    };
    unsafe {
        set_string_value(c_map, 77);
        set_string_value(rust_map, 77);
    }
    assert_same(
        "implicit string default mode",
        unsafe { string_snapshot(c_map) },
        unsafe { string_snapshot(rust_map) },
    );
    unsafe {
        free_string(c, c_map);
        free_string(rust, rust_map);
    }

    // ERRORS 16: mode -1 is accepted and follows the binary branch.
    let mut binary_key = 0xaabb_ccdd_eeff_0011u64;
    let binary_size = size_of::<BinaryEntry>();
    let c_binary = unsafe {
        (c.hmput)(
            ptr::null_mut(),
            binary_size,
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        )
    };
    let rust_binary = unsafe {
        (rust.hmput)(
            ptr::null_mut(),
            binary_size,
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        )
    };
    assert_same(
        "mode -1 binary handling",
        unsafe { binary_snapshot(c_binary) },
        unsafe { binary_snapshot(rust_binary) },
    );
    unsafe {
        free_binary(c, c_binary);
        free_binary(rust, rust_binary);
    }

    // ERRORS 17: mode 4 is retained by shmode and takes the default copy arm.
    let mut raw_key = *b"mode4\0!!";
    unsafe {
        (c.rand_seed)(0x4444);
        (rust.rand_seed)(0x4444);
    }
    let mut c_mode4 = unsafe { (c.shmode)(binary_size, 4) };
    let mut rust_mode4 = unsafe { (rust.shmode)(binary_size, 4) };
    c_mode4 = unsafe {
        (c.hmput)(
            c_mode4,
            binary_size,
            raw_key.as_mut_ptr().cast(),
            raw_key.len(),
            4,
        )
    };
    rust_mode4 = unsafe {
        (rust.hmput)(
            rust_mode4,
            binary_size,
            raw_key.as_mut_ptr().cast(),
            raw_key.len(),
            4,
        )
    };
    let c_raw = unsafe { raw_map(c_mode4, binary_size) };
    let rust_raw = unsafe { raw_map(rust_mode4, binary_size) };
    let c_bytes = unsafe { std::slice::from_raw_parts(c_mode4.cast::<u8>(), binary_size) };
    let rust_bytes = unsafe { std::slice::from_raw_parts(rust_mode4.cast::<u8>(), binary_size) };
    assert_eq!(c_bytes, rust_bytes);
    assert_same("mode 4 table", unsafe { table_snapshot(c_raw) }, unsafe {
        table_snapshot(rust_raw)
    });
    unsafe {
        (c.hmfree)(c_raw, binary_size);
        (rust.hmfree)(rust_raw, binary_size);
    }
}

unsafe fn arena_snapshot(arena: &StringArena) -> (bool, usize, u8, u8) {
    (
        !arena.storage.is_null(),
        arena.remaining,
        arena.block,
        arena.mode,
    )
}

unsafe fn verify_arenas(c: &Api, rust: &Api) {
    // CONFIGS 35-38; ERRORS 13.
    let mut c_empty = StringArena::empty();
    let mut rust_empty = StringArena::empty();
    unsafe {
        (c.strreset)(&mut c_empty);
        (rust.strreset)(&mut rust_empty);
    }
    assert_same(
        "reset empty arena",
        unsafe { arena_snapshot(&c_empty) },
        unsafe { arena_snapshot(&rust_empty) },
    );

    let mut c_arena = StringArena::empty();
    let mut rust_arena = StringArena::empty();
    let mut rng = Rng::new(0x23cd_88ea_51b7_900d);
    for index in 0..160 {
        let len = if index == 0 {
            0
        } else if index == 80 {
            1600
        } else {
            rng.usize(96)
        };
        let mut bytes = Vec::with_capacity(len);
        while bytes.len() < len {
            bytes.push((rng.next_u64() as u8).max(1));
        }
        let string = CString::new(bytes).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, string.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result) }.to_bytes(),
            unsafe { CStr::from_ptr(rust_result) }.to_bytes(),
            "arena string index={index}"
        );
        assert_same(
            &format!("arena state index={index}"),
            unsafe { arena_snapshot(&c_arena) },
            unsafe { arena_snapshot(&rust_arena) },
        );
    }

    let exact_chars = c_arena.remaining.saturating_sub(1);
    if exact_chars > 0 {
        let exact = CString::new(vec![b'x'; exact_chars]).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, exact.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, exact.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result) }.to_bytes(),
            unsafe { CStr::from_ptr(rust_result) }.to_bytes()
        );
        assert_same(
            "arena exact remaining boundary",
            unsafe { arena_snapshot(&c_arena) },
            unsafe { arena_snapshot(&rust_arena) },
        );
    }

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
    assert_same(
        "reset populated arena",
        unsafe { arena_snapshot(&c_arena) },
        unsafe { arena_snapshot(&rust_arena) },
    );
    assert_eq!(unsafe { arena_snapshot(&c_arena) }, (false, 0, 0, 0));
}

unsafe fn verify_misc(c: &Api, rust: &Api) {
    // CONFIGS 39-40; ERRORS 14-15.
    let values = [
        c_int::MIN,
        c_int::MIN + 1,
        -100_000,
        -1,
        0,
        1,
        100_000,
        c_int::MAX - 1,
        c_int::MAX,
    ];
    for value in values {
        let c_text = unsafe { CStr::from_ptr((c.strkey)(value)) }
            .to_bytes()
            .to_vec();
        let rust_text = unsafe { CStr::from_ptr((rust.strkey)(value)) }
            .to_bytes()
            .to_vec();
        assert_same("strkey", c_text, rust_text);
        unsafe {
            (c.arr_ins)(value);
            (rust.arr_ins)(value);
        }
    }
}

#[test]
fn differential_all_configurations_and_errors() {
    let (c_path, rust_path) = library_paths();
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    unsafe {
        let c = Api::load(c_path);
        let rust = Api::load(rust_path);
        verify_hashes(&c, &rust);
        verify_arrays(&c, &rust);
        verify_default_and_sentinel_paths(&c, &rust);
        verify_binary_maps(&c, &rust);
        verify_string_maps(&c, &rust);
        verify_arenas(&c, &rust);
        verify_misc(&c, &rust);
    }
}
