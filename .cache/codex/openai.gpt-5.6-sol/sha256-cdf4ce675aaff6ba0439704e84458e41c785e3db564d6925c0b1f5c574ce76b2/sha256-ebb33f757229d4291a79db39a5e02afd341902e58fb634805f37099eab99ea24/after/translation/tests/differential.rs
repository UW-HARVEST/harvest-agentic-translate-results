use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;

const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;
const BUCKET_LENGTH: usize = 8;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type RandSeed = unsafe extern "C" fn(usize);
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type HmGeti = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    hash_bytes: HashBytes,
    hash_string: HashString,
    rand_seed: RandSeed,
    hmget: HmGet,
    hmget_ts: HmGetTs,
    hmput_default: HmPutDefault,
    hmput: HmPut,
    hmdel: HmDel,
    hmfree: HmFree,
    shmode: ShMode,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    hm_geti: HmGeti,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }.unwrap()
            };
        }
        let result = Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            shmode: symbol!("stbds_shmode_func", ShMode),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            hm_geti: symbol!("hm_geti", HmGeti),
            _library: library,
        };
        result
    }
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

impl StringArena {
    fn empty() -> Self {
        Self {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
}

#[repr(C)]
#[derive(Clone, Copy)]
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
struct Entry {
    key: u64,
    value: u64,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-aKhAwV.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libhm_geti_lib.so")
}

unsafe fn load_pair() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

unsafe fn header(raw_array: *mut c_void) -> *mut ArrayHeader {
    unsafe { (raw_array as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader }
}

unsafe fn raw_from_map(map: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(elem_size) as *mut c_void }
}

unsafe fn map_header(map: *mut c_void, elem_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_from_map(map, elem_size)) }
}

unsafe fn table(map: *mut c_void, elem_size: usize) -> *mut HashIndex {
    unsafe { (*map_header(map, elem_size)).hash_table as *mut HashIndex }
}

#[derive(Debug, PartialEq, Eq)]
struct BinarySnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<u8>,
    table: Option<TableSnapshot>,
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
    buckets: Vec<HashBucket>,
}

unsafe fn binary_snapshot(map: *mut c_void, elem_size: usize) -> BinarySnapshot {
    let raw = unsafe { raw_from_map(map, elem_size) };
    let h = unsafe { *header(raw) };
    let entries =
        unsafe { std::slice::from_raw_parts(raw as *const u8, h.length * elem_size).to_vec() };
    let hash_table = h.hash_table as *mut HashIndex;
    let table = if hash_table.is_null() {
        None
    } else {
        let t = unsafe { &*hash_table };
        let bucket_count = t.slot_count / BUCKET_LENGTH;
        Some(TableSnapshot {
            slot_count: t.slot_count,
            used_count: t.used_count,
            used_count_threshold: t.used_count_threshold,
            used_count_shrink_threshold: t.used_count_shrink_threshold,
            tombstone_count: t.tombstone_count,
            tombstone_count_threshold: t.tombstone_count_threshold,
            seed: t.seed,
            slot_count_log2: t.slot_count_log2,
            string_remaining: t.string.remaining,
            string_block: t.string.block,
            string_mode: t.string.mode,
            buckets: unsafe { std::slice::from_raw_parts(t.storage, bucket_count).to_vec() },
        })
    };
    BinarySnapshot {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        entries,
        table,
    }
}

unsafe fn put_entry(api: &Api, map: &mut *mut Entry, key: u64, value: u64, mode: c_int) {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmput)(
            *map as *mut c_void,
            size_of::<Entry>(),
            &mut key_copy as *mut u64 as *mut c_void,
            size_of::<u64>(),
            mode,
        ) as *mut Entry
    };
    let index = unsafe { (*map_header(*map as *mut c_void, size_of::<Entry>())).temp };
    unsafe {
        (*map.offset(index)).key = key;
        (*map.offset(index)).value = value;
    }
}

unsafe fn get_entry(api: &Api, map: &mut *mut Entry, key: u64, ts: bool) -> isize {
    let mut key_copy = key;
    if ts {
        let mut temp = 777isize;
        *map = unsafe {
            (api.hmget_ts)(
                *map as *mut c_void,
                size_of::<Entry>(),
                &mut key_copy as *mut u64 as *mut c_void,
                size_of::<u64>(),
                &mut temp,
                HM_BINARY,
            ) as *mut Entry
        };
        temp
    } else {
        *map = unsafe {
            (api.hmget)(
                *map as *mut c_void,
                size_of::<Entry>(),
                &mut key_copy as *mut u64 as *mut c_void,
                size_of::<u64>(),
                HM_BINARY,
            ) as *mut Entry
        };
        unsafe { (*map_header(*map as *mut c_void, size_of::<Entry>())).temp }
    }
}

unsafe fn del_entry(api: &Api, map: &mut *mut Entry, key: u64, key_offset: usize) -> isize {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmdel)(
            *map as *mut c_void,
            size_of::<Entry>(),
            &mut key_copy as *mut u64 as *mut c_void,
            size_of::<u64>(),
            key_offset,
            HM_BINARY,
        ) as *mut Entry
    };
    if map.is_null() || (*map).is_null() {
        0
    } else {
        unsafe { (*map_header(*map as *mut c_void, size_of::<Entry>())).temp }
    }
}

unsafe fn free_entry_map(api: &Api, map: *mut Entry) {
    if !map.is_null() {
        unsafe {
            (api.hmfree)(
                raw_from_map(map as *mut c_void, size_of::<Entry>()),
                size_of::<Entry>(),
            )
        };
    }
}

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
}

#[test]
fn dynamic_symbol_surface_matches() {
    let symbols = |path: &Path| {
        let output = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut names: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
            .collect();
        names.sort();
        names
    };
    assert_eq!(symbols(&c_library_path()), symbols(&rust_library_path()));
}

#[test]
fn hashes_match_for_all_tail_shapes_and_random_data() {
    unsafe {
        let (c, rust) = load_pair();
        let mut rng = Rng::new(0x8d12_7acf_4421_9901);
        for len in 0..=263usize {
            for _ in 0..40 {
                let mut bytes = vec![0u8; len.max(1)];
                for byte in bytes.iter_mut().take(len) {
                    *byte = rng.next() as u8;
                }
                let seed = rng.next() as usize;
                let pointer = if len == 0 {
                    null_mut()
                } else {
                    bytes.as_mut_ptr() as *mut c_void
                };
                assert_eq!(
                    (c.hash_bytes)(pointer, len, seed),
                    (rust.hash_bytes)(pointer, len, seed),
                    "byte hash mismatch at len={len}, seed={seed:#x}"
                );
            }
        }

        let string_cases = [
            vec![],
            b"a".to_vec(),
            b"hello world".to_vec(),
            vec![0x80, 0xff, 0x7f, 0x01],
        ];
        for seed in [0, 1, usize::MAX, 0x3141_5926] {
            for bytes in &string_cases {
                let mut terminated = bytes.clone();
                terminated.push(0);
                let pointer = terminated.as_mut_ptr() as *mut c_char;
                assert_eq!(
                    (c.hash_string)(pointer, seed),
                    (rust.hash_string)(pointer, seed)
                );
            }
        }
    }
}

#[test]
fn array_growth_and_free_match() {
    unsafe {
        let (c, rust) = load_pair();
        for elem_size in [1usize, 3, 4, 7, 16] {
            let c_none = (c.arrgrow)(null_mut(), elem_size, 0, 0);
            let r_none = (rust.arrgrow)(null_mut(), elem_size, 0, 0);
            assert!(c_none.is_null() && r_none.is_null());

            for &(add, min) in &[(0, 1), (1, 0), (0, 4), (0, 9)] {
                let mut ca = (c.arrgrow)(null_mut(), elem_size, add, min);
                let mut ra = (rust.arrgrow)(null_mut(), elem_size, add, min);
                let ch = *header(ca);
                let rh = *header(ra);
                assert_eq!(
                    (ch.length, ch.capacity, ch.temp),
                    (rh.length, rh.capacity, rh.temp)
                );
                assert!(ch.hash_table.is_null() && rh.hash_table.is_null());
                std::ptr::write_bytes(ca, 0x5a, elem_size * ch.capacity);
                std::ptr::write_bytes(ra, 0x5a, elem_size * rh.capacity);

                let old_c = ca;
                let old_r = ra;
                ca = (c.arrgrow)(ca, elem_size, 0, ch.capacity);
                ra = (rust.arrgrow)(ra, elem_size, 0, rh.capacity);
                assert_eq!(ca, old_c);
                assert_eq!(ra, old_r);

                (*header(ca)).length = ch.capacity;
                (*header(ra)).length = rh.capacity;
                ca = (c.arrgrow)(ca, elem_size, 1, 0);
                ra = (rust.arrgrow)(ra, elem_size, 1, 0);
                assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
                assert_eq!(
                    std::slice::from_raw_parts(ca as *const u8, elem_size * ch.capacity),
                    std::slice::from_raw_parts(ra as *const u8, elem_size * rh.capacity)
                );

                let explicit = (*header(ca)).capacity * 3 + 1;
                ca = (c.arrgrow)(ca, elem_size, 0, explicit);
                ra = (rust.arrgrow)(ra, elem_size, 0, explicit);
                assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
                (c.arrfree)(ca);
                (rust.arrfree)(ra);
            }
        }
    }
}

#[test]
fn binary_map_randomized_state_matches() {
    unsafe {
        let (c, rust) = load_pair();
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut cm: *mut Entry = null_mut();
        let mut rm: *mut Entry = null_mut();

        cm = (c.hmput_default)(cm as *mut c_void, size_of::<Entry>()) as *mut Entry;
        rm = (rust.hmput_default)(rm as *mut c_void, size_of::<Entry>()) as *mut Entry;
        (*cm.offset(-1)).value = 0xdead_beef;
        (*rm.offset(-1)).value = 0xdead_beef;
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );

        let before_c = cm;
        let before_r = rm;
        cm = (c.hmput_default)(cm as *mut c_void, size_of::<Entry>()) as *mut Entry;
        rm = (rust.hmput_default)(rm as *mut c_void, size_of::<Entry>()) as *mut Entry;
        assert_eq!(cm, before_c);
        assert_eq!(rm, before_r);

        let mut rng = Rng::new(0x621a_70b4_993e_1207);
        for step in 0..4000 {
            let key = rng.next() % 180;
            match rng.next() % 5 {
                0 | 1 => {
                    let value = rng.next();
                    put_entry(&c, &mut cm, key, value, HM_BINARY);
                    put_entry(&rust, &mut rm, key, value, HM_BINARY);
                }
                2 => {
                    let ci = get_entry(&c, &mut cm, key, false);
                    let ri = get_entry(&rust, &mut rm, key, false);
                    assert_eq!(ci, ri);
                    if ci >= 0 {
                        assert_eq!(*cm.offset(ci), *rm.offset(ri));
                    }
                }
                3 => {
                    let ci = get_entry(&c, &mut cm, key, true);
                    let ri = get_entry(&rust, &mut rm, key, true);
                    assert_eq!(ci, ri);
                    if ci >= 0 {
                        assert_eq!(*cm.offset(ci), *rm.offset(ri));
                    }
                }
                _ => assert_eq!(
                    del_entry(&c, &mut cm, key, 0),
                    del_entry(&rust, &mut rm, key, 0)
                ),
            }
            assert_eq!(
                binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
                binary_snapshot(rm as *mut c_void, size_of::<Entry>()),
                "map state diverged at operation {step}"
            );
        }

        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);
    }
}

unsafe fn fresh_single_entry_map(api: &Api, seed: usize, key: u64) -> *mut Entry {
    unsafe { (api.rand_seed)(seed) };
    let mut map = null_mut();
    unsafe { put_entry(api, &mut map, key, key.wrapping_mul(7), HM_BINARY) };
    map
}

#[test]
fn deterministic_map_branch_transitions_match() {
    unsafe {
        let (c, rust) = load_pair();

        let c_same_a = fresh_single_entry_map(&c, 0x1111, 44);
        let c_same_b = fresh_single_entry_map(&c, 0x1111, 44);
        let r_same_a = fresh_single_entry_map(&rust, 0x1111, 44);
        let r_same_b = fresh_single_entry_map(&rust, 0x1111, 44);
        assert_eq!(
            binary_snapshot(c_same_a as *mut c_void, size_of::<Entry>()),
            binary_snapshot(c_same_b as *mut c_void, size_of::<Entry>())
        );
        assert_eq!(
            binary_snapshot(r_same_a as *mut c_void, size_of::<Entry>()),
            binary_snapshot(r_same_b as *mut c_void, size_of::<Entry>())
        );
        assert_eq!(
            binary_snapshot(c_same_a as *mut c_void, size_of::<Entry>()),
            binary_snapshot(r_same_a as *mut c_void, size_of::<Entry>())
        );
        for map in [c_same_a, c_same_b] {
            free_entry_map(&c, map);
        }
        for map in [r_same_a, r_same_b] {
            free_entry_map(&rust, map);
        }

        let c_different = fresh_single_entry_map(&c, 0x2222, 44);
        let r_different = fresh_single_entry_map(&rust, 0x2222, 44);
        assert_eq!(
            binary_snapshot(c_different as *mut c_void, size_of::<Entry>()),
            binary_snapshot(r_different as *mut c_void, size_of::<Entry>())
        );
        assert_eq!(
            (*table(c_different as *mut c_void, size_of::<Entry>())).seed,
            0x2222
        );
        free_entry_map(&c, c_different);
        free_entry_map(&rust, r_different);

        (c.rand_seed)(0x3333);
        (rust.rand_seed)(0x3333);
        let mut cm: *mut Entry = null_mut();
        let mut rm: *mut Entry = null_mut();
        put_entry(&c, &mut cm, 1, 10, HM_BINARY);
        put_entry(&rust, &mut rm, 1, 10, HM_BINARY);
        let seed = (*table(cm as *mut c_void, size_of::<Entry>())).seed;
        let mut same_slot_keys = Vec::new();
        for candidate in 2..100_000u64 {
            let mut key = candidate;
            let mut hash =
                (c.hash_bytes)(&mut key as *mut u64 as *mut c_void, size_of::<u64>(), seed);
            if hash < 2 {
                hash += 2;
            }
            if hash & 7 == 7 {
                same_slot_keys.push(candidate);
                if same_slot_keys.len() == 2 {
                    break;
                }
            }
        }
        assert_eq!(same_slot_keys.len(), 2);
        put_entry(&c, &mut cm, same_slot_keys[0], 20, HM_BINARY);
        put_entry(&rust, &mut rm, same_slot_keys[0], 20, HM_BINARY);
        assert_eq!(get_entry(&c, &mut cm, same_slot_keys[1], true), -1);
        assert_eq!(get_entry(&rust, &mut rm, same_slot_keys[1], true), -1);
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );
        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);

        (c.rand_seed)(0x4444);
        (rust.rand_seed)(0x4444);
        cm = null_mut();
        rm = null_mut();
        for key in 0..4 {
            put_entry(&c, &mut cm, key, key + 100, HM_BINARY);
            put_entry(&rust, &mut rm, key, key + 100, HM_BINARY);
        }
        assert_eq!(del_entry(&c, &mut cm, 1, 0), 1);
        assert_eq!(del_entry(&rust, &mut rm, 1, 0), 1);
        let c_table = table(cm as *mut c_void, size_of::<Entry>());
        let tombstone_slot = std::slice::from_raw_parts(
            (*c_table).storage as *const HashBucket,
            (*c_table).slot_count / BUCKET_LENGTH,
        )
        .iter()
        .flat_map(|bucket| bucket.index)
        .position(|index| index == -2)
        .unwrap();
        let seed = (*c_table).seed;
        let mut replacement = 10_000u64;
        loop {
            let mut candidate = replacement;
            let mut hash = (c.hash_bytes)(
                &mut candidate as *mut u64 as *mut c_void,
                size_of::<u64>(),
                seed,
            );
            if hash < 2 {
                hash += 2;
            }
            if hash & ((*c_table).slot_count - 1) == tombstone_slot {
                break;
            }
            replacement += 1;
        }
        let before_tombstones = (*c_table).tombstone_count;
        put_entry(&c, &mut cm, replacement, 999, HM_BINARY);
        put_entry(&rust, &mut rm, replacement, 999, HM_BINARY);
        assert_eq!(
            (*table(cm as *mut c_void, size_of::<Entry>())).tombstone_count,
            before_tombstones - 1
        );
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );
        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);

        (c.rand_seed)(0x5555);
        (rust.rand_seed)(0x5555);
        cm = null_mut();
        rm = null_mut();
        for key in 0..5 {
            put_entry(&c, &mut cm, key, key, HM_BINARY);
            put_entry(&rust, &mut rm, key, key, HM_BINARY);
        }
        del_entry(&c, &mut cm, 0, 0);
        del_entry(&rust, &mut rm, 0, 0);
        del_entry(&c, &mut cm, 1, 0);
        del_entry(&rust, &mut rm, 1, 0);
        assert_eq!(
            (*table(cm as *mut c_void, size_of::<Entry>())).tombstone_count,
            0
        );
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );
        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);

        (c.rand_seed)(0x6666);
        (rust.rand_seed)(0x6666);
        cm = null_mut();
        rm = null_mut();
        for key in 0..160 {
            put_entry(&c, &mut cm, key, key, HM_BINARY);
            put_entry(&rust, &mut rm, key, key, HM_BINARY);
        }
        let original_slots = (*table(cm as *mut c_void, size_of::<Entry>())).slot_count;
        for key in 0..150 {
            del_entry(&c, &mut cm, key, 0);
            del_entry(&rust, &mut rm, key, 0);
        }
        assert!((*table(cm as *mut c_void, size_of::<Entry>())).slot_count < original_slots);
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );
        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);
    }
}

#[test]
fn binary_key_sizes_and_nonzero_delete_offset_match() {
    unsafe {
        let (c, rust) = load_pair();
        for key_size in [1usize, 4, 8, 13] {
            for payload_size in [0usize, 9] {
                (c.rand_seed)(99 + key_size + payload_size);
                (rust.rand_seed)(99 + key_size + payload_size);
                let elem_size = key_size + payload_size;
                let mut cm = null_mut();
                let mut rm = null_mut();
                for n in 0..80u8 {
                    let mut key = vec![0u8; key_size];
                    for (index, byte) in key.iter_mut().enumerate() {
                        *byte = n.wrapping_mul(17).wrapping_add(index as u8);
                    }
                    cm = (c.hmput)(
                        cm,
                        elem_size,
                        key.as_mut_ptr() as *mut c_void,
                        key_size,
                        HM_BINARY,
                    );
                    rm = (rust.hmput)(
                        rm,
                        elem_size,
                        key.as_mut_ptr() as *mut c_void,
                        key_size,
                        HM_BINARY,
                    );
                    if payload_size != 0 {
                        let ci = (*map_header(cm, elem_size)).temp as usize;
                        let ri = (*map_header(rm, elem_size)).temp as usize;
                        std::ptr::write_bytes(
                            (cm as *mut u8).add(ci * elem_size + key_size),
                            n,
                            payload_size,
                        );
                        std::ptr::write_bytes(
                            (rm as *mut u8).add(ri * elem_size + key_size),
                            n,
                            payload_size,
                        );
                    }
                }
                assert_eq!(
                    binary_snapshot(cm, elem_size),
                    binary_snapshot(rm, elem_size)
                );
                (c.hmfree)(raw_from_map(cm, elem_size), elem_size);
                (rust.hmfree)(raw_from_map(rm, elem_size), elem_size);
            }
        }

        let mut cm: *mut Entry = null_mut();
        let mut rm: *mut Entry = null_mut();
        for key in 10..18u64 {
            put_entry(&c, &mut cm, key, key * 3, HM_BINARY);
            put_entry(&rust, &mut rm, key, key * 3, HM_BINARY);
        }
        for index in 0..8 {
            let ce = cm.add(index);
            let re = rm.add(index);
            (*ce).value = (*ce).key;
            (*re).value = (*re).key;
        }
        let key = 13u64;
        assert_eq!(
            del_entry(&c, &mut cm, key, size_of::<u64>()),
            del_entry(&rust, &mut rm, key, size_of::<u64>())
        );
        assert_eq!(
            binary_snapshot(cm as *mut c_void, size_of::<Entry>()),
            binary_snapshot(rm as *mut c_void, size_of::<Entry>())
        );
        free_entry_map(&c, cm);
        free_entry_map(&rust, rm);
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
    map: &mut *mut StringEntry,
    key: *mut c_char,
    value: u64,
    mode: c_int,
) {
    *map = unsafe {
        (api.hmput)(
            *map as *mut c_void,
            size_of::<StringEntry>(),
            key as *mut c_void,
            size_of::<*mut c_char>(),
            mode,
        ) as *mut StringEntry
    };
    let index = unsafe { (*map_header(*map as *mut c_void, size_of::<StringEntry>())).temp };
    unsafe { (*map.offset(index)).value = value };
}

unsafe fn string_snapshot(map: *mut StringEntry) -> (BinarySnapshot, Vec<(Vec<u8>, u64)>) {
    let mut raw = unsafe { binary_snapshot(map as *mut c_void, size_of::<StringEntry>()) };
    raw.entries.clear();
    let length = raw.length.saturating_sub(1);
    let mut values = Vec::new();
    for index in 0..length {
        let entry = unsafe { *map.add(index) };
        values.push((
            unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
            entry.value,
        ));
    }
    (raw, values)
}

#[test]
fn string_modes_and_operations_match() {
    unsafe {
        let (c, rust) = load_pair();
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 4, 255, -1] {
            (c.rand_seed)(0x9000 + mode as usize);
            (rust.rand_seed)(0x9000 + mode as usize);
            let mut cm = (c.shmode)(size_of::<StringEntry>(), mode) as *mut StringEntry;
            let mut rm = (rust.shmode)(size_of::<StringEntry>(), mode) as *mut StringEntry;
            let out_of_range_string_mode = mode > SH_ARENA;
            let call_mode = if mode == SH_NONE || mode == -1 {
                HM_BINARY
            } else {
                HM_STRING
            };
            if call_mode == HM_BINARY {
                let mut keys: Vec<u64> = (0..40).map(|x| x * 13).collect();
                for (index, key) in keys.iter_mut().enumerate() {
                    cm = (c.hmput)(
                        cm as *mut c_void,
                        size_of::<StringEntry>(),
                        key as *mut u64 as *mut c_void,
                        size_of::<u64>(),
                        HM_BINARY,
                    ) as *mut StringEntry;
                    rm = (rust.hmput)(
                        rm as *mut c_void,
                        size_of::<StringEntry>(),
                        key as *mut u64 as *mut c_void,
                        size_of::<u64>(),
                        HM_BINARY,
                    ) as *mut StringEntry;
                    let ci = (*map_header(cm as *mut c_void, size_of::<StringEntry>())).temp;
                    let ri = (*map_header(rm as *mut c_void, size_of::<StringEntry>())).temp;
                    (*cm.offset(ci)).value = index as u64;
                    (*rm.offset(ri)).value = index as u64;
                }
                assert_eq!(
                    binary_snapshot(cm as *mut c_void, size_of::<StringEntry>()),
                    binary_snapshot(rm as *mut c_void, size_of::<StringEntry>())
                );
            } else if out_of_range_string_mode {
                let key = CString::new("abcdefgh-valid-after-copy").unwrap();
                put_string(&c, &mut cm, key.as_ptr() as *mut c_char, 91, HM_STRING);
                put_string(&rust, &mut rm, key.as_ptr() as *mut c_char, 91, HM_STRING);
                assert_eq!(
                    binary_snapshot(cm as *mut c_void, size_of::<StringEntry>()),
                    binary_snapshot(rm as *mut c_void, size_of::<StringEntry>())
                );
            } else {
                let mut storage: Vec<CString> = vec![CString::new("").unwrap()];
                storage.extend((0..120).map(|n| CString::new(format!("key-{n:03}")).unwrap()));
                for (index, key) in storage.iter_mut().enumerate() {
                    put_string(
                        &c,
                        &mut cm,
                        key.as_ptr() as *mut c_char,
                        index as u64,
                        HM_STRING,
                    );
                    put_string(
                        &rust,
                        &mut rm,
                        key.as_ptr() as *mut c_char,
                        index as u64,
                        HM_STRING,
                    );
                }
                for index in (0..storage.len()).step_by(3) {
                    put_string(
                        &c,
                        &mut cm,
                        storage[index].as_ptr() as *mut c_char,
                        10_000 + index as u64,
                        HM_STRING,
                    );
                    put_string(
                        &rust,
                        &mut rm,
                        storage[index].as_ptr() as *mut c_char,
                        10_000 + index as u64,
                        HM_STRING,
                    );
                }
                let missing = CString::new("missing").unwrap();
                let mut ct = 77;
                let mut rt = 88;
                cm = (c.hmget_ts)(
                    cm as *mut c_void,
                    size_of::<StringEntry>(),
                    missing.as_ptr() as *mut c_void,
                    size_of::<*mut c_char>(),
                    &mut ct,
                    HM_STRING,
                ) as *mut StringEntry;
                rm = (rust.hmget_ts)(
                    rm as *mut c_void,
                    size_of::<StringEntry>(),
                    missing.as_ptr() as *mut c_void,
                    size_of::<*mut c_char>(),
                    &mut rt,
                    HM_STRING,
                ) as *mut StringEntry;
                assert_eq!(ct, rt);

                for index in (1..storage.len()).step_by(4) {
                    cm = (c.hmdel)(
                        cm as *mut c_void,
                        size_of::<StringEntry>(),
                        storage[index].as_ptr() as *mut c_void,
                        size_of::<*mut c_char>(),
                        0,
                        HM_STRING,
                    ) as *mut StringEntry;
                    rm = (rust.hmdel)(
                        rm as *mut c_void,
                        size_of::<StringEntry>(),
                        storage[index].as_ptr() as *mut c_void,
                        size_of::<*mut c_char>(),
                        0,
                        HM_STRING,
                    ) as *mut StringEntry;
                    assert_eq!(string_snapshot(cm), string_snapshot(rm));
                }
                assert_eq!(string_snapshot(cm), string_snapshot(rm));

                let ownership_key = CString::new(format!("ownership-{mode}")).unwrap();
                put_string(
                    &c,
                    &mut cm,
                    ownership_key.as_ptr() as *mut c_char,
                    77,
                    HM_STRING,
                );
                put_string(
                    &rust,
                    &mut rm,
                    ownership_key.as_ptr() as *mut c_char,
                    77,
                    HM_STRING,
                );
                let ci = (*map_header(cm as *mut c_void, size_of::<StringEntry>())).temp;
                let ri = (*map_header(rm as *mut c_void, size_of::<StringEntry>())).temp;
                assert_eq!(
                    (*cm.offset(ci)).key == ownership_key.as_ptr() as *mut c_char,
                    mode == SH_DEFAULT
                );
                assert_eq!(
                    (*rm.offset(ri)).key == ownership_key.as_ptr() as *mut c_char,
                    mode == SH_DEFAULT
                );
                assert_eq!(
                    CStr::from_ptr((*cm.offset(ci)).key),
                    CStr::from_ptr((*rm.offset(ri)).key)
                );
            }
            (c.hmfree)(
                raw_from_map(cm as *mut c_void, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            (rust.hmfree)(
                raw_from_map(rm as *mut c_void, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }
    }
}

#[test]
fn string_arena_boundaries_match() {
    unsafe {
        let (c, rust) = load_pair();
        let mut ca = StringArena::empty();
        let mut ra = StringArena::empty();
        let mut rng = Rng::new(0x1199_ae22_7500_0041);
        let lengths = [
            0usize,
            1,
            7,
            31,
            127,
            255,
            510,
            511,
            512,
            513,
            1023,
            1024,
            4097,
            (1 << 20) - 1,
            1 << 20,
            (1 << 20) + 1,
        ];
        for &len in &lengths {
            let mut bytes = vec![b'x'; len];
            for byte in &mut bytes {
                *byte = b'a' + (rng.next() % 26) as u8;
            }
            let value = CString::new(bytes).unwrap();
            let cp = (c.stralloc)(&mut ca, value.as_ptr() as *mut c_char);
            let rp = (rust.stralloc)(&mut ra, value.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(
                (ca.remaining, ca.block, ca.mode),
                (ra.remaining, ra.block, ra.mode)
            );
        }
        for n in 0..3000 {
            let value = CString::new(format!("small-{n:04}")).unwrap();
            let cp = (c.stralloc)(&mut ca, value.as_ptr() as *mut c_char);
            let rp = (rust.stralloc)(&mut ra, value.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(cp), CStr::from_ptr(rp));
            assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        }

        let exact_len = ca.remaining;
        assert!(exact_len > 1);
        let exact = CString::new(vec![b'e'; exact_len - 1]).unwrap();
        let cp = (c.stralloc)(&mut ca, exact.as_ptr() as *mut c_char);
        let rp = (rust.stralloc)(&mut ra, exact.as_ptr() as *mut c_char);
        assert_eq!(CStr::from_ptr(cp), CStr::from_ptr(rp));
        assert_eq!(ca.remaining, 0);
        assert_eq!(ca.remaining, ra.remaining);

        let one = CString::new("z").unwrap();
        (c.stralloc)(&mut ca, one.as_ptr() as *mut c_char);
        (rust.stralloc)(&mut ra, one.as_ptr() as *mut c_char);
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));

        let mut cap_ca = StringArena {
            storage: null_mut(),
            remaining: 0,
            block: 22,
            mode: 0,
        };
        let mut cap_ra = cap_ca;
        (c.stralloc)(&mut cap_ca, one.as_ptr() as *mut c_char);
        (rust.stralloc)(&mut cap_ra, one.as_ptr() as *mut c_char);
        assert_eq!(cap_ca.block, 22);
        assert_eq!(
            (cap_ca.remaining, cap_ca.block),
            (cap_ra.remaining, cap_ra.block)
        );
        (c.strreset)(&mut cap_ca);
        (rust.strreset)(&mut cap_ra);

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
fn strkey_and_hm_geti_match() {
    unsafe {
        let (c, rust) = load_pair();
        let mut rng = Rng::new(0xa51c_e1d9_4400_128f);
        let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
        values.extend((0..2000).map(|_| rng.next() as c_int));
        for value in values {
            let cp = (c.strkey)(value);
            let rp = (rust.strkey)(value);
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
        }
        for num in [-100, -1, 0, 1, 2, 7, 8, 31, 100, 1000, 10_000] {
            (c.hm_geti)(num);
            (rust.hm_geti)(num);
        }
    }
}

#[test]
fn error_sentinels_and_null_guards_match() {
    unsafe {
        let (c, rust) = load_pair();
        assert_eq!(
            (c.hash_bytes)(null_mut(), 0, 123),
            (rust.hash_bytes)(null_mut(), 0, 123)
        );
        (c.hmfree)(null_mut(), size_of::<Entry>());
        (rust.hmfree)(null_mut(), size_of::<Entry>());

        let key = 123u64;
        let mut ct = 9;
        let mut rt = 9;
        let cm = (c.hmget_ts)(
            null_mut(),
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            &mut ct,
            HM_BINARY,
        );
        let rm = (rust.hmget_ts)(
            null_mut(),
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            &mut rt,
            HM_BINARY,
        );
        assert_eq!(ct, -1);
        assert_eq!(ct, rt);
        assert_eq!(
            binary_snapshot(cm, size_of::<Entry>()),
            binary_snapshot(rm, size_of::<Entry>())
        );

        let cm = (c.hmget)(
            cm,
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            HM_BINARY,
        );
        let rm = (rust.hmget)(
            rm,
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            HM_BINARY,
        );
        assert_eq!((*map_header(cm, size_of::<Entry>())).temp, -1);
        assert_eq!(
            (*map_header(cm, size_of::<Entry>())).temp,
            (*map_header(rm, size_of::<Entry>())).temp
        );

        assert!(
            (c.hmdel)(
                null_mut(),
                size_of::<Entry>(),
                &key as *const u64 as *mut c_void,
                size_of::<u64>(),
                0,
                HM_BINARY,
            )
            .is_null()
        );
        assert!(
            (rust.hmdel)(
                null_mut(),
                size_of::<Entry>(),
                &key as *const u64 as *mut c_void,
                size_of::<u64>(),
                0,
                HM_BINARY,
            )
            .is_null()
        );

        let cm2 = (c.hmdel)(
            cm,
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        let rm2 = (rust.hmdel)(
            rm,
            size_of::<Entry>(),
            &key as *const u64 as *mut c_void,
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        assert_eq!((*map_header(cm2, size_of::<Entry>())).temp, 0);
        assert_eq!(
            binary_snapshot(cm2, size_of::<Entry>()),
            binary_snapshot(rm2, size_of::<Entry>())
        );
        (c.hmfree)(raw_from_map(cm2, size_of::<Entry>()), size_of::<Entry>());
        (rust.hmfree)(raw_from_map(rm2, size_of::<Entry>()), size_of::<Entry>());

        let c_raw = (c.arrgrow)(null_mut(), size_of::<Entry>(), 0, 1);
        let r_raw = (rust.arrgrow)(null_mut(), size_of::<Entry>(), 0, 1);
        let c_zero_length_map = (c.hmput_default)(
            (c_raw as *mut u8).add(size_of::<Entry>()) as *mut c_void,
            size_of::<Entry>(),
        );
        let r_zero_length_map = (rust.hmput_default)(
            (r_raw as *mut u8).add(size_of::<Entry>()) as *mut c_void,
            size_of::<Entry>(),
        );
        assert_eq!(
            binary_snapshot(c_zero_length_map, size_of::<Entry>()),
            binary_snapshot(r_zero_length_map, size_of::<Entry>())
        );
        (c.hmfree)(
            raw_from_map(c_zero_length_map, size_of::<Entry>()),
            size_of::<Entry>(),
        );
        (rust.hmfree)(
            raw_from_map(r_zero_length_map, size_of::<Entry>()),
            size_of::<Entry>(),
        );
    }
}

fn crash_status(library: &str, case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ffi_crash_child", "--nocapture"])
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn null_pointer_crash_behavior_matches() {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        for case in [
            "arrfree_null",
            "hash_string_null",
            "hash_bytes_null_nonzero",
            "hmget_ts_null_temp",
            "hmput_null_key",
            "stralloc_null_arena",
            "stralloc_null_string",
            "strreset_null",
            "invalid_string_mode_second_put",
        ] {
            let c = crash_status("c", case);
            let rust = crash_status("rust", case);
            assert!(!c.success(), "C unexpectedly survived {case}");
            assert!(!rust.success(), "Rust unexpectedly survived {case}");
            assert_eq!(c.signal(), rust.signal(), "different signal for {case}");
        }
    }
}

#[test]
fn ffi_crash_child() {
    let Ok(which) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CRASH_CASE").unwrap();
    unsafe {
        let api = if which == "c" {
            Api::load(&c_library_path())
        } else {
            Api::load(&rust_library_path())
        };
        match case.as_str() {
            "arrfree_null" => (api.arrfree)(null_mut()),
            "hash_string_null" => {
                (api.hash_string)(null_mut(), 0);
            }
            "hash_bytes_null_nonzero" => {
                (api.hash_bytes)(null_mut(), 1, 0);
            }
            "hmget_ts_null_temp" => {
                let mut key = 1u64;
                (api.hmget_ts)(
                    null_mut(),
                    size_of::<Entry>(),
                    &mut key as *mut u64 as *mut c_void,
                    size_of::<u64>(),
                    null_mut(),
                    HM_BINARY,
                );
            }
            "hmput_null_key" => {
                (api.hmput)(
                    null_mut(),
                    size_of::<Entry>(),
                    null_mut(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            }
            "stralloc_null_arena" => {
                let value = CString::new("x").unwrap();
                (api.stralloc)(null_mut(), value.as_ptr() as *mut c_char);
            }
            "stralloc_null_string" => {
                let mut arena = StringArena::empty();
                (api.stralloc)(&mut arena, null_mut());
            }
            "strreset_null" => (api.strreset)(null_mut()),
            "invalid_string_mode_second_put" => {
                let mut map = (api.shmode)(size_of::<StringEntry>(), 4) as *mut StringEntry;
                let first = CString::new("abcdefgh-first").unwrap();
                put_string(&api, &mut map, first.as_ptr() as *mut c_char, 1, HM_STRING);
                put_string(&api, &mut map, first.as_ptr() as *mut c_char, 2, HM_STRING);
            }
            _ => panic!("unknown crash case"),
        }
    }
}
