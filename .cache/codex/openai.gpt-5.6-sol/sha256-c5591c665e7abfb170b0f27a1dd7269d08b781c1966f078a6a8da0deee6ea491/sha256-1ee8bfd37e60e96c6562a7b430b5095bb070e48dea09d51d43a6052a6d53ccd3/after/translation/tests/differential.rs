#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type StrPut = unsafe extern "C" fn(c_int);

#[derive(Clone, Copy)]
struct Api {
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_string: HashString,
    hash_bytes: HashBytes,
    hmfree: HmFree,
    hmget_ts: HmGetTs,
    hmget: HmGet,
    hmput_default: HmPutDefault,
    hmput: HmPut,
    shmode: ShMode,
    hmdel: HmDel,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    str_put: StrPut,
}

impl Api {
    unsafe fn load(library: &Library) -> Self {
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap()
            };
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_string: symbol!("stbds_hash_string", HashString),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            shmode: symbol!("stbds_shmode_func", ShMode),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            str_put: symbol!("str_put", StrPut),
        }
    }
}

struct Libraries {
    _c: Library,
    _rust: Library,
    c: Api,
    rust: Api,
}

fn find_rust_library() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let candidates = [
        profile.join("libstr_put_lib.so"),
        deps.join("libstr_put_lib.so"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libstr_put_lib.so"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .expect("Rust cdylib was not built")
}

fn libraries() -> &'static Libraries {
    static LIBRARIES: OnceLock<Libraries> = OnceLock::new();
    LIBRARIES.get_or_init(|| unsafe {
        let c_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-gSZO9L.so");
        let rust_path = find_rust_library();
        let c = Library::new(&c_path)
            .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
        let rust = Library::new(&rust_path)
            .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
        let c_api = Api::load(&c);
        let rust_api = Api::load(&rust);
        Libraries {
            _c: c,
            _rust: rust,
            c: c_api,
            rust: rust_api,
        }
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Arena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Bucket {
    hash: [usize; 8],
    index: [isize; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    string: Arena,
    storage: *mut Bucket,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BinaryEntry {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

unsafe fn header(data: *mut c_void) -> *mut Header {
    (data as *mut u8).sub(size_of::<Header>()) as *mut Header
}

unsafe fn map_raw(map: *mut c_void, elemsize: usize) -> *mut c_void {
    (map as *mut u8).sub(elemsize) as *mut c_void
}

unsafe fn map_header(map: *mut c_void, elemsize: usize) -> *mut Header {
    header(map_raw(map, elemsize))
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
    buckets: Vec<([usize; 8], [isize; 8])>,
}

unsafe fn table_snapshot(map: *mut c_void, elemsize: usize) -> Option<TableSnapshot> {
    let table = (*map_header(map, elemsize)).hash_table as *mut HashIndex;
    if table.is_null() {
        return None;
    }
    let bucket_count = (*table).slot_count / 8;
    let buckets = (0..bucket_count)
        .map(|index| {
            let bucket = *(*table).storage.add(index);
            (bucket.hash, bucket.index)
        })
        .collect();
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

#[derive(Debug, PartialEq, Eq)]
struct BinaryMapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    table: Option<TableSnapshot>,
    entries: Vec<BinaryEntry>,
}

unsafe fn binary_snapshot(map: *mut BinaryEntry) -> BinaryMapSnapshot {
    let hdr = &*map_header(map.cast(), size_of::<BinaryEntry>());
    BinaryMapSnapshot {
        length: hdr.length - 1,
        capacity: hdr.capacity,
        temp: hdr.temp,
        table: table_snapshot(map.cast(), size_of::<BinaryEntry>()),
        entries: (0..hdr.length - 1).map(|i| *map.add(i)).collect(),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StringMapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    table: Option<TableSnapshot>,
    entries: Vec<(Vec<u8>, i64)>,
}

unsafe fn string_snapshot(map: *mut StringEntry) -> StringMapSnapshot {
    let hdr = &*map_header(map.cast(), size_of::<StringEntry>());
    StringMapSnapshot {
        length: hdr.length - 1,
        capacity: hdr.capacity,
        temp: hdr.temp,
        table: table_snapshot(map.cast(), size_of::<StringEntry>()),
        entries: (0..hdr.length - 1)
            .map(|i| {
                let entry = &*map.add(i);
                (CStr::from_ptr(entry.key).to_bytes().to_vec(), entry.value)
            })
            .collect(),
    }
}

#[derive(Clone, Copy)]
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

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next_u64() as u8).collect()
    }
}

fn global_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn assert_docs_have_rows() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (name, expected) in [("SYMBOLS.md", 16), ("ERRORS.md", 25), ("CONFIGS.md", 76)] {
        let text = fs::read_to_string(manifest.join(name)).unwrap();
        let count = text.lines().filter(|line| {
            line.starts_with("| ")
                && line
                    .as_bytes()
                    .get(2)
                    .is_some_and(|byte| byte.is_ascii_digit())
        });
        assert_eq!(count.count(), expected, "{name} row count");
    }
}

#[test]
fn arrays_hashes_and_strings_match() {
    let _guard = global_lock();
    assert_docs_have_rows();
    let libs = libraries();
    unsafe {
        // CONFIGS 1-6: all arrgrow capacity-selection branches and arrfree.
        assert!((libs.c.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        assert!((libs.rust.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        let mut c_array = (libs.c.arrgrow)(ptr::null_mut(), 4, 1, 0);
        let mut r_array = (libs.rust.arrgrow)(ptr::null_mut(), 4, 1, 0);
        assert_eq!((*header(c_array)).capacity, 4);
        assert_eq!((*header(c_array)).capacity, (*header(r_array)).capacity);
        (*header(c_array)).length = 3;
        (*header(r_array)).length = 3;
        for i in 0..3 {
            *(c_array as *mut u32).add(i) = (i as u32) * 17 + 5;
            *(r_array as *mut u32).add(i) = (i as u32) * 17 + 5;
        }

        let old_c = c_array;
        let old_r = r_array;
        c_array = (libs.c.arrgrow)(c_array, 4, 0, 4);
        r_array = (libs.rust.arrgrow)(r_array, 4, 0, 4);
        assert_eq!(c_array, old_c);
        assert_eq!(r_array, old_r);

        c_array = (libs.c.arrgrow)(c_array, 4, 2, 0);
        r_array = (libs.rust.arrgrow)(r_array, 4, 2, 0);
        assert_eq!((*header(c_array)).capacity, 8);
        assert_eq!((*header(c_array)).capacity, (*header(r_array)).capacity);
        c_array = (libs.c.arrgrow)(c_array, 4, 0, 37);
        r_array = (libs.rust.arrgrow)(r_array, 4, 0, 37);
        assert_eq!((*header(c_array)).capacity, 37);
        assert_eq!((*header(c_array)).capacity, (*header(r_array)).capacity);
        assert_eq!(
            std::slice::from_raw_parts(c_array as *const u32, 3),
            std::slice::from_raw_parts(r_array as *const u32, 3)
        );
        (libs.c.arrfree)(c_array);
        (libs.rust.arrfree)(r_array);

        let c_addlen = (libs.c.arrgrow)(ptr::null_mut(), 7, 11, 0);
        let r_addlen = (libs.rust.arrgrow)(ptr::null_mut(), 7, 11, 0);
        assert_eq!((*header(c_addlen)).capacity, 11);
        assert_eq!((*header(c_addlen)).capacity, (*header(r_addlen)).capacity);
        (libs.c.arrfree)(c_addlen);
        (libs.rust.arrfree)(r_addlen);

        // CONFIGS 9-22: every string shape and every SipHash tail arm.
        let mut rng = Rng::new(0x5eed_f00d_dead_beef);
        for length in 0..=160 {
            for _ in 0..24 {
                let mut bytes = rng.bytes(length);
                let seed = rng.next_u64() as usize;
                let c_hash = (libs.c.hash_bytes)(bytes.as_mut_ptr().cast(), bytes.len(), seed);
                let r_hash = (libs.rust.hash_bytes)(bytes.as_mut_ptr().cast(), bytes.len(), seed);
                assert_eq!(c_hash, r_hash, "hash_bytes length={length} seed={seed:#x}");
            }
        }
        let c_empty = (libs.c.hash_bytes)(ptr::null_mut(), 0, 123);
        let r_empty = (libs.rust.hash_bytes)(ptr::null_mut(), 0, 123);
        assert_eq!(c_empty, r_empty);

        let explicit_strings = [
            vec![],
            vec![b'a'],
            b"the quick brown fox jumps over the lazy dog".to_vec(),
            vec![0x80, 0x9f, 0xfe, 0xff],
        ];
        for bytes in explicit_strings {
            for seed in [0, 1, usize::MAX, 0x3141_5926] {
                let mut terminated = bytes.clone();
                terminated.push(0);
                let c_hash = (libs.c.hash_string)(terminated.as_mut_ptr().cast(), seed);
                let r_hash = (libs.rust.hash_string)(terminated.as_mut_ptr().cast(), seed);
                assert_eq!(c_hash, r_hash, "hash_string bytes={bytes:?} seed={seed:#x}");
            }
        }
        for _ in 0..512 {
            let length = (rng.next_u64() % 96) as usize;
            let mut bytes: Vec<u8> = rng
                .bytes(length)
                .into_iter()
                .map(|byte| if byte == 0 { 1 } else { byte })
                .collect();
            bytes.push(0);
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (libs.c.hash_string)(bytes.as_mut_ptr().cast(), seed),
                (libs.rust.hash_string)(bytes.as_mut_ptr().cast(), seed)
            );
        }

        // CONFIGS 62-68: arena regular blocks, dedicated blocks, and reset.
        let mut c_arena = Arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut r_arena = c_arena;
        let mut arena_inputs = vec![
            CString::new("").unwrap(),
            CString::new("a").unwrap(),
            CString::new("small randomized payload").unwrap(),
            CString::new(vec![b'x'; 700]).unwrap(),
        ];
        for index in 0..180 {
            let length = 1 + index % 79;
            arena_inputs.push(CString::new(vec![b'a' + (index % 26) as u8; length]).unwrap());
        }
        arena_inputs.push(CString::new(vec![b'z'; (1 << 20) + 17]).unwrap());

        for input in &arena_inputs {
            let c_result = (libs.c.stralloc)(&mut c_arena, input.as_ptr().cast_mut());
            let r_result = (libs.rust.stralloc)(&mut r_arena, input.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(c_result).to_bytes(), input.as_bytes());
            assert_eq!(CStr::from_ptr(r_result).to_bytes(), input.as_bytes());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(
                (c_arena.remaining, c_arena.block, c_arena.mode),
                (r_arena.remaining, r_arena.block, r_arena.mode)
            );
        }
        (libs.c.strreset)(&mut c_arena);
        (libs.rust.strreset)(&mut r_arena);
        assert_eq!(
            (
                c_arena.storage,
                c_arena.remaining,
                c_arena.block,
                c_arena.mode
            ),
            (ptr::null_mut(), 0, 0, 0)
        );
        assert_eq!(
            (
                r_arena.storage,
                r_arena.remaining,
                r_arena.block,
                r_arena.mode
            ),
            (ptr::null_mut(), 0, 0, 0)
        );
        let mut empty_c = Arena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut empty_r = empty_c;
        (libs.c.strreset)(&mut empty_c);
        (libs.rust.strreset)(&mut empty_r);
        assert_eq!(empty_c.remaining, empty_r.remaining);

        // CONFIGS 69-72: all decimal sign and int boundary shapes.
        for number in [c_int::MIN, -1_234_567, -1, 0, 1, 42, 1_234_567, c_int::MAX] {
            let c_value = CStr::from_ptr((libs.c.strkey)(number)).to_bytes().to_vec();
            let r_value = CStr::from_ptr((libs.rust.strkey)(number))
                .to_bytes()
                .to_vec();
            assert_eq!(c_value, r_value, "strkey({number})");
            assert_eq!(c_value, format!("test_{number}").as_bytes());
        }
    }
}

unsafe fn set_binary_value(map: *mut BinaryEntry, value: i64) {
    let index = (*map_header(map.cast(), size_of::<BinaryEntry>())).temp as usize;
    (*map.add(index)).value = value;
}

unsafe fn set_string_value(map: *mut StringEntry, value: i64) {
    let index = (*map_header(map.cast(), size_of::<StringEntry>())).temp as usize;
    (*map.add(index)).value = value;
}

#[test]
fn map_lifecycle_and_modes_match() {
    let _guard = global_lock();
    let libs = libraries();
    unsafe {
        let binary_size = size_of::<BinaryEntry>();

        // CONFIGS 27-37 and ERRORS 6-12: null/default/miss sentinel behavior.
        (libs.c.hmfree)(ptr::null_mut(), binary_size);
        (libs.rust.hmfree)(ptr::null_mut(), binary_size);
        assert_eq!(
            (libs.c.hmdel)(ptr::null_mut(), binary_size, ptr::null_mut(), 0, 0, 0),
            ptr::null_mut()
        );
        assert_eq!(
            (libs.rust.hmdel)(ptr::null_mut(), binary_size, ptr::null_mut(), 0, 0, 0),
            ptr::null_mut()
        );

        let mut key = 0x1234_5678_9abc_def0u64;
        let mut c_temp = 99isize;
        let mut r_temp = 99isize;
        let mut c_map = (libs.c.hmget_ts)(
            ptr::null_mut(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        ) as *mut BinaryEntry;
        let mut r_map = (libs.rust.hmget_ts)(
            ptr::null_mut(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut r_temp,
            0,
        ) as *mut BinaryEntry;
        assert_eq!((c_temp, r_temp), (-1, -1));
        assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));

        c_temp = 99;
        r_temp = 99;
        c_map = (libs.c.hmget_ts)(
            c_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        ) as *mut BinaryEntry;
        r_map = (libs.rust.hmget_ts)(
            r_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut r_temp,
            0,
        ) as *mut BinaryEntry;
        assert_eq!((c_temp, r_temp), (-1, -1));
        assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));
        c_map = (libs.c.hmget)(
            c_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        ) as *mut BinaryEntry;
        r_map = (libs.rust.hmget)(
            r_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        ) as *mut BinaryEntry;
        assert_eq!((*map_header(c_map.cast(), binary_size)).temp, -1);
        assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));

        let c_before = c_map;
        let r_before = r_map;
        c_map = (libs.c.hmdel)(
            c_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        ) as *mut BinaryEntry;
        r_map = (libs.rust.hmdel)(
            r_map.cast(),
            binary_size,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        ) as *mut BinaryEntry;
        assert_eq!(c_map, c_before);
        assert_eq!(r_map, r_before);
        assert_eq!((*map_header(c_map.cast(), binary_size)).temp, 0);
        assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));
        (libs.c.hmfree)(map_raw(c_map.cast(), binary_size), binary_size);
        (libs.rust.hmfree)(map_raw(r_map.cast(), binary_size), binary_size);

        let c_default = (libs.c.hmput_default)(ptr::null_mut(), binary_size) as *mut BinaryEntry;
        let r_default = (libs.rust.hmput_default)(ptr::null_mut(), binary_size) as *mut BinaryEntry;
        assert_eq!(binary_snapshot(c_default), binary_snapshot(r_default));
        assert_eq!(
            (*map_raw(c_default.cast(), binary_size).cast::<BinaryEntry>()).key,
            0
        );
        assert_eq!(
            (*map_raw(r_default.cast(), binary_size).cast::<BinaryEntry>()).key,
            0
        );
        let c_same = (libs.c.hmput_default)(c_default.cast(), binary_size);
        let r_same = (libs.rust.hmput_default)(r_default.cast(), binary_size);
        assert_eq!(c_same, c_default.cast());
        assert_eq!(r_same, r_default.cast());
        (libs.c.hmfree)(map_raw(c_default.cast(), binary_size), binary_size);
        (libs.rust.hmfree)(map_raw(r_default.cast(), binary_size), binary_size);

        let c_raw = (libs.c.arrgrow)(ptr::null_mut(), binary_size, 1, 0);
        let r_raw = (libs.rust.arrgrow)(ptr::null_mut(), binary_size, 1, 0);
        let c_empty_map = (c_raw as *mut u8).add(binary_size).cast::<BinaryEntry>();
        let r_empty_map = (r_raw as *mut u8).add(binary_size).cast::<BinaryEntry>();
        let c_from_empty =
            (libs.c.hmput_default)(c_empty_map.cast(), binary_size) as *mut BinaryEntry;
        let r_from_empty =
            (libs.rust.hmput_default)(r_empty_map.cast(), binary_size) as *mut BinaryEntry;
        assert_eq!(binary_snapshot(c_from_empty), binary_snapshot(r_from_empty));
        (libs.c.hmfree)(map_raw(c_from_empty.cast(), binary_size), binary_size);
        (libs.rust.hmfree)(map_raw(r_from_empty.cast(), binary_size), binary_size);

        // CONFIGS 7-8, 23, 30, 33, 38-40, 45-46, 53-61.
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0xfeed_face_cafe_beef] {
            (libs.c.rand_seed)(seed);
            (libs.rust.rand_seed)(seed);
            let mut c_map: *mut BinaryEntry = ptr::null_mut();
            let mut r_map: *mut BinaryEntry = ptr::null_mut();
            let mut keys = Vec::new();
            let mut rng = Rng::new(seed as u64 ^ 0xa11c_e55);
            for index in 0..96 {
                let mut key = rng.next_u64();
                while keys.contains(&key) {
                    key = rng.next_u64();
                }
                keys.push(key);
                let mode = if index == 0 { -7 } else { 0 };
                c_map = (libs.c.hmput)(
                    c_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    mode,
                ) as *mut BinaryEntry;
                r_map = (libs.rust.hmput)(
                    r_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    mode,
                ) as *mut BinaryEntry;
                set_binary_value(c_map, index as i64 * -31);
                set_binary_value(r_map, index as i64 * -31);
                assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));
            }

            for &index in &[0usize, 1, 5, 31, 63, 95] {
                let mut key = keys[index];
                let mut c_temp = -9;
                let mut r_temp = -9;
                c_map = (libs.c.hmget_ts)(
                    c_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    &mut c_temp,
                    0,
                ) as *mut BinaryEntry;
                r_map = (libs.rust.hmget_ts)(
                    r_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    &mut r_temp,
                    0,
                ) as *mut BinaryEntry;
                assert_eq!(c_temp, r_temp);
                assert_eq!(
                    (*c_map.add(c_temp as usize)).value,
                    (*r_map.add(r_temp as usize)).value
                );

                c_map = (libs.c.hmput)(
                    c_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                ) as *mut BinaryEntry;
                r_map = (libs.rust.hmput)(
                    r_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                ) as *mut BinaryEntry;
                set_binary_value(c_map, index as i64 + 10_000);
                set_binary_value(r_map, index as i64 + 10_000);
                assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));
            }

            let mut absent = 0xdead_beef_dead_beefu64;
            while keys.contains(&absent) {
                absent = absent.wrapping_add(1);
            }
            let c_before = c_map;
            let r_before = r_map;
            c_map = (libs.c.hmdel)(
                c_map.cast(),
                binary_size,
                (&mut absent as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            ) as *mut BinaryEntry;
            r_map = (libs.rust.hmdel)(
                r_map.cast(),
                binary_size,
                (&mut absent as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            ) as *mut BinaryEntry;
            assert_eq!(c_map, c_before);
            assert_eq!(r_map, r_before);
            assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));

            for index in (0..keys.len()).filter(|i| i % 3 == 0 || *i == 94) {
                let mut key = keys[index];
                let delete_mode = if index == 0 { -7 } else { 0 };
                c_map = (libs.c.hmdel)(
                    c_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                    delete_mode,
                ) as *mut BinaryEntry;
                r_map = (libs.rust.hmdel)(
                    r_map.cast(),
                    binary_size,
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                    delete_mode,
                ) as *mut BinaryEntry;
                assert_eq!(binary_snapshot(c_map), binary_snapshot(r_map));
            }
            (libs.c.hmfree)(map_raw(c_map.cast(), binary_size), binary_size);
            (libs.rust.hmfree)(map_raw(r_map.cast(), binary_size), binary_size);
        }

        // A small table crosses the tombstone rebuild threshold without growing.
        (libs.c.rand_seed)(777);
        (libs.rust.rand_seed)(777);
        let mut c_small: *mut BinaryEntry = ptr::null_mut();
        let mut r_small: *mut BinaryEntry = ptr::null_mut();
        for key_value in 10u64..15 {
            let mut key = key_value;
            c_small = (libs.c.hmput)(
                c_small.cast(),
                binary_size,
                (&mut key as *mut u64).cast(),
                8,
                0,
            ) as *mut BinaryEntry;
            r_small = (libs.rust.hmput)(
                r_small.cast(),
                binary_size,
                (&mut key as *mut u64).cast(),
                8,
                0,
            ) as *mut BinaryEntry;
            set_binary_value(c_small, key_value as i64 * 17);
            set_binary_value(r_small, key_value as i64 * 17);
            assert_eq!(binary_snapshot(c_small), binary_snapshot(r_small));
        }
        let mut deleted_key = 11u64;
        c_small = (libs.c.hmdel)(
            c_small.cast(),
            binary_size,
            (&mut deleted_key as *mut u64).cast(),
            8,
            0,
            0,
        ) as *mut BinaryEntry;
        r_small = (libs.rust.hmdel)(
            r_small.cast(),
            binary_size,
            (&mut deleted_key as *mut u64).cast(),
            8,
            0,
            0,
        ) as *mut BinaryEntry;
        assert_eq!(binary_snapshot(c_small), binary_snapshot(r_small));
        let small_table = binary_snapshot(c_small).table.unwrap();
        let deleted_slot = small_table.buckets[0]
            .1
            .iter()
            .position(|index| *index == -2)
            .unwrap();
        let mut replacement_key = 1000u64;
        loop {
            let mut candidate = replacement_key;
            let mut hash = (libs.c.hash_bytes)((&mut candidate as *mut u64).cast(), 8, 777);
            if hash < 2 {
                hash += 2;
            }
            if hash & 7 == deleted_slot {
                break;
            }
            replacement_key += 1;
        }
        c_small = (libs.c.hmput)(
            c_small.cast(),
            binary_size,
            (&mut replacement_key as *mut u64).cast(),
            8,
            0,
        ) as *mut BinaryEntry;
        r_small = (libs.rust.hmput)(
            r_small.cast(),
            binary_size,
            (&mut replacement_key as *mut u64).cast(),
            8,
            0,
        ) as *mut BinaryEntry;
        set_binary_value(c_small, 9999);
        set_binary_value(r_small, 9999);
        assert_eq!(binary_snapshot(c_small), binary_snapshot(r_small));
        assert_eq!(binary_snapshot(c_small).table.unwrap().tombstone_count, 0);

        for key_value in [13u64, 12] {
            let mut key = key_value;
            c_small = (libs.c.hmdel)(
                c_small.cast(),
                binary_size,
                (&mut key as *mut u64).cast(),
                8,
                0,
                0,
            ) as *mut BinaryEntry;
            r_small = (libs.rust.hmdel)(
                r_small.cast(),
                binary_size,
                (&mut key as *mut u64).cast(),
                8,
                0,
                0,
            ) as *mut BinaryEntry;
            assert_eq!(binary_snapshot(c_small), binary_snapshot(r_small));
        }
        assert_eq!(binary_snapshot(c_small).table.unwrap().tombstone_count, 0);
        (libs.c.hmfree)(map_raw(c_small.cast(), binary_size), binary_size);
        (libs.rust.hmfree)(map_raw(r_small.cast(), binary_size), binary_size);

        // Delete the sole/last element.
        let mut sole_key = 42u64;
        let mut c_sole = (libs.c.hmput)(
            ptr::null_mut(),
            binary_size,
            (&mut sole_key as *mut u64).cast(),
            8,
            0,
        ) as *mut BinaryEntry;
        let mut r_sole = (libs.rust.hmput)(
            ptr::null_mut(),
            binary_size,
            (&mut sole_key as *mut u64).cast(),
            8,
            0,
        ) as *mut BinaryEntry;
        set_binary_value(c_sole, 42);
        set_binary_value(r_sole, 42);
        c_sole = (libs.c.hmdel)(
            c_sole.cast(),
            binary_size,
            (&mut sole_key as *mut u64).cast(),
            8,
            0,
            0,
        ) as *mut BinaryEntry;
        r_sole = (libs.rust.hmdel)(
            r_sole.cast(),
            binary_size,
            (&mut sole_key as *mut u64).cast(),
            8,
            0,
            0,
        ) as *mut BinaryEntry;
        assert_eq!(binary_snapshot(c_sole), binary_snapshot(r_sole));
        assert_eq!(binary_snapshot(c_sole).length, 0);
        (libs.c.hmfree)(map_raw(c_sole.cast(), binary_size), binary_size);
        (libs.rust.hmfree)(map_raw(r_sole.cast(), binary_size), binary_size);

        // CONFIGS 24-26, 31, 34, 41-44, 47-52, 58.
        let string_size = size_of::<StringEntry>();
        for ownership_mode in [0, 1, 2, 3, 4, 255] {
            (libs.c.rand_seed)(0xabc0 + ownership_mode as usize);
            (libs.rust.rand_seed)(0xabc0 + ownership_mode as usize);
            let mut c_map = (libs.c.shmode)(string_size, ownership_mode) as *mut StringEntry;
            let mut r_map = (libs.rust.shmode)(string_size, ownership_mode) as *mut StringEntry;
            assert_eq!(string_snapshot(c_map), string_snapshot(r_map));

            let strings: Vec<CString> = (0..24)
                .map(|i| CString::new(format!("mode_{ownership_mode}_key_{i:03}")).unwrap())
                .collect();
            for (index, key) in strings.iter().enumerate() {
                let binary_storage = !matches!(ownership_mode, 1..=3);
                let call_mode = if binary_storage { 0 } else { 1 };
                let mut key_pointer = key.as_ptr().cast_mut();
                let key_arg = if binary_storage {
                    (&mut key_pointer as *mut *mut c_char).cast()
                } else {
                    key.as_ptr().cast_mut().cast()
                };
                c_map = (libs.c.hmput)(
                    c_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    call_mode,
                ) as *mut StringEntry;
                r_map = (libs.rust.hmput)(
                    r_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    call_mode,
                ) as *mut StringEntry;
                set_string_value(c_map, index as i64 * 101);
                set_string_value(r_map, index as i64 * 101);
                assert_eq!(string_snapshot(c_map), string_snapshot(r_map));
            }

            for &index in &[0usize, 7, 23] {
                let key = &strings[index];
                let binary_storage = !matches!(ownership_mode, 1..=3);
                let call_mode = if binary_storage { 0 } else { 1 };
                let mut key_pointer = key.as_ptr().cast_mut();
                let key_arg = if binary_storage {
                    (&mut key_pointer as *mut *mut c_char).cast()
                } else {
                    key.as_ptr().cast_mut().cast()
                };
                let mut c_temp = -99;
                let mut r_temp = -99;
                c_map = (libs.c.hmget_ts)(
                    c_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    call_mode,
                ) as *mut StringEntry;
                r_map = (libs.rust.hmget_ts)(
                    r_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    &mut r_temp,
                    call_mode,
                ) as *mut StringEntry;
                assert_eq!(c_temp, r_temp);
                c_map = (libs.c.hmget)(
                    c_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    call_mode,
                ) as *mut StringEntry;
                r_map = (libs.rust.hmget)(
                    r_map.cast(),
                    string_size,
                    key_arg,
                    size_of::<*mut c_char>(),
                    call_mode,
                ) as *mut StringEntry;
                assert_eq!(string_snapshot(c_map), string_snapshot(r_map));
            }

            let delete_key = &strings[5];
            let binary_storage = !matches!(ownership_mode, 1..=3);
            let call_mode = if binary_storage { 0 } else { 1 };
            let mut delete_pointer = delete_key.as_ptr().cast_mut();
            let delete_arg = if binary_storage {
                (&mut delete_pointer as *mut *mut c_char).cast()
            } else {
                delete_key.as_ptr().cast_mut().cast()
            };
            c_map = (libs.c.hmdel)(
                c_map.cast(),
                string_size,
                delete_arg,
                size_of::<*mut c_char>(),
                0,
                call_mode,
            ) as *mut StringEntry;
            r_map = (libs.rust.hmdel)(
                r_map.cast(),
                string_size,
                delete_arg,
                size_of::<*mut c_char>(),
                0,
                call_mode,
            ) as *mut StringEntry;
            assert_eq!(string_snapshot(c_map), string_snapshot(r_map));
            (libs.c.hmfree)(map_raw(c_map.cast(), string_size), string_size);
            (libs.rust.hmfree)(map_raw(r_map.cast(), string_size), string_size);
        }

        let out_of_range_key = CString::new("positive_out_of_range_mode").unwrap();
        let mut c_out: *mut StringEntry = ptr::null_mut();
        let mut r_out: *mut StringEntry = ptr::null_mut();
        c_out = (libs.c.hmput)(
            c_out.cast(),
            string_size,
            out_of_range_key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            99,
        ) as *mut StringEntry;
        r_out = (libs.rust.hmput)(
            r_out.cast(),
            string_size,
            out_of_range_key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            99,
        ) as *mut StringEntry;
        set_string_value(c_out, 123);
        set_string_value(r_out, 123);
        assert_eq!(string_snapshot(c_out), string_snapshot(r_out));
        c_out = (libs.c.hmdel)(
            c_out.cast(),
            string_size,
            out_of_range_key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            99,
        ) as *mut StringEntry;
        r_out = (libs.rust.hmdel)(
            r_out.cast(),
            string_size,
            out_of_range_key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            99,
        ) as *mut StringEntry;
        assert_eq!(string_snapshot(c_out), string_snapshot(r_out));
        (libs.c.hmfree)(map_raw(c_out.cast(), string_size), string_size);
        (libs.rust.hmfree)(map_raw(r_out.cast(), string_size), string_size);
    }
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [-1; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    assert_eq!(fflush(ptr::null_mut()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(fds[1], 1), 1);
    assert_eq!(close(fds[1]), 0);

    call();

    assert_eq!(fflush(ptr::null_mut()), 0);
    assert_eq!(dup2(saved_stdout, 1), 1);
    assert_eq!(close(saved_stdout), 0);

    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(close(fds[0]), 0);
    output
}

#[test]
fn str_put_output_matches() {
    let _guard = global_lock();
    let libs = libraries();
    unsafe {
        // CONFIGS 73-76 and the three str_put internal postcondition assertions.
        let mut rng = Rng::new(0x55aa_1234);
        let mut counts = vec![-10, -1, 0, 1, 2, 2000];
        counts.extend((0..64).map(|_| (rng.next_u64() % 500) as c_int));
        for count in counts {
            let c_output = capture_stdout(|| (libs.c.str_put)(count));
            let rust_output = capture_stdout(|| (libs.rust.str_put)(count));
            assert_eq!(c_output, rust_output, "str_put({count})");
            assert_eq!(c_output, format!("a {count}\n").as_bytes());
        }
    }
}

#[test]
fn fault_child_entry() {
    let Some(case) = std::env::var_os("DIFFERENTIAL_FAULT_CASE") else {
        return;
    };
    let use_rust = std::env::var_os("DIFFERENTIAL_FAULT_LIBRARY").unwrap() == "rust";
    let libs = libraries();
    let api = if use_rust { libs.rust } else { libs.c };
    let case = case.to_string_lossy();
    unsafe {
        match case.as_ref() {
            "arrgrow_oversized" => {
                (api.arrgrow)(ptr::null_mut(), 1, 0, usize::MAX - 64);
            }
            "arrfree_null" => (api.arrfree)(ptr::null_mut()),
            "hash_string_null" => {
                (api.hash_string)(ptr::null_mut(), 0);
            }
            "hash_bytes_null_positive" => {
                (api.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hash_bytes_oversized" => {
                let mut byte = 0u8;
                (api.hash_bytes)((&mut byte as *mut u8).cast(), usize::MAX, 0);
            }
            "hmget_temp_null" => {
                (api.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    0,
                );
            }
            "hmput_null_binary_key" => {
                (api.hmput)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    1,
                    0,
                );
            }
            "hmput_null_string_key" => {
                (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    1,
                );
            }
            "stralloc_null_arena" => {
                let string = CString::new("x").unwrap();
                (api.stralloc)(ptr::null_mut(), string.as_ptr().cast_mut());
            }
            "stralloc_null_string" => {
                let mut arena = Arena {
                    storage: ptr::null_mut(),
                    remaining: 0,
                    block: 0,
                    mode: 0,
                };
                (api.stralloc)(&mut arena, ptr::null_mut());
            }
            "strreset_null" => (api.strreset)(ptr::null_mut()),
            unknown => panic!("unknown fault case {unknown}"),
        }
    }
}

#[cfg(unix)]
fn termination(status: std::process::ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

fn run_fault_child(case: &str, library: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("fault_child_entry")
        .arg("--exact")
        .arg("--nocapture")
        .env("DIFFERENTIAL_FAULT_CASE", case)
        .env("DIFFERENTIAL_FAULT_LIBRARY", library)
        .status()
        .unwrap()
}

#[test]
fn invalid_inputs_match_process_results() {
    let _guard = global_lock();
    // ERRORS 1-3, 5, 8, and 13-15, plus generic null key boundaries.
    for case in [
        "arrgrow_oversized",
        "arrfree_null",
        "hash_string_null",
        "hash_bytes_null_positive",
        "hash_bytes_oversized",
        "hmget_temp_null",
        "hmput_null_binary_key",
        "hmput_null_string_key",
        "stralloc_null_arena",
        "stralloc_null_string",
        "strreset_null",
    ] {
        let c_status = run_fault_child(case, "c");
        let rust_status = run_fault_child(case, "rust");
        assert!(!c_status.success(), "C unexpectedly accepted {case}");
        assert!(!rust_status.success(), "Rust unexpectedly accepted {case}");
        assert_eq!(
            termination(c_status),
            termination(rust_status),
            "different process rejection for {case}"
        );
    }

    // ERRORS 16-25 are internal invariants. Verify every assertion remains
    // cataloged; the valid randomized tests above execute their reachable sites.
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"))
            .unwrap();
    let assert_count = source
        .lines()
        .filter(|line| line.contains("STBDS_ASSERT(") && !line.starts_with("#define"))
        .count();
    assert_eq!(assert_count, 10);
}
