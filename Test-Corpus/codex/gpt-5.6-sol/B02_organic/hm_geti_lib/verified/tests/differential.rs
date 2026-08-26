use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fmt::Debug;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Mutex, MutexGuard};

static FFI_TEST_LOCK: Mutex<()> = Mutex::new(());

type Arrgrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type Arrfree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type HmGeti = unsafe extern "C" fn(c_int);

#[derive(Clone, Copy)]
struct Api {
    arrgrow: Arrgrow,
    arrfree: Arrfree,
    rand_seed: RandSeed,
    hash_string: HashString,
    hash_bytes: HashBytes,
    hmfree: HmFree,
    hmget_ts: HmGetTs,
    hmget: HmGet,
    hmdefault: HmDefault,
    hmput: HmPut,
    shmode: ShMode,
    hmdel: HmDel,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    hm_geti: HmGeti,
}

impl Api {
    unsafe fn load(library: &Library) -> Self {
        unsafe fn get<T: Copy>(library: &Library, name: &[u8]) -> T {
            unsafe { *library.get::<T>(name).unwrap() }
        }

        unsafe {
            Self {
                arrgrow: get(library, b"stbds_arrgrowf\0"),
                arrfree: get(library, b"stbds_arrfreef\0"),
                rand_seed: get(library, b"stbds_rand_seed\0"),
                hash_string: get(library, b"stbds_hash_string\0"),
                hash_bytes: get(library, b"stbds_hash_bytes\0"),
                hmfree: get(library, b"stbds_hmfree_func\0"),
                hmget_ts: get(library, b"stbds_hmget_key_ts\0"),
                hmget: get(library, b"stbds_hmget_key\0"),
                hmdefault: get(library, b"stbds_hmput_default\0"),
                hmput: get(library, b"stbds_hmput_key\0"),
                shmode: get(library, b"stbds_shmode_func\0"),
                hmdel: get(library, b"stbds_hmdel_key\0"),
                stralloc: get(library, b"stbds_stralloc\0"),
                strreset: get(library, b"stbds_strreset\0"),
                strkey: get(library, b"strkey\0"),
                hm_geti: get(library, b"hm_geti\0"),
            }
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
    _guard: MutexGuard<'static, ()>,
    _c_library: Library,
    _rust_library: Library,
}

impl Libraries {
    fn load() -> Self {
        let guard = FFI_TEST_LOCK.lock().unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(root);
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(c_path).unwrap();
            let rust_library = Library::new(rust_path).unwrap();
            let c = Api::load(&c_library);
            let rust = Api::load(&rust_library);
            Self {
                c,
                rust,
                _guard: guard,
                _c_library: c_library,
                _rust_library: rust_library,
            }
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let direct = root.join("target").join(profile).join("libhm_geti_lib.so");
    if direct.is_file() {
        return direct;
    }

    let deps = root.join("target").join(profile).join("deps");
    let mut candidates: Vec<_> = std::fs::read_dir(&deps)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libhm_geti_lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    candidates.pop().unwrap_or(direct)
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
struct StringBlock {
    next: *mut StringBlock,
    storage: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Default for StringArena {
    fn default() -> Self {
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

#[derive(Debug, PartialEq, Eq)]
struct HeaderSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    has_table: bool,
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
    arena_remaining: usize,
    arena_block: u8,
    string_mode: u8,
    buckets: Vec<([usize; 8], [isize; 8])>,
}

#[derive(Debug, PartialEq, Eq)]
struct BinaryMapSnapshot {
    header: HeaderSnapshot,
    table: Option<TableSnapshot>,
    bytes: Vec<u8>,
}

unsafe fn array_header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { (array as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader }
}

unsafe fn map_raw(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(element_size) as *mut c_void }
}

unsafe fn header_snapshot(raw: *mut c_void) -> HeaderSnapshot {
    let header = unsafe { &*array_header(raw) };
    HeaderSnapshot {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        has_table: !header.hash_table.is_null(),
    }
}

unsafe fn table_snapshot(raw: *mut c_void) -> Option<TableSnapshot> {
    let table = unsafe { (*array_header(raw)).hash_table as *mut HashIndex };
    if table.is_null() {
        return None;
    }
    let table = unsafe { &*table };
    let mut buckets = Vec::with_capacity(table.slot_count / 8);
    for index in 0..table.slot_count / 8 {
        let bucket = unsafe { &*table.storage.add(index) };
        buckets.push((bucket.hash, bucket.index));
    }
    Some(TableSnapshot {
        slot_count: table.slot_count,
        used_count: table.used_count,
        used_count_threshold: table.used_count_threshold,
        used_count_shrink_threshold: table.used_count_shrink_threshold,
        tombstone_count: table.tombstone_count,
        tombstone_count_threshold: table.tombstone_count_threshold,
        seed: table.seed,
        slot_count_log2: table.slot_count_log2,
        arena_remaining: table.string.remaining,
        arena_block: table.string.block,
        string_mode: table.string.mode,
        buckets,
    })
}

unsafe fn binary_map_snapshot(map: *mut c_void, element_size: usize) -> BinaryMapSnapshot {
    let raw = unsafe { map_raw(map, element_size) };
    let header = unsafe { header_snapshot(raw) };
    let byte_len = header.length * element_size;
    let bytes = unsafe { std::slice::from_raw_parts(raw as *const u8, byte_len) }.to_vec();
    let table = unsafe { table_snapshot(raw) };
    BinaryMapSnapshot {
        header,
        table,
        bytes,
    }
}

fn assert_same<T: PartialEq + Debug>(left: T, right: T, context: &str) {
    assert_eq!(left, right, "{context}");
}

#[derive(Clone, Copy)]
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

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

unsafe fn reset_seeds(libraries: &Libraries, seed: usize) {
    unsafe {
        (libraries.c.rand_seed)(seed);
        (libraries.rust.rand_seed)(seed);
    }
}

unsafe fn free_map(api: Api, map: *mut c_void, element_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(map_raw(map, element_size), element_size) };
    }
}

unsafe fn map_temp(map: *mut c_void, element_size: usize) -> isize {
    unsafe { (*array_header(map_raw(map, element_size))).temp }
}

unsafe fn put_binary(
    api: Api,
    map: &mut *mut c_void,
    element_size: usize,
    key: &mut [u8],
    mode: c_int,
    marker: u8,
) -> isize {
    *map = unsafe {
        (api.hmput)(
            *map,
            element_size,
            key.as_mut_ptr() as *mut c_void,
            key.len(),
            mode,
        )
    };
    let index = unsafe { map_temp(*map, element_size) };
    let entry = unsafe {
        std::slice::from_raw_parts_mut(
            (*map as *mut u8).add(index as usize * element_size),
            element_size,
        )
    };
    entry.fill(marker);
    entry[..key.len()].copy_from_slice(key);
    index
}

unsafe fn get_binary(
    api: Api,
    map: &mut *mut c_void,
    element_size: usize,
    key: &mut [u8],
    mode: c_int,
) -> isize {
    *map = unsafe {
        (api.hmget)(
            *map,
            element_size,
            key.as_mut_ptr() as *mut c_void,
            key.len(),
            mode,
        )
    };
    unsafe { map_temp(*map, element_size) }
}

#[test]
fn hashes_match_for_all_length_partitions_and_random_values() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x5eed_1234_d00d_beef);
    let seeds = [0, 1, 0x3141_5926, usize::MAX, 0xa5a5_5a5a_1234_5678];

    for &seed in &seeds {
        let mut empty = [0u8];
        let c = unsafe { (libraries.c.hash_string)(empty.as_mut_ptr().cast(), seed) };
        let rust = unsafe { (libraries.rust.hash_string)(empty.as_mut_ptr().cast(), seed) };
        assert_eq!(c, rust, "empty string, seed {seed:#x}");

        for length in 1..=96 {
            for sample in 0..32 {
                let mut string = vec![0u8; length + 1];
                rng.fill(&mut string[..length]);
                for byte in &mut string[..length] {
                    if *byte == 0 {
                        *byte = 0x80;
                    }
                }
                let c = unsafe { (libraries.c.hash_string)(string.as_mut_ptr().cast(), seed) };
                let rust =
                    unsafe { (libraries.rust.hash_string)(string.as_mut_ptr().cast(), seed) };
                assert_eq!(c, rust, "string length {length}, sample {sample}");
            }
        }
    }

    for full_words in 0..=5 {
        for remainder in 0..=7 {
            let length = full_words * size_of::<usize>() + remainder;
            for sample in 0..128 {
                let mut bytes = vec![0u8; length.max(1)];
                rng.fill(&mut bytes);
                let seed = rng.next_u64() as usize;
                let pointer = if length == 0 && sample == 0 {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                let c = unsafe { (libraries.c.hash_bytes)(pointer, length, seed) };
                let rust = unsafe { (libraries.rust.hash_bytes)(pointer, length, seed) };
                assert_eq!(
                    c, rust,
                    "byte hash full_words={full_words}, remainder={remainder}, sample={sample}"
                );
            }
        }
    }

    for length in [255usize, 256, 257, 4096] {
        let mut bytes = vec![0u8; length];
        rng.fill(&mut bytes);
        let seed = rng.next_u64() as usize;
        let c = unsafe { (libraries.c.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed) };
        let rust = unsafe { (libraries.rust.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed) };
        assert_eq!(c, rust, "oversized valid byte buffer length {length}");
    }
}

#[test]
fn array_growth_branches_and_free_match() {
    let libraries = Libraries::load();

    unsafe fn snapshot(array: *mut c_void, initialized: usize) -> (HeaderSnapshot, Vec<u8>) {
        (
            unsafe { header_snapshot(array) },
            unsafe { std::slice::from_raw_parts(array as *const u8, initialized) }.to_vec(),
        )
    }

    unsafe {
        let mut c = (libraries.c.arrgrow)(ptr::null_mut(), 4, 0, 0);
        let mut rust = (libraries.rust.arrgrow)(ptr::null_mut(), 4, 0, 0);
        assert!(c.is_null());
        assert_eq!(c, rust);

        c = (libraries.c.arrgrow)(ptr::null_mut(), 3, 1, 0);
        rust = (libraries.rust.arrgrow)(ptr::null_mut(), 3, 1, 0);
        assert_same(snapshot(c, 0), snapshot(rust, 0), "minimum capacity");
        assert_eq!((*array_header(c)).capacity, 4);
        (libraries.c.arrfree)(c);
        (libraries.rust.arrfree)(rust);

        c = (libraries.c.arrgrow)(ptr::null_mut(), 3, 7, 2);
        rust = (libraries.rust.arrgrow)(ptr::null_mut(), 3, 7, 2);
        assert_same(snapshot(c, 0), snapshot(rust, 0), "addlen controls");
        assert_eq!((*array_header(c)).capacity, 7);
        (libraries.c.arrfree)(c);
        (libraries.rust.arrfree)(rust);

        for element_size in [1usize, 4, 17] {
            c = (libraries.c.arrgrow)(ptr::null_mut(), element_size, 0, 11);
            rust = (libraries.rust.arrgrow)(ptr::null_mut(), element_size, 0, 11);
            assert_same(
                snapshot(c, 0),
                snapshot(rust, 0),
                "explicit minimum capacity",
            );
            assert_eq!((*array_header(c)).capacity, 11);
            (libraries.c.arrfree)(c);
            (libraries.rust.arrfree)(rust);
        }

        c = (libraries.c.arrgrow)(ptr::null_mut(), 4, 0, 8);
        rust = (libraries.rust.arrgrow)(ptr::null_mut(), 4, 0, 8);
        (*array_header(c)).length = 4;
        (*array_header(rust)).length = 4;
        for index in 0..16 {
            *(c as *mut u8).add(index) = index as u8;
            *(rust as *mut u8).add(index) = index as u8;
        }
        let old_c = c;
        let old_rust = rust;
        c = (libraries.c.arrgrow)(c, 4, 1, 0);
        rust = (libraries.rust.arrgrow)(rust, 4, 1, 0);
        assert_eq!(c, old_c, "C early return must retain pointer");
        assert_eq!(rust, old_rust, "Rust early return must retain pointer");
        assert_same(snapshot(c, 16), snapshot(rust, 16), "available capacity");

        (*array_header(c)).length = 8;
        (*array_header(rust)).length = 8;
        c = (libraries.c.arrgrow)(c, 4, 1, 0);
        rust = (libraries.rust.arrgrow)(rust, 4, 1, 0);
        assert_eq!((*array_header(c)).capacity, 16);
        assert_same(snapshot(c, 16), snapshot(rust, 16), "doubling growth");
        (libraries.c.arrfree)(c);
        (libraries.rust.arrfree)(rust);

        c = (libraries.c.arrgrow)(ptr::null_mut(), 8, 0, 4);
        rust = (libraries.rust.arrgrow)(ptr::null_mut(), 8, 0, 4);
        c = (libraries.c.arrgrow)(c, 8, 0, 25);
        rust = (libraries.rust.arrgrow)(rust, 8, 0, 25);
        assert_eq!((*array_header(c)).capacity, 25);
        assert_same(snapshot(c, 0), snapshot(rust, 0), "requested growth");
        (libraries.c.arrfree)(c);
        (libraries.rust.arrfree)(rust);
    }
}

#[test]
fn null_default_and_tableless_map_paths_match() {
    let libraries = Libraries::load();
    const ELEMENT_SIZE: usize = 16;

    unsafe {
        (libraries.c.hmfree)(ptr::null_mut(), ELEMENT_SIZE);
        (libraries.rust.hmfree)(ptr::null_mut(), ELEMENT_SIZE);
        assert!(
            (libraries.c.hmdel)(ptr::null_mut(), ELEMENT_SIZE, ptr::null_mut(), 0, 0, 0).is_null()
        );
        assert!(
            (libraries.rust.hmdel)(ptr::null_mut(), ELEMENT_SIZE, ptr::null_mut(), 0, 0, 0)
                .is_null()
        );

        let mut c_temp = 99;
        let mut rust_temp = 99;
        let mut c_map = (libraries.c.hmget_ts)(
            ptr::null_mut(),
            ELEMENT_SIZE,
            ptr::null_mut(),
            0,
            &mut c_temp,
            0,
        );
        let mut rust_map = (libraries.rust.hmget_ts)(
            ptr::null_mut(),
            ELEMENT_SIZE,
            ptr::null_mut(),
            0,
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "null get_ts",
        );

        let mut key = [7u8; 4];
        c_map = (libraries.c.hmget)(c_map, ELEMENT_SIZE, key.as_mut_ptr().cast(), 4, 0);
        rust_map = (libraries.rust.hmget)(rust_map, ELEMENT_SIZE, key.as_mut_ptr().cast(), 4, 0);
        assert_eq!(map_temp(c_map, ELEMENT_SIZE), -1);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "tableless get",
        );
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);

        c_map = (libraries.c.hmget)(ptr::null_mut(), ELEMENT_SIZE, ptr::null_mut(), 0, 0);
        rust_map = (libraries.rust.hmget)(ptr::null_mut(), ELEMENT_SIZE, ptr::null_mut(), 0, 0);
        assert_eq!(map_temp(c_map, ELEMENT_SIZE), -1);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "direct null hmget",
        );
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);

        c_map = (libraries.c.hmdefault)(ptr::null_mut(), ELEMENT_SIZE);
        rust_map = (libraries.rust.hmdefault)(ptr::null_mut(), ELEMENT_SIZE);
        let before = binary_map_snapshot(c_map, ELEMENT_SIZE);
        c_map = (libraries.c.hmdefault)(c_map, ELEMENT_SIZE);
        rust_map = (libraries.rust.hmdefault)(rust_map, ELEMENT_SIZE);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "default map",
        );
        assert_eq!(before, binary_map_snapshot(c_map, ELEMENT_SIZE));
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);
    }
}

#[test]
fn binary_maps_match_across_key_widths_modes_and_random_operations() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x1234_5678_9abc_def0);

    for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
        for &key_size in &[0usize, 1, 4, 8, 16] {
            for &mode in &[0, -1, c_int::MIN] {
                let element_size = (key_size + 8).max(8);
                let mut c_map = ptr::null_mut();
                let mut rust_map = ptr::null_mut();
                unsafe { reset_seeds(&libraries, seed) };

                for operation in 0..160 {
                    let mut key = vec![0u8; key_size];
                    rng.fill(&mut key);
                    if key_size >= 4 {
                        key[..4].copy_from_slice(&((operation % 37) as u32).to_ne_bytes());
                    } else if key_size > 0 {
                        key[0] = (operation % 19) as u8;
                    }

                    if operation % 3 != 2 {
                        let marker = rng.next_u64() as u8;
                        let c_index = unsafe {
                            put_binary(
                                libraries.c,
                                &mut c_map,
                                element_size,
                                &mut key,
                                mode,
                                marker,
                            )
                        };
                        let rust_index = unsafe {
                            put_binary(
                                libraries.rust,
                                &mut rust_map,
                                element_size,
                                &mut key,
                                mode,
                                marker,
                            )
                        };
                        assert_eq!(c_index, rust_index, "put index");
                    } else {
                        let c_index = unsafe {
                            get_binary(libraries.c, &mut c_map, element_size, &mut key, mode)
                        };
                        let rust_index = unsafe {
                            get_binary(libraries.rust, &mut rust_map, element_size, &mut key, mode)
                        };
                        assert_eq!(c_index, rust_index, "get index");
                    }

                    assert_same(
                        unsafe { binary_map_snapshot(c_map, element_size) },
                        unsafe { binary_map_snapshot(rust_map, element_size) },
                        &format!(
                            "binary map seed={seed:#x} key_size={key_size} mode={mode} op={operation}"
                        ),
                    );
                }

                unsafe {
                    free_map(libraries.c, c_map, element_size);
                    free_map(libraries.rust, rust_map, element_size);
                }
            }
        }
    }
}

#[test]
fn binary_probe_delete_rebuild_and_shrink_paths_match() {
    let libraries = Libraries::load();
    const ELEMENT_SIZE: usize = 16;
    const KEY_SIZE: usize = 8;

    unsafe {
        reset_seeds(&libraries, 0x9876_5432);
        let mut colliding_keys = Vec::<[u8; KEY_SIZE]>::new();
        let mut candidate = 0u64;
        while colliding_keys.len() < 3 {
            let mut key = candidate.to_ne_bytes();
            let hash = (libraries.c.hash_bytes)(key.as_mut_ptr().cast(), KEY_SIZE, 0x9876_5432);
            if hash & 7 == 6 {
                colliding_keys.push(key);
            }
            candidate += 1;
        }

        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        for (index, key) in colliding_keys[..2].iter_mut().enumerate() {
            put_binary(libraries.c, &mut c_map, ELEMENT_SIZE, key, 0, index as u8);
            put_binary(
                libraries.rust,
                &mut rust_map,
                ELEMENT_SIZE,
                key,
                0,
                index as u8,
            );
        }
        let c_missing = get_binary(
            libraries.c,
            &mut c_map,
            ELEMENT_SIZE,
            &mut colliding_keys[2],
            0,
        );
        let rust_missing = get_binary(
            libraries.rust,
            &mut rust_map,
            ELEMENT_SIZE,
            &mut colliding_keys[2],
            0,
        );
        assert_eq!(c_missing, -1);
        assert_eq!(c_missing, rust_missing, "wrapped missing probe");
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "wrapped probe state",
        );

        let mut first_candidate = 0u64;
        let mut first_span_key = loop {
            let mut candidate_key = first_candidate.to_ne_bytes();
            let hash =
                (libraries.c.hash_bytes)(candidate_key.as_mut_ptr().cast(), KEY_SIZE, 0x9876_5432);
            if hash & 7 == 0 {
                break candidate_key;
            }
            first_candidate += 1;
        };
        let c_first_missing = get_binary(
            libraries.c,
            &mut c_map,
            ELEMENT_SIZE,
            &mut first_span_key,
            0,
        );
        let rust_first_missing = get_binary(
            libraries.rust,
            &mut rust_map,
            ELEMENT_SIZE,
            &mut first_span_key,
            0,
        );
        assert_eq!(c_first_missing, -1);
        assert_eq!(
            c_first_missing, rust_first_missing,
            "first-span missing probe"
        );
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);

        reset_seeds(&libraries, 0x1357_2468);
        c_map = ptr::null_mut();
        rust_map = ptr::null_mut();
        let mut keys: Vec<[u8; KEY_SIZE]> = (0..96u64).map(u64::to_ne_bytes).collect();
        for (index, key) in keys.iter_mut().enumerate() {
            put_binary(libraries.c, &mut c_map, ELEMENT_SIZE, key, 0, index as u8);
            put_binary(
                libraries.rust,
                &mut rust_map,
                ELEMENT_SIZE,
                key,
                0,
                index as u8,
            );
            assert_same(
                binary_map_snapshot(c_map, ELEMENT_SIZE),
                binary_map_snapshot(rust_map, ELEMENT_SIZE),
                "growth threshold",
            );
        }

        let grown_slots = table_snapshot(map_raw(c_map, ELEMENT_SIZE))
            .unwrap()
            .slot_count;
        assert!(grown_slots > 8);

        for &index in &[20usize, 95, 0, 50, 2, 70] {
            c_map = (libraries.c.hmdel)(
                c_map,
                ELEMENT_SIZE,
                keys[index].as_mut_ptr().cast(),
                KEY_SIZE,
                0,
                0,
            );
            rust_map = (libraries.rust.hmdel)(
                rust_map,
                ELEMENT_SIZE,
                keys[index].as_mut_ptr().cast(),
                KEY_SIZE,
                0,
                0,
            );
            assert_same(
                binary_map_snapshot(c_map, ELEMENT_SIZE),
                binary_map_snapshot(rust_map, ELEMENT_SIZE),
                "final/non-final deletion",
            );
        }

        let mut missing = 10_000u64.to_ne_bytes();
        c_map = (libraries.c.hmdel)(
            c_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            0,
            0,
        );
        rust_map = (libraries.rust.hmdel)(
            rust_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            0,
            0,
        );
        assert_eq!(map_temp(c_map, ELEMENT_SIZE), 0);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "missing deletion",
        );

        for key in keys.iter_mut().skip(1) {
            c_map =
                (libraries.c.hmdel)(c_map, ELEMENT_SIZE, key.as_mut_ptr().cast(), KEY_SIZE, 0, 0);
            rust_map = (libraries.rust.hmdel)(
                rust_map,
                ELEMENT_SIZE,
                key.as_mut_ptr().cast(),
                KEY_SIZE,
                0,
                0,
            );
            assert_same(
                binary_map_snapshot(c_map, ELEMENT_SIZE),
                binary_map_snapshot(rust_map, ELEMENT_SIZE),
                "delete/rebuild/shrink",
            );
        }
        let final_slots = table_snapshot(map_raw(c_map, ELEMENT_SIZE))
            .unwrap()
            .slot_count;
        assert!(final_slots < grown_slots, "shrink branch was not reached");
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);
    }
}

#[test]
fn deletion_with_nonzero_key_offset_matches() {
    let libraries = Libraries::load();
    const ELEMENT_SIZE: usize = 16;
    const KEY_SIZE: usize = 4;
    const KEY_OFFSET: usize = 8;

    unsafe {
        reset_seeds(&libraries, 77);
        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        let mut keys: Vec<[u8; KEY_SIZE]> = (0..12u32).map(|value| value.to_ne_bytes()).collect();

        for (index, key) in keys.iter_mut().enumerate() {
            let c_index = put_binary(libraries.c, &mut c_map, ELEMENT_SIZE, key, 0, index as u8);
            let rust_index = put_binary(
                libraries.rust,
                &mut rust_map,
                ELEMENT_SIZE,
                key,
                0,
                index as u8,
            );
            ptr::copy_nonoverlapping(
                key.as_ptr(),
                (c_map as *mut u8).add(c_index as usize * ELEMENT_SIZE + KEY_OFFSET),
                KEY_SIZE,
            );
            ptr::copy_nonoverlapping(
                key.as_ptr(),
                (rust_map as *mut u8).add(rust_index as usize * ELEMENT_SIZE + KEY_OFFSET),
                KEY_SIZE,
            );
        }

        let key = &mut keys[3];
        c_map = (libraries.c.hmdel)(
            c_map,
            ELEMENT_SIZE,
            key.as_mut_ptr().cast(),
            KEY_SIZE,
            KEY_OFFSET,
            0,
        );
        rust_map = (libraries.rust.hmdel)(
            rust_map,
            ELEMENT_SIZE,
            key.as_mut_ptr().cast(),
            KEY_SIZE,
            KEY_OFFSET,
            0,
        );
        assert_eq!(map_temp(c_map, ELEMENT_SIZE), 1);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "nonzero key offset",
        );
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);
    }
}

#[repr(C)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct StringMapSnapshot {
    header: HeaderSnapshot,
    table: TableSnapshot,
    default_value: i64,
    entries: Vec<(Vec<u8>, i64)>,
}

unsafe fn string_map_snapshot(map: *mut c_void) -> StringMapSnapshot {
    let element_size = size_of::<StringEntry>();
    let raw = unsafe { map_raw(map, element_size) };
    let header = unsafe { header_snapshot(raw) };
    let default_value = unsafe { (*(raw as *const StringEntry)).value };
    let mut entries = Vec::with_capacity(header.length.saturating_sub(1));
    for index in 0..header.length.saturating_sub(1) {
        let entry = unsafe { &*((map as *const StringEntry).add(index)) };
        let key = unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec();
        entries.push((key, entry.value));
    }
    StringMapSnapshot {
        header,
        table: unsafe { table_snapshot(raw) }.unwrap(),
        default_value,
        entries,
    }
}

unsafe fn put_string(
    api: Api,
    map: &mut *mut c_void,
    key: &CString,
    mode: c_int,
    value: i64,
) -> isize {
    let element_size = size_of::<StringEntry>();
    *map = unsafe {
        (api.hmput)(
            *map,
            element_size,
            key.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { map_temp(*map, element_size) };
    unsafe {
        (*(map.cast::<StringEntry>().add(index as usize))).value = value;
    }
    index
}

unsafe fn get_string(api: Api, map: &mut *mut c_void, key: &CString, mode: c_int) -> isize {
    let element_size = size_of::<StringEntry>();
    *map = unsafe {
        (api.hmget)(
            *map,
            element_size,
            key.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            mode,
        )
    };
    unsafe { map_temp(*map, element_size) }
}

#[test]
fn string_map_ownership_growth_update_and_delete_paths_match() {
    let libraries = Libraries::load();
    let element_size = size_of::<StringEntry>();

    for &ownership_mode in &[1, 2, 3] {
        unsafe { reset_seeds(&libraries, 0x4242_1010 + ownership_mode as usize) };
        let mut c_map = unsafe { (libraries.c.shmode)(element_size, ownership_mode) };
        let mut rust_map = unsafe { (libraries.rust.shmode)(element_size, ownership_mode) };
        let mut storage = Vec::new();

        for operation in 0..120 {
            let key_number = operation % 73;
            let key = CString::new(format!(
                "key_{key_number:03}_{}",
                "x".repeat(operation % 19)
            ))
            .unwrap();
            storage.push(key);
            let key = storage.last().unwrap();
            let c_index =
                unsafe { put_string(libraries.c, &mut c_map, key, 1, operation as i64 * 17) };
            let rust_index =
                unsafe { put_string(libraries.rust, &mut rust_map, key, 1, operation as i64 * 17) };
            assert_eq!(c_index, rust_index, "string put index");
            assert_same(
                unsafe { string_map_snapshot(c_map) },
                unsafe { string_map_snapshot(rust_map) },
                &format!("string ownership={ownership_mode}, operation={operation}"),
            );
        }

        let existing = CString::new("key_010_xxxxxxxxxx").unwrap();
        let c_found = unsafe { get_string(libraries.c, &mut c_map, &existing, 1) };
        let rust_found = unsafe { get_string(libraries.rust, &mut rust_map, &existing, 1) };
        assert_eq!(c_found, rust_found);

        let missing = CString::new("not_present").unwrap();
        let c_missing = unsafe { get_string(libraries.c, &mut c_map, &missing, 1) };
        let rust_missing = unsafe { get_string(libraries.rust, &mut rust_map, &missing, 1) };
        assert_eq!(c_missing, -1);
        assert_eq!(c_missing, rust_missing);

        for key in [
            CString::new("key_010_xxxxxxxxxx").unwrap(),
            CString::new("key_072_xxxxxxxxxxxxxxx").unwrap(),
            CString::new("not_present").unwrap(),
        ] {
            c_map = unsafe {
                (libraries.c.hmdel)(
                    c_map,
                    element_size,
                    key.as_ptr() as *mut c_void,
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            rust_map = unsafe {
                (libraries.rust.hmdel)(
                    rust_map,
                    element_size,
                    key.as_ptr() as *mut c_void,
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            assert_same(
                unsafe { string_map_snapshot(c_map) },
                unsafe { string_map_snapshot(rust_map) },
                "string deletion",
            );
        }

        unsafe {
            free_map(libraries.c, c_map, element_size);
            free_map(libraries.rust, rust_map, element_size);
        }
    }
}

#[test]
fn out_of_range_map_modes_follow_c_integer_semantics() {
    let libraries = Libraries::load();
    let element_size = size_of::<StringEntry>();

    for &operation_mode in &[4, c_int::MAX] {
        unsafe { reset_seeds(&libraries, 9000 + operation_mode as usize) };
        let mut c_map = unsafe { (libraries.c.shmode)(element_size, 1) };
        let mut rust_map = unsafe { (libraries.rust.shmode)(element_size, 1) };
        let key = CString::new("invalid_mode_is_still_string").unwrap();
        let c_index = unsafe { put_string(libraries.c, &mut c_map, &key, operation_mode, 123) };
        let rust_index =
            unsafe { put_string(libraries.rust, &mut rust_map, &key, operation_mode, 123) };
        assert_eq!(c_index, rust_index);
        assert_eq!(
            unsafe { get_string(libraries.c, &mut c_map, &key, operation_mode) },
            unsafe { get_string(libraries.rust, &mut rust_map, &key, operation_mode) }
        );
        assert_same(
            unsafe { string_map_snapshot(c_map) },
            unsafe { string_map_snapshot(rust_map) },
            "out-of-range operation mode",
        );
        unsafe {
            free_map(libraries.c, c_map, element_size);
            free_map(libraries.rust, rust_map, element_size);
        }
    }

    for &stored_mode in &[0, 4, 255, c_int::MAX] {
        unsafe { reset_seeds(&libraries, 123) };
        let c_map = unsafe { (libraries.c.shmode)(element_size, stored_mode) };
        let rust_map = unsafe { (libraries.rust.shmode)(element_size, stored_mode) };
        let c_raw = unsafe { map_raw(c_map, element_size) };
        let rust_raw = unsafe { map_raw(rust_map, element_size) };
        assert_same(
            unsafe { header_snapshot(c_raw) },
            unsafe { header_snapshot(rust_raw) },
            "out-of-range shmode header",
        );
        assert_same(
            unsafe { table_snapshot(c_raw) },
            unsafe { table_snapshot(rust_raw) },
            "out-of-range shmode table",
        );
        let expected_mode = stored_mode as u8;
        assert_eq!(
            unsafe { table_snapshot(c_raw) }.unwrap().string_mode,
            expected_mode
        );

        unsafe {
            free_map(libraries.c, c_map, element_size);
            free_map(libraries.rust, rust_map, element_size);
        }
    }

    for &stored_mode in &[4, 255, c_int::MAX] {
        unsafe { reset_seeds(&libraries, 456) };
        let mut c_map = unsafe { (libraries.c.shmode)(element_size, stored_mode) };
        let mut rust_map = unsafe { (libraries.rust.shmode)(element_size, stored_mode) };
        let key = CString::new("0123456789abcdef").unwrap();
        c_map = unsafe {
            (libraries.c.hmput)(
                c_map,
                element_size,
                key.as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                1,
            )
        };
        rust_map = unsafe {
            (libraries.rust.hmput)(
                rust_map,
                element_size,
                key.as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                1,
            )
        };
        assert_same(
            unsafe { binary_map_snapshot(c_map, element_size) },
            unsafe { binary_map_snapshot(rust_map, element_size) },
            "switch default for invalid stored string mode",
        );
        unsafe {
            free_map(libraries.c, c_map, element_size);
            free_map(libraries.rust, rust_map, element_size);
        }
    }
}

fn arena_state(arena: &StringArena) -> (bool, usize, u8, u8) {
    (
        !arena.storage.is_null(),
        arena.remaining,
        arena.block,
        arena.mode,
    )
}

#[test]
fn string_arena_standard_dedicated_growth_cap_and_reset_match() {
    let libraries = Libraries::load();

    unsafe {
        let mut c_empty = StringArena::default();
        let mut rust_empty = StringArena::default();
        (libraries.c.strreset)(&mut c_empty);
        (libraries.rust.strreset)(&mut rust_empty);
        assert_eq!(arena_state(&c_empty), arena_state(&rust_empty));
        assert_eq!(arena_state(&c_empty), (false, 0, 0, 0));

        let mut c_arena = StringArena::default();
        let mut rust_arena = StringArena::default();
        for length in [0usize, 1, 7, 127, 255, 17, 3] {
            let input = CString::new(vec![b'a' + (length % 23) as u8; length]).unwrap();
            let c_pointer = (libraries.c.stralloc)(&mut c_arena, input.as_ptr().cast_mut());
            let rust_pointer =
                (libraries.rust.stralloc)(&mut rust_arena, input.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                CStr::from_ptr(rust_pointer).to_bytes()
            );
            assert_eq!(arena_state(&c_arena), arena_state(&rust_arena));
        }
        (libraries.c.strreset)(&mut c_arena);
        (libraries.rust.strreset)(&mut rust_arena);
        assert_eq!(arena_state(&c_arena), (false, 0, 0, 0));
        assert_eq!(arena_state(&c_arena), arena_state(&rust_arena));

        let mut c_mixed = StringArena::default();
        let mut rust_mixed = StringArena::default();
        for length in [600usize, 10, 1100, 20] {
            let input = CString::new(vec![b'z'; length]).unwrap();
            let c_pointer = (libraries.c.stralloc)(&mut c_mixed, input.as_ptr().cast_mut());
            let rust_pointer =
                (libraries.rust.stralloc)(&mut rust_mixed, input.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                CStr::from_ptr(rust_pointer).to_bytes()
            );
            assert_eq!(arena_state(&c_mixed), arena_state(&rust_mixed));
        }
        (libraries.c.strreset)(&mut c_mixed);
        (libraries.rust.strreset)(&mut rust_mixed);
        assert_eq!(arena_state(&c_mixed), (false, 0, 0, 0));
        assert_eq!(arena_state(&c_mixed), arena_state(&rust_mixed));

        let mut c_capped = StringArena::default();
        let mut rust_capped = StringArena::default();
        for iteration in 0..25 {
            let block_size = 512usize << (c_capped.block >> 1);
            let input = CString::new(vec![b'q'; block_size]).unwrap();
            let c_pointer = (libraries.c.stralloc)(&mut c_capped, input.as_ptr().cast_mut());
            let rust_pointer =
                (libraries.rust.stralloc)(&mut rust_capped, input.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes().len(),
                CStr::from_ptr(rust_pointer).to_bytes().len(),
                "capped arena iteration {iteration}"
            );
            assert_eq!(
                arena_state(&c_capped),
                arena_state(&rust_capped),
                "capped arena iteration {iteration}"
            );
        }
        assert_eq!(c_capped.block, 22, "1 MiB no-increment branch");
        (libraries.c.strreset)(&mut c_capped);
        (libraries.rust.strreset)(&mut rust_capped);
        assert_eq!(arena_state(&c_capped), (false, 0, 0, 0));
        assert_eq!(arena_state(&c_capped), arena_state(&rust_capped));
    }
}

#[test]
fn strkey_matches_for_boundaries_and_random_integers() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xfeed_face_cafe_beef);
    let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    values.extend((0..500).map(|_| rng.next_u64() as c_int));

    for value in values {
        let c = unsafe { CStr::from_ptr((libraries.c.strkey)(value)) }
            .to_bytes()
            .to_vec();
        let rust = unsafe { CStr::from_ptr((libraries.rust.strkey)(value)) }
            .to_bytes()
            .to_vec();
        assert_eq!(c, rust, "strkey({value})");
    }
}

#[test]
fn hm_geti_end_to_end_assertion_surface_matches() {
    let libraries = Libraries::load();
    for value in [-100, -1, 0, 1, 2, 3, 4, 7, 8, 13, 64, 257, 1024] {
        unsafe {
            (libraries.c.hm_geti)(value);
            (libraries.rust.hm_geti)(value);
        }
    }

    let mut rng = Rng::new(0x0ddc_0ffe_e15e_beef);
    for _ in 0..100 {
        let value = (rng.next_u64() % 512) as c_int;
        unsafe {
            (libraries.c.hm_geti)(value);
            (libraries.rust.hm_geti)(value);
        }
    }
}

#[test]
fn low_level_ts_sentinels_delete_rejections_and_zero_length_default_match() {
    let libraries = Libraries::load();
    const ELEMENT_SIZE: usize = 16;
    const KEY_SIZE: usize = 8;

    unsafe {
        reset_seeds(&libraries, 0x1122_3344);
        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        let mut key = 42u64.to_ne_bytes();
        put_binary(libraries.c, &mut c_map, ELEMENT_SIZE, &mut key, 0, 9);
        put_binary(libraries.rust, &mut rust_map, ELEMENT_SIZE, &mut key, 0, 9);

        let mut c_temp = -99;
        let mut rust_temp = -99;
        c_map = (libraries.c.hmget_ts)(
            c_map,
            ELEMENT_SIZE,
            key.as_mut_ptr().cast(),
            KEY_SIZE,
            &mut c_temp,
            0,
        );
        rust_map = (libraries.rust.hmget_ts)(
            rust_map,
            ELEMENT_SIZE,
            key.as_mut_ptr().cast(),
            KEY_SIZE,
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, 0);
        assert_eq!(c_temp, rust_temp);

        let mut missing = 999u64.to_ne_bytes();
        c_map = (libraries.c.hmget_ts)(
            c_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            &mut c_temp,
            0,
        );
        rust_map = (libraries.rust.hmget_ts)(
            rust_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);

        c_map = (libraries.c.hmdel)(
            c_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            0,
            0,
        );
        rust_map = (libraries.rust.hmdel)(
            rust_map,
            ELEMENT_SIZE,
            missing.as_mut_ptr().cast(),
            KEY_SIZE,
            0,
            0,
        );
        assert_eq!(map_temp(c_map, ELEMENT_SIZE), 0);
        assert_same(
            binary_map_snapshot(c_map, ELEMENT_SIZE),
            binary_map_snapshot(rust_map, ELEMENT_SIZE),
            "low-level found/missing sentinels",
        );
        free_map(libraries.c, c_map, ELEMENT_SIZE);
        free_map(libraries.rust, rust_map, ELEMENT_SIZE);

        let mut c_tableless = (libraries.c.hmget_ts)(
            ptr::null_mut(),
            ELEMENT_SIZE,
            ptr::null_mut(),
            0,
            &mut c_temp,
            0,
        );
        let mut rust_tableless = (libraries.rust.hmget_ts)(
            ptr::null_mut(),
            ELEMENT_SIZE,
            ptr::null_mut(),
            0,
            &mut rust_temp,
            0,
        );
        (*array_header(map_raw(c_tableless, ELEMENT_SIZE))).length = 0;
        (*array_header(map_raw(rust_tableless, ELEMENT_SIZE))).length = 0;
        c_tableless = (libraries.c.hmdefault)(c_tableless, ELEMENT_SIZE);
        rust_tableless = (libraries.rust.hmdefault)(rust_tableless, ELEMENT_SIZE);
        assert_same(
            binary_map_snapshot(c_tableless, ELEMENT_SIZE),
            binary_map_snapshot(rust_tableless, ELEMENT_SIZE),
            "zero-length default branch",
        );
        assert_eq!(
            (*array_header(map_raw(c_tableless, ELEMENT_SIZE))).length,
            1
        );
        free_map(libraries.c, c_tableless, ELEMENT_SIZE);
        free_map(libraries.rust, rust_tableless, ELEMENT_SIZE);
    }
}
