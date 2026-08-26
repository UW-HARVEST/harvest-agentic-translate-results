#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

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
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type StrPut = unsafe extern "C" fn(c_int);

struct Api {
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_bytes: HashBytes,
    hash_string: HashString,
    hmfree: HmFree,
    hmget: HmGet,
    hmget_ts: HmGetTs,
    hmput_default: HmPutDefault,
    hmput: HmPut,
    hmdel: HmDel,
    shmode: ShMode,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    str_put: StrPut,
    _library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        let library = Library::new(path).unwrap_or_else(|e| panic!("load {}: {e}", path.display()));
        unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            *library
                .get::<T>(name)
                .unwrap_or_else(|e| panic!("symbol {}: {e}", String::from_utf8_lossy(name)))
        }
        Self {
            arrgrow: symbol(&library, b"stbds_arrgrowf\0"),
            arrfree: symbol(&library, b"stbds_arrfreef\0"),
            rand_seed: symbol(&library, b"stbds_rand_seed\0"),
            hash_bytes: symbol(&library, b"stbds_hash_bytes\0"),
            hash_string: symbol(&library, b"stbds_hash_string\0"),
            hmfree: symbol(&library, b"stbds_hmfree_func\0"),
            hmget: symbol(&library, b"stbds_hmget_key\0"),
            hmget_ts: symbol(&library, b"stbds_hmget_key_ts\0"),
            hmput_default: symbol(&library, b"stbds_hmput_default\0"),
            hmput: symbol(&library, b"stbds_hmput_key\0"),
            hmdel: symbol(&library, b"stbds_hmdel_key\0"),
            shmode: symbol(&library, b"stbds_shmode_func\0"),
            stralloc: symbol(&library, b"stbds_stralloc\0"),
            strreset: symbol(&library, b"stbds_strreset\0"),
            strkey: symbol(&library, b"strkey\0"),
            str_put: symbol(&library, b"str_put\0"),
            _library: library,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("c_src/build/libtranslated_rust.so"),
        root.join("target/release/libstr_put_lib.so"),
    )
}

unsafe fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.exists(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.exists(),
        "missing Rust library: {}",
        rust_path.display()
    );
    (Api::open(&c_path), Api::open(&rust_path))
}

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
#[derive(Clone, Copy)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    buckets: Vec<([usize; 8], [isize; 8])>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MapSnapshot<T> {
    length: usize,
    capacity: usize,
    temp: isize,
    table: Option<TableSnapshot>,
    entries: Vec<T>,
}

#[inline]
unsafe fn header(a: *mut c_void) -> *mut ArrayHeader {
    a.cast::<ArrayHeader>().sub(1)
}

unsafe fn table_snapshot(raw: *mut c_void) -> Option<TableSnapshot> {
    let table = (*header(raw)).hash_table.cast::<HashIndex>();
    if table.is_null() {
        return None;
    }
    let mut buckets = Vec::new();
    for i in 0..((*table).slot_count >> 3) {
        let bucket = (*table).storage.add(i);
        buckets.push(((*bucket).hash, (*bucket).index));
    }
    Some(TableSnapshot {
        slot_count: (*table).slot_count,
        used_count: (*table).used_count,
        used_count_threshold: (*table).used_count_threshold,
        used_count_shrink_threshold: (*table).used_count_shrink_threshold,
        tombstone_count: (*table).tombstone_count,
        tombstone_count_threshold: (*table).tombstone_count_threshold,
        seed: (*table).seed,
        slot_count_log2: (*table).slot_count_log2,
        string_remaining: (*table).string.remaining,
        string_block: (*table).string.block,
        string_mode: (*table).string.mode,
        buckets,
    })
}

unsafe fn binary_snapshot(map: *mut BinaryEntry) -> MapSnapshot<BinaryEntry> {
    let raw = map.sub(1);
    let h = *header(raw.cast());
    MapSnapshot {
        length: h.length - 1,
        capacity: h.capacity,
        temp: h.temp,
        table: table_snapshot(raw.cast()),
        entries: (0..h.length - 1).map(|i| *map.add(i)).collect(),
    }
}

unsafe fn string_snapshot(map: *mut StringEntry) -> MapSnapshot<(Vec<u8>, i64)> {
    let raw = map.sub(1);
    let h = *header(raw.cast());
    MapSnapshot {
        length: h.length - 1,
        capacity: h.capacity,
        temp: h.temp,
        table: table_snapshot(raw.cast()),
        entries: (0..h.length - 1)
            .map(|i| {
                let item = *map.add(i);
                (CStr::from_ptr(item.key).to_bytes().to_vec(), item.value)
            })
            .collect(),
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn usize(&mut self) -> usize {
        self.next() as usize
    }
}

#[test]
fn symbols_load_from_both_shared_objects() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let _ = apis();
    }
}

#[test]
fn hashes_match_for_randomized_shapes_and_boundaries() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0x6f4a_2c91_73de_b805);
        for len in 0..=96 {
            for _ in 0..64 {
                let mut bytes = vec![0u8; len];
                for byte in &mut bytes {
                    *byte = rng.next() as u8;
                }
                let seed = rng.usize();
                let pointer = if len == 0 && seed & 1 == 0 {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                assert_eq!(
                    (c.hash_bytes)(pointer, len, seed),
                    (rust.hash_bytes)(pointer, len, seed),
                    "hash_bytes len={len} seed={seed:#x}"
                );
            }
        }

        for len in 0..=96 {
            for _ in 0..64 {
                let mut bytes = Vec::with_capacity(len + 1);
                for _ in 0..len {
                    bytes.push((rng.next() as u8 % 255) + 1);
                }
                bytes.push(0);
                let seed = rng.usize();
                assert_eq!(
                    (c.hash_string)(bytes.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(bytes.as_mut_ptr().cast(), seed),
                    "hash_string len={len} seed={seed:#x}"
                );
            }
        }
    }
}

unsafe fn array_trace(api: &Api) -> Vec<(usize, usize, bool)> {
    let mut trace = Vec::new();
    let zero = (api.arrgrow)(ptr::null_mut(), size_of::<u64>(), 0, 0);
    trace.push((0, 0, zero.is_null()));

    let mut array = (api.arrgrow)(ptr::null_mut(), size_of::<u64>(), 0, 1);
    trace.push(((*header(array)).length, (*header(array)).capacity, false));
    (*header(array)).length = 3;
    for &(add, minimum) in &[(0, 3), (1, 0), (2, 0), (0, 17), (20, 2)] {
        let old = array;
        array = (api.arrgrow)(array, size_of::<u64>(), add, minimum);
        trace.push((
            (*header(array)).length,
            (*header(array)).capacity,
            old == array,
        ));
    }
    (api.arrfree)(array);

    let huge = (api.arrgrow)(ptr::null_mut(), 0, usize::MAX, 0);
    trace.push(((*header(huge)).length, (*header(huge)).capacity, false));
    (api.arrfree)(huge);

    let mut rng = Rng::new(0x9137_44aa_f063_2d19);
    let mut array = ptr::null_mut();
    for _ in 0..128 {
        let add = rng.usize() % 7;
        let minimum = rng.usize() % 96;
        array = (api.arrgrow)(array, size_of::<u32>(), add, minimum);
        if !array.is_null() {
            let h = &mut *header(array);
            trace.push((h.length, h.capacity, false));
            h.length = (h.length + add).min(h.capacity);
        }
    }
    if !array.is_null() {
        (api.arrfree)(array);
    }
    trace
}

#[test]
fn array_growth_and_free_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        assert_eq!(array_trace(&c), array_trace(&rust));
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BinaryEvent {
    State(MapSnapshot<BinaryEntry>),
    Get(isize),
    Delete(bool),
}

unsafe fn binary_map_trace(
    api: &Api,
    initial_mode: Option<c_int>,
    mode: c_int,
    seed: usize,
) -> Vec<BinaryEvent> {
    (api.rand_seed)(seed);
    let mut map: *mut BinaryEntry = match initial_mode {
        Some(mode) => (api.shmode)(size_of::<BinaryEntry>(), mode).cast(),
        None => ptr::null_mut(),
    };
    let mut events = Vec::new();
    let mut rng = Rng::new(0x2d33_61b7_043a_8fc1);

    for step in 0..180 {
        let key = if step < 40 {
            (step * 17) as u64
        } else {
            rng.next() % 61
        };
        match step % 5 {
            0 | 1 | 2 => {
                let mut key_arg = key;
                map = (api.hmput)(
                    map.cast(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key_arg).cast(),
                    size_of::<u64>(),
                    mode,
                )
                .cast();
                let index = (*header(map.sub(1).cast())).temp as usize;
                (*map.add(index)).value = (rng.next() as i64).wrapping_add(step as i64);
                events.push(BinaryEvent::State(binary_snapshot(map)));
            }
            3 => {
                let mut key_arg = key;
                let mut temp = 777isize;
                map = (api.hmget_ts)(
                    map.cast(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key_arg).cast(),
                    size_of::<u64>(),
                    &mut temp,
                    mode,
                )
                .cast();
                events.push(BinaryEvent::Get(temp));
            }
            _ => {
                let mut key_arg = key;
                map = (api.hmdel)(
                    map.cast(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key_arg).cast(),
                    size_of::<u64>(),
                    0,
                    mode,
                )
                .cast();
                let deleted = (*header(map.sub(1).cast())).temp != 0;
                events.push(BinaryEvent::Delete(deleted));
                events.push(BinaryEvent::State(binary_snapshot(map)));
            }
        }
    }
    (api.hmfree)(map.sub(1).cast(), size_of::<BinaryEntry>());
    events
}

unsafe fn default_and_missing_trace(api: &Api) -> Vec<(usize, usize, isize, bool)> {
    let mut result = Vec::new();
    (api.hmfree)(ptr::null_mut(), size_of::<BinaryEntry>());
    let mut key = 9u64;
    let mut temp = 123isize;
    let mut map = (api.hmget_ts)(
        ptr::null_mut(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        &mut temp,
        0,
    )
    .cast::<BinaryEntry>();
    let raw = map.sub(1);
    result.push((
        (*header(raw.cast())).length,
        (*header(raw.cast())).capacity,
        temp,
        (*header(raw.cast())).hash_table.is_null(),
    ));
    temp = 456;
    let old = map;
    map = (api.hmget_ts)(
        map.cast(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        &mut temp,
        0,
    )
    .cast();
    result.push((
        (*header(map.sub(1).cast())).length,
        (*header(map.sub(1).cast())).capacity,
        temp,
        old == map,
    ));
    map = (api.hmget)(
        map.cast(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
    )
    .cast();
    result.push((
        (*header(map.sub(1).cast())).length,
        (*header(map.sub(1).cast())).capacity,
        (*header(map.sub(1).cast())).temp,
        true,
    ));
    let same = (api.hmput_default)(map.cast(), size_of::<BinaryEntry>()).cast::<BinaryEntry>();
    result.push((
        (*header(same.sub(1).cast())).length,
        (*header(same.sub(1).cast())).capacity,
        (*header(same.sub(1).cast())).temp,
        same == map,
    ));
    let deleted = (api.hmdel)(
        map.cast(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
        0,
    )
    .cast::<BinaryEntry>();
    result.push((
        (*header(deleted.sub(1).cast())).length,
        (*header(deleted.sub(1).cast())).capacity,
        (*header(deleted.sub(1).cast())).temp,
        deleted == map,
    ));
    (api.hmfree)(deleted.sub(1).cast(), size_of::<BinaryEntry>());

    let null_default =
        (api.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>()).cast::<BinaryEntry>();
    result.push((
        (*header(null_default.sub(1).cast())).length,
        (*header(null_default.sub(1).cast())).capacity,
        (*header(null_default.sub(1).cast())).temp,
        (*null_default.sub(1)).key == 0 && (*null_default.sub(1)).value == 0,
    ));
    (api.hmfree)(null_default.sub(1).cast(), size_of::<BinaryEntry>());

    let raw_zero = (api.arrgrow)(ptr::null_mut(), size_of::<BinaryEntry>(), 0, 1);
    let zero_map = raw_zero.cast::<u8>().add(size_of::<BinaryEntry>());
    let from_zero =
        (api.hmput_default)(zero_map.cast(), size_of::<BinaryEntry>()).cast::<BinaryEntry>();
    result.push((
        (*header(from_zero.sub(1).cast())).length,
        (*header(from_zero.sub(1).cast())).capacity,
        (*header(from_zero.sub(1).cast())).temp,
        (*from_zero.sub(1)).key == 0 && (*from_zero.sub(1)).value == 0,
    ));
    (api.hmfree)(from_zero.sub(1).cast(), size_of::<BinaryEntry>());

    let null_delete = (api.hmdel)(
        ptr::null_mut(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
        0,
    );
    result.push((0, 0, 0, null_delete.is_null()));
    result
}

unsafe fn unsigned_used_count_trace(api: &Api) -> (usize, usize, isize) {
    let mut map = child_put_binary(api, ptr::null_mut(), 42);
    let table = (*header(map.sub(1).cast())).hash_table.cast::<HashIndex>();
    (*table).used_count = 0;
    let mut key = 42u64;
    map = (api.hmdel)(
        map.cast(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
        0,
    )
    .cast();
    let result = (
        (*table).used_count,
        (*header(map.sub(1).cast())).length,
        (*header(map.sub(1).cast())).temp,
    );
    (api.hmfree)(map.sub(1).cast(), size_of::<BinaryEntry>());
    result
}

#[test]
fn binary_maps_defaults_growth_deletion_and_errors_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        assert_eq!(
            default_and_missing_trace(&c),
            default_and_missing_trace(&rust)
        );
        assert_eq!(unsigned_used_count_trace(&c), (usize::MAX, 1, 1));
        assert_eq!(
            unsigned_used_count_trace(&rust),
            unsigned_used_count_trace(&c)
        );
        assert_eq!(
            binary_map_trace(&c, None, 0, 0x1234_5678_9abc_def0),
            binary_map_trace(&rust, None, 0, 0x1234_5678_9abc_def0)
        );
        assert_eq!(
            binary_map_trace(&c, None, -1, 0x1234_5678_9abc_def0),
            binary_map_trace(&rust, None, -1, 0x1234_5678_9abc_def0)
        );
        assert_eq!(
            binary_map_trace(&c, Some(0), 0, 0x1234_5678_9abc_def0),
            binary_map_trace(&rust, Some(0), 0, 0x1234_5678_9abc_def0)
        );
        assert_eq!(
            binary_map_trace(&c, Some(-1), -1, 0x1234_5678_9abc_def0),
            binary_map_trace(&rust, Some(-1), -1, 0x1234_5678_9abc_def0)
        );
        assert_eq!(
            binary_map_trace(&c, None, 0, 0),
            binary_map_trace(&rust, None, 0, 0)
        );
        assert_eq!(
            binary_map_trace(&c, None, 0, usize::MAX),
            binary_map_trace(&rust, None, 0, usize::MAX)
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StringEvent {
    State(MapSnapshot<(Vec<u8>, i64)>),
    Get(isize),
    Delete(bool),
}

unsafe fn string_map_trace(
    api: &Api,
    initial_mode: Option<c_int>,
    call_mode: c_int,
) -> Vec<StringEvent> {
    (api.rand_seed)(0xa182_b3c4_d5e6_f708);
    let keys: Vec<CString> = (0..36)
        .map(|i| CString::new(format!("key_{:02}_{}", i, "x".repeat(i % 9))).unwrap())
        .collect();
    let mut map: *mut StringEntry = match initial_mode {
        Some(mode) => (api.shmode)(size_of::<StringEntry>(), mode).cast(),
        None => ptr::null_mut(),
    };
    let mut events = Vec::new();

    for i in 0..36 {
        let key = keys[i].as_ptr().cast_mut();
        map = (api.hmput)(
            map.cast(),
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            call_mode,
        )
        .cast();
        let index = (*header(map.sub(1).cast())).temp as usize;
        (*map.add(index)).value = (i as i64) * 101 - 77;
        events.push(StringEvent::State(string_snapshot(map)));
    }

    let duplicate = CString::new(keys[7].as_bytes()).unwrap();
    map = (api.hmput)(
        map.cast(),
        size_of::<StringEntry>(),
        duplicate.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        call_mode,
    )
    .cast();
    let index = (*header(map.sub(1).cast())).temp as usize;
    (*map.add(index)).value = 999_777;
    events.push(StringEvent::State(string_snapshot(map)));

    for i in (0..36).step_by(3) {
        let mut temp = 88isize;
        map = (api.hmget_ts)(
            map.cast(),
            size_of::<StringEntry>(),
            keys[i].as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            &mut temp,
            call_mode,
        )
        .cast();
        events.push(StringEvent::Get(temp));
    }
    let missing = CString::new("not_present").unwrap();
    map = (api.hmget)(
        map.cast(),
        size_of::<StringEntry>(),
        missing.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        call_mode,
    )
    .cast();
    events.push(StringEvent::Get((*header(map.sub(1).cast())).temp));

    if call_mode != 2 {
        for i in (1..31).step_by(2) {
            map = (api.hmdel)(
                map.cast(),
                size_of::<StringEntry>(),
                keys[i].as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                call_mode,
            )
            .cast();
            events.push(StringEvent::Delete((*header(map.sub(1).cast())).temp != 0));
            events.push(StringEvent::State(string_snapshot(map)));
        }
    }
    (api.hmfree)(map.sub(1).cast(), size_of::<StringEntry>());
    events
}

#[test]
fn string_map_modes_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        for &(initial, call_mode) in &[
            (None, 1),
            (None, 2),
            (Some(1), 1),
            (Some(2), 1),
            (Some(3), 1),
        ] {
            let c_trace = string_map_trace(&c, initial, call_mode);
            let rust_trace = string_map_trace(&rust, initial, call_mode);
            assert_eq!(
                c_trace, rust_trace,
                "initial_mode={initial:?} call_mode={call_mode}"
            );
        }
    }
}

#[test]
fn child_out_of_range_mode_delete() {
    let Some(path) = std::env::var_os("STBDS_ABORT_LIBRARY") else {
        return;
    };
    unsafe {
        let api = Api::open(Path::new(&path));
        let keys = [
            CString::new("first").unwrap(),
            CString::new("second").unwrap(),
        ];
        let mut map: *mut StringEntry = ptr::null_mut();
        for (i, key) in keys.iter().enumerate() {
            map = (api.hmput)(
                map.cast(),
                size_of::<StringEntry>(),
                key.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                2,
            )
            .cast();
            let index = (*header(map.sub(1).cast())).temp as usize;
            (*map.add(index)).value = i as i64;
        }
        let _ = (api.hmdel)(
            map.cast(),
            size_of::<StringEntry>(),
            keys[0].as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            2,
        );
        panic!("mode=2 non-last deletion unexpectedly returned");
    }
}

#[test]
fn out_of_range_mode_delete_rejection_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let executable = std::env::current_exe().unwrap();
    let (c_path, rust_path) = library_paths();
    let run = |path: &Path| {
        Command::new(&executable)
            .arg("child_out_of_range_mode_delete")
            .arg("--exact")
            .arg("--test-threads=1")
            .env("STBDS_ABORT_LIBRARY", path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
    };
    let c_status = run(&c_path);
    let rust_status = run(&rust_path);
    assert_eq!(c_status.signal(), Some(6));
    assert_eq!(rust_status.signal(), c_status.signal());
}

unsafe fn child_put_binary(api: &Api, map: *mut BinaryEntry, mut key: u64) -> *mut BinaryEntry {
    let map = (api.hmput)(
        map.cast(),
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
    )
    .cast::<BinaryEntry>();
    let index = (*header(map.sub(1).cast())).temp as usize;
    (*map.add(index)).value = key as i64;
    map
}

#[test]
fn child_corrupted_invariant() {
    let (Some(path), Some(case)) = (
        std::env::var_os("STBDS_ABORT_LIBRARY"),
        std::env::var_os("STBDS_ASSERT_CASE"),
    ) else {
        return;
    };
    unsafe {
        let api = Api::open(Path::new(&path));
        let mut map: *mut BinaryEntry = ptr::null_mut();
        match case.to_str().unwrap() {
            "make-index-threshold" => {
                map = child_put_binary(&api, map, 10);
                let table = (*header(map.sub(1).cast())).hash_table.cast::<HashIndex>();
                (*table).slot_count = 1;
                (*table).used_count_threshold = 0;
                let _ = child_put_binary(&api, map, 20);
            }
            "delete-slot-range" => {
                for key in 10..16 {
                    map = child_put_binary(&api, map, key);
                }
                let table = (*header(map.sub(1).cast())).hash_table.cast::<HashIndex>();
                let bucket = (*table).storage;
                let slot = (1..8)
                    .find(|&i| (*bucket).index[i] >= 0)
                    .expect("expected an occupied nonzero slot");
                let index = (*bucket).index[slot] as usize;
                let mut key = (*map.add(index)).key;
                (*table).slot_count = 1;
                let _ = (api.hmdel)(
                    map.cast(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                );
            }
            "delete-index-mismatch" => {
                map = child_put_binary(&api, map, 10);
                map = child_put_binary(&api, map, 20);
                map = child_put_binary(&api, map, 30);
                let table = (*header(map.sub(1).cast())).hash_table.cast::<HashIndex>();
                let mut index_two_slot = None;
                for i in 0..8 {
                    if (*(*table).storage).index[i] == 2 {
                        index_two_slot = Some(i);
                    }
                }
                let slot = index_two_slot.expect("missing final entry bucket");
                (*map.add(1)).key = (*map.add(2)).key;
                (*(*table).storage).index[slot] = 1;
                let mut key = (*map).key;
                let _ = (api.hmdel)(
                    map.cast(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                );
            }
            other => panic!("unknown assertion case {other}"),
        }
        panic!("corrupted invariant unexpectedly returned");
    }
}

#[test]
fn corrupted_invariant_rejections_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let executable = std::env::current_exe().unwrap();
    let (c_path, rust_path) = library_paths();
    for case in [
        "make-index-threshold",
        "delete-slot-range",
        "delete-index-mismatch",
    ] {
        let run = |path: &Path| {
            Command::new(&executable)
                .arg("child_corrupted_invariant")
                .arg("--exact")
                .arg("--test-threads=1")
                .env("STBDS_ABORT_LIBRARY", path)
                .env("STBDS_ASSERT_CASE", case)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
        };
        let c_status = run(&c_path);
        let rust_status = run(&rust_path);
        assert_eq!(c_status.signal(), Some(6), "C case {case}");
        assert_eq!(rust_status.signal(), c_status.signal(), "case {case}");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArenaEvent {
    returned: Vec<u8>,
    remaining: usize,
    block: u8,
    has_storage: bool,
}

unsafe fn arena_trace(api: &Api) -> Vec<ArenaEvent> {
    let mut arena = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut events = Vec::new();
    let mut rng = Rng::new(0x71c2_a993_5de0_4b8f);
    let mut lengths = vec![0, 1, 7, 127, 255, 511, 512, 513, 900, 1500];
    for power in 9..=20 {
        lengths.push((1usize << power) - 17);
        lengths.push((1usize << power) + 17);
    }
    for len in lengths {
        let mut bytes = Vec::with_capacity(len + 1);
        for _ in 0..len {
            bytes.push(b'a' + (rng.next() % 26) as u8);
        }
        bytes.push(0);
        let result = (api.stralloc)(&mut arena, bytes.as_mut_ptr().cast());
        events.push(ArenaEvent {
            returned: CStr::from_ptr(result).to_bytes().to_vec(),
            remaining: arena.remaining,
            block: arena.block,
            has_storage: !arena.storage.is_null(),
        });
    }
    (api.strreset)(&mut arena);
    events.push(ArenaEvent {
        returned: Vec::new(),
        remaining: arena.remaining,
        block: arena.block,
        has_storage: !arena.storage.is_null(),
    });
    (api.strreset)(&mut arena);
    events
}

#[test]
fn string_arena_and_strkey_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        assert_eq!(arena_trace(&c), arena_trace(&rust));
        for value in [c_int::MIN, -1_000_000, -1, 0, 1, 1_000_000, c_int::MAX] {
            let c_value = CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec();
            let rust_value = CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec();
            assert_eq!(c_value, rust_value, "strkey({value})");
        }
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [0; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let saved = dup(1);
    assert!(saved >= 0);
    fflush(ptr::null_mut());
    assert_eq!(dup2(fds[1], 1), 1);
    close(fds[1]);
    call();
    fflush(ptr::null_mut());
    assert_eq!(dup2(saved, 1), 1);
    close(saved);
    let mut output = Vec::new();
    File::from_raw_fd(fds[0]).read_to_end(&mut output).unwrap();
    output
}

#[test]
fn composed_str_put_output_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = apis();
        for value in [-100, -1, 0, 1, 4, 100, 2_000] {
            let c_output = capture_stdout(|| (c.str_put)(value));
            let rust_output = capture_stdout(|| (rust.str_put)(value));
            assert_eq!(c_output, rust_output, "str_put({value})");
        }
    }
}
