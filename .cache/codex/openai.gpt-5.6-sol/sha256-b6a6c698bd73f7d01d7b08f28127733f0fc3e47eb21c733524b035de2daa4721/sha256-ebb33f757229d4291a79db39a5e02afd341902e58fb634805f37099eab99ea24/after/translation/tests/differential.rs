#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::ptr::null_mut;
use std::sync::Mutex;

static PROCESS_IO_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
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
#[derive(Clone, Copy)]
struct BinaryEntry {
    key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: u64,
}

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type Helxo = unsafe extern "C" fn(c_char);

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libharvest-work-9FYxtf.so"),
        root.join("target/release/libhelxo_lib.so"),
    )
}

fn load_libraries() -> (Library, Library) {
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
        (
            Library::new(c_path).expect("load C shared library"),
            Library::new(rust_path).expect("load Rust shared library"),
        )
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let value = self.next_u64().to_ne_bytes();
            chunk.copy_from_slice(&value[..chunk.len()]);
        }
    }
}

unsafe fn array_header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<u8>().sub(size_of::<ArrayHeader>()).cast()
}

unsafe fn raw_from_hash(hash: *mut c_void, element_size: usize) -> *mut c_void {
    hash.cast::<u8>().sub(element_size).cast()
}

unsafe fn hash_header(hash: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    array_header(raw_from_hash(hash, element_size))
}

unsafe fn hash_index(hash: *mut c_void, element_size: usize) -> *mut HashIndex {
    (*hash_header(hash, element_size)).hash_table.cast()
}

#[derive(Debug, PartialEq, Eq)]
struct ArraySnapshot {
    length: usize,
    capacity: usize,
    bytes: Vec<u8>,
}

unsafe fn array_snapshot(array: *mut c_void, element_size: usize) -> ArraySnapshot {
    let header = &*array_header(array);
    let bytes = std::slice::from_raw_parts(
        array.cast::<u8>(),
        header.length.saturating_mul(element_size),
    )
    .to_vec();
    ArraySnapshot {
        length: header.length,
        capacity: header.capacity,
        bytes,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BinaryMapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    slot_count: usize,
    used_count: usize,
    tombstones: usize,
    seed: usize,
    entries: Vec<(u64, u64)>,
}

unsafe fn binary_map_snapshot(hash: *mut c_void) -> BinaryMapSnapshot {
    let element_size = size_of::<BinaryEntry>();
    let header = &*hash_header(hash, element_size);
    let table = header.hash_table.cast::<HashIndex>();
    let count = header.length.saturating_sub(1);
    let entries = std::slice::from_raw_parts(hash.cast::<BinaryEntry>(), count)
        .iter()
        .map(|entry| (entry.key, entry.value))
        .collect();
    BinaryMapSnapshot {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        slot_count: if table.is_null() {
            0
        } else {
            (*table).slot_count
        },
        used_count: if table.is_null() {
            0
        } else {
            (*table).used_count
        },
        tombstones: if table.is_null() {
            0
        } else {
            (*table).tombstone_count
        },
        seed: if table.is_null() { 0 } else { (*table).seed },
        entries,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StringMapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    slot_count: usize,
    used_count: usize,
    tombstones: usize,
    seed: usize,
    string_mode: u8,
    entries: Vec<(Vec<u8>, u64)>,
}

unsafe fn string_map_snapshot(hash: *mut c_void) -> StringMapSnapshot {
    let element_size = size_of::<StringEntry>();
    let header = &*hash_header(hash, element_size);
    let table = header.hash_table.cast::<HashIndex>();
    let count = header.length.saturating_sub(1);
    let entries = std::slice::from_raw_parts(hash.cast::<StringEntry>(), count)
        .iter()
        .map(|entry| (CStr::from_ptr(entry.key).to_bytes().to_vec(), entry.value))
        .collect();
    StringMapSnapshot {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        slot_count: if table.is_null() {
            0
        } else {
            (*table).slot_count
        },
        used_count: if table.is_null() {
            0
        } else {
            (*table).used_count
        },
        tombstones: if table.is_null() {
            0
        } else {
            (*table).tombstone_count
        },
        seed: if table.is_null() { 0 } else { (*table).seed },
        string_mode: if table.is_null() {
            0
        } else {
            (*table).string.mode
        },
        entries,
    }
}

unsafe fn set_binary_value(hash: *mut c_void, value: u64) {
    let index = (*hash_header(hash, size_of::<BinaryEntry>())).temp as usize;
    (*hash.cast::<BinaryEntry>().add(index)).value = value;
}

unsafe fn set_string_value(hash: *mut c_void, value: u64) {
    let index = (*hash_header(hash, size_of::<StringEntry>())).temp as usize;
    (*hash.cast::<StringEntry>().add(index)).value = value;
}

#[test]
fn hashes_match_all_source_shapes() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_hash_bytes = c.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap();
        let r_hash_bytes = rust.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap();
        let c_hash_string = c.get::<HashString>(b"stbds_hash_string\0").unwrap();
        let r_hash_string = rust.get::<HashString>(b"stbds_hash_string\0").unwrap();
        let mut rng = Rng::new(0x9a76_01f4_0ddc_1137);

        for length in 0..=63usize {
            for _ in 0..64 {
                let mut bytes = vec![0u8; length.max(1)];
                rng.fill(&mut bytes);
                for &seed in &[0, 1, rng.next_u64() as usize, usize::MAX] {
                    let c_value = c_hash_bytes(bytes.as_mut_ptr().cast(), length, seed);
                    let r_value = r_hash_bytes(bytes.as_mut_ptr().cast(), length, seed);
                    assert_eq!(
                        c_value, r_value,
                        "hash_bytes mismatch for length={length}, seed={seed:#x}"
                    );
                }
            }
        }

        for length in 0..=96usize {
            for _ in 0..32 {
                let mut bytes = vec![0u8; length + 1];
                rng.fill(&mut bytes[..length]);
                for byte in &mut bytes[..length] {
                    if *byte == 0 {
                        *byte = 0x80;
                    }
                }
                bytes[length] = 0;
                for &seed in &[0, 1, rng.next_u64() as usize, usize::MAX] {
                    let c_value = c_hash_string(bytes.as_mut_ptr().cast(), seed);
                    let r_value = r_hash_string(bytes.as_mut_ptr().cast(), seed);
                    assert_eq!(
                        c_value, r_value,
                        "hash_string mismatch for length={length}, seed={seed:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn arrays_match_growth_and_payload_rules() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_grow = c.get::<ArrGrow>(b"stbds_arrgrowf\0").unwrap();
        let r_grow = rust.get::<ArrGrow>(b"stbds_arrgrowf\0").unwrap();
        let c_free = c.get::<ArrFree>(b"stbds_arrfreef\0").unwrap();
        let r_free = rust.get::<ArrFree>(b"stbds_arrfreef\0").unwrap();
        let mut rng = Rng::new(0x43bc_f01e_d15c_9912);
        assert!(c_grow(null_mut(), 1, 0, 0).is_null());
        assert!(r_grow(null_mut(), 1, 0, 0).is_null());

        for &element_size in &[1usize, 4, 24] {
            for &(add, minimum) in &[(0, 1), (1, 0), (3, 0), (0, 4), (0, 17)] {
                let mut c_array = c_grow(null_mut(), element_size, add, minimum);
                let mut r_array = r_grow(null_mut(), element_size, add, minimum);
                assert_eq!(
                    array_snapshot(c_array, element_size),
                    array_snapshot(r_array, element_size)
                );

                let first_length = add.max(minimum.min(add));
                (*array_header(c_array)).length =
                    first_length.min((*array_header(c_array)).capacity);
                (*array_header(r_array)).length =
                    first_length.min((*array_header(r_array)).capacity);
                let byte_count = (*array_header(c_array)).length * element_size;
                let mut payload = vec![0u8; byte_count];
                rng.fill(&mut payload);
                std::ptr::copy_nonoverlapping(payload.as_ptr(), c_array.cast(), byte_count);
                std::ptr::copy_nonoverlapping(payload.as_ptr(), r_array.cast(), byte_count);

                let old_capacity = (*array_header(c_array)).capacity;
                for &(more, requested) in &[
                    (0, old_capacity),
                    (1, old_capacity + 1),
                    (0, old_capacity.saturating_mul(2) + 3),
                ] {
                    c_array = c_grow(c_array, element_size, more, requested);
                    r_array = r_grow(r_array, element_size, more, requested);
                    assert_eq!(
                        array_snapshot(c_array, element_size),
                        array_snapshot(r_array, element_size),
                        "array mismatch element_size={element_size}, add={more}, min={requested}"
                    );
                }
                c_free(c_array);
                r_free(r_array);
            }
        }
    }
}

unsafe fn exercise_binary_map(lib: &Library, mode: c_int) -> Vec<BinaryMapSnapshot> {
    let rand_seed = lib.get::<RandSeed>(b"stbds_rand_seed\0").unwrap();
    let put_default = lib.get::<HmPutDefault>(b"stbds_hmput_default\0").unwrap();
    let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
    let get_ts = lib.get::<HmGetTs>(b"stbds_hmget_key_ts\0").unwrap();
    let get = lib.get::<HmGet>(b"stbds_hmget_key\0").unwrap();
    let del = lib.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
    let free_map = lib.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
    let mut snapshots = Vec::new();
    let element_size = size_of::<BinaryEntry>();

    rand_seed(0x1234_5678_9abc_def0);
    free_map(null_mut(), element_size);

    let mut null_key = 77u64;
    let mut temp = 99isize;
    let null_get = get_ts(
        null_mut(),
        element_size,
        (&mut null_key as *mut u64).cast(),
        size_of::<u64>(),
        &mut temp,
        mode,
    );
    assert_eq!(temp, -1);
    snapshots.push(binary_map_snapshot(null_get));
    free_map(raw_from_hash(null_get, element_size), element_size);

    let mut hash = put_default(null_mut(), element_size);
    snapshots.push(binary_map_snapshot(hash));
    let original = hash;
    hash = put_default(hash, element_size);
    assert_eq!(hash, original);
    snapshots.push(binary_map_snapshot(hash));

    temp = 123;
    let returned = get_ts(
        hash,
        element_size,
        (&mut null_key as *mut u64).cast(),
        size_of::<u64>(),
        &mut temp,
        mode,
    );
    assert_eq!(returned, hash);
    assert_eq!(temp, -1);
    snapshots.push(binary_map_snapshot(hash));

    hash = get(
        hash,
        element_size,
        (&mut null_key as *mut u64).cast(),
        size_of::<u64>(),
        mode,
    );
    snapshots.push(binary_map_snapshot(hash));

    let mut rng = Rng::new(0xf3a5_90bc_71d2_4410);
    let mut keys = Vec::new();
    for index in 0..96u64 {
        let mut key = rng.next_u64() ^ index.rotate_left(17);
        keys.push(key);
        hash = put(
            hash,
            element_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            mode,
        );
        set_binary_value(hash, rng.next_u64());
        if index < 10 || index % 7 == 0 {
            snapshots.push(binary_map_snapshot(hash));
        }
    }

    for (index, key) in keys.iter_mut().enumerate() {
        if index % 5 == 0 {
            hash = put(
                hash,
                element_size,
                (key as *mut u64).cast(),
                size_of::<u64>(),
                mode,
            );
            set_binary_value(hash, 0xa5a5_0000_0000_0000 | index as u64);
            snapshots.push(binary_map_snapshot(hash));
        }
    }

    for &index in &[0usize, 17, 63, 95] {
        temp = -99;
        let returned = get_ts(
            hash,
            element_size,
            (&mut keys[index] as *mut u64).cast(),
            size_of::<u64>(),
            &mut temp,
            mode,
        );
        assert_eq!(returned, hash);
        assert!(temp >= 0);
        snapshots.push(binary_map_snapshot(hash));
    }

    let mut missing = 0xfeed_face_dead_beefu64;
    while keys.contains(&missing) {
        missing = missing.wrapping_add(1);
    }
    hash = del(
        hash,
        element_size,
        (&mut missing as *mut u64).cast(),
        size_of::<u64>(),
        0,
        mode,
    );
    snapshots.push(binary_map_snapshot(hash));

    for index in (0..keys.len()).step_by(2) {
        hash = del(
            hash,
            element_size,
            (&mut keys[index] as *mut u64).cast(),
            size_of::<u64>(),
            0,
            mode,
        );
        if index < 12 || index % 10 == 0 {
            snapshots.push(binary_map_snapshot(hash));
        }
    }

    for index in (1..keys.len()).step_by(2).take(38) {
        hash = del(
            hash,
            element_size,
            (&mut keys[index] as *mut u64).cast(),
            size_of::<u64>(),
            0,
            mode,
        );
        if index % 9 == 1 {
            snapshots.push(binary_map_snapshot(hash));
        }
    }
    snapshots.push(binary_map_snapshot(hash));
    free_map(raw_from_hash(hash, element_size), element_size);
    snapshots
}

#[test]
fn binary_hash_map_pipeline_matches() {
    let (c, rust) = load_libraries();
    unsafe {
        assert_eq!(exercise_binary_map(&c, 0), exercise_binary_map(&rust, 0));
        assert_eq!(exercise_binary_map(&c, -1), exercise_binary_map(&rust, -1));
    }
}

unsafe fn exercise_string_map(lib: &Library, ownership_mode: c_int) -> Vec<StringMapSnapshot> {
    let rand_seed = lib.get::<RandSeed>(b"stbds_rand_seed\0").unwrap();
    let shmode = lib.get::<ShMode>(b"stbds_shmode_func\0").unwrap();
    let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
    let get_ts = lib.get::<HmGetTs>(b"stbds_hmget_key_ts\0").unwrap();
    let get = lib.get::<HmGet>(b"stbds_hmget_key\0").unwrap();
    let del = lib.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
    let free_map = lib.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
    let element_size = size_of::<StringEntry>();
    let mut snapshots = Vec::new();

    rand_seed(0x0ddc_0ffe_e123_4567);
    let mut hash = shmode(element_size, ownership_mode);
    snapshots.push(string_map_snapshot(hash));
    let mut sources: Vec<Vec<u8>> = Vec::new();

    for index in 0..88usize {
        let length = match index % 9 {
            0 => 0,
            1 => 1,
            2 => 7,
            3 => 31,
            4 => 127,
            5 => 255,
            6 => 511,
            7 => 513,
            _ => 19,
        };
        let mut source = Vec::with_capacity(length + 16);
        source.extend_from_slice(format!("k{index:03}_").as_bytes());
        while source.len() < length.max(5) {
            let byte = ((index * 37 + source.len() * 19) % 255 + 1) as u8;
            source.push(byte);
        }
        source.push(0);
        sources.push(source);
        let key = sources.last_mut().unwrap();
        hash = put(
            hash,
            element_size,
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            1,
        );
        set_string_value(hash, 0x3300_0000_0000_0000 | index as u64);
        if ownership_mode == 2 || ownership_mode == 3 {
            key[0] = b'X';
        }
        if index < 12 || index % 8 == 0 {
            snapshots.push(string_map_snapshot(hash));
        }
    }

    for index in (0..sources.len()).step_by(11) {
        let key = if ownership_mode == 2 || ownership_mode == 3 {
            let snapshot_key = &string_map_snapshot(hash).entries[index].0;
            CString::new(snapshot_key.clone()).unwrap()
        } else {
            CString::new(&sources[index][..sources[index].len() - 1]).unwrap()
        };
        let mut temp = -99isize;
        let returned = get_ts(
            hash,
            element_size,
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            &mut temp,
            1,
        );
        assert_eq!(returned, hash);
        assert!(temp >= 0);
        hash = get(
            hash,
            element_size,
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            1,
        );
        snapshots.push(string_map_snapshot(hash));
    }

    let missing = CString::new("definitely_missing_key").unwrap();
    hash = del(
        hash,
        element_size,
        missing.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        0,
        1,
    );
    snapshots.push(string_map_snapshot(hash));

    let original_keys: Vec<CString> = string_map_snapshot(hash)
        .entries
        .iter()
        .map(|(key, _)| CString::new(key.clone()).unwrap())
        .collect();
    for index in (0..original_keys.len()).step_by(2) {
        hash = del(
            hash,
            element_size,
            original_keys[index].as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            1,
        );
        if index < 12 || index % 12 == 0 {
            snapshots.push(string_map_snapshot(hash));
        }
    }
    snapshots.push(string_map_snapshot(hash));
    free_map(raw_from_hash(hash, element_size), element_size);
    snapshots
}

#[test]
fn string_hash_map_ownership_modes_match() {
    let (c, rust) = load_libraries();
    unsafe {
        for mode in [1, 2, 3] {
            assert_eq!(
                exercise_string_map(&c, mode),
                exercise_string_map(&rust, mode),
                "string map mismatch in ownership mode {mode}"
            );
        }
    }
}

#[test]
fn mode_boundaries_and_initial_states_match() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_shmode = c.get::<ShMode>(b"stbds_shmode_func\0").unwrap();
        let r_shmode = rust.get::<ShMode>(b"stbds_shmode_func\0").unwrap();
        let c_free = c.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
        let r_free = rust.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();

        for &mode in &[0, 1, 2, 3, 257, c_int::MAX] {
            let element_size = if mode == 0 {
                size_of::<BinaryEntry>()
            } else {
                size_of::<StringEntry>()
            };
            let c_hash = c_shmode(element_size, mode);
            let r_hash = r_shmode(element_size, mode);
            let c_header = *hash_header(c_hash, element_size);
            let r_header = *hash_header(r_hash, element_size);
            assert_eq!(c_header.length, r_header.length);
            assert_eq!(c_header.capacity, r_header.capacity);
            assert_eq!(c_header.temp, r_header.temp);
            let c_table = hash_index(c_hash, element_size);
            let r_table = hash_index(r_hash, element_size);
            assert_eq!((*c_table).slot_count, (*r_table).slot_count);
            assert_eq!((*c_table).used_count, (*r_table).used_count);
            assert_eq!((*c_table).string.mode, (*r_table).string.mode);
            c_free(raw_from_hash(c_hash, element_size), element_size);
            r_free(raw_from_hash(r_hash, element_size), element_size);
        }
    }
}

fn arena_shape(arena: &StringArena) -> (usize, usize, u8, u8) {
    let mut blocks = 0usize;
    let mut current = arena.storage;
    unsafe {
        while !current.is_null() {
            blocks += 1;
            current = (*current).next;
            assert!(blocks < 10_000, "arena block list cycle");
        }
    }
    (blocks, arena.remaining, arena.block, arena.mode)
}

unsafe fn exercise_arena(lib: &Library) -> Vec<(Vec<u8>, (usize, usize, u8, u8))> {
    let alloc = lib.get::<StrAlloc>(b"stbds_stralloc\0").unwrap();
    let reset = lib.get::<StrReset>(b"stbds_strreset\0").unwrap();
    let mut arena = StringArena {
        storage: null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut results = Vec::new();
    let mut rng = Rng::new(0x6f1d_201b_91aa_770e);

    for index in 0..160usize {
        let length = match index % 12 {
            0 => 0,
            1 => 1,
            2 => 7,
            3 => 63,
            4 => 127,
            5 => 255,
            6 => 510,
            7 => 511,
            8 => 512,
            9 => 513,
            10 => 1023,
            _ => (rng.next_u64() as usize % 700) + 1,
        };
        let mut bytes = vec![0u8; length + 1];
        rng.fill(&mut bytes[..length]);
        for byte in &mut bytes[..length] {
            if *byte == 0 {
                *byte = 1;
            }
        }
        let returned = alloc(&mut arena, bytes.as_mut_ptr().cast());
        results.push((
            CStr::from_ptr(returned).to_bytes().to_vec(),
            arena_shape(&arena),
        ));
    }

    let mut oversized = vec![b'q'; (1 << 20) + 17];
    oversized.push(0);
    let returned = alloc(&mut arena, oversized.as_mut_ptr().cast());
    results.push((
        CStr::from_ptr(returned).to_bytes().to_vec(),
        arena_shape(&arena),
    ));
    reset(&mut arena);
    results.push((Vec::new(), arena_shape(&arena)));
    reset(&mut arena);
    results.push((Vec::new(), arena_shape(&arena)));
    results
}

#[test]
fn string_arena_allocation_and_reset_match() {
    let (c, rust) = load_libraries();
    unsafe {
        assert_eq!(exercise_arena(&c), exercise_arena(&rust));

        let c_alloc = c.get::<StrAlloc>(b"stbds_stralloc\0").unwrap();
        let r_alloc = rust.get::<StrAlloc>(b"stbds_stralloc\0").unwrap();
        let c_reset = c.get::<StrReset>(b"stbds_strreset\0").unwrap();
        let r_reset = rust.get::<StrReset>(b"stbds_strreset\0").unwrap();
        let mut c_arena = StringArena {
            storage: null_mut(),
            remaining: 0,
            block: 22,
            mode: 7,
        };
        let mut r_arena = c_arena;
        let mut value = CString::new("max-block-boundary")
            .unwrap()
            .into_bytes_with_nul();
        let c_value = c_alloc(&mut c_arena, value.as_mut_ptr().cast());
        let r_value = r_alloc(&mut r_arena, value.as_mut_ptr().cast());
        assert_eq!(
            CStr::from_ptr(c_value).to_bytes(),
            CStr::from_ptr(r_value).to_bytes()
        );
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
        c_reset(&mut c_arena);
        r_reset(&mut r_arena);
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));

        let mut c_fresh = StringArena {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut r_fresh = c_fresh;
        let mut oversized = vec![b'z'; 513];
        oversized.push(0);
        let c_value = c_alloc(&mut c_fresh, oversized.as_mut_ptr().cast());
        let r_value = r_alloc(&mut r_fresh, oversized.as_mut_ptr().cast());
        assert_eq!(
            CStr::from_ptr(c_value).to_bytes(),
            CStr::from_ptr(r_value).to_bytes()
        );
        assert_eq!(arena_shape(&c_fresh), arena_shape(&r_fresh));
        c_reset(&mut c_fresh);
        r_reset(&mut r_fresh);
    }
}

#[test]
fn strkey_matches_integer_boundaries() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_strkey = c.get::<StrKey>(b"strkey\0").unwrap();
        let r_strkey = rust.get::<StrKey>(b"strkey\0").unwrap();
        let mut rng = Rng::new(0xb519_70f2_541a_cc83);
        let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
        values.extend((0..128).map(|_| rng.next_u64() as c_int));
        for value in values {
            let c_bytes = CStr::from_ptr(c_strkey(value)).to_bytes().to_vec();
            let r_bytes = CStr::from_ptr(r_strkey(value)).to_bytes().to_vec();
            assert_eq!(c_bytes, r_bytes, "strkey mismatch for {value}");
        }
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = PROCESS_IO_LOCK.lock().unwrap();
    let mut fds = [-1, -1];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    assert_eq!(fflush(null_mut()), 0);
    let saved = dup(1);
    assert!(saved >= 0);
    assert_eq!(dup2(fds[1], 1), 1);
    assert_eq!(close(fds[1]), 0);
    call();
    assert_eq!(fflush(null_mut()), 0);
    assert_eq!(dup2(saved, 1), 1);
    assert_eq!(close(saved), 0);

    let mut output = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let count = read(fds[0], chunk.as_mut_ptr().cast(), chunk.len());
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    assert_eq!(close(fds[0]), 0);
    output
}

#[test]
fn helxo_stdout_matches_byte_for_byte() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_helxo = c.get::<Helxo>(b"helxo\0").unwrap();
        let r_helxo = rust.get::<Helxo>(b"helxo\0").unwrap();
        let mut rng = Rng::new(0x18e4_dd21_887a_1f90);
        let mut letters = vec![0u8, b'A', b'x', 0x7f, 0x80, 0xff];
        letters.extend((0..64).map(|_| rng.next_u64() as u8));
        for letter in letters {
            let c_output = capture_stdout(|| c_helxo(letter as c_char));
            let r_output = capture_stdout(|| r_helxo(letter as c_char));
            assert_eq!(
                c_output, r_output,
                "helxo stdout mismatch for byte {letter:#04x}"
            );
        }
    }
}

#[test]
fn safe_error_sentinels_and_enum_boundaries_match() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_put = c.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
        let r_put = rust.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
        let c_del = c.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
        let r_del = rust.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
        let c_free = c.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
        let r_free = rust.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
        let c_hash_bytes = c.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap();
        let r_hash_bytes = rust.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap();
        assert_eq!(
            c_hash_bytes(null_mut(), 0, usize::MAX),
            r_hash_bytes(null_mut(), 0, usize::MAX)
        );

        assert!(c_del(null_mut(), 16, null_mut(), 8, 0, 0).is_null());
        assert!(r_del(null_mut(), 16, null_mut(), 8, 0, 0).is_null());
        c_free(null_mut(), 16);
        r_free(null_mut(), 16);

        for &mode in &[1, 2, c_int::MAX] {
            let key = CString::new(format!("mode_{mode}")).unwrap();
            let c_hash = c_put(
                null_mut(),
                size_of::<StringEntry>(),
                key.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            let r_hash = r_put(
                null_mut(),
                size_of::<StringEntry>(),
                key.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            set_string_value(c_hash, mode as u64);
            set_string_value(r_hash, mode as u64);
            assert_eq!(
                string_map_snapshot(c_hash),
                string_map_snapshot(r_hash),
                "out-of-range string mode mismatch: {mode}"
            );
            c_free(
                raw_from_hash(c_hash, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            r_free(
                raw_from_hash(r_hash, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }
    }
}

#[test]
fn preallocated_default_and_seeded_maps_match() {
    let (c, rust) = load_libraries();
    unsafe {
        let c_grow = c.get::<ArrGrow>(b"stbds_arrgrowf\0").unwrap();
        let r_grow = rust.get::<ArrGrow>(b"stbds_arrgrowf\0").unwrap();
        let c_default = c.get::<HmPutDefault>(b"stbds_hmput_default\0").unwrap();
        let r_default = rust.get::<HmPutDefault>(b"stbds_hmput_default\0").unwrap();
        let c_free = c.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
        let r_free = rust.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
        let element_size = size_of::<BinaryEntry>();
        let c_raw = c_grow(null_mut(), element_size, 0, 1);
        let r_raw = r_grow(null_mut(), element_size, 0, 1);
        let c_hash = c_default(c_raw.cast::<u8>().add(element_size).cast(), element_size);
        let r_hash = r_default(r_raw.cast::<u8>().add(element_size).cast(), element_size);
        assert_eq!(binary_map_snapshot(c_hash), binary_map_snapshot(r_hash));
        c_free(raw_from_hash(c_hash, element_size), element_size);
        r_free(raw_from_hash(r_hash, element_size), element_size);

        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            let c_seed = c.get::<RandSeed>(b"stbds_rand_seed\0").unwrap();
            let r_seed = rust.get::<RandSeed>(b"stbds_rand_seed\0").unwrap();
            let c_put = c.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
            let r_put = rust.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
            c_seed(seed);
            r_seed(seed);
            let mut key = seed as u64 ^ 0xa55a_6996_1234_5678;
            let c_hash = c_put(
                null_mut(),
                element_size,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            let r_hash = r_put(
                null_mut(),
                element_size,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            set_binary_value(c_hash, seed as u64 ^ 0x55aa_aa55);
            set_binary_value(r_hash, seed as u64 ^ 0x55aa_aa55);
            assert_eq!(binary_map_snapshot(c_hash), binary_map_snapshot(r_hash));
            c_free(raw_from_hash(c_hash, element_size), element_size);
            r_free(raw_from_hash(r_hash, element_size), element_size);
        }
    }
}

unsafe fn wrapped_probe_result(lib: &Library) -> (isize, BinaryMapSnapshot) {
    let seed_fn = lib.get::<RandSeed>(b"stbds_rand_seed\0").unwrap();
    let hash_fn = lib.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap();
    let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
    let get_ts = lib.get::<HmGetTs>(b"stbds_hmget_key_ts\0").unwrap();
    let free_map = lib.get::<HmFree>(b"stbds_hmfree_func\0").unwrap();
    let seed = 0x7a11_5eed_d00d_f00dusize;
    seed_fn(seed);
    let mut candidates = [0u64; 5];
    for (offset, target_slot) in (3usize..=7).enumerate() {
        let mut candidate = (target_slot as u64) << 48;
        loop {
            let hash = hash_fn((&mut candidate as *mut u64).cast(), size_of::<u64>(), seed);
            if hash >= 2 && hash & 7 == target_slot {
                candidates[offset] = candidate;
                break;
            }
            candidate = candidate.wrapping_add(1);
        }
    }
    let mut hash = null_mut();
    for key in &mut candidates[..5] {
        hash = put(
            hash,
            size_of::<BinaryEntry>(),
            (key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        );
        set_binary_value(hash, *key ^ 0x55aa);
    }
    let table = hash_index(hash, size_of::<BinaryEntry>());
    assert_eq!((*table).slot_count, 8);
    for slot in 3..=7 {
        assert_ne!((*(*table).storage).hash[slot], 0);
    }
    assert_eq!((*(*table).storage).hash[0], 0);

    let mut missing = 0xf00d_cafe_0000_0000u64;
    loop {
        let missing_hash = hash_fn((&mut missing as *mut u64).cast(), size_of::<u64>(), seed);
        if missing_hash >= 2
            && missing_hash & 7 == 3
            && !binary_map_snapshot(hash)
                .entries
                .iter()
                .any(|(key, _)| *key == missing)
        {
            break;
        }
        missing = missing.wrapping_add(1);
    }
    let mut temp = 123isize;
    let returned = get_ts(
        hash,
        size_of::<BinaryEntry>(),
        (&mut missing as *mut u64).cast(),
        size_of::<u64>(),
        &mut temp,
        0,
    );
    assert_eq!(returned, hash);
    let snapshot = binary_map_snapshot(hash);
    free_map(
        raw_from_hash(hash, size_of::<BinaryEntry>()),
        size_of::<BinaryEntry>(),
    );
    (temp, snapshot)
}

#[test]
fn missing_key_wrapped_probe_segment_matches() {
    let (c, rust) = load_libraries();
    unsafe {
        assert_eq!(wrapped_probe_result(&c), wrapped_probe_result(&rust));
    }
}

unsafe fn run_crash_probe(lib: &Library, probe: &str) {
    match probe {
        "arrfree_null" => {
            lib.get::<ArrFree>(b"stbds_arrfreef\0").unwrap()(null_mut());
        }
        "hash_string_null" => {
            lib.get::<HashString>(b"stbds_hash_string\0").unwrap()(null_mut(), 0);
        }
        "hash_bytes_null" => {
            lib.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap()(null_mut(), 1, 0);
        }
        "hash_bytes_null_oversized" => {
            lib.get::<HashBytes>(b"stbds_hash_bytes\0").unwrap()(
                null_mut(),
                usize::MAX,
                usize::MAX,
            );
        }
        "stralloc_null_arena" => {
            let mut value = CString::new("x").unwrap().into_bytes_with_nul();
            lib.get::<StrAlloc>(b"stbds_stralloc\0").unwrap()(
                null_mut(),
                value.as_mut_ptr().cast(),
            );
        }
        "stralloc_null_string" => {
            let mut arena = StringArena {
                storage: null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            lib.get::<StrAlloc>(b"stbds_stralloc\0").unwrap()(&mut arena, null_mut());
        }
        "strreset_null" => {
            lib.get::<StrReset>(b"stbds_strreset\0").unwrap()(null_mut());
        }
        "hmget_null_temp" => {
            let mut key = 1u64;
            lib.get::<HmGetTs>(b"stbds_hmget_key_ts\0").unwrap()(
                null_mut(),
                size_of::<BinaryEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                null_mut(),
                0,
            );
        }
        "make_hash_assert" => {
            let shmode = lib.get::<ShMode>(b"stbds_shmode_func\0").unwrap();
            let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
            let hash = shmode(size_of::<BinaryEntry>(), 0);
            let table = hash_index(hash, size_of::<BinaryEntry>());
            (*table).slot_count = 1;
            (*table).used_count = 0;
            (*table).used_count_threshold = 0;
            let mut key = 7u64;
            put(
                hash,
                size_of::<BinaryEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
        }
        "hmdel_slot_assert" => {
            let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
            let del = lib.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
            let mut hash = null_mut();
            for key in 1u64..=6 {
                let mut current = key.wrapping_mul(0x9e37_79b9_7f4a_7c15);
                hash = put(
                    hash,
                    size_of::<BinaryEntry>(),
                    (&mut current as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                );
                set_binary_value(hash, key);
            }
            let table = hash_index(hash, size_of::<BinaryEntry>());
            let bucket = (*table).storage;
            let slot = (1..8)
                .find(|&item| (*bucket).index[item] >= 0)
                .expect("need occupied nonzero slot");
            let index = (*bucket).index[slot] as usize;
            let mut key = (*hash.cast::<BinaryEntry>().add(index)).key;
            (*table).slot_count = 1;
            del(
                hash,
                size_of::<BinaryEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
        }
        "hmdel_move_assert" => {
            let put = lib.get::<HmPut>(b"stbds_hmput_key\0").unwrap();
            let del = lib.get::<HmDel>(b"stbds_hmdel_key\0").unwrap();
            let mut hash = null_mut();
            for mut key in [11u64, 22, 33] {
                hash = put(
                    hash,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                );
                set_binary_value(hash, key);
            }
            let table = hash_index(hash, size_of::<BinaryEntry>());
            let final_index = 2isize;
            for item in 0..8 {
                if (*(*table).storage).index[item] == final_index {
                    (*(*table).storage).hash[item] = 0;
                    break;
                }
            }
            let mut delete_key = 11u64;
            del(
                hash,
                size_of::<BinaryEntry>(),
                (&mut delete_key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
        }
        other => panic!("unknown crash probe {other}"),
    }
}

#[test]
fn ffi_crash_probe() {
    let Ok(probe) = std::env::var("HARVEST_FFI_PROBE") else {
        return;
    };
    let kind = std::env::var("HARVEST_FFI_LIBRARY").unwrap();
    let (c_path, rust_path) = library_paths();
    let path = if kind == "c" { c_path } else { rust_path };
    unsafe {
        let lib = Library::new(path).unwrap();
        run_crash_probe(&lib, &probe);
    }
}

fn probe_status(kind: &str, probe: &str) -> (ExitStatus, Vec<u8>) {
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_crash_probe")
        .arg("--nocapture")
        .env("HARVEST_FFI_LIBRARY", kind)
        .env("HARVEST_FFI_PROBE", probe)
        .output()
        .unwrap();
    let mut diagnostics = output.stdout;
    diagnostics.extend_from_slice(&output.stderr);
    (output.status, diagnostics)
}

#[test]
fn fatal_error_paths_match_process_status() {
    let probes = [
        "arrfree_null",
        "hash_string_null",
        "hash_bytes_null",
        "hash_bytes_null_oversized",
        "stralloc_null_arena",
        "stralloc_null_string",
        "strreset_null",
        "hmget_null_temp",
        "make_hash_assert",
        "hmdel_slot_assert",
        "hmdel_move_assert",
    ];
    for probe in probes {
        let (c_status, c_output) = probe_status("c", probe);
        let (r_status, r_output) = probe_status("rust", probe);
        assert!(
            !c_status.success(),
            "C probe unexpectedly survived: {probe}"
        );
        assert!(
            !r_status.success(),
            "Rust probe unexpectedly survived: {probe}"
        );
        assert_eq!(
            c_status.signal(),
            r_status.signal(),
            "different terminating signal for {probe}\nC:\n{}\nRust:\n{}",
            String::from_utf8_lossy(&c_output),
            String::from_utf8_lossy(&r_output)
        );
    }
}
