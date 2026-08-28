#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr::null_mut;
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
type HmDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ArrDel = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_bytes: HashBytes,
    hash_string: HashString,
    hmfree: HmFree,
    hmget: HmGet,
    hmget_ts: HmGetTs,
    hmdefault: HmDefault,
    hmput: HmPut,
    hmdel: HmDel,
    shmode: ShMode,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    arr_del: ArrDel,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = Library::new(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let value = library
                    .get::<$ty>(concat!($name, "\0").as_bytes())
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *value
            }};
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmdefault: symbol!("stbds_hmput_default", HmDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            shmode: symbol!("stbds_shmode_func", ShMode),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            arr_del: symbol!("arr_del", ArrDel),
            _library: library,
        }
    }
}

fn apis() -> (Api, Api) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        (
            Api::load(manifest.join("../c_src/build/libharvest-work-wMaLju.so")),
            Api::load(manifest.join("target/release/libarr_del_lib.so")),
        )
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
struct StringArena {
    storage: *mut c_void,
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
struct HashIndexPrefix {
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
    storage: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Record {
    key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringRecord {
    key: *mut c_char,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OffsetStringRecord {
    key: *mut c_char,
    duplicate_key: *mut c_char,
    value: i64,
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    array.cast::<ArrayHeader>().sub(1)
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    map.cast::<u8>().sub(element_size).cast()
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    header(raw_map(map, element_size))
}

unsafe fn map_len(map: *mut c_void, element_size: usize) -> usize {
    (*map_header(map, element_size)).length - 1
}

unsafe fn map_temp(map: *mut c_void, element_size: usize) -> isize {
    (*map_header(map, element_size)).temp
}

unsafe fn table(map: *mut c_void, element_size: usize) -> *mut HashIndexPrefix {
    (*map_header(map, element_size)).hash_table.cast()
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    (api.hmfree)(raw_map(map, element_size), element_size);
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

unsafe fn put_record(
    api: &Api,
    map: *mut c_void,
    key: u64,
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let mut key = key;
    let map = (api.hmput)(
        map,
        size_of::<Record>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        mode,
    );
    let index = map_temp(map, size_of::<Record>());
    assert!(index >= 0);
    (*map.cast::<Record>().offset(index)).value = value;
    map
}

unsafe fn get_record(api: &Api, map: *mut c_void, key: u64, mode: c_int) -> (isize, u64) {
    let mut key = key;
    let map = (api.hmget)(
        map,
        size_of::<Record>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        mode,
    );
    let index = map_temp(map, size_of::<Record>());
    let value = if index < 0 {
        0
    } else {
        (*map.cast::<Record>().offset(index)).value
    };
    (index, value)
}

unsafe fn del_record(api: &Api, map: *mut c_void, key: u64, mode: c_int) -> *mut c_void {
    let mut key = key;
    (api.hmdel)(
        map,
        size_of::<Record>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        0,
        mode,
    )
}

unsafe fn put_string(
    api: &Api,
    map: *mut c_void,
    key: *mut c_char,
    value: i64,
    mode: c_int,
) -> *mut c_void {
    let map = (api.hmput)(
        map,
        size_of::<StringRecord>(),
        key.cast(),
        size_of::<*mut c_char>(),
        mode,
    );
    let index = map_temp(map, size_of::<StringRecord>());
    (*map.cast::<StringRecord>().offset(index)).value = value;
    map
}

unsafe fn get_string(api: &Api, map: *mut c_void, key: *mut c_char, mode: c_int) -> (isize, i64) {
    let map = (api.hmget)(
        map,
        size_of::<StringRecord>(),
        key.cast(),
        size_of::<*mut c_char>(),
        mode,
    );
    let index = map_temp(map, size_of::<StringRecord>());
    let value = if index < 0 {
        0
    } else {
        (*map.cast::<StringRecord>().offset(index)).value
    };
    (index, value)
}

fn c_string(mut bytes: Vec<u8>) -> Vec<u8> {
    for byte in &mut bytes {
        if *byte == 0 {
            *byte = 1;
        }
    }
    bytes.push(0);
    bytes
}

#[test]
fn configs_01_through_05_and_28_29_arrays_and_simple_exports() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x48f0_214a_990d_126b);

    unsafe {
        for iteration in 0..96usize {
            let element_size = [1, 2, 4, 8, 16, 24][iteration % 6];
            let add_len = 1 + iteration % 3;
            let c_array = (c.arrgrow)(null_mut(), element_size, add_len, 0);
            let r_array = (rust.arrgrow)(null_mut(), element_size, add_len, 0);
            assert_eq!((*header(c_array)).capacity, (*header(r_array)).capacity);
            assert_eq!((*header(c_array)).length, 0);
            assert_eq!((*header(r_array)).length, 0);

            let length = 1 + iteration % (*header(c_array)).capacity;
            (*header(c_array)).length = length;
            (*header(r_array)).length = length;
            let byte_len = length * element_size;
            let mut contents = vec![0; byte_len];
            rng.fill(&mut contents);
            std::ptr::copy_nonoverlapping(contents.as_ptr(), c_array.cast(), byte_len);
            std::ptr::copy_nonoverlapping(contents.as_ptr(), r_array.cast(), byte_len);

            let old_c = c_array;
            let old_r = r_array;
            let c_fit = (c.arrgrow)(c_array, element_size, 0, (*header(c_array)).capacity);
            let r_fit = (rust.arrgrow)(r_array, element_size, 0, (*header(r_array)).capacity);
            assert_eq!(c_fit, old_c);
            assert_eq!(r_fit, old_r);

            let request = (*header(c_fit)).capacity + 1 + iteration % 9;
            let c_grown = (c.arrgrow)(c_fit, element_size, request, 0);
            let r_grown = (rust.arrgrow)(r_fit, element_size, request, 0);
            assert_eq!((*header(c_grown)).capacity, (*header(r_grown)).capacity);
            assert_eq!((*header(c_grown)).length, (*header(r_grown)).length);
            assert_eq!(
                std::slice::from_raw_parts(c_grown.cast::<u8>(), byte_len),
                std::slice::from_raw_parts(r_grown.cast::<u8>(), byte_len)
            );

            let explicit = (*header(c_grown)).capacity * 2 + 3;
            let c_explicit = (c.arrgrow)(c_grown, element_size, 0, explicit);
            let r_explicit = (rust.arrgrow)(r_grown, element_size, 0, explicit);
            assert_eq!((*header(c_explicit)).capacity, explicit);
            assert_eq!((*header(r_explicit)).capacity, explicit);
            (c.arrfree)(c_explicit);
            (rust.arrfree)(r_explicit);
        }

        for number in [c_int::MIN, -1001, -1, 0, 1, 42, c_int::MAX] {
            let c_value = CStr::from_ptr((c.strkey)(number)).to_bytes().to_vec();
            let r_value = CStr::from_ptr((rust.strkey)(number)).to_bytes().to_vec();
            assert_eq!(c_value, r_value);
            (c.arr_del)(number);
            (rust.arr_del)(number);
        }
        for _ in 0..96 {
            let number = rng.next_u64() as c_int;
            let c_value = CStr::from_ptr((c.strkey)(number)).to_bytes().to_vec();
            let r_value = CStr::from_ptr((rust.strkey)(number)).to_bytes().to_vec();
            assert_eq!(c_value, r_value);
            (c.arr_del)(number);
            (rust.arr_del)(number);
        }
    }
}

#[test]
fn configs_06_through_09_hashes() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0xb708_e31f_9812_a431);

    unsafe {
        for &seed in &[0, 1, 0x3141_5926, usize::MAX] {
            assert_eq!(
                (c.hash_bytes)(null_mut(), 0, seed),
                (rust.hash_bytes)(null_mut(), 0, seed)
            );
        }

        for len in 0..160usize {
            for _ in 0..24 {
                let mut bytes = vec![0; len];
                rng.fill(&mut bytes);
                let seed = rng.next_u64() as usize;
                assert_eq!(
                    (c.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                    (rust.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                    "byte hash mismatch at length {len}"
                );
            }
        }

        for len in [0, 1, 2, 7, 8, 31, 255, 1024] {
            for _ in 0..24 {
                let mut bytes = vec![0; len];
                rng.fill(&mut bytes);
                let mut string = c_string(bytes);
                let seed = rng.next_u64() as usize;
                assert_eq!(
                    (c.hash_string)(string.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(string.as_mut_ptr().cast(), seed),
                    "string hash mismatch at length {len}"
                );
            }
        }
    }
}

#[test]
fn configs_10_through_15_binary_maps() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6c62_2c66_40ba_f2e7);

    unsafe {
        for key_size in [0, 1, 2, 4, 7, 8, 9, 16] {
            (c.rand_seed)(0x5151_7171 + key_size);
            (rust.rand_seed)(0x5151_7171 + key_size);
            let element_size = key_size + 8;
            let mut c_sized_map = null_mut();
            let mut r_sized_map = null_mut();
            for index in 0..48usize {
                let mut c_key = vec![0; key_size.max(1)];
                rng.fill(&mut c_key);
                if key_size > 0 {
                    c_key[0] = index as u8;
                }
                let mut r_key = c_key.clone();
                c_sized_map = (c.hmput)(
                    c_sized_map,
                    element_size,
                    c_key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                r_sized_map = (rust.hmput)(
                    r_sized_map,
                    element_size,
                    r_key.as_mut_ptr().cast(),
                    key_size,
                    0,
                );
                assert_eq!(
                    map_temp(c_sized_map, element_size),
                    map_temp(r_sized_map, element_size)
                );
                assert_eq!(
                    map_len(c_sized_map, element_size),
                    map_len(r_sized_map, element_size)
                );
            }
            free_map(&c, c_sized_map, element_size);
            free_map(&rust, r_sized_map, element_size);
        }

        (c.rand_seed)(0x1020_3040);
        (rust.rand_seed)(0x1020_3040);
        let mut c_map = (c.hmdefault)(null_mut(), size_of::<Record>());
        let mut r_map = (rust.hmdefault)(null_mut(), size_of::<Record>());
        assert_eq!(map_len(c_map, size_of::<Record>()), 0);
        assert_eq!(map_len(r_map, size_of::<Record>()), 0);
        assert_eq!(
            std::slice::from_raw_parts(
                c_map.cast::<u8>().sub(size_of::<Record>()),
                size_of::<Record>()
            ),
            std::slice::from_raw_parts(
                r_map.cast::<u8>().sub(size_of::<Record>()),
                size_of::<Record>()
            )
        );
        assert_eq!((c.hmdefault)(c_map, size_of::<Record>()), c_map);
        assert_eq!((rust.hmdefault)(r_map, size_of::<Record>()), r_map);

        let mut keys = Vec::new();
        for index in 0..192u64 {
            let key = rng.next_u64() ^ index.rotate_left(17);
            let value = rng.next_u64();
            keys.push((key, value));
            c_map = put_record(&c, c_map, key, value, 0);
            r_map = put_record(&rust, r_map, key, value, 0);
            assert_eq!(
                map_len(c_map, size_of::<Record>()),
                map_len(r_map, size_of::<Record>())
            );
            assert_eq!(
                (*table(c_map, size_of::<Record>())).slot_count,
                (*table(r_map, size_of::<Record>())).slot_count
            );
        }

        for &(key, value) in &keys {
            assert_eq!(
                get_record(&c, c_map, key, 0),
                get_record(&rust, r_map, key, 0)
            );
            assert_eq!(get_record(&c, c_map, key, 0).1, value);
            let mut c_temp = 99;
            let mut r_temp = 99;
            let mut c_key = key;
            let mut r_key = key;
            c_map = (c.hmget_ts)(
                c_map,
                size_of::<Record>(),
                (&mut c_key as *mut u64).cast(),
                8,
                &mut c_temp,
                0,
            );
            r_map = (rust.hmget_ts)(
                r_map,
                size_of::<Record>(),
                (&mut r_key as *mut u64).cast(),
                8,
                &mut r_temp,
                0,
            );
            assert_eq!(c_temp, r_temp);
        }

        for index in (0..keys.len()).step_by(3) {
            let key = keys[index].0;
            let value = rng.next_u64();
            c_map = put_record(&c, c_map, key, value, 0);
            r_map = put_record(&rust, r_map, key, value, 0);
            assert_eq!(
                get_record(&c, c_map, key, 0),
                get_record(&rust, r_map, key, 0)
            );
            keys[index].1 = value;
        }

        for &(key, _) in keys.iter().take(150) {
            c_map = del_record(&c, c_map, key, 0);
            r_map = del_record(&rust, r_map, key, 0);
            assert_eq!(
                map_temp(c_map, size_of::<Record>()),
                map_temp(r_map, size_of::<Record>())
            );
            assert_eq!(
                map_len(c_map, size_of::<Record>()),
                map_len(r_map, size_of::<Record>())
            );
            assert_eq!(
                (*table(c_map, size_of::<Record>())).slot_count,
                (*table(r_map, size_of::<Record>())).slot_count
            );
        }

        for index in 0..96u64 {
            let key = 0xf000_0000_0000_0000 | index;
            c_map = put_record(&c, c_map, key, index * 9, 0);
            r_map = put_record(&rust, r_map, key, index * 9, 0);
            assert_eq!(
                get_record(&c, c_map, key, 0),
                get_record(&rust, r_map, key, 0)
            );
        }

        free_map(&c, c_map, size_of::<Record>());
        free_map(&rust, r_map, size_of::<Record>());

        (c.rand_seed)(0xaabb_ccdd);
        (rust.rand_seed)(0xaabb_ccdd);
        let mut colliding = Vec::new();
        for candidate in 0..10_000u64 {
            let mut value = candidate;
            let hash = (c.hash_bytes)(
                (&mut value as *mut u64).cast(),
                size_of::<u64>(),
                0xaabb_ccdd,
            );
            if hash & 7 == 3 {
                colliding.push(candidate);
                if colliding.len() == 3 {
                    break;
                }
            }
        }
        assert_eq!(colliding.len(), 3);
        let mut c_reuse = null_mut();
        let mut r_reuse = null_mut();
        for &key in &colliding[..2] {
            c_reuse = put_record(&c, c_reuse, key, key + 1, 0);
            r_reuse = put_record(&rust, r_reuse, key, key + 1, 0);
        }
        c_reuse = del_record(&c, c_reuse, colliding[0], 0);
        r_reuse = del_record(&rust, r_reuse, colliding[0], 0);
        assert_eq!((*table(c_reuse, size_of::<Record>())).tombstone_count, 1);
        assert_eq!((*table(r_reuse, size_of::<Record>())).tombstone_count, 1);
        c_reuse = put_record(&c, c_reuse, colliding[2], 99, 0);
        r_reuse = put_record(&rust, r_reuse, colliding[2], 99, 0);
        assert_eq!((*table(c_reuse, size_of::<Record>())).tombstone_count, 0);
        assert_eq!((*table(r_reuse, size_of::<Record>())).tombstone_count, 0);
        free_map(&c, c_reuse, size_of::<Record>());
        free_map(&rust, r_reuse, size_of::<Record>());

        let mut c_rebuild = null_mut();
        let mut r_rebuild = null_mut();
        for key in 0..80u64 {
            c_rebuild = put_record(&c, c_rebuild, key, key, 0);
            r_rebuild = put_record(&rust, r_rebuild, key, key, 0);
        }
        assert_eq!((*table(c_rebuild, size_of::<Record>())).slot_count, 128);
        for key in 0..25u64 {
            c_rebuild = del_record(&c, c_rebuild, key, 0);
            r_rebuild = del_record(&rust, r_rebuild, key, 0);
        }
        assert_eq!((*table(c_rebuild, size_of::<Record>())).slot_count, 128);
        assert_eq!((*table(c_rebuild, size_of::<Record>())).tombstone_count, 0);
        assert_eq!((*table(r_rebuild, size_of::<Record>())).tombstone_count, 0);
        free_map(&c, c_rebuild, size_of::<Record>());
        free_map(&rust, r_rebuild, size_of::<Record>());
    }
}

#[test]
fn configs_16_through_20_string_maps() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x0ee2_84aa_a031_f116);

    unsafe {
        for mode in [1, 2, 3] {
            (c.rand_seed)(0x8877_6655);
            (rust.rand_seed)(0x8877_6655);
            let mut c_map = if mode == 1 {
                (c.shmode)(size_of::<StringRecord>(), 1)
            } else {
                (c.shmode)(size_of::<StringRecord>(), mode)
            };
            let mut r_map = if mode == 1 {
                (rust.shmode)(size_of::<StringRecord>(), 1)
            } else {
                (rust.shmode)(size_of::<StringRecord>(), mode)
            };

            let mut c_keys = Vec::<Vec<u8>>::new();
            let mut r_keys = Vec::<Vec<u8>>::new();
            for index in 0..96usize {
                let len = match index % 8 {
                    0 => 0,
                    1 => 1,
                    2 => 7,
                    3 => 31,
                    4 => 255,
                    5 => 511,
                    6 => 513,
                    _ => 16 + index,
                };
                let mut bytes = vec![0; len];
                rng.fill(&mut bytes);
                bytes.extend_from_slice(format!("_{index}").as_bytes());
                let key = c_string(bytes);
                c_keys.push(key.clone());
                r_keys.push(key);
                let value = rng.next_u64() as i64;
                c_map = put_string(
                    &c,
                    c_map,
                    c_keys.last_mut().unwrap().as_mut_ptr().cast(),
                    value,
                    1,
                );
                r_map = put_string(
                    &rust,
                    r_map,
                    r_keys.last_mut().unwrap().as_mut_ptr().cast(),
                    value,
                    1,
                );
                assert_eq!(
                    map_len(c_map, size_of::<StringRecord>()),
                    map_len(r_map, size_of::<StringRecord>())
                );
                assert_eq!(
                    get_string(&c, c_map, c_keys.last_mut().unwrap().as_mut_ptr().cast(), 1),
                    get_string(
                        &rust,
                        r_map,
                        r_keys.last_mut().unwrap().as_mut_ptr().cast(),
                        1
                    )
                );
            }

            if mode == 2 {
                let original = CStr::from_ptr((*c_map.cast::<StringRecord>()).key)
                    .to_bytes()
                    .to_vec();
                c_keys[0][0] = c_keys[0][0].wrapping_add(1).max(1);
                r_keys[0][0] = r_keys[0][0].wrapping_add(1).max(1);
                assert_eq!(
                    CStr::from_ptr((*c_map.cast::<StringRecord>()).key).to_bytes(),
                    original
                );
                assert_eq!(
                    CStr::from_ptr((*r_map.cast::<StringRecord>()).key).to_bytes(),
                    original
                );
            }

            for index in (0..96).step_by(4) {
                c_map = (c.hmdel)(
                    c_map,
                    size_of::<StringRecord>(),
                    c_keys[index].as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                r_map = (rust.hmdel)(
                    r_map,
                    size_of::<StringRecord>(),
                    r_keys[index].as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                assert_eq!(
                    map_temp(c_map, size_of::<StringRecord>()),
                    map_temp(r_map, size_of::<StringRecord>())
                );
                assert_eq!(
                    map_len(c_map, size_of::<StringRecord>()),
                    map_len(r_map, size_of::<StringRecord>())
                );
            }

            for index in (1..96).step_by(13) {
                let value = rng.next_u64() as i64;
                c_map = put_string(&c, c_map, c_keys[index].as_mut_ptr().cast(), value, 1);
                r_map = put_string(&rust, r_map, r_keys[index].as_mut_ptr().cast(), value, 1);
                assert_eq!(
                    get_string(&c, c_map, c_keys[index].as_mut_ptr().cast(), 1),
                    get_string(&rust, r_map, r_keys[index].as_mut_ptr().cast(), 1)
                );
            }

            free_map(&c, c_map, size_of::<StringRecord>());
            free_map(&rust, r_map, size_of::<StringRecord>());
        }
    }
}

#[test]
fn configs_21_through_24_string_arena() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0xcc7a_91c0_c007_5599);

    unsafe {
        let mut c_arena = StringArena::empty();
        let mut r_arena = StringArena::empty();
        for len in [0, 1, 7, 255, 510, 512, 513, 1024, 4097] {
            let mut bytes = vec![0; len];
            rng.fill(&mut bytes);
            let mut c_value = c_string(bytes.clone());
            let mut r_value = c_string(bytes);
            let c_result = (c.stralloc)(&mut c_arena, c_value.as_mut_ptr().cast());
            let r_result = (rust.stralloc)(&mut r_arena, r_value.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(c_arena.remaining, r_arena.remaining);
            assert_eq!(c_arena.block, r_arena.block);
        }
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert!(c_arena.storage.is_null() && r_arena.storage.is_null());
        assert_eq!(c_arena.remaining, 0);
        assert_eq!(r_arena.remaining, 0);

        let mut oversized = vec![0; 700];
        rng.fill(&mut oversized);
        let mut c_oversized = c_string(oversized.clone());
        let mut r_oversized = c_string(oversized);
        let c_result = (c.stralloc)(&mut c_arena, c_oversized.as_mut_ptr().cast());
        let r_result = (rust.stralloc)(&mut r_arena, r_oversized.as_mut_ptr().cast());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes(),
            CStr::from_ptr(r_result).to_bytes()
        );
        assert_eq!(c_arena.remaining, r_arena.remaining);
        let mut c_second = c_oversized.clone();
        let mut r_second = r_oversized.clone();
        let c_result = (c.stralloc)(&mut c_arena, c_second.as_mut_ptr().cast());
        let r_result = (rust.stralloc)(&mut r_arena, r_second.as_mut_ptr().cast());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes(),
            CStr::from_ptr(r_result).to_bytes()
        );
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        for _ in 0..24 {
            let block_size = 512usize << ((c_arena.block as usize) >> 1);
            let mut bytes = vec![0; block_size.saturating_sub(1)];
            rng.fill(&mut bytes);
            let mut c_value = c_string(bytes.clone());
            let mut r_value = c_string(bytes);
            let c_result = (c.stralloc)(&mut c_arena, c_value.as_mut_ptr().cast());
            let r_result = (rust.stralloc)(&mut r_arena, r_value.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(c_arena.remaining, r_arena.remaining);
            assert_eq!(c_arena.block, r_arena.block);
        }
        assert_eq!(c_arena.block, r_arena.block);
        assert_eq!(c_arena.block, 22);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        let mut c_empty = StringArena::empty();
        let mut r_empty = StringArena::empty();
        (c.strreset)(&mut c_empty);
        (rust.strreset)(&mut r_empty);
        assert!(c_empty.storage.is_null() && r_empty.storage.is_null());
    }
}

#[test]
fn configs_25_through_27_modes_and_offsets() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd75b_54ea_0c91_2b77);

    unsafe {
        for mode in [c_int::MIN, -257, -1, 0, 1, 2, 3, 4, 255, 256, c_int::MAX] {
            let c_map = (c.shmode)(size_of::<Record>(), mode);
            let r_map = (rust.shmode)(size_of::<Record>(), mode);
            assert_eq!(map_len(c_map, size_of::<Record>()), 0);
            assert_eq!(map_len(r_map, size_of::<Record>()), 0);
            assert_eq!(
                (*table(c_map, size_of::<Record>())).string.mode,
                (*table(r_map, size_of::<Record>())).string.mode
            );
            free_map(&c, c_map, size_of::<Record>());
            free_map(&rust, r_map, size_of::<Record>());
        }

        for mode in [c_int::MIN, -99, 0] {
            (c.rand_seed)(77);
            (rust.rand_seed)(77);
            let mut c_map = null_mut();
            let mut r_map = null_mut();
            for key in 10..80u64 {
                c_map = put_record(&c, c_map, key, key * 3, mode);
                r_map = put_record(&rust, r_map, key, key * 3, mode);
            }
            for key in 10..80u64 {
                assert_eq!(
                    get_record(&c, c_map, key, mode),
                    get_record(&rust, r_map, key, mode)
                );
            }
            free_map(&c, c_map, size_of::<Record>());
            free_map(&rust, r_map, size_of::<Record>());
        }

        for mode in [2, 7, c_int::MAX] {
            let mut c_key = b"out_of_range_mode\0".to_vec();
            let mut r_key = c_key.clone();
            let mut c_map = put_string(&c, null_mut(), c_key.as_mut_ptr().cast(), 17, mode);
            let mut r_map = put_string(&rust, null_mut(), r_key.as_mut_ptr().cast(), 17, mode);
            assert_eq!(
                get_string(&c, c_map, c_key.as_mut_ptr().cast(), mode),
                get_string(&rust, r_map, r_key.as_mut_ptr().cast(), mode)
            );
            c_map = (c.hmdel)(
                c_map,
                size_of::<StringRecord>(),
                c_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                mode,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<StringRecord>(),
                r_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                0,
                mode,
            );
            assert_eq!(
                map_len(c_map, size_of::<StringRecord>()),
                map_len(r_map, size_of::<StringRecord>())
            );
            free_map(&c, c_map, size_of::<StringRecord>());
            free_map(&rust, r_map, size_of::<StringRecord>());
        }

        for iteration in 0..64 {
            let offset = 8usize;
            let key = rng.next_u64() ^ iteration;
            let mut c_map = put_record(&c, null_mut(), key, key, 0);
            let mut r_map = put_record(&rust, null_mut(), key, key, 0);
            let c_record = c_map.cast::<Record>();
            let r_record = r_map.cast::<Record>();
            (*c_record).value = (*c_record).key;
            (*r_record).value = (*r_record).key;
            let mut c_key = key;
            let mut r_key = key;
            c_map = (c.hmdel)(
                c_map,
                size_of::<Record>(),
                (&mut c_key as *mut u64).cast(),
                8,
                offset,
                0,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<Record>(),
                (&mut r_key as *mut u64).cast(),
                8,
                offset,
                0,
            );
            assert_eq!(
                map_len(c_map, size_of::<Record>()),
                map_len(r_map, size_of::<Record>())
            );
            assert_eq!(
                map_temp(c_map, size_of::<Record>()),
                map_temp(r_map, size_of::<Record>())
            );
            free_map(&c, c_map, size_of::<Record>());
            free_map(&rust, r_map, size_of::<Record>());
        }

        for iteration in 0..48 {
            let mut c_string_key =
                c_string(format!("offset_{iteration}_{}", rng.next_u64()).into_bytes());
            let mut r_string_key = c_string_key.clone();
            let mut c_string_map = (c.shmode)(size_of::<OffsetStringRecord>(), 1);
            let mut r_string_map = (rust.shmode)(size_of::<OffsetStringRecord>(), 1);
            c_string_map = (c.hmput)(
                c_string_map,
                size_of::<OffsetStringRecord>(),
                c_string_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            r_string_map = (rust.hmput)(
                r_string_map,
                size_of::<OffsetStringRecord>(),
                r_string_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            (*c_string_map.cast::<OffsetStringRecord>()).duplicate_key =
                (*c_string_map.cast::<OffsetStringRecord>()).key;
            (*r_string_map.cast::<OffsetStringRecord>()).duplicate_key =
                (*r_string_map.cast::<OffsetStringRecord>()).key;
            c_string_map = (c.hmdel)(
                c_string_map,
                size_of::<OffsetStringRecord>(),
                c_string_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                size_of::<*mut c_char>(),
                1,
            );
            r_string_map = (rust.hmdel)(
                r_string_map,
                size_of::<OffsetStringRecord>(),
                r_string_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                size_of::<*mut c_char>(),
                1,
            );
            assert_eq!(
                map_len(c_string_map, size_of::<OffsetStringRecord>()),
                map_len(r_string_map, size_of::<OffsetStringRecord>())
            );
            assert_eq!(
                map_temp(c_string_map, size_of::<OffsetStringRecord>()),
                map_temp(r_string_map, size_of::<OffsetStringRecord>())
            );
            free_map(&c, c_string_map, size_of::<OffsetStringRecord>());
            free_map(&rust, r_string_map, size_of::<OffsetStringRecord>());
        }
    }
}

#[test]
fn errors_01_through_09_defined_rejections_and_sentinels() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();

    unsafe {
        assert!((c.arrgrow)(null_mut(), 8, 0, 0).is_null());
        assert!((rust.arrgrow)(null_mut(), 8, 0, 0).is_null());

        (c.hmfree)(null_mut(), size_of::<Record>());
        (rust.hmfree)(null_mut(), size_of::<Record>());

        let mut key = 123u64;
        let mut c_temp = 77;
        let mut r_temp = 77;
        let mut c_map = (c.hmget_ts)(
            null_mut(),
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            &mut c_temp,
            0,
        );
        let mut r_map = (rust.hmget_ts)(
            null_mut(),
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            &mut r_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, r_temp);
        assert_eq!(
            map_len(c_map, size_of::<Record>()),
            map_len(r_map, size_of::<Record>())
        );

        c_temp = 77;
        r_temp = 77;
        c_map = (c.hmget_ts)(
            c_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            &mut c_temp,
            0,
        );
        r_map = (rust.hmget_ts)(
            r_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            &mut r_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, r_temp);

        c_map = (c.hmget)(
            c_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            0,
        );
        r_map = (rust.hmget)(
            r_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            0,
        );
        assert_eq!(map_temp(c_map, size_of::<Record>()), -1);
        assert_eq!(
            map_temp(c_map, size_of::<Record>()),
            map_temp(r_map, size_of::<Record>())
        );

        assert!((c.hmdel)(null_mut(), size_of::<Record>(), null_mut(), 8, 0, 0).is_null());
        assert!((rust.hmdel)(null_mut(), size_of::<Record>(), null_mut(), 8, 0, 0).is_null());

        c_map = (c.hmdel)(
            c_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            0,
            0,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<Record>(),
            (&mut key as *mut u64).cast(),
            8,
            0,
            0,
        );
        assert_eq!(map_temp(c_map, size_of::<Record>()), 0);
        assert_eq!(
            map_temp(c_map, size_of::<Record>()),
            map_temp(r_map, size_of::<Record>())
        );

        c_map = put_record(&c, c_map, 456, 999, 0);
        r_map = put_record(&rust, r_map, 456, 999, 0);
        let missing = 789u64;
        c_temp = 77;
        r_temp = 77;
        c_map = (c.hmget_ts)(
            c_map,
            size_of::<Record>(),
            (&missing as *const u64 as *mut u64).cast(),
            8,
            &mut c_temp,
            0,
        );
        r_map = (rust.hmget_ts)(
            r_map,
            size_of::<Record>(),
            (&missing as *const u64 as *mut u64).cast(),
            8,
            &mut r_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, r_temp);

        c_map = (c.hmdel)(
            c_map,
            size_of::<Record>(),
            (&missing as *const u64 as *mut u64).cast(),
            8,
            0,
            0,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<Record>(),
            (&missing as *const u64 as *mut u64).cast(),
            8,
            0,
            0,
        );
        assert_eq!(map_temp(c_map, size_of::<Record>()), 0);
        assert_eq!(
            map_temp(c_map, size_of::<Record>()),
            map_temp(r_map, size_of::<Record>())
        );

        free_map(&c, c_map, size_of::<Record>());
        free_map(&rust, r_map, size_of::<Record>());
    }
}
