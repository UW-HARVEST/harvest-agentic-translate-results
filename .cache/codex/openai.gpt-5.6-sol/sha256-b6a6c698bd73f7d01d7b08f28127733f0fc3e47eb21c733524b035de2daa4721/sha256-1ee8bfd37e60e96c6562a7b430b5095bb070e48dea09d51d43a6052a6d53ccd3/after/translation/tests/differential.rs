use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::mem::{size_of, zeroed};
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
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
type Helxo = unsafe extern "C" fn(c_char);

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
    hmdefault: HmDefault,
    hmput: HmPut,
    shmode: ShMode,
    hmdel: HmDel,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    helxo: Helxo,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap_or_else(|error| {
            panic!("failed to load {}: {error}", path.display());
        });
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
            hmdefault: symbol!("stbds_hmput_default", HmDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            shmode: symbol!("stbds_shmode_func", ShMode),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            helxo: symbol!("helxo", Helxo),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-g09eEL.so")
}

fn rust_library() -> PathBuf {
    manifest_dir().join("target/release/libhelxo_lib.so")
}

unsafe fn apis() -> (Api, Api) {
    assert!(c_library().is_file(), "C shared object was not built");
    assert!(
        rust_library().is_file(),
        "Rust release shared object was not built"
    );
    unsafe { (Api::load(&c_library()), Api::load(&rust_library())) }
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
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
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

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe {
        array
            .cast::<u8>()
            .sub(size_of::<ArrayHeader>())
            .cast::<ArrayHeader>()
    }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, element_size)) }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, limit: usize) -> usize {
        (self.next() as usize) % limit
    }
}

// CONFIGS C01-C07; ERRORS E17-E18.
#[test]
fn arrays_match_across_growth_branches() {
    unsafe {
        let (c, rust) = apis();
        for element_size in [1usize, 4, 8, 16, 31] {
            for requested in 0..20 {
                let ca = (c.arrgrow)(null_mut(), element_size, 0, requested);
                let ra = (rust.arrgrow)(null_mut(), element_size, 0, requested);
                if requested == 0 {
                    assert!(ca.is_null());
                    assert!(ra.is_null());
                    continue;
                }
                assert_eq!((*header(ca)).length, (*header(ra)).length);
                assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
                assert_eq!((*header(ca)).capacity, requested.max(4));

                let initial_capacity = (*header(ca)).capacity;
                (*header(ca)).length = initial_capacity.saturating_sub(1);
                (*header(ra)).length = initial_capacity.saturating_sub(1);
                for index in 0..element_size * initial_capacity {
                    *ca.cast::<u8>().add(index) = index.wrapping_mul(37) as u8;
                    *ra.cast::<u8>().add(index) = index.wrapping_mul(37) as u8;
                }

                let unchanged_c = (c.arrgrow)(ca, element_size, 0, initial_capacity);
                let unchanged_r = (rust.arrgrow)(ra, element_size, 0, initial_capacity);
                assert_eq!(unchanged_c, ca);
                assert_eq!(unchanged_r, ra);

                let grown_c = (c.arrgrow)(unchanged_c, element_size, 2, 0);
                let grown_r = (rust.arrgrow)(unchanged_r, element_size, 2, 0);
                assert_eq!((*header(grown_c)).length, (*header(grown_r)).length);
                assert_eq!((*header(grown_c)).capacity, (*header(grown_r)).capacity);
                assert_eq!(
                    std::slice::from_raw_parts(
                        grown_c.cast::<u8>(),
                        element_size * initial_capacity
                    ),
                    std::slice::from_raw_parts(
                        grown_r.cast::<u8>(),
                        element_size * initial_capacity
                    )
                );

                let explicit = (*header(grown_c)).capacity * 2 + 3;
                let grown_c = (c.arrgrow)(grown_c, element_size, 0, explicit);
                let grown_r = (rust.arrgrow)(grown_r, element_size, 0, explicit);
                assert_eq!((*header(grown_c)).capacity, explicit);
                assert_eq!((*header(grown_r)).capacity, explicit);
                (c.arrfree)(grown_c);
                (rust.arrfree)(grown_r);
            }
        }

        let ca = (c.arrgrow)(null_mut(), 0, 0, usize::MAX);
        let ra = (rust.arrgrow)(null_mut(), 0, 0, usize::MAX);
        assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
        (c.arrfree)(ca);
        (rust.arrfree)(ra);

        let ca = (c.arrgrow)(null_mut(), 0, usize::MAX, 0);
        let ra = (rust.arrgrow)(null_mut(), 0, usize::MAX, 0);
        assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
        (c.arrfree)(ca);
        (rust.arrfree)(ra);

        let overflowing_element_size = usize::MAX / 4 + 1;
        let ca = (c.arrgrow)(null_mut(), overflowing_element_size, 0, 4);
        let ra = (rust.arrgrow)(null_mut(), overflowing_element_size, 0, 4);
        assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
        (c.arrfree)(ca);
        (rust.arrfree)(ra);
    }
}

// CONFIGS C08-C14.
#[test]
fn hash_functions_match_randomized_inputs() {
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0xc0ff_ee12_3456_789a);
        let seeds = [0, 1, 0x3141_5926, 1usize << (usize::BITS - 1), usize::MAX];

        for &seed in &seeds {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let empty = CString::new("").unwrap();
            assert_eq!(
                (c.hash_string)(empty.as_ptr().cast_mut(), seed),
                (rust.hash_string)(empty.as_ptr().cast_mut(), seed)
            );
            assert_eq!(
                (c.hash_bytes)(null_mut(), 0, seed),
                (rust.hash_bytes)(null_mut(), 0, seed)
            );
            let cm = (c.shmode)(size_of::<BinEntry>(), 0);
            let rm = (rust.shmode)(size_of::<BinEntry>(), 0);
            let ct = (*map_header(cm, size_of::<BinEntry>()))
                .hash_table
                .cast::<HashIndex>();
            let rt = (*map_header(rm, size_of::<BinEntry>()))
                .hash_table
                .cast::<HashIndex>();
            assert_eq!((*ct).seed, seed);
            assert_eq!((*rt).seed, seed);
            free_bin(&c, cm);
            free_bin(&rust, rm);
        }

        for _ in 0..800 {
            let seed = rng.next() as usize;
            let length = rng.usize(96);
            let mut bytes = (0..length).map(|_| rng.next() as u8).collect::<Vec<_>>();
            assert_eq!(
                (c.hash_bytes)(bytes.as_mut_ptr().cast(), bytes.len(), seed),
                (rust.hash_bytes)(bytes.as_mut_ptr().cast(), bytes.len(), seed),
                "byte hash mismatch for length {length}, seed {seed:#x}"
            );

            let string_length = rng.usize(64);
            let mut string = (0..string_length)
                .map(|_| {
                    let byte = rng.next() as u8;
                    if byte == 0 { 0x80 } else { byte }
                })
                .collect::<Vec<_>>();
            string.push(0);
            assert_eq!(
                (c.hash_string)(string.as_mut_ptr().cast(), seed),
                (rust.hash_string)(string.as_mut_ptr().cast(), seed),
                "string hash mismatch for length {string_length}, seed {seed:#x}"
            );
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct BinEntry {
    key: u64,
    value: u64,
}

unsafe fn put_bin(api: &Api, map: *mut c_void, key: u64, value: u64) -> *mut c_void {
    let mut key = key;
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<BinEntry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    let index = unsafe { (*map_header(map, size_of::<BinEntry>())).temp as usize };
    unsafe { (*map.cast::<BinEntry>().add(index)).value = value };
    map
}

unsafe fn get_bin(api: &Api, map: *mut c_void, key: u64) -> (*mut c_void, isize) {
    let mut key = key;
    let mut index = 777isize;
    let map = unsafe {
        (api.hmget_ts)(
            map,
            size_of::<BinEntry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut index,
            0,
        )
    };
    (map, index)
}

unsafe fn del_bin(api: &Api, map: *mut c_void, key: u64) -> *mut c_void {
    let mut key = key;
    unsafe {
        (api.hmdel)(
            map,
            size_of::<BinEntry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    }
}

unsafe fn bin_snapshot(map: *mut c_void) -> (usize, usize, isize, Vec<(u64, u64)>) {
    let header = unsafe { &*map_header(map, size_of::<BinEntry>()) };
    let entries = (0..header.length - 1)
        .map(|index| {
            let entry = unsafe { &*map.cast::<BinEntry>().add(index) };
            (entry.key, entry.value)
        })
        .collect();
    (header.length, header.capacity, header.temp, entries)
}

unsafe fn free_bin(api: &Api, map: *mut c_void) {
    unsafe { (api.hmfree)(raw_map(map, size_of::<BinEntry>()), size_of::<BinEntry>()) };
}

// CONFIGS C15-C21, C24, C28-C35, C46; ERRORS E01-E08.
#[test]
fn binary_map_low_level_pipeline_matches() {
    unsafe {
        let (c, rust) = apis();

        assert!((c.hmdel)(null_mut(), 16, null_mut(), 0, 0, 0).is_null());
        assert!((rust.hmdel)(null_mut(), 16, null_mut(), 0, 0, 0).is_null());
        (c.hmfree)(null_mut(), 16);
        (rust.hmfree)(null_mut(), 16);

        for mode in [0, 1] {
            let mut key = 99u64;
            let mut ct = 5;
            let mut rt = 5;
            let cm = (c.hmget_ts)(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                8,
                &mut ct,
                mode,
            );
            let rm = (rust.hmget_ts)(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                8,
                &mut rt,
                mode,
            );
            assert_eq!((ct, rt), (-1, -1));
            assert_eq!((*map_header(cm, 16)).length, (*map_header(rm, 16)).length);
            free_bin(&c, cm);
            free_bin(&rust, rm);
        }

        let cm = (c.hmdefault)(null_mut(), size_of::<BinEntry>());
        let rm = (rust.hmdefault)(null_mut(), size_of::<BinEntry>());
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        let cm2 = (c.hmdefault)(cm, size_of::<BinEntry>());
        let rm2 = (rust.hmdefault)(rm, size_of::<BinEntry>());
        assert_eq!(cm2, cm);
        assert_eq!(rm2, rm);
        let (cm2, ci) = get_bin(&c, cm2, 123);
        let (rm2, ri) = get_bin(&rust, rm2, 123);
        assert_eq!((ci, ri), (-1, -1));
        assert_eq!(bin_snapshot(cm2), bin_snapshot(rm2));
        free_bin(&c, cm2);
        free_bin(&rust, rm2);

        (c.rand_seed)(0xfeed_beef);
        (rust.rand_seed)(0xfeed_beef);
        let mut cm = (c.shmode)(size_of::<BinEntry>(), 0);
        let mut rm = (rust.shmode)(size_of::<BinEntry>(), 0);
        let mut rng = Rng::new(0x5eed_1234_9876_abcd);
        for step in 0..1200 {
            let key = rng.usize(96) as u64;
            match rng.usize(5) {
                0 | 1 => {
                    let value = rng.next();
                    cm = put_bin(&c, cm, key, value);
                    rm = put_bin(&rust, rm, key, value);
                }
                2 => {
                    let (next_c, ci) = get_bin(&c, cm, key);
                    let (next_r, ri) = get_bin(&rust, rm, key);
                    cm = next_c;
                    rm = next_r;
                    assert_eq!(ci, ri, "lookup index at step {step}");
                    if ci >= 0 {
                        assert_eq!(
                            (*cm.cast::<BinEntry>().add(ci as usize)).value,
                            (*rm.cast::<BinEntry>().add(ri as usize)).value
                        );
                    }
                }
                _ => {
                    cm = del_bin(&c, cm, key);
                    rm = del_bin(&rust, rm, key);
                }
            }
            assert_eq!(bin_snapshot(cm), bin_snapshot(rm), "state at step {step}");
        }
        free_bin(&c, cm);
        free_bin(&rust, rm);
    }
}

unsafe fn table(map: *mut c_void, element_size: usize) -> *mut HashIndex {
    unsafe { (*map_header(map, element_size)).hash_table.cast() }
}

unsafe fn keys_for_bucket(api: &Api, seed: usize, bucket: usize, count: usize) -> Vec<u64> {
    let mut keys = Vec::new();
    let mut candidate = 0u64;
    while keys.len() < count {
        let mut bytes = candidate.to_ne_bytes();
        let hash = unsafe { (api.hash_bytes)(bytes.as_mut_ptr().cast(), bytes.len(), seed) };
        let adjusted = if hash < 2 { hash + 2 } else { hash };
        if adjusted & 7 == bucket {
            keys.push(candidate);
        }
        candidate += 1;
    }
    keys
}

// CONFIGS C31-C33 and ERRORS E03-E04 with deterministic table states.
#[test]
fn probe_delete_rebuild_and_shrink_branches_match() {
    unsafe {
        let (c, rust) = apis();

        let seed = 0x4444_5555;
        let collision_keys = keys_for_bucket(&c, seed, 7, 4);
        assert_eq!(collision_keys, keys_for_bucket(&rust, seed, 7, 4));
        (c.rand_seed)(seed);
        (rust.rand_seed)(seed);
        let mut cm = put_bin(&c, null_mut(), collision_keys[0], 1);
        let mut rm = put_bin(&rust, null_mut(), collision_keys[0], 1);

        let (next_c, ci) = get_bin(&c, cm, collision_keys[1]);
        let (next_r, ri) = get_bin(&rust, rm, collision_keys[1]);
        cm = next_c;
        rm = next_r;
        assert_eq!((ci, ri), (-1, -1)); // Wrapped segment reaches slot zero.

        let empty_bucket_key = keys_for_bucket(&c, seed, 3, 1)[0];
        let (next_c, ci) = get_bin(&c, cm, empty_bucket_key);
        let (next_r, ri) = get_bin(&rust, rm, empty_bucket_key);
        cm = next_c;
        rm = next_r;
        assert_eq!((ci, ri), (-1, -1)); // First segment reaches an empty slot.

        cm = put_bin(&c, cm, collision_keys[1], 2);
        rm = put_bin(&rust, rm, collision_keys[1], 2);
        cm = del_bin(&c, cm, collision_keys[0]);
        rm = del_bin(&rust, rm, collision_keys[0]);
        assert_eq!((*table(cm, 16)).tombstone_count, 1);
        assert_eq!((*table(rm, 16)).tombstone_count, 1);
        cm = put_bin(&c, cm, collision_keys[2], 3);
        rm = put_bin(&rust, rm, collision_keys[2], 3);
        assert_eq!((*table(cm, 16)).tombstone_count, 0);
        assert_eq!((*table(rm, 16)).tombstone_count, 0);
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        free_bin(&c, cm);
        free_bin(&rust, rm);

        (c.rand_seed)(0x5555);
        (rust.rand_seed)(0x5555);
        let mut cm = null_mut();
        let mut rm = null_mut();
        for key in 0..5 {
            cm = put_bin(&c, cm, key, key);
            rm = put_bin(&rust, rm, key, key);
        }
        cm = del_bin(&c, cm, 0);
        rm = del_bin(&rust, rm, 0);
        assert_eq!((*table(cm, 16)).tombstone_count, 1);
        cm = del_bin(&c, cm, 1);
        rm = del_bin(&rust, rm, 1);
        assert_eq!((*table(cm, 16)).slot_count, 8);
        assert_eq!((*table(rm, 16)).slot_count, 8);
        assert_eq!((*table(cm, 16)).tombstone_count, 0);
        assert_eq!((*table(rm, 16)).tombstone_count, 0);
        free_bin(&c, cm);
        free_bin(&rust, rm);

        (c.rand_seed)(0x6666);
        (rust.rand_seed)(0x6666);
        let mut cm = null_mut();
        let mut rm = null_mut();
        for key in 0..50 {
            cm = put_bin(&c, cm, key, key * 10);
            rm = put_bin(&rust, rm, key, key * 10);
        }
        assert_eq!((*table(cm, 16)).slot_count, 128);
        assert_eq!((*table(rm, 16)).slot_count, 128);
        let mut saw_shrink = false;
        for key in 0..30 {
            cm = del_bin(&c, cm, key);
            rm = del_bin(&rust, rm, key);
            assert_eq!((*table(cm, 16)).slot_count, (*table(rm, 16)).slot_count);
            saw_shrink |= (*table(cm, 16)).slot_count < 128;
        }
        assert!(saw_shrink);
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        free_bin(&c, cm);
        free_bin(&rust, rm);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: u64,
}

unsafe fn put_string(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let map = unsafe {
        (api.hmput)(
            map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { (*map_header(map, size_of::<StringEntry>())).temp as usize };
    unsafe { (*map.cast::<StringEntry>().add(index)).value = value };
    map
}

unsafe fn get_string(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    mode: c_int,
) -> (*mut c_void, isize) {
    let mut index = 777isize;
    let map = unsafe {
        (api.hmget_ts)(
            map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            &mut index,
            mode,
        )
    };
    (map, index)
}

unsafe fn del_string(api: &Api, map: *mut c_void, key: *mut c_char, mode: c_int) -> *mut c_void {
    unsafe {
        (api.hmdel)(
            map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            0,
            mode,
        )
    }
}

unsafe fn string_snapshot(map: *mut c_void) -> (usize, usize, isize, Vec<(Vec<u8>, u64)>) {
    let header = unsafe { &*map_header(map, size_of::<StringEntry>()) };
    let entries = (0..header.length - 1)
        .map(|index| {
            let entry = unsafe { &*map.cast::<StringEntry>().add(index) };
            let key = unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec();
            (key, entry.value)
        })
        .collect();
    (header.length, header.capacity, header.temp, entries)
}

unsafe fn free_string(api: &Api, map: *mut c_void) {
    unsafe {
        (api.hmfree)(
            raw_map(map, size_of::<StringEntry>()),
            size_of::<StringEntry>(),
        )
    };
}

unsafe fn run_string_pipeline(c: &Api, rust: &Api, ownership_mode: Option<c_int>) {
    let keys = (0..72)
        .map(|index| CString::new(format!("key_{index:03}_{}", index * 7919)).unwrap())
        .collect::<Vec<_>>();
    unsafe {
        (c.rand_seed)(0xa55a_55aa_1234_5678);
        (rust.rand_seed)(0xa55a_55aa_1234_5678);
    }
    let (mut cm, mut rm, operation_mode) = match ownership_mode {
        Some(mode) => unsafe {
            (
                (c.shmode)(size_of::<StringEntry>(), mode),
                (rust.shmode)(size_of::<StringEntry>(), mode),
                1,
            )
        },
        None => (null_mut(), null_mut(), 1),
    };
    cm = unsafe { put_string(c, cm, keys[0].as_ptr().cast_mut(), 0, operation_mode) };
    rm = unsafe { put_string(rust, rm, keys[0].as_ptr().cast_mut(), 0, operation_mode) };
    let mut rng = Rng::new(0x1234_9876_dead_beef ^ ownership_mode.unwrap_or(9) as u64);
    for step in 0..900 {
        let key_index = rng.usize(keys.len());
        let key = keys[key_index].as_ptr().cast_mut();
        match rng.usize(5) {
            0 | 1 => {
                let value = rng.next();
                cm = unsafe { put_string(c, cm, key, value, operation_mode) };
                rm = unsafe { put_string(rust, rm, key, value, operation_mode) };
            }
            2 => {
                let (next_c, ci) = unsafe { get_string(c, cm, key, operation_mode) };
                let (next_r, ri) = unsafe { get_string(rust, rm, key, operation_mode) };
                cm = next_c;
                rm = next_r;
                assert_eq!(ci, ri, "string lookup index at step {step}");
                if ci >= 0 {
                    unsafe {
                        assert_eq!(
                            (*cm.cast::<StringEntry>().add(ci as usize)).value,
                            (*rm.cast::<StringEntry>().add(ri as usize)).value
                        );
                    }
                }
            }
            _ => {
                cm = unsafe { del_string(c, cm, key, operation_mode) };
                rm = unsafe { del_string(rust, rm, key, operation_mode) };
            }
        }
        assert_eq!(
            unsafe { string_snapshot(cm) },
            unsafe { string_snapshot(rm) },
            "string map state at step {step}, ownership {ownership_mode:?}"
        );
    }
    unsafe {
        free_string(c, cm);
        free_string(rust, rm);
    }
}

// CONFIGS C22-C27, C30-C33, C35-C36, C47-C48; ERROR E19.
#[test]
fn string_map_modes_and_pipelines_match() {
    unsafe {
        let (c, rust) = apis();

        let missing = CString::new("missing").unwrap();
        let cm = (c.hmdefault)(null_mut(), size_of::<StringEntry>());
        let rm = (rust.hmdefault)(null_mut(), size_of::<StringEntry>());
        let (cm, ci) = get_string(&c, cm, missing.as_ptr().cast_mut(), 1);
        let (rm, ri) = get_string(&rust, rm, missing.as_ptr().cast_mut(), 1);
        assert_eq!((ci, ri), (-1, -1));
        free_string(&c, cm);
        free_string(&rust, rm);

        for implicit_mode in [1, 2, 4] {
            let empty = CString::new("").unwrap();
            let other = CString::new("borrowed").unwrap();
            (c.rand_seed)(0x1020_3040 + implicit_mode as usize);
            (rust.rand_seed)(0x1020_3040 + implicit_mode as usize);
            let mut cm = put_string(&c, null_mut(), empty.as_ptr().cast_mut(), 1, implicit_mode);
            let mut rm = put_string(
                &rust,
                null_mut(),
                empty.as_ptr().cast_mut(),
                1,
                implicit_mode,
            );
            cm = put_string(&c, cm, other.as_ptr().cast_mut(), 2, implicit_mode);
            rm = put_string(&rust, rm, other.as_ptr().cast_mut(), 2, implicit_mode);
            cm = put_string(&c, cm, other.as_ptr().cast_mut(), 3, implicit_mode);
            rm = put_string(&rust, rm, other.as_ptr().cast_mut(), 3, implicit_mode);
            assert_eq!(string_snapshot(cm), string_snapshot(rm));
            free_string(&c, cm);
            free_string(&rust, rm);
        }

        for ownership_mode in [2, 3] {
            let mut c_source = b"owned_source\0".to_vec();
            let mut r_source = b"owned_source\0".to_vec();
            let cm = (c.shmode)(size_of::<StringEntry>(), ownership_mode);
            let rm = (rust.shmode)(size_of::<StringEntry>(), ownership_mode);
            let cm = put_string(&c, cm, c_source.as_mut_ptr().cast(), 41, 1);
            let rm = put_string(&rust, rm, r_source.as_mut_ptr().cast(), 41, 1);
            c_source[..5].copy_from_slice(b"xxxxx");
            r_source[..5].copy_from_slice(b"yyyyy");
            assert_eq!(string_snapshot(cm), string_snapshot(rm));
            assert_eq!(string_snapshot(cm).3[0].0, b"owned_source");
            free_string(&c, cm);
            free_string(&rust, rm);
        }

        for invalid_mode in [-1, 4] {
            let cm = (c.shmode)(size_of::<StringEntry>(), invalid_mode);
            let rm = (rust.shmode)(size_of::<StringEntry>(), invalid_mode);
            let ct = (*map_header(cm, size_of::<StringEntry>()))
                .hash_table
                .cast::<HashIndex>();
            let rt = (*map_header(rm, size_of::<StringEntry>()))
                .hash_table
                .cast::<HashIndex>();
            assert_eq!((*ct).string.mode, (*rt).string.mode);
            assert_eq!((*ct).string.mode, invalid_mode as u8);
            free_string(&c, cm);
            free_string(&rust, rm);
        }
    }

    unsafe {
        let (c, rust) = apis();
        run_string_pipeline(&c, &rust, None);
        run_string_pipeline(&c, &rust, Some(1));
        run_string_pipeline(&c, &rust, Some(2));
        run_string_pipeline(&c, &rust, Some(3));
    }
}

#[repr(C)]
struct WideEntry {
    key: [u8; 16],
    value: u64,
}

// CONFIG C20 key-width axis.
#[test]
fn binary_key_widths_match() {
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0x8888_7777_6666_5555);
        for key_size in [1usize, 4, 8, 16] {
            (c.rand_seed)(0x9911 + key_size);
            (rust.rand_seed)(0x9911 + key_size);
            let mut cm = null_mut();
            let mut rm = null_mut();
            for step in 0..180 {
                let mut key = [0u8; 16];
                for byte in &mut key[..key_size] {
                    *byte = rng.next() as u8;
                }
                cm = (c.hmput)(
                    cm,
                    size_of::<WideEntry>(),
                    key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                rm = (rust.hmput)(
                    rm,
                    size_of::<WideEntry>(),
                    key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                let ci = (*map_header(cm, size_of::<WideEntry>())).temp;
                let ri = (*map_header(rm, size_of::<WideEntry>())).temp;
                assert_eq!(ci, ri);
                (*cm.cast::<WideEntry>().add(ci as usize)).value = step;
                (*rm.cast::<WideEntry>().add(ri as usize)).value = step;
                let count = (*map_header(cm, size_of::<WideEntry>())).length - 1;
                assert_eq!(count, (*map_header(rm, size_of::<WideEntry>())).length - 1);
                for index in 0..count {
                    assert_eq!(
                        &(&(*cm.cast::<WideEntry>().add(index)).key)[..key_size],
                        &(&(*rm.cast::<WideEntry>().add(index)).key)[..key_size]
                    );
                    assert_eq!(
                        (*cm.cast::<WideEntry>().add(index)).value,
                        (*rm.cast::<WideEntry>().add(index)).value
                    );
                }
            }
            (c.hmfree)(raw_map(cm, size_of::<WideEntry>()), size_of::<WideEntry>());
            (rust.hmfree)(raw_map(rm, size_of::<WideEntry>()), size_of::<WideEntry>());
        }
    }
}

// CONFIGS C37-C42.
#[test]
fn string_arena_growth_oversize_and_reset_match() {
    unsafe {
        let (c, rust) = apis();
        let mut ca: StringArena = zeroed();
        let mut ra: StringArena = zeroed();
        let mut rng = Rng::new(0xabc0_1234_5555_9999);

        for _ in 0..500 {
            let length = rng.usize(96);
            let input = CString::new(
                (0..length)
                    .map(|_| b'a' + rng.usize(26) as u8)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let cp = (c.stralloc)(&mut ca, input.as_ptr().cast_mut());
            let rp = (rust.stralloc)(&mut ra, input.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(
                (ca.remaining, ca.block, ca.mode),
                (ra.remaining, ra.block, ra.mode)
            );
        }
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
        assert!(ca.storage.is_null() && ra.storage.is_null());
        assert_eq!((ca.remaining, ca.block, ca.mode), (0, 0, 0));
        assert_eq!((ra.remaining, ra.block, ra.mode), (0, 0, 0));

        for _ in 0..24 {
            let block_size = 512usize << ((ca.block as usize) >> 1);
            let block_size = block_size.min(1 << 20);
            let input = CString::new(vec![b'z'; block_size - 1]).unwrap();
            let cp = (c.stralloc)(&mut ca, input.as_ptr().cast_mut());
            let rp = (rust.stralloc)(&mut ra, input.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        }
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);

        for existing_storage in [false, true] {
            let mut ca: StringArena = zeroed();
            let mut ra: StringArena = zeroed();
            if existing_storage {
                let small = CString::new("small").unwrap();
                (c.stralloc)(&mut ca, small.as_ptr().cast_mut());
                (rust.stralloc)(&mut ra, small.as_ptr().cast_mut());
            }
            let oversize = CString::new(vec![b'q'; 2048]).unwrap();
            let cp = (c.stralloc)(&mut ca, oversize.as_ptr().cast_mut());
            let rp = (rust.stralloc)(&mut ra, oversize.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
            (c.strreset)(&mut ca);
            (rust.strreset)(&mut ra);
            assert!(ca.storage.is_null() && ra.storage.is_null());
        }

        let mut ca: StringArena = zeroed();
        let mut ra: StringArena = zeroed();
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
        assert_eq!(
            (ca.remaining, ca.block, ca.mode),
            (ra.remaining, ra.block, ra.mode)
        );
    }
}

// CONFIG C43.
#[test]
fn strkey_static_buffer_matches() {
    unsafe {
        let (c, rust) = apis();
        for number in [c_int::MIN, -1_000_000, -1, 0, 1, 42, 1_000_000, c_int::MAX] {
            let c_pointer = (c.strkey)(number);
            let r_pointer = (rust.strkey)(number);
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                CStr::from_ptr(r_pointer).to_bytes()
            );
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                format!("test_{number}").as_bytes()
            );
        }

        let c_first = (c.strkey)(1);
        let r_first = (rust.strkey)(1);
        (c.strkey)(2);
        (rust.strkey)(2);
        assert_eq!(CStr::from_ptr(c_first).to_bytes(), b"test_2");
        assert_eq!(CStr::from_ptr(r_first).to_bytes(), b"test_2");
    }
}

unsafe extern "C" {
    fn pipe(file_descriptors: *mut c_int) -> c_int;
    fn dup(file_descriptor: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(file_descriptor: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(function: Helxo, letter: c_char) -> Vec<u8> {
    unsafe {
        fflush(null_mut());
        let mut descriptors = [0; 2];
        assert_eq!(pipe(descriptors.as_mut_ptr()), 0);
        let saved = dup(1);
        assert!(saved >= 0);
        assert_eq!(dup2(descriptors[1], 1), 1);
        close(descriptors[1]);
        function(letter);
        fflush(null_mut());
        assert_eq!(dup2(saved, 1), 1);
        close(saved);
        let mut output = Vec::new();
        File::from_raw_fd(descriptors[0])
            .read_to_end(&mut output)
            .unwrap();
        output
    }
}

// CONFIGS C44-C45.
#[test]
fn helxo_stdout_matches_byte_for_byte() {
    unsafe {
        let (c, rust) = apis();
        for letter in [
            0,
            b'\n' as c_char,
            b'Q' as c_char,
            0x7f,
            0x80u8 as c_char,
            0xffu8 as c_char,
        ] {
            (c.rand_seed)(0x3141_5926);
            (rust.rand_seed)(0x3141_5926);
            let c_output = capture_stdout(c.helxo, letter);
            let rust_output = capture_stdout(rust.helxo, letter);
            assert_eq!(c_output, rust_output, "helxo output for {letter}");
        }
    }
}

// CONFIGS C18-C19 and the non-TS wrapper branch.
#[test]
fn hmget_wrapper_header_temp_matches() {
    unsafe {
        let (c, rust) = apis();
        for mode in [0, 1, 2, -1] {
            let mut key = if mode >= 1 {
                CString::new("missing").unwrap().into_bytes_with_nul()
            } else {
                0x1234_5678_u64.to_ne_bytes().to_vec()
            };
            let cm = (c.hmget)(
                null_mut(),
                size_of::<BinEntry>(),
                key.as_mut_ptr().cast(),
                key.len().min(8),
                mode,
            );
            let rm = (rust.hmget)(
                null_mut(),
                size_of::<BinEntry>(),
                key.as_mut_ptr().cast(),
                key.len().min(8),
                mode,
            );
            assert_eq!(
                (*map_header(cm, size_of::<BinEntry>())).temp,
                (*map_header(rm, size_of::<BinEntry>())).temp
            );
            assert_eq!((*map_header(cm, size_of::<BinEntry>())).temp, -1);
            free_bin(&c, cm);
            free_bin(&rust, rm);
        }
    }
}

unsafe fn run_isolated_case(api: &Api, case: &str) {
    match case {
        "null_arrfree" => unsafe { (api.arrfree)(null_mut()) },
        "null_hash_string" => unsafe {
            (api.hash_string)(null_mut(), 0);
        },
        "null_hash_bytes" => unsafe {
            (api.hash_bytes)(null_mut(), 1, 0);
        },
        "oversized_hash_bytes" => unsafe {
            (api.hash_bytes)(null_mut(), usize::MAX, 0);
        },
        "null_stralloc_arena" => unsafe {
            let value = CString::new("value").unwrap();
            (api.stralloc)(null_mut(), value.as_ptr().cast_mut());
        },
        "null_stralloc_string" => unsafe {
            let mut arena: StringArena = zeroed();
            (api.stralloc)(&mut arena, null_mut());
        },
        "null_strreset_arena" => unsafe {
            (api.strreset)(null_mut());
        },
        "null_lookup_temp" => unsafe {
            let mut key = 1u64;
            (api.hmget_ts)(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                8,
                null_mut(),
                0,
            );
        },
        "null_put_key" => unsafe {
            (api.hmput)(null_mut(), size_of::<BinEntry>(), null_mut(), 8, 0);
        },
        "null_get_key" => unsafe {
            let map = put_bin(api, null_mut(), 1, 1);
            let mut temporary = 0;
            (api.hmget_ts)(map, size_of::<BinEntry>(), null_mut(), 8, &mut temporary, 0);
        },
        "null_delete_key" => unsafe {
            let map = put_bin(api, null_mut(), 1, 1);
            (api.hmdel)(map, size_of::<BinEntry>(), null_mut(), 8, 0, 0);
        },
        "oversized_map_key" => unsafe {
            let map = put_bin(api, null_mut(), 1, 1);
            let mut key = 2u64;
            (api.hmput)(
                map,
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                usize::MAX,
                0,
            );
        },
        "assert_hash_index_threshold" => unsafe {
            (api.rand_seed)(0x1111);
            let map = put_bin(api, null_mut(), 1, 1);
            let table = (*map_header(map, size_of::<BinEntry>()))
                .hash_table
                .cast::<HashIndex>();
            (*table).slot_count = 0;
            (*table).used_count_threshold = 0;
            let _ = put_bin(api, map, 2, 2);
        },
        "assert_moved_key_missing" => unsafe {
            (api.rand_seed)(0x2222);
            let mut map = put_bin(api, null_mut(), 1, 1);
            map = put_bin(api, map, 2, 2);
            (*map.cast::<BinEntry>().add(1)).key = 3;
            let _ = del_bin(api, map, 1);
        },
        "assert_moved_index_wrong" => unsafe {
            (api.rand_seed)(0x3333);
            let mut map = put_bin(api, null_mut(), 1, 1);
            map = put_bin(api, map, 2, 2);
            map = put_bin(api, map, 3, 3);
            (*map.cast::<BinEntry>().add(2)).key = 2;
            let _ = del_bin(api, map, 1);
        },
        _ => panic!("unknown isolated case {case}"),
    }
}

#[test]
fn isolated_ffi_case() {
    let Some(case) = std::env::var_os("DIFFERENTIAL_ISOLATED_CASE") else {
        return;
    };
    let library = std::env::var("DIFFERENTIAL_LIBRARY").unwrap();
    let path = match library.as_str() {
        "c" => c_library(),
        "rust" => rust_library(),
        _ => panic!("unknown library {library}"),
    };
    unsafe {
        let api = Api::load(&path);
        run_isolated_case(&api, &case.to_string_lossy());
    }
}

fn isolated_status(library: &str, case: &str) -> (Option<i32>, Option<i32>, Vec<u8>) {
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("isolated_ffi_case")
        .arg("--exact")
        .arg("--nocapture")
        .env("DIFFERENTIAL_LIBRARY", library)
        .env("DIFFERENTIAL_ISOLATED_CASE", case)
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap();
    (output.status.code(), output.status.signal(), output.stderr)
}

// ERRORS E09, E13-E14, E16, and E18.
#[test]
fn isolated_faults_and_constructible_assertions_match() {
    for case in [
        "null_arrfree",
        "null_hash_string",
        "null_hash_bytes",
        "oversized_hash_bytes",
        "null_stralloc_arena",
        "null_stralloc_string",
        "null_strreset_arena",
        "null_lookup_temp",
        "null_put_key",
        "null_get_key",
        "null_delete_key",
        "oversized_map_key",
        "assert_hash_index_threshold",
        "assert_moved_key_missing",
        "assert_moved_index_wrong",
    ] {
        let c = isolated_status("c", case);
        let rust = isolated_status("rust", case);
        assert!(
            c.0 != Some(0) || c.1.is_some(),
            "C unexpectedly accepted {case}"
        );
        assert_eq!(
            (c.0, c.1),
            (rust.0, rust.1),
            "termination mismatch for {case}\nC stderr: {}\nRust stderr: {}",
            String::from_utf8_lossy(&c.2),
            String::from_utf8_lossy(&rust.2)
        );
    }
}

// ERRORS E10-E12 and E15 are postcondition/unsigned invariants that valid public
// calls cannot falsify. This test audits their presence and the many FFI tests
// above execute their normal non-aborting paths.
#[test]
fn internal_assert_surface_is_preserved() {
    let c = std::fs::read_to_string(manifest_dir().join("../c_src/src/lib.c")).unwrap();
    let rust = std::fs::read_to_string(manifest_dir().join("src/lib.rs")).unwrap();
    for expression in [
        "STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a));",
        "STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count);",
        "STBDS_ASSERT(table->used_count >= 0);",
        "STBDS_ASSERT(slot >= 0);",
        "STBDS_ASSERT(b->index[i] == final_index);",
        "STBDS_ASSERT(len <= a->remaining);",
    ] {
        assert!(c.contains(expression), "missing C assertion: {expression}");
    }
    for expression in [
        "assert!(item as usize + 1 <= array_capacity(raw_array));",
        "assert!(slot < (*table).slot_count as isize);",
        "used_count >= 0",
        "assert!(slot >= 0);",
        "assert!((*bucket).index[bucket_item] == final_index);",
        "assert!(length <= (*arena).remaining);",
    ] {
        assert!(
            rust.contains(expression),
            "missing Rust parity point: {expression}"
        );
    }
}

// Generic zero-sized key boundary from E17.
#[test]
fn zero_sized_null_binary_key_matches() {
    unsafe {
        let (c, rust) = apis();
        (c.rand_seed)(0x7777);
        (rust.rand_seed)(0x7777);
        let cm = (c.hmput)(null_mut(), size_of::<BinEntry>(), null_mut(), 0, 0);
        let rm = (rust.hmput)(null_mut(), size_of::<BinEntry>(), null_mut(), 0, 0);
        assert_eq!(
            (*map_header(cm, size_of::<BinEntry>())).temp,
            (*map_header(rm, size_of::<BinEntry>())).temp
        );
        let mut ci = 5;
        let mut ri = 5;
        let cm = (c.hmget_ts)(cm, size_of::<BinEntry>(), null_mut(), 0, &mut ci, 0);
        let rm = (rust.hmget_ts)(rm, size_of::<BinEntry>(), null_mut(), 0, &mut ri, 0);
        assert_eq!(ci, ri);
        free_bin(&c, cm);
        free_bin(&rust, rm);
    }
}
