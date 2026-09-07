#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

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
#[derive(Clone, Copy)]
struct HashBucket {
    hash: [usize; 8],
    index: [isize; 8],
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
struct BinEntry {
    key: u64,
    value: u64,
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
type StrPut = unsafe extern "C" fn(c_int);

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
    str_put: StrPut,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = Library::new(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *library
                    .get::<$ty>(concat!($name, "\0").as_bytes())
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        let api = Self {
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
            _library: library,
        };
        api
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("../c_src/build/libharvest-work-vRYfpk.so");
    let rust_path = root.join("target/release/libstr_put_lib.so");
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

unsafe fn header(data: *mut c_void) -> *mut ArrayHeader {
    (data as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader
}

unsafe fn raw_from_map(map: *mut c_void, elemsize: usize) -> *mut c_void {
    (map as *mut u8).sub(elemsize) as *mut c_void
}

unsafe fn map_header(map: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    header(raw_from_map(map, elemsize))
}

unsafe fn hash_index(map: *mut c_void, elemsize: usize) -> *mut HashIndex {
    (*map_header(map, elemsize)).hash_table as *mut HashIndex
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

unsafe fn assert_array_state_equal(c: *mut c_void, rust: *mut c_void, payload_len: usize) {
    let ch = *header(c);
    let rh = *header(rust);
    assert_eq!(ch.length, rh.length);
    assert_eq!(ch.capacity, rh.capacity);
    assert_eq!(ch.temp, rh.temp);
    assert_eq!(ch.hash_table.is_null(), rh.hash_table.is_null());
    let cb = std::slice::from_raw_parts(c as *const u8, payload_len);
    let rb = std::slice::from_raw_parts(rust as *const u8, payload_len);
    assert_eq!(cb, rb);
}

#[test]
fn arrays_hashes_and_seeded_table_creation_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        for elemsize in [0usize, 1, 2, 3, 4, 8, 16, 31] {
            assert!((c.arrgrow)(ptr::null_mut(), elemsize, 0, 0).is_null());
            assert!((rust.arrgrow)(ptr::null_mut(), elemsize, 0, 0).is_null());

            let mut ca = (c.arrgrow)(ptr::null_mut(), elemsize, 1, 0);
            let mut ra = (rust.arrgrow)(ptr::null_mut(), elemsize, 1, 0);
            assert_array_state_equal(ca, ra, 0);
            assert_eq!((*header(ca)).capacity, 4);

            (*header(ca)).length = 3;
            (*header(ra)).length = 3;
            for index in 0..(3 * elemsize) {
                *(ca as *mut u8).add(index) = (index as u8).wrapping_mul(37);
                *(ra as *mut u8).add(index) = (index as u8).wrapping_mul(37);
            }

            let old_capacity = (*header(ca)).capacity;
            let old_c = ca;
            let old_r = ra;
            ca = (c.arrgrow)(ca, elemsize, 0, old_capacity);
            ra = (rust.arrgrow)(ra, elemsize, 0, old_capacity);
            assert_eq!(ca, old_c);
            assert_eq!(ra, old_r);
            assert_array_state_equal(ca, ra, 3 * elemsize);

            ca = (c.arrgrow)(ca, elemsize, 2, 0);
            ra = (rust.arrgrow)(ra, elemsize, 2, 0);
            assert_array_state_equal(ca, ra, 3 * elemsize);
            assert_eq!((*header(ca)).capacity, old_capacity * 2);

            ca = (c.arrgrow)(ca, elemsize, 0, 37);
            ra = (rust.arrgrow)(ra, elemsize, 0, 37);
            assert_array_state_equal(ca, ra, 3 * elemsize);
            assert_eq!((*header(ca)).capacity, 37);
            (c.arrfree)(ca);
            (rust.arrfree)(ra);

            let ca = (c.arrgrow)(ptr::null_mut(), elemsize, 9, 2);
            let ra = (rust.arrgrow)(ptr::null_mut(), elemsize, 9, 2);
            assert_array_state_equal(ca, ra, 0);
            assert_eq!((*header(ca)).capacity, 9);
            (c.arrfree)(ca);
            (rust.arrfree)(ra);
        }

        let mut random = 0x4d59_5df4_d0f3_3173u64;
        let lengths = [
            0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 63, 64, 127,
        ];
        for &length in &lengths {
            for _ in 0..80 {
                let seed = next_random(&mut random) as usize;
                let mut bytes = vec![0u8; length.max(1)];
                for byte in bytes.iter_mut().take(length) {
                    *byte = next_random(&mut random) as u8;
                }
                let cp = if length == 0 {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr() as *mut c_void
                };
                assert_eq!(
                    (c.hash_bytes)(cp, length, seed),
                    (rust.hash_bytes)(cp, length, seed),
                    "hash_bytes length={length} seed={seed}"
                );
            }
        }

        for length in [0usize, 1, 2, 7, 31, 255] {
            for _ in 0..80 {
                let seed = next_random(&mut random) as usize;
                let mut bytes = Vec::with_capacity(length + 1);
                for _ in 0..length {
                    let mut byte = next_random(&mut random) as u8;
                    if byte == 0 {
                        byte = 0x80;
                    }
                    bytes.push(byte);
                }
                bytes.push(0);
                assert_eq!(
                    (c.hash_string)(bytes.as_mut_ptr() as *mut c_char, seed),
                    (rust.hash_string)(bytes.as_mut_ptr() as *mut c_char, seed),
                    "hash_string length={length} seed={seed}"
                );
            }
        }

        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0xdead_beef_cafe_babe] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let cm = (c.shmode)(size_of::<BinEntry>(), 0);
            let rm = (rust.shmode)(size_of::<BinEntry>(), 0);
            let ct = &*hash_index(cm, size_of::<BinEntry>());
            let rt = &*hash_index(rm, size_of::<BinEntry>());
            assert_eq!(ct.seed, rt.seed);
            assert_eq!(ct.slot_count, rt.slot_count);
            assert_eq!(ct.used_count_threshold, rt.used_count_threshold);
            (c.hmfree)(
                raw_from_map(cm, size_of::<BinEntry>()),
                size_of::<BinEntry>(),
            );
            (rust.hmfree)(
                raw_from_map(rm, size_of::<BinEntry>()),
                size_of::<BinEntry>(),
            );
        }
    }
}

unsafe fn arena_alloc(api: &Api, arena: &mut StringArena, bytes: &[u8]) -> (Vec<u8>, usize, u8) {
    let string = CString::new(bytes).unwrap();
    let result = (api.stralloc)(arena, string.as_ptr() as *mut c_char);
    (
        CStr::from_ptr(result).to_bytes().to_vec(),
        arena.remaining,
        arena.block,
    )
}

#[test]
fn string_arena_allocation_and_reset_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let mut ca = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut ra = ca;
        let shapes = [0usize, 1, 7, 127, 380, 511, 512, 513, 900, 1023, 1024, 4097];
        for (number, length) in shapes.into_iter().enumerate() {
            let bytes = vec![b'a' + (number % 20) as u8; length];
            assert_eq!(
                arena_alloc(&c, &mut ca, &bytes),
                arena_alloc(&rust, &mut ra, &bytes),
                "arena length={length}"
            );
        }
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
        assert!(ca.storage.is_null());
        assert!(ra.storage.is_null());
        assert_eq!(ca.remaining, 0);
        assert_eq!(ra.remaining, 0);
        assert_eq!(ca.block, 0);
        assert_eq!(ra.block, 0);

        let huge = vec![b'z'; (1 << 20) + 17];
        for _ in 0..24 {
            assert_eq!(
                arena_alloc(&c, &mut ca, &huge),
                arena_alloc(&rust, &mut ra, &huge)
            );
        }
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);

        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
        assert_eq!(ca.remaining, ra.remaining);
        assert_eq!(ca.block, ra.block);
    }
}

unsafe fn put_bin(api: &Api, map: &mut *mut c_void, key: u64, value: u64, mode: c_int) {
    let mut key_value = key;
    *map = (api.hmput)(
        *map,
        size_of::<BinEntry>(),
        &mut key_value as *mut u64 as *mut c_void,
        size_of::<u64>(),
        mode,
    );
    let index = (*map_header(*map, size_of::<BinEntry>())).temp as usize;
    let entry = (*map as *mut BinEntry).add(index);
    (*entry).value = value;
}

unsafe fn get_bin_ts(api: &Api, map: *mut c_void, key: u64, mode: c_int) -> isize {
    let mut key_value = key;
    let mut temp = isize::MIN;
    let returned = (api.hmget_ts)(
        map,
        size_of::<BinEntry>(),
        &mut key_value as *mut u64 as *mut c_void,
        size_of::<u64>(),
        &mut temp,
        mode,
    );
    assert_eq!(returned, map);
    temp
}

unsafe fn get_bin(api: &Api, map: *mut c_void, key: u64, mode: c_int) -> isize {
    let mut key_value = key;
    let returned = (api.hmget)(
        map,
        size_of::<BinEntry>(),
        &mut key_value as *mut u64 as *mut c_void,
        size_of::<u64>(),
        mode,
    );
    assert_eq!(returned, map);
    (*map_header(map, size_of::<BinEntry>())).temp
}

unsafe fn del_bin(api: &Api, map: *mut c_void, key: u64, mode: c_int) -> *mut c_void {
    let mut key_value = key;
    (api.hmdel)(
        map,
        size_of::<BinEntry>(),
        &mut key_value as *mut u64 as *mut c_void,
        size_of::<u64>(),
        0,
        mode,
    )
}

unsafe fn bin_snapshot(map: *mut c_void) -> Vec<BinEntry> {
    let length = (*map_header(map, size_of::<BinEntry>())).length - 1;
    let mut entries = std::slice::from_raw_parts(map as *const BinEntry, length).to_vec();
    entries.sort_by_key(|entry| entry.key);
    entries
}

unsafe fn assert_map_scalars_equal(cm: *mut c_void, rm: *mut c_void, elemsize: usize) {
    let ch = &*map_header(cm, elemsize);
    let rh = &*map_header(rm, elemsize);
    assert_eq!(ch.length, rh.length);
    assert_eq!(ch.capacity, rh.capacity);
    assert_eq!(ch.temp, rh.temp);
    assert_eq!(ch.hash_table.is_null(), rh.hash_table.is_null());
    if !ch.hash_table.is_null() {
        let ct = &*(ch.hash_table as *mut HashIndex);
        let rt = &*(rh.hash_table as *mut HashIndex);
        assert_eq!(ct.slot_count, rt.slot_count);
        assert_eq!(ct.used_count, rt.used_count);
        assert_eq!(ct.used_count_threshold, rt.used_count_threshold);
        assert_eq!(
            ct.used_count_shrink_threshold,
            rt.used_count_shrink_threshold
        );
        assert_eq!(ct.tombstone_count, rt.tombstone_count);
        assert_eq!(ct.tombstone_count_threshold, rt.tombstone_count_threshold);
        assert_eq!(ct.seed, rt.seed);
        assert_eq!(ct.slot_count_log2, rt.slot_count_log2);
        assert_eq!(ct.string.remaining, rt.string.remaining);
        assert_eq!(ct.string.block, rt.string.block);
        assert_eq!(ct.string.mode, rt.string.mode);
    }
}

#[test]
fn binary_hash_map_low_level_pipeline_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);

        let mut temp_c = 7isize;
        let mut temp_r = 7isize;
        let mut key = 99u64;
        let mut cm = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut temp_c,
            0,
        );
        let mut rm = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut temp_r,
            0,
        );
        assert_eq!(temp_c, -1);
        assert_eq!(temp_c, temp_r);
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        cm = (c.hmput_default)(cm, size_of::<BinEntry>());
        rm = (rust.hmput_default)(rm, size_of::<BinEntry>());
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        for number in 0..180u64 {
            let key = number
                .wrapping_mul(0x9e37_79b9)
                .rotate_left((number % 63) as u32);
            put_bin(&c, &mut cm, key, number ^ 0xa5a5, 0);
            put_bin(&rust, &mut rm, key, number ^ 0xa5a5, 0);
            assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
            assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());
            assert_eq!(get_bin_ts(&c, cm, key, 0), get_bin_ts(&rust, rm, key, 0));
            assert_eq!(get_bin(&c, cm, key, 0), get_bin(&rust, rm, key, 0));
        }

        for key in [u64::MAX, 17, 0xfeed_face_dead_beef] {
            assert_eq!(get_bin_ts(&c, cm, key, 0), -1);
            assert_eq!(get_bin_ts(&c, cm, key, 0), get_bin_ts(&rust, rm, key, 0));
        }

        let mut wrapped_c: *mut c_void = ptr::null_mut();
        let mut wrapped_r: *mut c_void = ptr::null_mut();
        (c.rand_seed)(0x7654_3210);
        (rust.rand_seed)(0x7654_3210);
        let mut candidates = [None, None];
        let mut missing = None;
        for candidate in 0u64..100_000 {
            let mut value = candidate;
            let hash = (c.hash_bytes)(
                &mut value as *mut u64 as *mut c_void,
                size_of::<u64>(),
                0x7654_3210,
            );
            match hash & 7 {
                6 if candidates[0].is_none() => candidates[0] = Some(candidate),
                7 if candidates[1].is_none() => candidates[1] = Some(candidate),
                6 if candidates[0] != Some(candidate) && missing.is_none() => {
                    missing = Some(candidate)
                }
                _ => {}
            }
            if candidates.iter().all(Option::is_some) && missing.is_some() {
                break;
            }
        }
        for key in candidates.into_iter().map(Option::unwrap) {
            put_bin(&c, &mut wrapped_c, key, key, 0);
            put_bin(&rust, &mut wrapped_r, key, key, 0);
        }
        let missing = missing.unwrap();
        assert_eq!(get_bin_ts(&c, wrapped_c, missing, 0), -1);
        assert_eq!(
            get_bin_ts(&c, wrapped_c, missing, 0),
            get_bin_ts(&rust, wrapped_r, missing, 0)
        );
        (c.hmfree)(
            raw_from_map(wrapped_c, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(wrapped_r, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        for number in (0..180u64).step_by(2) {
            let key = number
                .wrapping_mul(0x9e37_79b9)
                .rotate_left((number % 63) as u32);
            cm = del_bin(&c, cm, key, 0);
            rm = del_bin(&rust, rm, key, 0);
            assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
            assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());
        }
        for number in 200..320u64 {
            put_bin(&c, &mut cm, number, number * 3, -1);
            put_bin(&rust, &mut rm, number, number * 3, -1);
        }
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        let absent = 0x1111_2222_3333_4444;
        cm = del_bin(&c, cm, absent, 0);
        rm = del_bin(&rust, rm, absent, 0);
        assert_eq!((*map_header(cm, size_of::<BinEntry>())).temp, 0);
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        (c.hmfree)(
            raw_from_map(cm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(rm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        let mut cm = (c.shmode)(size_of::<BinEntry>(), 0);
        let mut rm = (rust.shmode)(size_of::<BinEntry>(), 0);
        for number in 0..32u64 {
            put_bin(&c, &mut cm, number, number + 1000, 0);
            put_bin(&rust, &mut rm, number, number + 1000, 0);
        }
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());
        (c.hmfree)(
            raw_from_map(cm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(rm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        let mut cm: *mut c_void = ptr::null_mut();
        let mut rm: *mut c_void = ptr::null_mut();
        for number in 0..3u64 {
            put_bin(&c, &mut cm, number, number, 0);
            put_bin(&rust, &mut rm, number, number, 0);
        }
        cm = del_bin(&c, cm, 2, 0);
        rm = del_bin(&rust, rm, 2, 0);
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        cm = del_bin(&c, cm, 0, 0);
        rm = del_bin(&rust, rm, 0, 0);
        assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
        (c.hmfree)(
            raw_from_map(cm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(rm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        (c.hmfree)(ptr::null_mut(), size_of::<BinEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<BinEntry>());
    }
}

unsafe fn put_string(
    api: &Api,
    map: &mut *mut c_void,
    key: *mut c_char,
    value: c_int,
    operation_mode: c_int,
) -> *mut c_char {
    *map = (api.hmput)(
        *map,
        size_of::<StringEntry>(),
        key as *mut c_void,
        size_of::<*mut c_char>(),
        operation_mode,
    );
    let index = (*map_header(*map, size_of::<StringEntry>())).temp as usize;
    let entry = (*map as *mut StringEntry).add(index);
    (*entry).value = value;
    (*entry).key
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, c_int)> {
    let length = (*map_header(map, size_of::<StringEntry>())).length - 1;
    let entries = std::slice::from_raw_parts(map as *const StringEntry, length);
    let mut result: Vec<_> = entries
        .iter()
        .map(|entry| (CStr::from_ptr(entry.key).to_bytes().to_vec(), entry.value))
        .collect();
    result.sort();
    result
}

unsafe fn get_string_ts(api: &Api, map: *mut c_void, key: *mut c_char, mode: c_int) -> isize {
    let mut temp = isize::MIN;
    let returned = (api.hmget_ts)(
        map,
        size_of::<StringEntry>(),
        key as *mut c_void,
        size_of::<*mut c_char>(),
        &mut temp,
        mode,
    );
    assert_eq!(returned, map);
    temp
}

unsafe fn del_string(api: &Api, map: *mut c_void, key: *mut c_char, mode: c_int) -> *mut c_void {
    (api.hmdel)(
        map,
        size_of::<StringEntry>(),
        key as *mut c_void,
        size_of::<*mut c_char>(),
        0,
        mode,
    )
}

#[test]
fn string_hash_map_modes_and_deletion_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let selected_mode = std::env::var("DIFF_MODE")
            .ok()
            .and_then(|value| value.parse::<c_int>().ok());
        for ownership_mode in [1, 2, 3] {
            if selected_mode.is_some_and(|selected| selected != ownership_mode) {
                continue;
            }
            let operation_mode = 1;
            (c.rand_seed)(0xabc0 + ownership_mode as usize);
            (rust.rand_seed)(0xabc0 + ownership_mode as usize);
            let mut cm = (c.shmode)(size_of::<StringEntry>(), ownership_mode);
            let mut rm = (rust.shmode)(size_of::<StringEntry>(), ownership_mode);
            let mut keys: Vec<CString> = (0..90)
                .map(|number| CString::new(format!("key_{ownership_mode}_{number:03}")).unwrap())
                .collect();
            keys.push(CString::new("").unwrap());
            keys.push(CString::new("L".repeat(700)).unwrap());

            for (number, key) in keys.iter().enumerate() {
                let pointer = key.as_ptr() as *mut c_char;
                let c_stored = put_string(&c, &mut cm, pointer, number as c_int, operation_mode);
                let r_stored = put_string(&rust, &mut rm, pointer, number as c_int, operation_mode);
                assert_eq!(
                    CStr::from_ptr(c_stored).to_bytes(),
                    CStr::from_ptr(r_stored).to_bytes()
                );
                if ownership_mode == 1 {
                    assert_eq!(c_stored, pointer);
                    assert_eq!(r_stored, pointer);
                } else if ownership_mode == 2 || ownership_mode == 3 {
                    assert_ne!(c_stored, pointer);
                    assert_ne!(r_stored, pointer);
                }
            }
            assert_eq!(string_snapshot(cm), string_snapshot(rm));
            assert_map_scalars_equal(cm, rm, size_of::<StringEntry>());

            for key in keys.iter().step_by(7) {
                assert_eq!(
                    get_string_ts(&c, cm, key.as_ptr() as *mut c_char, operation_mode),
                    get_string_ts(&rust, rm, key.as_ptr() as *mut c_char, operation_mode)
                );
            }
            let missing = CString::new("definitely_missing").unwrap();
            assert_eq!(
                get_string_ts(&c, cm, missing.as_ptr() as *mut c_char, operation_mode),
                -1
            );
            assert_eq!(
                get_string_ts(&c, cm, missing.as_ptr() as *mut c_char, operation_mode),
                get_string_ts(&rust, rm, missing.as_ptr() as *mut c_char, operation_mode)
            );

            for key in keys.iter().skip(1).step_by(3) {
                cm = del_string(&c, cm, key.as_ptr() as *mut c_char, operation_mode);
                rm = del_string(&rust, rm, key.as_ptr() as *mut c_char, operation_mode);
                assert_eq!(string_snapshot(cm), string_snapshot(rm));
                assert_map_scalars_equal(cm, rm, size_of::<StringEntry>());
            }
            cm = del_string(&c, cm, missing.as_ptr() as *mut c_char, operation_mode);
            rm = del_string(&rust, rm, missing.as_ptr() as *mut c_char, operation_mode);
            assert_map_scalars_equal(cm, rm, size_of::<StringEntry>());

            (c.hmfree)(
                raw_from_map(cm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            (rust.hmfree)(
                raw_from_map(rm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [0; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let saved = dup(1);
    assert!(saved >= 0);
    fflush(ptr::null_mut());
    assert_eq!(dup2(fds[1], 1), 1);
    call();
    fflush(ptr::null_mut());
    assert_eq!(dup2(saved, 1), 1);
    close(saved);
    close(fds[1]);
    let mut file = File::from_raw_fd(fds[0]);
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    output
}

#[test]
fn strkey_and_str_put_stdout_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        for value in [c_int::MIN, -1_000_000, -1, 0, 1, 42, 1_000_000, c_int::MAX] {
            let c_value = CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec();
            let r_value = CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec();
            assert_eq!(c_value, r_value, "strkey({value})");
        }

        for value in [-17, -1, 0, 1, 2, 17, 513, 4096] {
            let c_output = capture_stdout(|| (c.str_put)(value));
            let rust_output = capture_stdout(|| (rust.str_put)(value));
            assert_eq!(c_output, rust_output, "str_put({value})");
            assert_eq!(c_output, format!("a {value}\n").as_bytes());
        }
    }
}

#[test]
fn null_sentinels_zero_lengths_and_out_of_range_modes_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        (c.hmfree)(ptr::null_mut(), 0);
        (rust.hmfree)(ptr::null_mut(), 0);
        assert!((c.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 0, 0, 0).is_null());
        assert!((rust.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 0, 0, 0).is_null());
        assert_eq!(
            (c.hash_bytes)(ptr::null_mut(), 0, usize::MAX),
            (rust.hash_bytes)(ptr::null_mut(), 0, usize::MAX)
        );

        let mut cm = (c.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        let mut rm = (rust.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        let c_unchanged = (c.hmdel)(cm, size_of::<BinEntry>(), ptr::null_mut(), 0, 0, 0);
        let r_unchanged = (rust.hmdel)(rm, size_of::<BinEntry>(), ptr::null_mut(), 0, 0, 0);
        assert_eq!(c_unchanged, cm);
        assert_eq!(r_unchanged, rm);
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());

        let mut key = 5u64;
        let mut ct = 0isize;
        let mut rt = 0isize;
        cm = (c.hmget_ts)(
            cm,
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut ct,
            -1,
        );
        rm = (rust.hmget_ts)(
            rm,
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut rt,
            -1,
        );
        assert_eq!(ct, -1);
        assert_eq!(ct, rt);
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());
        (c.hmfree)(
            raw_from_map(cm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(rm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        let mut key = 9u64;
        let cm = (c.hmget)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
        );
        let rm = (rust.hmget)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
        );
        assert_eq!((*map_header(cm, size_of::<BinEntry>())).temp, -1);
        assert_map_scalars_equal(cm, rm, size_of::<BinEntry>());
        (c.hmfree)(
            raw_from_map(cm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );
        (rust.hmfree)(
            raw_from_map(rm, size_of::<BinEntry>()),
            size_of::<BinEntry>(),
        );

        for mode in [2, 3, c_int::MAX] {
            let key = CString::new(format!("mode_{mode}")).unwrap();
            let mut cm: *mut c_void = ptr::null_mut();
            let mut rm: *mut c_void = ptr::null_mut();
            put_string(&c, &mut cm, key.as_ptr() as *mut c_char, mode, mode);
            put_string(&rust, &mut rm, key.as_ptr() as *mut c_char, mode, mode);
            assert_eq!(string_snapshot(cm), string_snapshot(rm));
            (c.hmfree)(
                raw_from_map(cm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            (rust.hmfree)(
                raw_from_map(rm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }
    }
}

#[test]
fn crash_probe_child() {
    let Ok(case) = std::env::var("DIFF_CRASH_CASE") else {
        return;
    };
    let library = std::env::var("DIFF_CRASH_LIBRARY").unwrap();
    let (c, rust) = libraries();
    let api = if library == "c" { &c } else { &rust };
    unsafe {
        match case.as_str() {
            "arrfree_null" => (api.arrfree)(ptr::null_mut()),
            "hash_string_null" => {
                std::hint::black_box((api.hash_string)(ptr::null_mut(), 7));
            }
            "hash_bytes_nonzero_null" => {
                std::hint::black_box((api.hash_bytes)(ptr::null_mut(), 1, 7));
            }
            "hash_bytes_oversized_null" => {
                std::hint::black_box((api.hash_bytes)(ptr::null_mut(), usize::MAX, usize::MAX));
            }
            "strreset_null" => (api.strreset)(ptr::null_mut()),
            "stralloc_null_arena" => {
                let value = CString::new("x").unwrap();
                std::hint::black_box((api.stralloc)(
                    ptr::null_mut(),
                    value.as_ptr() as *mut c_char,
                ));
            }
            "hash_index_threshold_assert" => {
                let mut map = (api.shmode)(size_of::<BinEntry>(), 0);
                let table = hash_index(map, size_of::<BinEntry>());
                (*table).slot_count = 1;
                (*table).used_count = 0;
                (*table).used_count_threshold = 0;
                put_bin(api, &mut map, 7, 7, 0);
            }
            "moved_slot_missing_assert" => {
                let mut map: *mut c_void = ptr::null_mut();
                for key in 0..3u64 {
                    put_bin(api, &mut map, key, key, 0);
                }
                let table = hash_index(map, size_of::<BinEntry>());
                for bucket_number in 0..((*table).slot_count >> 3) {
                    let bucket = (*table).storage.add(bucket_number);
                    for index in 0..8 {
                        if (*bucket).index[index] == 2 {
                            (*bucket).hash[index] = 0;
                            (*bucket).index[index] = -1;
                        }
                    }
                }
                std::hint::black_box(del_bin(api, map, 0, 0));
            }
            "moved_index_mismatch_assert" => {
                let mut map: *mut c_void = ptr::null_mut();
                for key in 0..3u64 {
                    put_bin(api, &mut map, key, key, 0);
                }
                (*(map as *mut BinEntry).add(1)).key = 2;
                let table = hash_index(map, size_of::<BinEntry>());
                for bucket_number in 0..((*table).slot_count >> 3) {
                    let bucket = (*table).storage.add(bucket_number);
                    for index in 0..8 {
                        if (*bucket).index[index] == 2 {
                            (*bucket).index[index] = 1;
                        }
                    }
                }
                std::hint::black_box(del_bin(api, map, 0, 0));
            }
            _ => panic!("unknown crash case"),
        }
    }
    panic!("crash probe unexpectedly returned");
}

#[test]
fn unguarded_null_pointer_process_behavior_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let executable = std::env::current_exe().unwrap();
    for case in [
        "arrfree_null",
        "hash_string_null",
        "hash_bytes_nonzero_null",
        "hash_bytes_oversized_null",
        "strreset_null",
        "stralloc_null_arena",
        "hash_index_threshold_assert",
        "moved_slot_missing_assert",
        "moved_index_mismatch_assert",
    ] {
        let run = |library: &str| {
            Command::new(&executable)
                .arg("crash_probe_child")
                .arg("--exact")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env("DIFF_CRASH_CASE", case)
                .env("DIFF_CRASH_LIBRARY", library)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "C unexpectedly returned for {case}");
        assert!(
            !rust_status.success(),
            "Rust unexpectedly returned for {case}"
        );
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different process signal for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
