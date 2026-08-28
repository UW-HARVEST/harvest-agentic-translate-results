use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;
use std::slice;

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
type ArrPush = unsafe extern "C" fn(c_int);

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
    key: i32,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringPair {
    key: *mut c_char,
    value: i32,
}

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
    arr_push: ArrPush,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(path).unwrap() };
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap() }
            };
        }
        Self {
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
            arr_push: symbol!("arr_push", ArrPush),
            _library: library,
        }
    }
}

fn apis() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("../c_src/build/libharvest-work-exQmBS.so");
    let debug_path = root.join("target/debug/libarr_push_lib.so");
    let release_path = root.join("target/release/libarr_push_lib.so");
    let rust_path = if debug_path.is_file() {
        debug_path
    } else {
        release_path
    };
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<u8>().sub(size_of::<ArrayHeader>()).cast() }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

unsafe fn compare_array_case(
    c: &Api,
    rust: &Api,
    element_size: usize,
    initial_capacity: usize,
    add_len: usize,
    min_capacity: usize,
) {
    let c_array = unsafe {
        (c.arrgrow)(
            ptr::null_mut(),
            element_size,
            initial_capacity,
            initial_capacity,
        )
    };
    let rust_array = unsafe {
        (rust.arrgrow)(
            ptr::null_mut(),
            element_size,
            initial_capacity,
            initial_capacity,
        )
    };
    assert_eq!(c_array.is_null(), rust_array.is_null());
    if c_array.is_null() {
        return;
    }

    let initial_len = unsafe { (*header(c_array)).length };
    unsafe {
        (*header(c_array)).length = initial_capacity;
        (*header(rust_array)).length = initial_capacity;
        for index in 0..element_size * initial_capacity {
            *c_array.cast::<u8>().add(index) = index.wrapping_mul(37) as u8;
            *rust_array.cast::<u8>().add(index) = index.wrapping_mul(37) as u8;
        }
    }
    assert_eq!(initial_len, 0);

    let old_c = c_array;
    let old_rust = rust_array;
    let old_capacity = unsafe { (*header(c_array)).capacity };
    let no_growth = initial_capacity.wrapping_add(add_len).max(min_capacity) <= old_capacity;
    let c_result = unsafe { (c.arrgrow)(c_array, element_size, add_len, min_capacity) };
    let rust_result = unsafe { (rust.arrgrow)(rust_array, element_size, add_len, min_capacity) };
    let c_header = unsafe { *header(c_result) };
    let rust_header = unsafe { *header(rust_result) };
    assert_eq!(c_header.length, rust_header.length);
    assert_eq!(c_header.capacity, rust_header.capacity);
    assert_eq!(c_header.temp, rust_header.temp);
    assert_eq!(
        unsafe { slice::from_raw_parts(c_result.cast::<u8>(), element_size * initial_capacity) },
        unsafe { slice::from_raw_parts(rust_result.cast::<u8>(), element_size * initial_capacity) }
    );
    if no_growth {
        assert_eq!(c_result, old_c);
        assert_eq!(rust_result, old_rust);
    }
    unsafe {
        (c.arrfree)(c_result);
        (rust.arrfree)(rust_result);
    }
}

#[test]
fn arrays_and_void_entry_point_match() {
    let (c, rust) = apis();
    unsafe {
        assert!((c.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        assert!((rust.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        for element_size in [1, 4, 16] {
            for capacity in 1..=3 {
                compare_array_case(&c, &rust, element_size, capacity, 0, capacity);
            }
            compare_array_case(&c, &rust, element_size, 4, 0, 4);
            compare_array_case(&c, &rust, element_size, 4, 1, 0);
            compare_array_case(&c, &rust, element_size, 4, 0, 8);
            compare_array_case(&c, &rust, element_size, 5, 7, 2);
        }
        for number in [-100, 0, 1, 49, 50, 51, 151, 503] {
            (c.arr_push)(number);
            (rust.arr_push)(number);
        }
    }
}

#[test]
fn hashes_match_for_randomized_lengths_and_seeds() {
    let (c, rust) = apis();
    let mut random = 0x9e37_79b9_7f4a_7c15;
    unsafe {
        for _ in 0..128 {
            let seed = next_random(&mut random) as usize;
            assert_eq!(
                (c.hash_bytes)(ptr::null_mut(), 0, seed),
                (rust.hash_bytes)(ptr::null_mut(), 0, seed)
            );
            for len in 0..=39 {
                let mut bytes = vec![0u8; len.max(1)];
                for byte in &mut bytes {
                    *byte = next_random(&mut random) as u8;
                }
                assert_eq!(
                    (c.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                    (rust.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                    "byte hash mismatch at len={len}, seed={seed:#x}, bytes={bytes:02x?}"
                );
            }
        }
    }
}

#[test]
fn strings_and_arena_match() {
    let (c, rust) = apis();
    let mut random = 0x243f_6a88_85a3_08d3;
    unsafe {
        for len in 0..=80 {
            for _ in 0..16 {
                let mut bytes = Vec::with_capacity(len + 1);
                for _ in 0..len {
                    let mut byte = next_random(&mut random) as u8;
                    if byte == 0 {
                        byte = 0x80;
                    }
                    bytes.push(byte);
                }
                bytes.push(0);
                let seed = next_random(&mut random) as usize;
                assert_eq!(
                    (c.hash_string)(bytes.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(bytes.as_mut_ptr().cast(), seed)
                );
            }
        }

        let mut c_arena = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut rust_arena = c_arena;
        for len in [0, 1, 7, 127, 376, 511, 512, 513, 700, 1025, 4097] {
            let text = CString::new(vec![b'x'; len]).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, text.as_ptr().cast_mut());
            let rust_result = (rust.stralloc)(&mut rust_arena, text.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(rust_result).to_bytes()
            );
            assert_eq!(c_arena.remaining, rust_arena.remaining);
            assert_eq!(c_arena.block, rust_arena.block);
        }
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
        assert_eq!(c_arena.remaining, 0);
        assert_eq!(rust_arena.remaining, 0);
        assert!(c_arena.storage.is_null());
        assert!(rust_arena.storage.is_null());
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
}

#[test]
fn arena_block_schedule_reaches_and_stays_at_maximum() {
    let (c, rust) = apis();
    unsafe {
        let mut c_arena = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut rust_arena = c_arena;
        for _ in 0..26 {
            let block_size = 512usize << (c_arena.block as usize >> 1);
            let text = CString::new(vec![b'z'; block_size]).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, text.as_ptr().cast_mut());
            let rust_result = (rust.stralloc)(&mut rust_arena, text.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(rust_result).to_bytes()
            );
            assert_eq!(c_arena.block, rust_arena.block);
            assert_eq!(c_arena.remaining, rust_arena.remaining);
        }
        assert_eq!(c_arena.block, 22);
        assert_eq!(rust_arena.block, 22);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
}

#[test]
fn strkey_matches() {
    let (c, rust) = apis();
    unsafe {
        for number in [c_int::MIN, -1000, -10, -1, 0, 1, 9, 10, 999, c_int::MAX] {
            let expected = CStr::from_ptr((c.strkey)(number)).to_bytes().to_vec();
            let actual = CStr::from_ptr((rust.strkey)(number)).to_bytes().to_vec();
            assert_eq!(expected, actual);
        }
    }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> ArrayHeader {
    unsafe { *header(raw_map(map, element_size)) }
}

unsafe fn pair_snapshot(map: *mut c_void) -> Vec<Pair> {
    if map.is_null() {
        return Vec::new();
    }
    let metadata = unsafe { map_header(map, size_of::<Pair>()) };
    unsafe {
        slice::from_raw_parts(map.cast::<Pair>(), metadata.length.checked_sub(1).unwrap()).to_vec()
    }
}

unsafe fn compare_pair_maps(c_map: *mut c_void, rust_map: *mut c_void) {
    assert_eq!(unsafe { pair_snapshot(c_map) }, unsafe {
        pair_snapshot(rust_map)
    });
    if !c_map.is_null() {
        let c_header = unsafe { map_header(c_map, size_of::<Pair>()) };
        let rust_header = unsafe { map_header(rust_map, size_of::<Pair>()) };
        assert_eq!(c_header.length, rust_header.length);
        assert_eq!(c_header.capacity, rust_header.capacity);
        assert_eq!(c_header.temp, rust_header.temp);
        assert_eq!(
            c_header.hash_table.is_null(),
            rust_header.hash_table.is_null()
        );
    }
}

unsafe fn put_pair(api: &Api, map: *mut c_void, key: i32, value: i32, mode: c_int) -> *mut c_void {
    let mut key_value = key;
    let result = unsafe {
        (api.hmput)(
            map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key_value).cast(),
            size_of::<i32>(),
            mode,
        )
    };
    unsafe {
        let metadata = map_header(result, size_of::<Pair>());
        let entry = result.cast::<Pair>().add(metadata.temp as usize);
        (*entry).key = key;
        (*entry).value = value;
    }
    result
}

unsafe fn get_pair(api: &Api, map: *mut c_void, key: i32, mode: c_int) -> (isize, *mut c_void) {
    let mut key_value = key;
    let mut temp = isize::MIN;
    let result = unsafe {
        (api.hmget_ts)(
            map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key_value).cast(),
            size_of::<i32>(),
            &mut temp,
            mode,
        )
    };
    (temp, result)
}

unsafe fn free_pair_map(api: &Api, map: *mut c_void) {
    if map.is_null() {
        unsafe { (api.hmfree)(ptr::null_mut(), size_of::<Pair>()) };
    } else {
        unsafe { (api.hmfree)(raw_map(map, size_of::<Pair>()), size_of::<Pair>()) };
    }
}

#[test]
fn map_null_default_and_error_sentinels_match() {
    let (c, rust) = apis();
    unsafe {
        (c.hmfree)(ptr::null_mut(), size_of::<Pair>());
        (rust.hmfree)(ptr::null_mut(), size_of::<Pair>());

        let mut key = 77i32;
        let mut c_temp = 999;
        let mut rust_temp = 999;
        let c_from_null = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut c_temp,
            0,
        );
        let rust_from_null = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);
        compare_pair_maps(c_from_null, rust_from_null);
        free_pair_map(&c, c_from_null);
        free_pair_map(&rust, rust_from_null);

        let mut c_map = (c.hmput_default)(ptr::null_mut(), size_of::<Pair>());
        let mut rust_map = (rust.hmput_default)(ptr::null_mut(), size_of::<Pair>());
        compare_pair_maps(c_map, rust_map);
        let old_c = c_map;
        let old_rust = rust_map;
        c_map = (c.hmput_default)(c_map, size_of::<Pair>());
        rust_map = (rust.hmput_default)(rust_map, size_of::<Pair>());
        assert_eq!(c_map, old_c);
        assert_eq!(rust_map, old_rust);

        let (c_missing, c_map_after) = get_pair(&c, c_map, 1234, 0);
        let (rust_missing, rust_map_after) = get_pair(&rust, rust_map, 1234, 0);
        c_map = c_map_after;
        rust_map = rust_map_after;
        assert_eq!(c_missing, -1);
        assert_eq!(c_missing, rust_missing);
        compare_pair_maps(c_map, rust_map);

        c_map = (c.hmget)(
            c_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
        );
        rust_map = (rust.hmget)(
            rust_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
        );
        assert_eq!(map_header(c_map, size_of::<Pair>()).temp, -1);
        compare_pair_maps(c_map, rust_map);

        let c_before = c_map;
        let rust_before = rust_map;
        c_map = (c.hmdel)(
            c_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        );
        rust_map = (rust.hmdel)(
            rust_map,
            size_of::<Pair>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        );
        assert_eq!(c_map, c_before);
        assert_eq!(rust_map, rust_before);
        assert_eq!(map_header(c_map, size_of::<Pair>()).temp, 0);
        compare_pair_maps(c_map, rust_map);

        assert!(
            (c.hmdel)(
                ptr::null_mut(),
                size_of::<Pair>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<i32>(),
                0,
                0,
            )
            .is_null()
        );
        assert!(
            (rust.hmdel)(
                ptr::null_mut(),
                size_of::<Pair>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<i32>(),
                0,
                0,
            )
            .is_null()
        );
        free_pair_map(&c, c_map);
        free_pair_map(&rust, rust_map);
    }
}

#[test]
fn randomized_binary_maps_match_through_growth_lookup_and_delete() {
    let (c, rust) = apis();
    let mut random = 0xd1b5_4a32_d192_ed03;
    unsafe {
        for mode in [0, -1] {
            for round in 0..12 {
                let seed = next_random(&mut random) as usize;
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
                let mut c_map = ptr::null_mut();
                let mut rust_map = ptr::null_mut();
                let mut keys = Vec::new();

                for index in 0..48 {
                    let key = (next_random(&mut random) as i32)
                        .wrapping_add(index)
                        .wrapping_add(round);
                    let value = next_random(&mut random) as i32;
                    c_map = put_pair(&c, c_map, key, value, mode);
                    rust_map = put_pair(&rust, rust_map, key, value, mode);
                    keys.push(key);
                    compare_pair_maps(c_map, rust_map);

                    let (c_index, c_after) = get_pair(&c, c_map, key, mode);
                    let (rust_index, rust_after) = get_pair(&rust, rust_map, key, mode);
                    c_map = c_after;
                    rust_map = rust_after;
                    assert_eq!(c_index, rust_index);
                    assert!(c_index >= 0);
                }

                for &key in keys.iter().step_by(5) {
                    let replacement = next_random(&mut random) as i32;
                    c_map = put_pair(&c, c_map, key, replacement, mode);
                    rust_map = put_pair(&rust, rust_map, key, replacement, mode);
                    compare_pair_maps(c_map, rust_map);
                }

                for _ in 0..40 {
                    let missing = next_random(&mut random) as i32;
                    let (c_index, c_after) = get_pair(&c, c_map, missing, mode);
                    let (rust_index, rust_after) = get_pair(&rust, rust_map, missing, mode);
                    c_map = c_after;
                    rust_map = rust_after;
                    assert_eq!(c_index, rust_index);
                }

                for &key in keys.iter().rev().take(38) {
                    c_map = (c.hmdel)(
                        c_map,
                        size_of::<Pair>(),
                        ptr::addr_of!(key).cast_mut().cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    );
                    rust_map = (rust.hmdel)(
                        rust_map,
                        size_of::<Pair>(),
                        ptr::addr_of!(key).cast_mut().cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    );
                    compare_pair_maps(c_map, rust_map);
                    assert_eq!(map_header(c_map, size_of::<Pair>()).temp, 1);
                }
                for missing in [i32::MIN, i32::MAX, 0x5555_5555] {
                    c_map = (c.hmdel)(
                        c_map,
                        size_of::<Pair>(),
                        ptr::addr_of!(missing).cast_mut().cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    );
                    rust_map = (rust.hmdel)(
                        rust_map,
                        size_of::<Pair>(),
                        ptr::addr_of!(missing).cast_mut().cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    );
                    compare_pair_maps(c_map, rust_map);
                }
                free_pair_map(&c, c_map);
                free_pair_map(&rust, rust_map);
            }
        }
    }
}

#[test]
fn binary_missing_lookup_wraps_within_bucket() {
    let (c, rust) = apis();
    let seed = 0x0123_4567_89ab_cdefusize;
    unsafe {
        (c.rand_seed)(seed);
        (rust.rand_seed)(seed);
        let mut colliding_keys = Vec::new();
        for candidate in 0i32..100_000 {
            let mut key = candidate;
            let hash = (c.hash_bytes)(ptr::addr_of_mut!(key).cast(), size_of::<i32>(), seed);
            if hash & 7 == 7 {
                colliding_keys.push(candidate);
                if colliding_keys.len() == 6 {
                    break;
                }
            }
        }
        assert_eq!(colliding_keys.len(), 6);

        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        for (index, &key) in colliding_keys[..5].iter().enumerate() {
            c_map = put_pair(&c, c_map, key, index as i32, 0);
            rust_map = put_pair(&rust, rust_map, key, index as i32, 0);
        }
        let missing = colliding_keys[5];
        let (c_index, c_after) = get_pair(&c, c_map, missing, 0);
        let (rust_index, rust_after) = get_pair(&rust, rust_map, missing, 0);
        assert_eq!(c_index, -1);
        assert_eq!(c_index, rust_index);
        compare_pair_maps(c_after, rust_after);
        free_pair_map(&c, c_after);
        free_pair_map(&rust, rust_after);
    }
}

unsafe fn byte_map_snapshot(map: *mut c_void, element_size: usize) -> Vec<u8> {
    let metadata = unsafe { map_header(map, element_size) };
    unsafe {
        slice::from_raw_parts(
            map.cast::<u8>(),
            (metadata.length - 1).wrapping_mul(element_size),
        )
        .to_vec()
    }
}

#[test]
fn binary_key_widths_match() {
    let (c, rust) = apis();
    let mut random = 0x94d0_49bb_1331_11eb;
    unsafe {
        for key_size in [1usize, 4, 8, 16] {
            let element_size = key_size + 8;
            let mut c_map = ptr::null_mut();
            let mut rust_map = ptr::null_mut();
            for _ in 0..64 {
                let mut key = vec![0u8; key_size];
                for byte in &mut key {
                    *byte = next_random(&mut random) as u8;
                }
                c_map = (c.hmput)(c_map, element_size, key.as_mut_ptr().cast(), key_size, 0);
                rust_map =
                    (rust.hmput)(rust_map, element_size, key.as_mut_ptr().cast(), key_size, 0);
                let c_metadata = map_header(c_map, element_size);
                let rust_metadata = map_header(rust_map, element_size);
                assert_eq!(c_metadata.temp, rust_metadata.temp);
                let payload = next_random(&mut random).to_ne_bytes();
                ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    c_map
                        .cast::<u8>()
                        .add(c_metadata.temp as usize * element_size + key_size),
                    payload.len(),
                );
                ptr::copy_nonoverlapping(
                    payload.as_ptr(),
                    rust_map
                        .cast::<u8>()
                        .add(rust_metadata.temp as usize * element_size + key_size),
                    payload.len(),
                );
                assert_eq!(
                    byte_map_snapshot(c_map, element_size),
                    byte_map_snapshot(rust_map, element_size)
                );
            }
            (c.hmfree)(raw_map(c_map, element_size), element_size);
            (rust.hmfree)(raw_map(rust_map, element_size), element_size);
        }
    }
}

unsafe fn put_string(
    api: &Api,
    map: *mut c_void,
    key: &CString,
    value: i32,
    mode: c_int,
) -> *mut c_void {
    let result = unsafe {
        (api.hmput)(
            map,
            size_of::<StringPair>(),
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let metadata = unsafe { map_header(result, size_of::<StringPair>()) };
    assert!(metadata.temp >= 0);
    unsafe {
        (*result.cast::<StringPair>().add(metadata.temp as usize)).value = value;
    }
    result
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, i32)> {
    if map.is_null() {
        return Vec::new();
    }
    let metadata = unsafe { map_header(map, size_of::<StringPair>()) };
    let entries = unsafe {
        slice::from_raw_parts(
            map.cast::<StringPair>(),
            metadata.length.checked_sub(1).unwrap(),
        )
    };
    entries
        .iter()
        .map(|entry| {
            (
                unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
                entry.value,
            )
        })
        .collect()
}

unsafe fn compare_string_maps(c_map: *mut c_void, rust_map: *mut c_void) {
    assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
        string_snapshot(rust_map)
    });
    let c_header = unsafe { map_header(c_map, size_of::<StringPair>()) };
    let rust_header = unsafe { map_header(rust_map, size_of::<StringPair>()) };
    assert_eq!(c_header.length, rust_header.length);
    assert_eq!(c_header.capacity, rust_header.capacity);
    assert_eq!(c_header.temp, rust_header.temp);
}

unsafe fn free_string_map(api: &Api, map: *mut c_void) {
    unsafe {
        (api.hmfree)(
            raw_map(map, size_of::<StringPair>()),
            size_of::<StringPair>(),
        )
    };
}

#[test]
fn randomized_string_maps_match_all_storage_modes() {
    let (c, rust) = apis();
    let mut random = 0xa409_3822_299f_31d0;
    unsafe {
        // None means normal hmput initialization (borrowed/default mode).
        for (initial_mode, operation_mode) in [(None, 1), (None, 2), (Some(2), 1), (Some(3), 1)] {
            for round in 0..8 {
                let seed = next_random(&mut random) as usize;
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
                let mut c_map = initial_mode
                    .map(|mode| (c.shmode)(size_of::<StringPair>(), mode))
                    .unwrap_or(ptr::null_mut());
                let mut rust_map = initial_mode
                    .map(|mode| (rust.shmode)(size_of::<StringPair>(), mode))
                    .unwrap_or(ptr::null_mut());
                let mut keys = vec![CString::new("").unwrap()];
                for index in 0..36 {
                    let len = if initial_mode == Some(3) && index == 5 {
                        700
                    } else {
                        (next_random(&mut random) % 48 + 1) as usize
                    };
                    let mut bytes = Vec::with_capacity(len);
                    for _ in 0..len {
                        bytes.push(b'a' + (next_random(&mut random) % 26) as u8);
                    }
                    bytes.extend_from_slice(format!("_{round}_{index}").as_bytes());
                    keys.push(CString::new(bytes).unwrap());
                }

                for (index, key) in keys.iter().enumerate() {
                    let value = next_random(&mut random) as i32;
                    c_map = put_string(&c, c_map, key, value, operation_mode);
                    rust_map = put_string(&rust, rust_map, key, value, operation_mode);
                    compare_string_maps(c_map, rust_map);

                    let mut c_temp = isize::MIN;
                    let mut rust_temp = isize::MIN;
                    c_map = (c.hmget_ts)(
                        c_map,
                        size_of::<StringPair>(),
                        key.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut c_temp,
                        operation_mode,
                    );
                    rust_map = (rust.hmget_ts)(
                        rust_map,
                        size_of::<StringPair>(),
                        key.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut rust_temp,
                        operation_mode,
                    );
                    assert_eq!(c_temp, rust_temp);
                    assert!(c_temp >= 0);
                    if index % 7 == 0 {
                        c_map = put_string(&c, c_map, key, !value, operation_mode);
                        rust_map = put_string(&rust, rust_map, key, !value, operation_mode);
                        compare_string_maps(c_map, rust_map);
                    }
                }

                for missing_index in 0..24 {
                    let missing = CString::new(format!("missing_{round}_{missing_index}")).unwrap();
                    let mut c_temp = 55;
                    let mut rust_temp = 55;
                    c_map = (c.hmget_ts)(
                        c_map,
                        size_of::<StringPair>(),
                        missing.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut c_temp,
                        operation_mode,
                    );
                    rust_map = (rust.hmget_ts)(
                        rust_map,
                        size_of::<StringPair>(),
                        missing.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut rust_temp,
                        operation_mode,
                    );
                    assert_eq!(c_temp, -1);
                    assert_eq!(c_temp, rust_temp);
                }

                for key in keys.iter().rev().take(29) {
                    c_map = (c.hmdel)(
                        c_map,
                        size_of::<StringPair>(),
                        key.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        operation_mode,
                    );
                    rust_map = (rust.hmdel)(
                        rust_map,
                        size_of::<StringPair>(),
                        key.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        operation_mode,
                    );
                    compare_string_maps(c_map, rust_map);
                }
                free_string_map(&c, c_map);
                free_string_map(&rust, rust_map);
            }
        }
    }
}

#[test]
fn shmode_accepts_exact_c_integer_surface() {
    let (c, rust) = apis();
    unsafe {
        for mode in [c_int::MIN, -1, 0, 1, 2, 3, 4, 255, 256, c_int::MAX] {
            let c_map = (c.shmode)(size_of::<StringPair>(), mode);
            let rust_map = (rust.shmode)(size_of::<StringPair>(), mode);
            let c_header = map_header(c_map, size_of::<StringPair>());
            let rust_header = map_header(rust_map, size_of::<StringPair>());
            assert_eq!(c_header.length, rust_header.length);
            assert_eq!(c_header.capacity, rust_header.capacity);
            assert_eq!(c_header.temp, rust_header.temp);
            assert!(!c_header.hash_table.is_null());
            assert!(!rust_header.hash_table.is_null());
            free_string_map(&c, c_map);
            free_string_map(&rust, rust_map);
        }
    }
}

#[test]
fn c_assert_and_range_surface_is_accounted_for() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("../c_src/src/lib.c")).unwrap();
    let errors = std::fs::read_to_string(root.join("ERRORS.md")).unwrap();
    for assertion in [
        "t->used_count_threshold + t->tombstone_count_threshold < t->slot_count",
        "(size_t) i+1 <= stbds_arrcap(a)",
        "slot < (ptrdiff_t) table->slot_count",
        "table->used_count >= 0",
        "slot >= 0",
        "b->index[i] == final_index",
        "len <= a->remaining",
        "arrlen(arr)==0",
    ] {
        assert!(
            source.contains(assertion),
            "C assertion disappeared: {assertion}"
        );
    }
    for required in [
        "STBDS_STRING_ARENA_BLOCKSIZE_MIN",
        "STBDS_STRING_ARENA_BLOCKSIZE_MAX",
        "STBDS_HM_STRING",
        "STBDS_SH_STRDUP",
        "STBDS_SH_ARENA",
        "internal invariant",
    ] {
        assert!(errors.contains(required), "ERRORS.md omits {required}");
    }
}
