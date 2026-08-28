use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::Mutex;

const BUCKET_LENGTH: usize = 8;
const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;

static PROCESS_STATE: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
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
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
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
    string: StringArena,
    storage: *mut HashBucket,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
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
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ShGeti = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
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
    sh_geti: ShGeti,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
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
            sh_geti: symbol!("sh_geti", ShGeti),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_apis() -> (Api, Api) {
    let root = manifest_dir();
    let c_path = root.join("../c_src/build/libharvest-work-punw4N.so");
    let rust_path = root.join("target/release/libsh_geti_lib.so");
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}; run cargo build --release",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

#[derive(Clone)]
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

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
}

unsafe fn raw_map(map: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(elem_size).cast() }
}

unsafe fn map_header(map: *mut c_void, elem_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, elem_size)) }
}

unsafe fn map_table(map: *mut c_void, elem_size: usize) -> *mut HashIndex {
    unsafe { (*map_header(map, elem_size)).hash_table.cast() }
}

#[derive(Debug, PartialEq, Eq)]
struct TableSnapshot {
    header_length: usize,
    header_capacity: usize,
    header_temp: isize,
    slot_count: Option<usize>,
    used_count: Option<usize>,
    used_threshold: Option<usize>,
    shrink_threshold: Option<usize>,
    tombstone_count: Option<usize>,
    tombstone_threshold: Option<usize>,
    seed: Option<usize>,
    slot_count_log2: Option<usize>,
    string_remaining: Option<usize>,
    string_block: Option<u8>,
    string_mode: Option<u8>,
    buckets: Vec<HashBucket>,
}

unsafe fn table_snapshot(map: *mut c_void, elem_size: usize) -> TableSnapshot {
    let array_header = unsafe { &*map_header(map, elem_size) };
    let table = array_header.hash_table.cast::<HashIndex>();
    if table.is_null() {
        return TableSnapshot {
            header_length: array_header.length,
            header_capacity: array_header.capacity,
            header_temp: array_header.temp,
            slot_count: None,
            used_count: None,
            used_threshold: None,
            shrink_threshold: None,
            tombstone_count: None,
            tombstone_threshold: None,
            seed: None,
            slot_count_log2: None,
            string_remaining: None,
            string_block: None,
            string_mode: None,
            buckets: Vec::new(),
        };
    }
    let table = unsafe { &*table };
    let bucket_count = table.slot_count / BUCKET_LENGTH;
    TableSnapshot {
        header_length: array_header.length,
        header_capacity: array_header.capacity,
        header_temp: array_header.temp,
        slot_count: Some(table.slot_count),
        used_count: Some(table.used_count),
        used_threshold: Some(table.used_count_threshold),
        shrink_threshold: Some(table.used_count_shrink_threshold),
        tombstone_count: Some(table.tombstone_count),
        tombstone_threshold: Some(table.tombstone_count_threshold),
        seed: Some(table.seed),
        slot_count_log2: Some(table.slot_count_log2),
        string_remaining: Some(table.string.remaining),
        string_block: Some(table.string.block),
        string_mode: Some(table.string.mode),
        buckets: unsafe { std::slice::from_raw_parts(table.storage, bucket_count) }.to_vec(),
    }
}

unsafe fn free_map(api: &Api, map: *mut c_void, elem_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_map(map, elem_size), elem_size) };
    }
}

unsafe fn put_binary(api: &Api, map: *mut c_void, key: &mut [u8], elem_size: usize) -> *mut c_void {
    unsafe {
        (api.hmput)(
            map,
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            HM_BINARY,
        )
    }
}

unsafe fn get_temp(map: *mut c_void, elem_size: usize) -> isize {
    unsafe { (*map_header(map, elem_size)).temp }
}

unsafe fn binary_keys(map: *mut c_void, elem_size: usize, key_size: usize) -> Vec<Vec<u8>> {
    let count = unsafe { (*map_header(map, elem_size)).length - 1 };
    (0..count)
        .map(|index| {
            unsafe { std::slice::from_raw_parts(map.cast::<u8>().add(index * elem_size), key_size) }
                .to_vec()
        })
        .collect()
}

unsafe fn string_entries(map: *mut c_void) -> Vec<(Vec<u8>, c_int)> {
    let elem_size = size_of::<StringEntry>();
    let count = unsafe { (*map_header(map, elem_size)).length - 1 };
    (0..count)
        .map(|index| {
            let entry = unsafe { &*map.cast::<StringEntry>().add(index) };
            (
                unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
                entry.value,
            )
        })
        .collect()
}

unsafe fn put_string(
    api: &Api,
    map: *mut c_void,
    key: &mut [u8],
    value: c_int,
    mode: c_int,
) -> *mut c_void {
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<StringEntry>(),
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { get_temp(map, size_of::<StringEntry>()) };
    unsafe { (*map.cast::<StringEntry>().offset(index)).value = value };
    map
}

fn arena_shape(arena: &StringArena) -> (usize, u8, u8, usize) {
    let mut blocks = 0usize;
    let mut current = arena.storage;
    unsafe {
        while !current.is_null() {
            blocks += 1;
            assert!(blocks < 10_000, "arena block list contains a cycle");
            current = (*current).next;
        }
    }
    (arena.remaining, arena.block, arena.mode, blocks)
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(null_mut()), 0);
        let mut fds = [0; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        call();
        assert_eq!(fflush(null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);
        assert_eq!(close(fds[1]), 0);

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
}

#[test]
fn valid_array_and_hash_surface_v01_v19() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173);

    unsafe {
        // V01: null allocation uses the minimum capacity of four.
        let c_array = (c.arrgrow)(null_mut(), 3, 0, 1);
        let rust_array = (rust.arrgrow)(null_mut(), 3, 0, 1);
        assert_eq!((*header(c_array)).capacity, 4);
        assert_eq!(*header(c_array), *header(rust_array));
        (c.arrfree)(c_array);
        (rust.arrfree)(rust_array);

        // V02: add length dominates the explicit minimum.
        let c_array = (c.arrgrow)(null_mut(), 5, 9, 2);
        let rust_array = (rust.arrgrow)(null_mut(), 5, 9, 2);
        assert_eq!((*header(c_array)).capacity, 9);
        assert_eq!(*header(c_array), *header(rust_array));
        (c.arrfree)(c_array);
        (rust.arrfree)(rust_array);

        // V03-V05: no growth, doubling growth, and explicit-minimum growth.
        let elem_size = 7;
        let mut c_array = (c.arrgrow)(null_mut(), elem_size, 0, 8);
        let mut rust_array = (rust.arrgrow)(null_mut(), elem_size, 0, 8);
        (*header(c_array)).length = 4;
        (*header(rust_array)).length = 4;
        let mut payload = vec![0u8; elem_size * 4];
        rng.fill(&mut payload);
        std::ptr::copy_nonoverlapping(payload.as_ptr(), c_array.cast(), payload.len());
        std::ptr::copy_nonoverlapping(payload.as_ptr(), rust_array.cast(), payload.len());

        let old_c = c_array;
        let old_rust = rust_array;
        c_array = (c.arrgrow)(c_array, elem_size, 2, 0);
        rust_array = (rust.arrgrow)(rust_array, elem_size, 2, 0);
        assert_eq!(c_array, old_c);
        assert_eq!(rust_array, old_rust);
        assert_eq!(*header(c_array), *header(rust_array));

        (*header(c_array)).length = 7;
        (*header(rust_array)).length = 7;
        c_array = (c.arrgrow)(c_array, elem_size, 2, 0);
        rust_array = (rust.arrgrow)(rust_array, elem_size, 2, 0);
        assert_eq!((*header(c_array)).capacity, 16);
        assert_eq!(*header(c_array), *header(rust_array));
        assert_eq!(
            std::slice::from_raw_parts(c_array.cast::<u8>(), payload.len()),
            std::slice::from_raw_parts(rust_array.cast::<u8>(), payload.len())
        );

        c_array = (c.arrgrow)(c_array, elem_size, 0, 40);
        rust_array = (rust.arrgrow)(rust_array, elem_size, 0, 40);
        assert_eq!((*header(c_array)).capacity, 40);
        assert_eq!(*header(c_array), *header(rust_array));
        assert_eq!(
            std::slice::from_raw_parts(c_array.cast::<u8>(), payload.len()),
            std::slice::from_raw_parts(rust_array.cast::<u8>(), payload.len())
        );
        (c.arrfree)(c_array);
        (rust.arrfree)(rust_array);

        // V06-V08: empty, ASCII, and high-bit C strings.
        for iteration in 0..256usize {
            let len = iteration % 48;
            let mut value = vec![0u8; len + 1];
            if iteration % 3 == 0 {
                for byte in &mut value[..len] {
                    *byte = 1 + (rng.next_u64() % 0x7e) as u8;
                }
            } else {
                for byte in &mut value[..len] {
                    *byte = 0x80 + (rng.next_u64() % 0x80) as u8;
                }
            }
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (c.hash_string)(value.as_mut_ptr().cast(), seed),
                (rust.hash_string)(value.as_mut_ptr().cast(), seed),
                "string hash mismatch at iteration {iteration}"
            );
        }

        // V09-V19: every tail case, full words, and multiple words.
        for len in 0..=80usize {
            for iteration in 0..64usize {
                let mut bytes = vec![0u8; len.max(1)];
                rng.fill(&mut bytes[..len]);
                if len >= 4 && iteration % 2 == 0 {
                    bytes[3] |= 0x80;
                }
                let seed = rng.next_u64() as usize;
                let pointer = if len == 0 && iteration == 0 {
                    null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                assert_eq!(
                    (c.hash_bytes)(pointer, len, seed),
                    (rust.hash_bytes)(pointer, len, seed),
                    "byte hash mismatch for len={len}, iteration={iteration}"
                );
            }
        }
    }
}

#[test]
fn valid_binary_map_surface_v20_v31_and_e01_e08() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);
    let elem_size = 24usize;

    unsafe {
        // V20: table creation observes each library's explicitly reset seed.
        for seed in [0, 1, usize::MAX, 0x3141_5926, 0xfeed_face_cafe_beef] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let mut key = rng.next_u64().to_ne_bytes().to_vec();
            let c_map = put_binary(&c, null_mut(), &mut key, elem_size);
            let rust_map = put_binary(&rust, null_mut(), &mut key, elem_size);
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V21 / E01: a null-map lookup allocates only the default element.
        let mut key = 7u32.to_ne_bytes();
        let mut c_temp = 123;
        let mut rust_temp = 123;
        let c_map = (c.hmget_ts)(
            null_mut(),
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut c_temp,
            HM_BINARY,
        );
        let rust_map = (rust.hmget_ts)(
            null_mut(),
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut rust_temp,
            HM_BINARY,
        );
        assert_eq!((c_temp, rust_temp), (-1, -1));
        assert_eq!(
            table_snapshot(c_map, elem_size),
            table_snapshot(rust_map, elem_size)
        );
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V22-V23 / E02 / E04: default-only map, idempotence, and missing lookup.
        let mut c_map = (c.hmput_default)(null_mut(), elem_size);
        let mut rust_map = (rust.hmput_default)(null_mut(), elem_size);
        assert_eq!(
            std::slice::from_raw_parts(raw_map(c_map, elem_size).cast::<u8>(), elem_size),
            &[0; 24]
        );
        assert_eq!(
            std::slice::from_raw_parts(raw_map(c_map, elem_size).cast::<u8>(), elem_size),
            std::slice::from_raw_parts(raw_map(rust_map, elem_size).cast::<u8>(), elem_size)
        );
        let old_c = c_map;
        let old_rust = rust_map;
        c_map = (c.hmput_default)(c_map, elem_size);
        rust_map = (rust.hmput_default)(rust_map, elem_size);
        assert_eq!(c_map, old_c);
        assert_eq!(rust_map, old_rust);
        c_map = (c.hmget)(
            c_map,
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            HM_BINARY,
        );
        rust_map = (rust.hmget)(
            rust_map,
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            HM_BINARY,
        );
        assert_eq!(
            (get_temp(c_map, elem_size), get_temp(rust_map, elem_size)),
            (-1, -1)
        );
        assert_eq!(
            table_snapshot(c_map, elem_size),
            table_snapshot(rust_map, elem_size)
        );
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V24-V26 / E03-E04: binary widths, found/missing, duplicates, and growth.
        for key_size in [1usize, 2, 4, 8, 16] {
            (c.rand_seed)(0x900d + key_size);
            (rust.rand_seed)(0x900d + key_size);
            let mut c_map = null_mut();
            let mut rust_map = null_mut();
            let mut inserted = Vec::<Vec<u8>>::new();

            for iteration in 0..40usize {
                let mut key = vec![0u8; key_size];
                rng.fill(&mut key);
                if iteration == 20 {
                    key.copy_from_slice(&inserted[3]);
                }
                c_map = (c.hmput)(
                    c_map,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    if iteration % 2 == 0 { HM_BINARY } else { -7 },
                );
                rust_map = (rust.hmput)(
                    rust_map,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    if iteration % 2 == 0 { HM_BINARY } else { -7 },
                );
                if !inserted.iter().any(|existing| existing == &key) {
                    inserted.push(key.clone());
                }
                assert_eq!(
                    table_snapshot(c_map, elem_size),
                    table_snapshot(rust_map, elem_size),
                    "map metadata mismatch for key_size={key_size}, iteration={iteration}"
                );
                assert_eq!(
                    binary_keys(c_map, elem_size, key_size),
                    binary_keys(rust_map, elem_size, key_size)
                );

                let mut found_temp_c = 999;
                let mut found_temp_rust = 999;
                c_map = (c.hmget_ts)(
                    c_map,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    &mut found_temp_c,
                    HM_BINARY,
                );
                rust_map = (rust.hmget_ts)(
                    rust_map,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key_size,
                    &mut found_temp_rust,
                    HM_BINARY,
                );
                assert_eq!(found_temp_c, found_temp_rust);
                assert!(found_temp_c >= 0);
            }

            let mut missing = vec![0u8; key_size];
            loop {
                rng.fill(&mut missing);
                if !inserted.iter().any(|key| key == &missing) {
                    break;
                }
            }
            let mut c_temp = 0;
            let mut rust_temp = 0;
            c_map = (c.hmget_ts)(
                c_map,
                elem_size,
                missing.as_mut_ptr().cast(),
                key_size,
                &mut c_temp,
                HM_BINARY,
            );
            rust_map = (rust.hmget_ts)(
                rust_map,
                elem_size,
                missing.as_mut_ptr().cast(),
                key_size,
                &mut rust_temp,
                HM_BINARY,
            );
            assert_eq!((c_temp, rust_temp), (-1, -1));
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert!(table_snapshot(c_map, elem_size).slot_count.unwrap() >= 32);
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V27 / E05-E08: null/no-table/missing deletion and null free.
        let c_null = (c.hmdel)(
            null_mut(),
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            HM_BINARY,
        );
        let rust_null = (rust.hmdel)(
            null_mut(),
            elem_size,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            HM_BINARY,
        );
        assert!(c_null.is_null() && rust_null.is_null());
        (c.hmfree)(null_mut(), elem_size);
        (rust.hmfree)(null_mut(), elem_size);

        let c_default = (c.hmput_default)(null_mut(), elem_size);
        let rust_default = (rust.hmput_default)(null_mut(), elem_size);
        assert_eq!(
            (c.hmdel)(
                c_default,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            ),
            c_default
        );
        assert_eq!(
            (rust.hmdel)(
                rust_default,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            ),
            rust_default
        );
        assert_eq!(
            table_snapshot(c_default, elem_size),
            table_snapshot(rust_default, elem_size)
        );
        free_map(&c, c_default, elem_size);
        free_map(&rust, rust_default, elem_size);

        // V28-V31: final/non-final moves, tombstone reuse/rebuild, and shrinking.
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        let mut keys: Vec<Vec<u8>> = (0u64..20)
            .map(|value| value.to_ne_bytes().to_vec())
            .collect();
        for key in &mut keys {
            c_map = put_binary(&c, c_map, key, elem_size);
            rust_map = put_binary(&rust, rust_map, key, elem_size);
        }
        assert_eq!(
            table_snapshot(c_map, elem_size),
            table_snapshot(rust_map, elem_size)
        );

        for delete_index in [19usize, 3] {
            let key = &mut keys[delete_index];
            c_map = (c.hmdel)(
                c_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            rust_map = (rust.hmdel)(
                rust_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert_eq!(
                binary_keys(c_map, elem_size, key.len()),
                binary_keys(rust_map, elem_size, key.len())
            );
        }

        let before_insert = table_snapshot(c_map, elem_size).tombstone_count.unwrap();
        let table = &*map_table(c_map, elem_size);
        let target_position = {
            let mut deleted = keys[3].clone();
            let hash = (c.hash_bytes)(deleted.as_mut_ptr().cast(), deleted.len(), table.seed);
            hash.max(2) & (table.slot_count - 1)
        };
        let mut replacement_value = 10_000u64;
        let mut replacement;
        loop {
            replacement = replacement_value.to_ne_bytes().to_vec();
            let hash = (c.hash_bytes)(
                replacement.as_mut_ptr().cast(),
                replacement.len(),
                table.seed,
            );
            if hash.max(2) & (table.slot_count - 1) == target_position {
                break;
            }
            replacement_value += 1;
        }
        c_map = put_binary(&c, c_map, &mut replacement, elem_size);
        rust_map = put_binary(&rust, rust_map, &mut replacement, elem_size);
        assert_eq!(
            table_snapshot(c_map, elem_size),
            table_snapshot(rust_map, elem_size)
        );
        assert!(table_snapshot(c_map, elem_size).tombstone_count.unwrap() <= before_insert);

        let mut next_delete = 0usize;
        while table_snapshot(c_map, elem_size).slot_count.unwrap() > 16 {
            if next_delete == 3 || next_delete == 19 {
                next_delete += 1;
                continue;
            }
            let key = &mut keys[next_delete];
            c_map = (c.hmdel)(
                c_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            rust_map = (rust.hmdel)(
                rust_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            next_delete += 1;
        }
        assert_eq!(table_snapshot(c_map, elem_size).slot_count, Some(16));
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V30 in isolation: two tombstones exceed the slot-8 threshold of one.
        (c.rand_seed)(77);
        (rust.rand_seed)(77);
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        let mut small_keys: Vec<Vec<u8>> = (100u64..104)
            .map(|value| value.to_ne_bytes().to_vec())
            .collect();
        for key in &mut small_keys {
            c_map = put_binary(&c, c_map, key, elem_size);
            rust_map = put_binary(&rust, rust_map, key, elem_size);
        }
        for key in &mut small_keys[..2] {
            c_map = (c.hmdel)(
                c_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            rust_map = (rust.hmdel)(
                rust_map,
                elem_size,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
        }
        let c_state = table_snapshot(c_map, elem_size);
        assert_eq!(c_state, table_snapshot(rust_map, elem_size));
        assert_eq!(
            (c_state.slot_count, c_state.tombstone_count),
            (Some(8), Some(0))
        );
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);
    }
}

#[test]
fn valid_string_map_surface_v32_v39() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_apis();
    let elem_size = size_of::<StringEntry>();
    let mut rng = Rng::new(0xe703_7ed1_a0b4_28db);

    unsafe {
        // V32: each declared mode creates the same empty table state.
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(0x1000 + mode as usize);
            (rust.rand_seed)(0x1000 + mode as usize);
            let c_map = (c.shmode)(elem_size, mode);
            let rust_map = (rust.shmode)(elem_size, mode);
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert_eq!((*map_table(c_map, elem_size)).string.mode, mode as u8);
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V33: C converts arbitrary int mode values to unsigned char.
        for mode in [-257, -1, 4, 255, 256, 257, c_int::MIN, c_int::MAX] {
            (c.rand_seed)(33);
            (rust.rand_seed)(33);
            let c_map = (c.shmode)(elem_size, mode);
            let rust_map = (rust.shmode)(elem_size, mode);
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert_eq!(
                (*map_table(c_map, elem_size)).string.mode,
                mode as u8,
                "mode conversion mismatch for {mode}"
            );
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V34: implicit string mode creates SH_DEFAULT and borrows key pointers.
        (c.rand_seed)(0x2222);
        (rust.rand_seed)(0x2222);
        let mut borrowed_keys = Vec::<Vec<u8>>::new();
        let mut c_map = null_mut();
        let mut rust_map = null_mut();
        for index in 0..32 {
            borrowed_keys.push(format!("borrowed_{index:03}\0").into_bytes());
            let key = borrowed_keys.last_mut().unwrap();
            c_map = put_string(&c, c_map, key, index * 7, HM_STRING);
            rust_map = put_string(&rust, rust_map, key, index * 7, HM_STRING);
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert_eq!(string_entries(c_map), string_entries(rust_map));
        }
        assert_eq!((*map_table(c_map, elem_size)).string.mode, SH_DEFAULT as u8);
        for (entry, source) in std::slice::from_raw_parts(c_map.cast::<StringEntry>(), 32)
            .iter()
            .zip(&borrowed_keys)
        {
            assert_eq!(entry.key, source.as_ptr().cast_mut().cast());
        }
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V35: strdup mode owns a copy and survives caller mutation.
        (c.rand_seed)(0x3333);
        (rust.rand_seed)(0x3333);
        let mut c_map = (c.shmode)(elem_size, SH_STRDUP);
        let mut rust_map = (rust.shmode)(elem_size, SH_STRDUP);
        let mut original = b"original-key\0".to_vec();
        c_map = put_string(&c, c_map, &mut original, 91, HM_STRING);
        rust_map = put_string(&rust, rust_map, &mut original, 91, HM_STRING);
        original[..8].copy_from_slice(b"mutated!");
        let mut lookup = b"original-key\0".to_vec();
        c_map = (c.hmget)(
            c_map,
            elem_size,
            lookup.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            HM_STRING,
        );
        rust_map = (rust.hmget)(
            rust_map,
            elem_size,
            lookup.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            HM_STRING,
        );
        assert_eq!(
            (get_temp(c_map, elem_size), get_temp(rust_map, elem_size)),
            (0, 0)
        );
        assert_eq!(string_entries(c_map), string_entries(rust_map));
        assert_eq!(string_entries(c_map)[0], (b"original-key".to_vec(), 91));
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V36: arena mode handles empty, short, and randomized keys.
        (c.rand_seed)(0x4444);
        (rust.rand_seed)(0x4444);
        let mut c_map = (c.shmode)(elem_size, SH_ARENA);
        let mut rust_map = (rust.shmode)(elem_size, SH_ARENA);
        let mut arena_keys = vec![b"\0".to_vec(), b"a\0".to_vec()];
        for index in 0..80usize {
            let len = 2 + index % 45;
            let mut key = vec![0u8; len + 1];
            for byte in &mut key[..len] {
                *byte = b'a' + (rng.next_u64() % 26) as u8;
            }
            arena_keys.push(key);
        }
        for (index, key) in arena_keys.iter_mut().enumerate() {
            c_map = put_string(&c, c_map, key, index as c_int - 7, HM_STRING);
            rust_map = put_string(&rust, rust_map, key, index as c_int - 7, HM_STRING);
        }
        assert_eq!(
            table_snapshot(c_map, elem_size),
            table_snapshot(rust_map, elem_size)
        );
        assert_eq!(string_entries(c_map), string_entries(rust_map));
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);

        // V37: duplicate keys replace values without growing in every ownership mode.
        for ownership in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(0x5000 + ownership as usize);
            (rust.rand_seed)(0x5000 + ownership as usize);
            let mut c_map = (c.shmode)(elem_size, ownership);
            let mut rust_map = (rust.shmode)(elem_size, ownership);
            let mut stable_keys = vec![b"duplicate\0".to_vec(), b"other\0".to_vec()];
            c_map = put_string(&c, c_map, &mut stable_keys[0], 1, HM_STRING);
            rust_map = put_string(&rust, rust_map, &mut stable_keys[0], 1, HM_STRING);
            c_map = put_string(&c, c_map, &mut stable_keys[1], 2, HM_STRING);
            rust_map = put_string(&rust, rust_map, &mut stable_keys[1], 2, HM_STRING);
            let before_length = (*map_header(c_map, elem_size)).length;
            let mut duplicate = b"duplicate\0".to_vec();
            c_map = put_string(&c, c_map, &mut duplicate, 99, HM_STRING);
            rust_map = put_string(&rust, rust_map, &mut duplicate, 99, HM_STRING);
            assert_eq!((*map_header(c_map, elem_size)).length, before_length);
            assert_eq!(
                table_snapshot(c_map, elem_size),
                table_snapshot(rust_map, elem_size)
            );
            assert_eq!(string_entries(c_map), string_entries(rust_map));
            assert_eq!(string_entries(c_map)[0].1, 99);
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V38: found/missing string deletes, including mode > HM_STRING.
        for (ownership, delete_mode) in [
            (SH_DEFAULT, HM_STRING),
            (SH_STRDUP, HM_STRING),
            (SH_ARENA, HM_STRING),
            (SH_ARENA, HM_STRING + 1),
        ] {
            (c.rand_seed)(0x6000 + ownership as usize + delete_mode as usize);
            (rust.rand_seed)(0x6000 + ownership as usize + delete_mode as usize);
            let mut c_map = (c.shmode)(elem_size, ownership);
            let mut rust_map = (rust.shmode)(elem_size, ownership);
            let mut keys: Vec<Vec<u8>> = (0..10)
                .map(|index| format!("delete_{index}\0").into_bytes())
                .collect();
            for (index, key) in keys.iter_mut().enumerate() {
                c_map = put_string(&c, c_map, key, index as c_int, HM_STRING);
                rust_map = put_string(&rust, rust_map, key, index as c_int, HM_STRING);
            }
            let mut missing = b"not-present\0".to_vec();
            c_map = (c.hmdel)(
                c_map,
                elem_size,
                missing.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                delete_mode,
            );
            rust_map = (rust.hmdel)(
                rust_map,
                elem_size,
                missing.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                delete_mode,
            );
            assert_eq!(
                (get_temp(c_map, elem_size), get_temp(rust_map, elem_size)),
                (0, 0)
            );
            let delete_indices: &[usize] = if delete_mode == HM_STRING {
                &[9, 2]
            } else {
                &[9]
            };
            for &index in delete_indices {
                c_map = (c.hmdel)(
                    c_map,
                    elem_size,
                    keys[index].as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    delete_mode,
                );
                rust_map = (rust.hmdel)(
                    rust_map,
                    elem_size,
                    keys[index].as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    delete_mode,
                );
                assert_eq!(
                    (get_temp(c_map, elem_size), get_temp(rust_map, elem_size)),
                    (1, 1)
                );
                assert_eq!(
                    table_snapshot(c_map, elem_size),
                    table_snapshot(rust_map, elem_size)
                );
                assert_eq!(string_entries(c_map), string_entries(rust_map));
            }
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }

        // V39: hmfree accepts a raw dynamic array without a hash table.
        let c_raw = (c.arrgrow)(null_mut(), elem_size, 0, 3);
        let rust_raw = (rust.arrgrow)(null_mut(), elem_size, 0, 3);
        (*header(c_raw)).length = 3;
        (*header(rust_raw)).length = 3;
        (c.hmfree)(c_raw, elem_size);
        (rust.hmfree)(rust_raw, elem_size);
    }
}

#[test]
fn valid_arena_and_public_workflow_surface_v40_v46() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x8ebc_6af0_9c88_c6e3);

    unsafe {
        // V40-V41: empty/short strings, then enough randomized strings for new blocks.
        let mut c_arena: StringArena = zeroed();
        let mut rust_arena: StringArena = zeroed();
        for iteration in 0..300usize {
            let len = if iteration == 0 {
                0
            } else {
                1 + (rng.next_u64() as usize % 80)
            };
            let mut string = vec![0u8; len + 1];
            for byte in &mut string[..len] {
                *byte = b'a' + (rng.next_u64() % 26) as u8;
            }
            let c_result = (c.stralloc)(&mut c_arena, string.as_mut_ptr().cast());
            let rust_result = (rust.stralloc)(&mut rust_arena, string.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(rust_result).to_bytes(),
                "arena payload mismatch at iteration {iteration}"
            );
            assert_eq!(arena_shape(&c_arena), arena_shape(&rust_arena));
        }
        assert!(arena_shape(&c_arena).3 > 1);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
        assert_eq!(
            std::slice::from_raw_parts(
                (&c_arena as *const StringArena).cast::<u8>(),
                size_of::<StringArena>()
            ),
            &[0; size_of::<StringArena>()]
        );
        assert_eq!(
            std::slice::from_raw_parts(
                (&c_arena as *const StringArena).cast::<u8>(),
                size_of::<StringArena>()
            ),
            std::slice::from_raw_parts(
                (&rust_arena as *const StringArena).cast::<u8>(),
                size_of::<StringArena>()
            )
        );

        // V42: oversize allocation with no storage and with existing storage.
        let mut c_arena: StringArena = zeroed();
        let mut rust_arena: StringArena = zeroed();
        for len in [700usize, 900] {
            let mut string = vec![b'x'; len + 1];
            string[len] = 0;
            let c_result = (c.stralloc)(&mut c_arena, string.as_mut_ptr().cast());
            let rust_result = (rust.stralloc)(&mut rust_arena, string.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(rust_result).to_bytes()
            );
            assert_eq!(arena_shape(&c_arena), arena_shape(&rust_arena));
        }
        assert_eq!(arena_shape(&c_arena).3, 2);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);

        // V43: values immediately around the 512-byte and 1 MiB block boundaries.
        for (initial_block, content_lengths) in [
            (0u8, vec![510usize, 511, 512, 513]),
            (
                22u8,
                vec![
                    (1usize << 20) - 2,
                    (1usize << 20) - 1,
                    1usize << 20,
                    (1usize << 20) + 1,
                ],
            ),
        ] {
            for len in content_lengths {
                let mut c_arena: StringArena = zeroed();
                let mut rust_arena: StringArena = zeroed();
                c_arena.block = initial_block;
                rust_arena.block = initial_block;
                let mut string = vec![b'z'; len + 1];
                string[len] = 0;
                let c_result = (c.stralloc)(&mut c_arena, string.as_mut_ptr().cast());
                let rust_result = (rust.stralloc)(&mut rust_arena, string.as_mut_ptr().cast());
                assert_eq!(
                    CStr::from_ptr(c_result).to_bytes(),
                    CStr::from_ptr(rust_result).to_bytes(),
                    "boundary payload mismatch for block={initial_block}, len={len}"
                );
                assert_eq!(arena_shape(&c_arena), arena_shape(&rust_arena));
                (c.strreset)(&mut c_arena);
                (rust.strreset)(&mut rust_arena);
                assert_eq!(arena_shape(&c_arena), (0, 0, 0, 0));
                assert_eq!(arena_shape(&c_arena), arena_shape(&rust_arena));
            }
        }

        // V44: strkey writes the same bytes for negative/zero/positive int values.
        let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
        values.extend((0..256).map(|_| rng.next_u64() as c_int));
        for value in values {
            let c_value = CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec();
            let rust_value = CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec();
            assert_eq!(c_value, rust_value, "strkey mismatch for {value}");
            assert_eq!(c_value, format!("test_{value}").as_bytes());
        }

        // V45-V46: compare the complete stdout byte stream and assertion behavior.
        let mut counts = vec![-100, -1, 0, 1, 2, 3, 4, 5, 7, 8, 12, 31, 48];
        counts.extend((0..32).map(|_| (rng.next_u64() % 50) as c_int));
        for count in counts {
            let seed = 0x7000usize.wrapping_add(count as usize);
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let c_output = capture_stdout(|| (c.sh_geti)(count));
            let rust_output = capture_stdout(|| (rust.sh_geti)(count));
            assert_eq!(c_output, rust_output, "sh_geti stdout mismatch for {count}");
        }
    }
}

unsafe fn trigger_abort_case(api: &Api, case: &str) {
    match case {
        "invalid_growth_thresholds" => unsafe {
            let elem_size = 8usize;
            let mut first = 1u64.to_ne_bytes().to_vec();
            let mut second = 2u64.to_ne_bytes().to_vec();
            let mut map = put_binary(api, null_mut(), &mut first, elem_size);
            let table = &mut *map_table(map, elem_size);
            table.slot_count = 1;
            table.used_count_threshold = 0;
            map = put_binary(api, map, &mut second, elem_size);
            let _ = map;
        },
        "moved_key_missing" => unsafe {
            let elem_size = size_of::<StringEntry>();
            let mut map = (api.shmode)(elem_size, SH_ARENA);
            let mut keys = [
                b"first\0".to_vec(),
                b"second\0".to_vec(),
                b"third\0".to_vec(),
            ];
            for (index, key) in keys.iter_mut().enumerate() {
                map = put_string(api, map, key, index as c_int, HM_STRING);
            }
            // C only dereferences a moved string key when mode == 1. Mode 2
            // takes the other branch and the moved key cannot be found.
            let _ = (api.hmdel)(
                map,
                elem_size,
                keys[0].as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                HM_STRING + 1,
            );
        },
        "moved_index_mismatch" => unsafe {
            let elem_size = 8usize;
            let mut map = null_mut();
            let mut keys = [
                0x1111_1111_1111_1111u64.to_ne_bytes().to_vec(),
                0x2222_2222_2222_2222u64.to_ne_bytes().to_vec(),
                0x3333_3333_3333_3333u64.to_ne_bytes().to_vec(),
            ];
            for key in &mut keys {
                map = put_binary(api, map, key, elem_size);
            }

            // Make entry 1 compare equal to the final entry, then redirect the
            // final entry's bucket to index 1. Deleting entry 0 moves the final
            // payload and reaches the line-849 index consistency assertion.
            std::ptr::copy_nonoverlapping(
                map.cast::<u8>().add(2 * elem_size),
                map.cast::<u8>().add(elem_size),
                elem_size,
            );
            let table = &mut *map_table(map, elem_size);
            let buckets =
                std::slice::from_raw_parts_mut(table.storage, table.slot_count / BUCKET_LENGTH);
            let final_slot = buckets
                .iter_mut()
                .flat_map(|bucket| bucket.index.iter_mut())
                .find(|index| **index == 2)
                .expect("final index missing from table");
            *final_slot = 1;
            let _ = (api.hmdel)(
                map,
                elem_size,
                keys[0].as_mut_ptr().cast(),
                elem_size,
                0,
                HM_BINARY,
            );
        },
        _ => panic!("unknown abort case {case}"),
    }
    panic!("abort case {case} unexpectedly returned");
}

#[test]
fn ffi_abort_child() {
    let Ok(case) = std::env::var("DIFF_ABORT_CASE") else {
        return;
    };
    let library = std::env::var("DIFF_ABORT_LIBRARY").unwrap();
    let root = manifest_dir();
    let path = match library.as_str() {
        "c" => root.join("../c_src/build/libharvest-work-punw4N.so"),
        "rust" => root.join("target/release/libsh_geti_lib.so"),
        _ => panic!("unknown library {library}"),
    };
    let api = unsafe { Api::load(&path) };
    unsafe { trigger_abort_case(&api, &case) };
}

#[test]
fn error_assertion_surface_e09_e21() {
    let _guard = PROCESS_STATE.lock().unwrap();
    let executable = std::env::current_exe().unwrap();

    // E09 and E13-E14: exact assertion rejections are isolated because assert aborts
    // the process. Both shared objects must terminate with the same signal.
    for case in [
        "invalid_growth_thresholds",
        "moved_key_missing",
        "moved_index_mismatch",
    ] {
        let run = |library: &str| {
            Command::new(&executable)
                .args(["--exact", "ffi_abort_child", "--nocapture"])
                .env("DIFF_ABORT_CASE", case)
                .env("DIFF_ABORT_LIBRARY", library)
                .output()
                .unwrap()
        };
        let c_output = run("c");
        let rust_output = run("rust");
        assert!(!c_output.status.success(), "C case {case} did not reject");
        assert!(
            !rust_output.status.success(),
            "Rust case {case} did not reject"
        );
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(
                c_output.status.signal(),
                rust_output.status.signal(),
                "different termination signal for {case}\nC stderr: {}\nRust stderr: {}",
                String::from_utf8_lossy(&c_output.stderr),
                String::from_utf8_lossy(&rust_output.stderr)
            );
        }
    }

    // E10-E12 and E15-E21 cannot be falsified by a defined FFI input:
    // post-growth capacity is guaranteed, slot bounds come from masked probes,
    // used_count is unsigned, arena remaining is established by the preceding
    // branch, and sh_geti's assertions check its own implementation. Their
    // assertion-bearing paths are exercised by V20-V31, V40-V43, and V45-V46.
}
