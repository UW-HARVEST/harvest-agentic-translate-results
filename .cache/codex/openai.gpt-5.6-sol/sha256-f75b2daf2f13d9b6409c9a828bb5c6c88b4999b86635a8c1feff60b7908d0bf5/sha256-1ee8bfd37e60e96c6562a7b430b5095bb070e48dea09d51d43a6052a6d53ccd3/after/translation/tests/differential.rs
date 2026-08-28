use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::ptr;

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
type ArrIns = unsafe extern "C" fn(c_int);

#[derive(Clone, Copy)]
struct Fns {
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
    arr_ins: ArrIns,
}

struct Api {
    _library: Library,
    f: Fns,
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
        let f = Fns {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmput_default: symbol!("stbds_hmput_default", HmPutDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            shmode: symbol!("stbds_shmode_func", ShMode),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            arr_ins: symbol!("arr_ins", ArrIns),
        };
        Self {
            _library: library,
            f,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
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
struct BinaryEntry {
    key: u32,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringEntry {
    key: *mut c_char,
    value: i32,
}

#[repr(C)]
struct HashIndexView {
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

unsafe fn header(data: *mut c_void) -> *mut ArrayHeader {
    unsafe { data.cast::<ArrayHeader>().sub(1) }
}

unsafe fn raw_map(map: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(elemsize).cast() }
}

unsafe fn map_header(map: *mut c_void, elemsize: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, elemsize)) }
}

unsafe fn map_len(map: *mut c_void, elemsize: usize) -> usize {
    if map.is_null() {
        0
    } else {
        unsafe { (*map_header(map, elemsize)).length - 1 }
    }
}

unsafe fn map_temp(map: *mut c_void, elemsize: usize) -> isize {
    unsafe { (*map_header(map, elemsize)).temp }
}

unsafe fn free_map(f: Fns, map: *mut c_void, elemsize: usize) {
    if !map.is_null() {
        unsafe { (f.hmfree)(raw_map(map, elemsize), elemsize) };
    }
}

unsafe fn table_view(map: *mut c_void, elemsize: usize) -> Option<(usize, usize, usize, u8)> {
    let table = unsafe {
        (*map_header(map, elemsize))
            .hash_table
            .cast::<HashIndexView>()
    };
    if table.is_null() {
        None
    } else {
        Some(unsafe {
            (
                (*table).slot_count,
                (*table).used_count,
                (*table).tombstone_count,
                (*table).string.mode,
            )
        })
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libharvest-work-xQR7Ht.so"),
        manifest.join("target/release/libarr_ins_lib.so"),
    )
}

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

unsafe fn put_binary(f: Fns, map: *mut c_void, key: u32, value: i32, mode: c_int) -> *mut c_void {
    let mut key = key;
    let map = unsafe {
        (f.hmput)(
            map,
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            mode,
        )
    };
    let index = unsafe { map_temp(map, size_of::<BinaryEntry>()) } as usize;
    unsafe { (*map.cast::<BinaryEntry>().add(index)).value = value };
    map
}

unsafe fn binary_entries(map: *mut c_void) -> Vec<BinaryEntry> {
    let len = unsafe { map_len(map, size_of::<BinaryEntry>()) };
    unsafe { std::slice::from_raw_parts(map.cast::<BinaryEntry>(), len) }.to_vec()
}

unsafe fn put_string(
    f: Fns,
    map: *mut c_void,
    key: *mut c_char,
    value: i32,
    mode: c_int,
) -> *mut c_void {
    let map = unsafe {
        (f.hmput)(
            map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { map_temp(map, size_of::<StringEntry>()) } as usize;
    unsafe { (*map.cast::<StringEntry>().add(index)).value = value };
    map
}

unsafe fn string_entries(map: *mut c_void) -> Vec<(Vec<u8>, i32)> {
    let len = unsafe { map_len(map, size_of::<StringEntry>()) };
    (0..len)
        .map(|index| {
            let entry = unsafe { *map.cast::<StringEntry>().add(index) };
            (
                unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
                entry.value,
            )
        })
        .collect()
}

unsafe fn compare_hashes(c: Fns, rust: Fns) {
    let mut rng = Rng::new(0x8c62_42d7_17ac_91e3);
    let seeds = [0, 1, usize::MAX, 0x3141_5926, 0xdead_beef_cafe_babe];

    for length in 0..=95 {
        for sample in 0..64 {
            let mut bytes = vec![0u8; length];
            rng.fill(&mut bytes);
            if sample % 4 == 0 {
                bytes.fill(0xff);
            }
            let seed = seeds[sample % seeds.len()] ^ rng.next_u64() as usize;
            let pointer = if bytes.is_empty() {
                ptr::null_mut()
            } else {
                bytes.as_mut_ptr().cast()
            };
            let c_hash = unsafe { (c.hash_bytes)(pointer, bytes.len(), seed) };
            let rust_hash = unsafe { (rust.hash_bytes)(pointer, bytes.len(), seed) };
            assert_eq!(
                c_hash, rust_hash,
                "hash_bytes differs for length={length}, sample={sample}, seed={seed:#x}"
            );
        }
    }

    for length in 0..=80 {
        for sample in 0..64 {
            let mut bytes = vec![0u8; length];
            for byte in &mut bytes {
                *byte = ((rng.next_u64() % 255) + 1) as u8;
            }
            if sample % 4 == 0 {
                bytes.fill(0xff);
            }
            let string = CString::new(bytes).unwrap();
            let seed = seeds[sample % seeds.len()] ^ rng.next_u64() as usize;
            let c_hash = unsafe { (c.hash_string)(string.as_ptr().cast_mut(), seed) };
            let rust_hash = unsafe { (rust.hash_string)(string.as_ptr().cast_mut(), seed) };
            assert_eq!(
                c_hash, rust_hash,
                "hash_string differs for length={length}, sample={sample}, seed={seed:#x}"
            );
        }
    }
}

unsafe fn compare_arrays(c: Fns, rust: Fns) {
    for &element_size in &[0usize, 1, 4, 12, 24] {
        let mut c_array: *mut c_void = ptr::null_mut();
        let mut rust_array: *mut c_void = ptr::null_mut();

        for &(add_length, min_capacity) in &[
            (0, 0),
            (1, 0),
            (0, 2),
            (0, 4),
            (1, 0),
            (7, 0),
            (0, 6),
            (0, 40),
            (0, 5),
        ] {
            let old_c = c_array;
            let old_rust = rust_array;
            let old_c_capacity = if old_c.is_null() {
                0
            } else {
                unsafe { (*header(old_c)).capacity }
            };
            let old_rust_capacity = if old_rust.is_null() {
                0
            } else {
                unsafe { (*header(old_rust)).capacity }
            };
            let preserved = old_c_capacity
                .min(old_rust_capacity)
                .wrapping_mul(element_size)
                .min(32);
            c_array = unsafe { (c.arrgrow)(c_array, element_size, add_length, min_capacity) };
            rust_array =
                unsafe { (rust.arrgrow)(rust_array, element_size, add_length, min_capacity) };

            if c_array.is_null() || rust_array.is_null() {
                assert!(c_array.is_null() && rust_array.is_null());
                continue;
            }
            assert_eq!(
                unsafe { (*header(c_array)).capacity },
                unsafe { (*header(rust_array)).capacity },
                "array capacity differs for element_size={element_size}, add={add_length}, min={min_capacity}"
            );
            assert_eq!(unsafe { (*header(c_array)).length }, unsafe {
                (*header(rust_array)).length
            });
            if min_capacity <= old_c_capacity && add_length == 0 && !old_c.is_null() {
                assert_eq!(old_c, c_array, "C unexpectedly reallocated");
            }
            if min_capacity <= old_rust_capacity && add_length == 0 && !old_rust.is_null() {
                assert_eq!(old_rust, rust_array, "Rust unexpectedly reallocated");
            }

            if element_size != 0 {
                for index in 0..preserved {
                    let expected = (index as u8).wrapping_mul(17);
                    assert_eq!(unsafe { *c_array.cast::<u8>().add(index) }, expected);
                    assert_eq!(unsafe { *rust_array.cast::<u8>().add(index) }, expected);
                }
                let bytes = unsafe { (*header(c_array)).capacity * element_size };
                for index in 0..bytes.min(32) {
                    unsafe {
                        *c_array.cast::<u8>().add(index) = (index as u8).wrapping_mul(17);
                        *rust_array.cast::<u8>().add(index) = (index as u8).wrapping_mul(17);
                    }
                }
                assert_eq!(
                    unsafe { std::slice::from_raw_parts(c_array.cast::<u8>(), bytes.min(32)) },
                    unsafe { std::slice::from_raw_parts(rust_array.cast::<u8>(), bytes.min(32)) }
                );
            }
        }

        unsafe {
            (c.arrfree)(c_array);
            (rust.arrfree)(rust_array);
        }
    }

    let c_large = unsafe { (c.arrgrow)(ptr::null_mut(), 1, 0, 1 << 20) };
    let rust_large = unsafe { (rust.arrgrow)(ptr::null_mut(), 1, 0, 1 << 20) };
    assert_eq!(unsafe { (*header(c_large)).capacity }, unsafe {
        (*header(rust_large)).capacity
    });
    unsafe {
        (c.arrfree)(c_large);
        (rust.arrfree)(rust_large);
    }
}

unsafe fn compare_utilities(c: Fns, rust: Fns) {
    for value in [
        c_int::MIN,
        -1000,
        -10,
        -1,
        0,
        1,
        9,
        10,
        99,
        100,
        1000,
        c_int::MAX,
    ] {
        let c_value = unsafe { CStr::from_ptr((c.strkey)(value)) }
            .to_bytes()
            .to_vec();
        let rust_value = unsafe { CStr::from_ptr((rust.strkey)(value)) }
            .to_bytes()
            .to_vec();
        assert_eq!(c_value, rust_value, "strkey differs for {value}");
    }

    let c_first = unsafe { (c.strkey)(12) };
    let rust_first = unsafe { (rust.strkey)(12) };
    unsafe {
        (c.strkey)(34);
        (rust.strkey)(34);
    }
    assert_eq!(
        unsafe { CStr::from_ptr(c_first) }.to_bytes(),
        unsafe { CStr::from_ptr(rust_first) }.to_bytes()
    );

    for value in [c_int::MIN, -1, 0, 1, 42, c_int::MAX] {
        unsafe {
            (c.arr_ins)(value);
            (rust.arr_ins)(value);
        }
    }
}

unsafe fn compare_map_error_sentinels(c: Fns, rust: Fns) {
    let mut key = 17u32;
    let mut c_temp = 999isize;
    let mut rust_temp = 999isize;
    let c_map = unsafe {
        (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut c_temp,
            0,
        )
    };
    let rust_map = unsafe {
        (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut rust_temp,
            0,
        )
    };
    assert_eq!((c_temp, rust_temp), (-1, -1));
    assert_eq!(unsafe { map_len(c_map, size_of::<BinaryEntry>()) }, 0);
    assert_eq!(unsafe { map_len(rust_map, size_of::<BinaryEntry>()) }, 0);
    assert_eq!(
        unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
        unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
    );

    let c_same = unsafe {
        (c.hmget_ts)(
            c_map,
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut c_temp,
            0,
        )
    };
    let rust_same = unsafe {
        (rust.hmget_ts)(
            rust_map,
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut rust_temp,
            0,
        )
    };
    assert_eq!(c_same, c_map);
    assert_eq!(rust_same, rust_map);
    assert_eq!((c_temp, rust_temp), (-1, -1));

    let c_deleted = unsafe {
        (c.hmdel)(
            c_map,
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            0,
            0,
        )
    };
    let rust_deleted = unsafe {
        (rust.hmdel)(
            rust_map,
            size_of::<BinaryEntry>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            0,
            0,
        )
    };
    assert_eq!(c_deleted, c_map);
    assert_eq!(rust_deleted, rust_map);
    assert_eq!(
        unsafe { map_temp(c_map, size_of::<BinaryEntry>()) },
        unsafe { map_temp(rust_map, size_of::<BinaryEntry>()) }
    );

    assert!(
        unsafe {
            (c.hmdel)(
                ptr::null_mut(),
                size_of::<BinaryEntry>(),
                (&mut key as *mut u32).cast(),
                size_of::<u32>(),
                0,
                0,
            )
        }
        .is_null()
    );
    assert!(
        unsafe {
            (rust.hmdel)(
                ptr::null_mut(),
                size_of::<BinaryEntry>(),
                (&mut key as *mut u32).cast(),
                size_of::<u32>(),
                0,
                0,
            )
        }
        .is_null()
    );

    unsafe {
        free_map(c, c_map, size_of::<BinaryEntry>());
        free_map(rust, rust_map, size_of::<BinaryEntry>());
        (c.hmfree)(ptr::null_mut(), size_of::<BinaryEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<BinaryEntry>());
    }
}

unsafe fn compare_binary_maps(c: Fns, rust: Fns) {
    for &seed in &[0usize, 1, usize::MAX, 0x3141_5926] {
        unsafe {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
        }
        let mut c_map = unsafe { (c.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>()) };
        let mut rust_map =
            unsafe { (rust.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>()) };
        assert_eq!(unsafe { map_len(c_map, size_of::<BinaryEntry>()) }, 0);
        assert_eq!(unsafe { map_len(rust_map, size_of::<BinaryEntry>()) }, 0);
        assert_eq!(
            unsafe { *raw_map(c_map, size_of::<BinaryEntry>()).cast::<BinaryEntry>() },
            BinaryEntry { key: 0, value: 0 }
        );
        assert_eq!(
            unsafe { *raw_map(rust_map, size_of::<BinaryEntry>()).cast::<BinaryEntry>() },
            BinaryEntry { key: 0, value: 0 }
        );
        assert_eq!(
            unsafe { (c.hmput_default)(c_map, size_of::<BinaryEntry>()) },
            c_map
        );
        assert_eq!(
            unsafe { (rust.hmput_default)(rust_map, size_of::<BinaryEntry>()) },
            rust_map
        );

        let mut keys = Vec::new();
        for index in 0..128u32 {
            let key = index.wrapping_mul(0x9e37_79b9).wrapping_add(11);
            keys.push(key);
            let mode = if index % 2 == 0 { 0 } else { -7 };
            c_map = unsafe { put_binary(c, c_map, key, index as i32 * -3, mode) };
            rust_map = unsafe { put_binary(rust, rust_map, key, index as i32 * -3, mode) };
            assert_eq!(
                unsafe { binary_entries(c_map) },
                unsafe { binary_entries(rust_map) },
                "binary insertion differs at index={index}, seed={seed}"
            );
            assert_eq!(
                unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
                unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
            );
        }

        for (index, &key) in keys.iter().enumerate() {
            let mut key = key;
            let mut c_temp = -99;
            let mut rust_temp = -99;
            let c_result = unsafe {
                (c.hmget_ts)(
                    c_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    &mut c_temp,
                    0,
                )
            };
            let rust_result = unsafe {
                (rust.hmget_ts)(
                    rust_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    &mut rust_temp,
                    0,
                )
            };
            assert_eq!(c_result, c_map);
            assert_eq!(rust_result, rust_map);
            assert_eq!(c_temp, rust_temp);
            assert_eq!(
                unsafe { *c_map.cast::<BinaryEntry>().add(c_temp as usize) },
                unsafe { *rust_map.cast::<BinaryEntry>().add(rust_temp as usize) }
            );
            if index % 9 == 0 {
                c_map = unsafe { put_binary(c, c_map, key, index as i32 + 7000, 0) };
                rust_map = unsafe { put_binary(rust, rust_map, key, index as i32 + 7000, 0) };
            }
        }

        for missing in 0..64u32 {
            let mut key = 0xf000_0000 | missing;
            let c_result = unsafe {
                (c.hmget)(
                    c_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                )
            };
            let rust_result = unsafe {
                (rust.hmget)(
                    rust_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                )
            };
            assert_eq!(c_result, c_map);
            assert_eq!(rust_result, rust_map);
            assert_eq!(unsafe { map_temp(c_map, size_of::<BinaryEntry>()) }, -1);
            assert_eq!(unsafe { map_temp(rust_map, size_of::<BinaryEntry>()) }, -1);
        }

        for &index in &[3usize, 127, 11, 64, 7, 96] {
            let mut key = keys[index];
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                )
            };
            assert_eq!(unsafe { map_temp(c_map, size_of::<BinaryEntry>()) }, 1);
            assert_eq!(unsafe { map_temp(rust_map, size_of::<BinaryEntry>()) }, 1);
            assert_eq!(unsafe { binary_entries(c_map) }, unsafe {
                binary_entries(rust_map)
            });
            assert_eq!(
                unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
                unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
            );
        }

        let tombstones_before_reuse = unsafe { table_view(c_map, size_of::<BinaryEntry>()) }
            .unwrap()
            .2;
        c_map = unsafe { put_binary(c, c_map, keys[3], 3333, 0) };
        rust_map = unsafe { put_binary(rust, rust_map, keys[3], 3333, 0) };
        let tombstones_after_reuse = unsafe { table_view(c_map, size_of::<BinaryEntry>()) }
            .unwrap()
            .2;
        assert_eq!(tombstones_after_reuse + 1, tombstones_before_reuse);
        assert_eq!(
            unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
            unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
        );

        for extra in 0..12u32 {
            let key = 0xe000_0000 + extra;
            c_map = unsafe { put_binary(c, c_map, key, extra as i32, 0) };
            rust_map = unsafe { put_binary(rust, rust_map, key, extra as i32, 0) };
        }
        assert_eq!(unsafe { binary_entries(c_map) }, unsafe {
            binary_entries(rust_map)
        });

        let mut index = 0;
        let mut saw_rebuild = false;
        let mut saw_shrink = false;
        while unsafe { map_len(c_map, size_of::<BinaryEntry>()) } > 10 {
            let before = unsafe { table_view(c_map, size_of::<BinaryEntry>()) }.unwrap();
            let current = unsafe { *c_map.cast::<BinaryEntry>().add(index) }.key;
            let mut key = current;
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                )
            };
            assert_eq!(unsafe { binary_entries(c_map) }, unsafe {
                binary_entries(rust_map)
            });
            assert_eq!(
                unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
                unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
            );
            let after = unsafe { table_view(c_map, size_of::<BinaryEntry>()) }.unwrap();
            saw_rebuild |= after.0 == before.0 && after.2 < before.2;
            saw_shrink |= after.0 < before.0;
            index = (index + 3) % unsafe { map_len(c_map, size_of::<BinaryEntry>()) };
        }
        assert!(saw_rebuild, "did not exercise tombstone rebuild");
        assert!(saw_shrink, "did not exercise table shrink");

        unsafe {
            free_map(c, c_map, size_of::<BinaryEntry>());
            free_map(rust, rust_map, size_of::<BinaryEntry>());
        }
    }
}

unsafe fn compare_string_maps(c: Fns, rust: Fns) {
    for &(storage_mode, operation_mode) in &[(1, 1), (1, 2), (2, 1), (3, 1), (257, 1)] {
        unsafe {
            (c.rand_seed)(0x1234_5678);
            (rust.rand_seed)(0x1234_5678);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<StringEntry>(), storage_mode) };
        let mut rust_map = unsafe { (rust.shmode)(size_of::<StringEntry>(), storage_mode) };
        assert_eq!(
            unsafe { table_view(c_map, size_of::<StringEntry>()) },
            unsafe { table_view(rust_map, size_of::<StringEntry>()) }
        );

        let c_strings: Vec<CString> = (0..80)
            .map(|index| CString::new(format!("key_{index:03}_cafe")).unwrap())
            .collect();
        let rust_strings: Vec<CString> = (0..80)
            .map(|index| CString::new(format!("key_{index:03}_cafe")).unwrap())
            .collect();

        for index in 0..80 {
            c_map = unsafe {
                put_string(
                    c,
                    c_map,
                    c_strings[index].as_ptr().cast_mut(),
                    index as i32 * 13,
                    operation_mode,
                )
            };
            rust_map = unsafe {
                put_string(
                    rust,
                    rust_map,
                    rust_strings[index].as_ptr().cast_mut(),
                    index as i32 * 13,
                    operation_mode,
                )
            };
            assert_eq!(unsafe { string_entries(c_map) }, unsafe {
                string_entries(rust_map)
            });
            assert_eq!(
                unsafe { table_view(c_map, size_of::<StringEntry>()) },
                unsafe { table_view(rust_map, size_of::<StringEntry>()) }
            );
        }

        for index in (0..80).step_by(7) {
            c_map = unsafe {
                put_string(
                    c,
                    c_map,
                    c_strings[index].as_ptr().cast_mut(),
                    9000 + index as i32,
                    operation_mode,
                )
            };
            rust_map = unsafe {
                put_string(
                    rust,
                    rust_map,
                    rust_strings[index].as_ptr().cast_mut(),
                    9000 + index as i32,
                    operation_mode,
                )
            };
        }

        for index in 0..80 {
            let c_key = c_strings[index].as_ptr().cast_mut().cast();
            let rust_key = rust_strings[index].as_ptr().cast_mut().cast();
            let mut c_temp = -99;
            let mut rust_temp = -99;
            unsafe {
                (c.hmget_ts)(
                    c_map,
                    size_of::<StringEntry>(),
                    c_key,
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    operation_mode,
                );
                (rust.hmget_ts)(
                    rust_map,
                    size_of::<StringEntry>(),
                    rust_key,
                    size_of::<*mut c_char>(),
                    &mut rust_temp,
                    operation_mode,
                );
            }
            assert_eq!(c_temp, rust_temp);
            assert!(c_temp >= 0);
        }

        let delete_indices: &[usize] = if operation_mode == 1 {
            &[4, 79, 11, 38, 0]
        } else {
            &[79]
        };
        for &index in delete_indices {
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    size_of::<StringEntry>(),
                    c_strings[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    operation_mode,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    size_of::<StringEntry>(),
                    rust_strings[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    operation_mode,
                )
            };
            assert_eq!(unsafe { string_entries(c_map) }, unsafe {
                string_entries(rust_map)
            });
            assert_eq!(
                unsafe { table_view(c_map, size_of::<StringEntry>()) },
                unsafe { table_view(rust_map, size_of::<StringEntry>()) }
            );
        }

        let missing = CString::new("missing-key").unwrap();
        let c_same = unsafe {
            (c.hmdel)(
                c_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                operation_mode,
            )
        };
        let rust_same = unsafe {
            (rust.hmdel)(
                rust_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                operation_mode,
            )
        };
        assert_eq!(c_same, c_map);
        assert_eq!(rust_same, rust_map);
        assert_eq!(unsafe { map_temp(c_map, size_of::<StringEntry>()) }, 0);
        assert_eq!(unsafe { map_temp(rust_map, size_of::<StringEntry>()) }, 0);

        unsafe {
            free_map(c, c_map, size_of::<StringEntry>());
            free_map(rust, rust_map, size_of::<StringEntry>());
        }
    }

    for &storage_mode in &[0, 4, 255, -1, 256] {
        unsafe {
            (c.rand_seed)(77);
            (rust.rand_seed)(77);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<BinaryEntry>(), storage_mode) };
        let mut rust_map = unsafe { (rust.shmode)(size_of::<BinaryEntry>(), storage_mode) };
        for index in 0..32u32 {
            let key = index.wrapping_mul(31);
            c_map = unsafe { put_binary(c, c_map, key, index as i32, 0) };
            rust_map = unsafe { put_binary(rust, rust_map, key, index as i32, 0) };
        }
        assert_eq!(unsafe { binary_entries(c_map) }, unsafe {
            binary_entries(rust_map)
        });
        assert_eq!(
            unsafe { table_view(c_map, size_of::<BinaryEntry>()) },
            unsafe { table_view(rust_map, size_of::<BinaryEntry>()) }
        );
        unsafe {
            free_map(c, c_map, size_of::<BinaryEntry>());
            free_map(rust, rust_map, size_of::<BinaryEntry>());
        }
    }
}

unsafe fn compare_arenas(c: Fns, rust: Fns) {
    let mut c_arena: StringArena = unsafe { zeroed() };
    let mut rust_arena: StringArena = unsafe { zeroed() };

    for length in [0usize, 1, 7, 127, 400, 500, 511] {
        let string = CString::new(vec![b'x'; length]).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, string.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result) }.to_bytes(),
            unsafe { CStr::from_ptr(rust_result) }.to_bytes()
        );
        assert_eq!(
            (c_arena.remaining, c_arena.block, c_arena.mode),
            (rust_arena.remaining, rust_arena.block, rust_arena.mode)
        );
    }

    let dedicated = CString::new(vec![b'd'; 8192]).unwrap();
    for _ in 0..3 {
        let c_result = unsafe { (c.stralloc)(&mut c_arena, dedicated.as_ptr().cast_mut()) };
        let rust_result =
            unsafe { (rust.stralloc)(&mut rust_arena, dedicated.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result) }.to_bytes(),
            unsafe { CStr::from_ptr(rust_result) }.to_bytes()
        );
        assert_eq!(
            (c_arena.remaining, c_arena.block),
            (rust_arena.remaining, rust_arena.block)
        );
    }

    let growth = CString::new(vec![b'g'; 65_535]).unwrap();
    for iteration in 0..96 {
        let c_result = unsafe { (c.stralloc)(&mut c_arena, growth.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, growth.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result) }.to_bytes(),
            unsafe { CStr::from_ptr(rust_result) }.to_bytes(),
            "arena content differs at iteration {iteration}"
        );
        assert_eq!(
            (c_arena.remaining, c_arena.block),
            (rust_arena.remaining, rust_arena.block),
            "arena state differs at iteration {iteration}"
        );
    }
    assert_eq!(c_arena.block, 22);
    assert_eq!(rust_arena.block, 22);

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
    assert_eq!(
        (
            c_arena.storage.is_null(),
            c_arena.remaining,
            c_arena.block,
            c_arena.mode
        ),
        (
            rust_arena.storage.is_null(),
            rust_arena.remaining,
            rust_arena.block,
            rust_arena.mode
        )
    );

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }

    let mut empty_c: StringArena = unsafe { zeroed() };
    let mut empty_rust: StringArena = unsafe { zeroed() };
    let huge = CString::new(vec![b'h'; 2048]).unwrap();
    let c_result = unsafe { (c.stralloc)(&mut empty_c, huge.as_ptr().cast_mut()) };
    let rust_result = unsafe { (rust.stralloc)(&mut empty_rust, huge.as_ptr().cast_mut()) };
    assert_eq!(
        unsafe { CStr::from_ptr(c_result) }.to_bytes(),
        unsafe { CStr::from_ptr(rust_result) }.to_bytes()
    );
    assert_eq!(
        (empty_c.remaining, empty_c.block),
        (empty_rust.remaining, empty_rust.block)
    );
    unsafe {
        (c.strreset)(&mut empty_c);
        (rust.strreset)(&mut empty_rust);
    }

    let over_max = CString::new(vec![b'm'; (1 << 20) + 1]).unwrap();
    let c_result = unsafe { (c.stralloc)(&mut empty_c, over_max.as_ptr().cast_mut()) };
    let rust_result = unsafe { (rust.stralloc)(&mut empty_rust, over_max.as_ptr().cast_mut()) };
    assert_eq!(
        unsafe { CStr::from_ptr(c_result) }.to_bytes(),
        unsafe { CStr::from_ptr(rust_result) }.to_bytes()
    );
    unsafe {
        (c.strreset)(&mut empty_c);
        (rust.strreset)(&mut empty_rust);
    }
}

#[test]
fn fatal_ffi_child() {
    let Ok(kind) = std::env::var("DIFFERENTIAL_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFFERENTIAL_CHILD_CASE").unwrap();
    let (c_path, rust_path) = library_paths();
    let path = if kind == "c" { c_path } else { rust_path };
    let api = unsafe { Api::load(&path) };

    unsafe {
        match case.as_str() {
            "arrfree-null" => (api.f.arrfree)(ptr::null_mut()),
            "hash-bytes-null" => {
                (api.f.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hash-string-null" => {
                (api.f.hash_string)(ptr::null_mut(), 0);
            }
            "hmget-null-temp" => {
                let mut key = 0u32;
                (api.f.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    ptr::null_mut(),
                    0,
                );
            }
            "stralloc-null-arena" => {
                let string = CString::new("x").unwrap();
                (api.f.stralloc)(ptr::null_mut(), string.as_ptr().cast_mut());
            }
            "stralloc-null-string" => {
                let mut arena: StringArena = zeroed();
                (api.f.stralloc)(&mut arena, ptr::null_mut());
            }
            "strreset-null" => (api.f.strreset)(ptr::null_mut()),
            "hmdel-out-of-range-mode" => {
                (api.f.rand_seed)(123);
                let first = CString::new("first").unwrap();
                let second = CString::new("second").unwrap();
                let mut map = (api.f.shmode)(size_of::<StringEntry>(), 1);
                map = put_string(api.f, map, first.as_ptr().cast_mut(), 1, 2);
                map = put_string(api.f, map, second.as_ptr().cast_mut(), 2, 2);
                (api.f.hmdel)(
                    map,
                    size_of::<StringEntry>(),
                    first.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    2,
                );
            }
            _ => panic!("unknown fatal case {case}"),
        }
    }
}

fn compare_fatal_boundaries() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};

    let executable = std::env::current_exe().unwrap();
    for case in [
        "arrfree-null",
        "hash-bytes-null",
        "hash-string-null",
        "hmget-null-temp",
        "stralloc-null-arena",
        "stralloc-null-string",
        "strreset-null",
        "hmdel-out-of-range-mode",
    ] {
        let run = |kind: &str| {
            Command::new(&executable)
                .args(["--exact", "fatal_ffi_child", "--nocapture"])
                .env("DIFFERENTIAL_CHILD_LIBRARY", kind)
                .env("DIFFERENTIAL_CHILD_CASE", case)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert!(!c_status.success(), "C unexpectedly accepted {case}");
        assert!(!rust_status.success(), "Rust unexpectedly accepted {case}");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "fatal signal differs for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn differential_surface() {
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

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };

    unsafe {
        compare_hashes(c.f, rust.f);
        compare_arrays(c.f, rust.f);
        compare_map_error_sentinels(c.f, rust.f);
        compare_binary_maps(c.f, rust.f);
        compare_string_maps(c.f, rust.f);
        compare_arenas(c.f, rust.f);
        compare_utilities(c.f, rust.f);

        let c_empty_hash = (c.f.hash_bytes)(ptr::null_mut(), 0, usize::MAX);
        let rust_empty_hash = (rust.f.hash_bytes)(ptr::null_mut(), 0, usize::MAX);
        assert_eq!(c_empty_hash, rust_empty_hash);

        let mut large = vec![0u8; 1 << 20];
        Rng::new(0xfeed_face).fill(&mut large);
        assert_eq!(
            (c.f.hash_bytes)(large.as_mut_ptr().cast(), large.len(), 0x1234),
            (rust.f.hash_bytes)(large.as_mut_ptr().cast(), large.len(), 0x1234)
        );
    }

    compare_fatal_boundaries();
}
