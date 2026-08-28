use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::ptr;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type RandSeed = unsafe extern "C" fn(usize);
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
type StrDups = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    hash_bytes: HashBytes,
    hash_string: HashString,
    rand_seed: RandSeed,
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
    str_dups: StrDups,
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
        let result = Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
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
            str_dups: symbol!("str_dups", StrDups),
            _library: library,
        };
        result
    }
}

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
struct Entry {
    key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

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

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next() as u8;
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = manifest.join("../c_src/build/libharvest-work-rgFqeQ.so");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let profile_library = manifest.join(format!("target/{profile}/libstr_dups_lib.so"));
    let rust_library = if profile_library.is_file() {
        profile_library
    } else {
        manifest.join("target/release/libstr_dups_lib.so")
    };
    assert!(c_library.is_file(), "missing {}", c_library.display());
    assert!(rust_library.is_file(), "missing {}", rust_library.display());
    (c_library, rust_library)
}

unsafe fn array_header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe {
        array
            .cast::<u8>()
            .sub(size_of::<ArrayHeader>())
            .cast::<ArrayHeader>()
    }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe {
        map.cast::<u8>()
            .sub(element_size + size_of::<ArrayHeader>())
            .cast::<ArrayHeader>()
    }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_map(map, element_size), element_size) };
    }
}

fn compare_hashes(c: &Api, rust: &Api) {
    let seeds = [0, 1, 0x3141_5926, usize::MAX];
    let mut rng = Rng::new(0x6d5a_56da_2b19_7c31);

    for length in 0..=255 {
        for iteration in 0..32 {
            let mut bytes = vec![0u8; length];
            rng.fill(&mut bytes);
            if length >= 4 && iteration % 2 == 0 {
                bytes[3] |= 0x80;
            }
            for seed in seeds {
                let pointer = if bytes.is_empty() {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                let c_hash = unsafe { (c.hash_bytes)(pointer, bytes.len(), seed) };
                let rust_hash = unsafe { (rust.hash_bytes)(pointer, bytes.len(), seed) };
                assert_eq!(
                    c_hash, rust_hash,
                    "byte hash mismatch at length={length}, iteration={iteration}, seed={seed}"
                );
            }
        }
    }

    for length in 0..=128 {
        for iteration in 0..32 {
            let mut bytes = vec![b'a'; length];
            rng.fill(&mut bytes);
            for byte in &mut bytes {
                if *byte == 0 {
                    *byte = 0x80;
                }
            }
            bytes.push(0);
            for seed in seeds {
                let c_hash = unsafe { (c.hash_string)(bytes.as_mut_ptr().cast::<c_char>(), seed) };
                let rust_hash =
                    unsafe { (rust.hash_string)(bytes.as_mut_ptr().cast::<c_char>(), seed) };
                assert_eq!(
                    c_hash, rust_hash,
                    "string hash mismatch at length={length}, iteration={iteration}, seed={seed}"
                );
            }
        }
    }
}

fn compare_arrays(c: &Api, rust: &Api) {
    unsafe {
        assert!((c.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        assert!((rust.arrgrow)(ptr::null_mut(), 4, 0, 0).is_null());
        let c_zero = (c.arrgrow)(ptr::null_mut(), 0, 0, 1);
        let rust_zero = (rust.arrgrow)(ptr::null_mut(), 0, 0, 1);
        assert_eq!((*array_header(c_zero)).capacity, 4);
        assert_eq!(
            (*array_header(c_zero)).capacity,
            (*array_header(rust_zero)).capacity
        );
        (c.arrfree)(c_zero);
        (rust.arrfree)(rust_zero);
    }

    for element_size in [1, 4, 16] {
        for requested in 1..=3 {
            let c_array = unsafe { (c.arrgrow)(ptr::null_mut(), element_size, 0, requested) };
            let rust_array = unsafe { (rust.arrgrow)(ptr::null_mut(), element_size, 0, requested) };
            unsafe {
                assert_eq!((*array_header(c_array)).length, 0);
                assert_eq!((*array_header(c_array)).capacity, 4);
                assert_eq!(
                    (*array_header(c_array)).capacity,
                    (*array_header(rust_array)).capacity
                );
                assert_eq!(
                    (*array_header(c_array)).temp,
                    (*array_header(rust_array)).temp
                );
                assert_eq!(
                    (*array_header(c_array)).hash_table.is_null(),
                    (*array_header(rust_array)).hash_table.is_null()
                );
                (c.arrfree)(c_array);
                (rust.arrfree)(rust_array);
            }
        }
    }

    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
    for _ in 0..256 {
        let initial = 4 + (rng.next() as usize % 60);
        let element_size = [1, 4, 8, 16][rng.next() as usize % 4];
        let mut c_array = unsafe { (c.arrgrow)(ptr::null_mut(), element_size, initial, 0) };
        let mut rust_array = unsafe { (rust.arrgrow)(ptr::null_mut(), element_size, initial, 0) };
        let initial_capacity = unsafe { (*array_header(c_array)).capacity };
        assert_eq!(initial_capacity, unsafe {
            (*array_header(rust_array)).capacity
        });

        let bytes = initial_capacity * element_size;
        let mut contents = vec![0u8; bytes];
        rng.fill(&mut contents);
        unsafe {
            ptr::copy_nonoverlapping(contents.as_ptr(), c_array.cast(), bytes);
            ptr::copy_nonoverlapping(contents.as_ptr(), rust_array.cast(), bytes);
            (*array_header(c_array)).length = initial_capacity / 2;
            (*array_header(rust_array)).length = initial_capacity / 2;
        }

        let requests = [
            initial_capacity,
            initial_capacity + 1,
            initial_capacity * 2 + 3,
        ];
        for requested in requests {
            c_array = unsafe { (c.arrgrow)(c_array, element_size, 0, requested) };
            rust_array = unsafe { (rust.arrgrow)(rust_array, element_size, 0, requested) };
            let c_header = unsafe { *array_header(c_array) };
            let rust_header = unsafe { *array_header(rust_array) };
            assert_eq!(c_header.length, rust_header.length);
            assert_eq!(c_header.capacity, rust_header.capacity);
            assert_eq!(c_header.temp, rust_header.temp);
            let preserved = initial_capacity * element_size;
            let c_bytes = unsafe { std::slice::from_raw_parts(c_array.cast::<u8>(), preserved) };
            let rust_bytes =
                unsafe { std::slice::from_raw_parts(rust_array.cast::<u8>(), preserved) };
            assert_eq!(c_bytes, rust_bytes);
        }

        unsafe {
            (*array_header(c_array)).length = 1;
            (*array_header(rust_array)).length = 1;
        }
        let c_before = c_array;
        let rust_before = rust_array;
        c_array = unsafe { (c.arrgrow)(c_array, element_size, usize::MAX, 0) };
        rust_array = unsafe { (rust.arrgrow)(rust_array, element_size, usize::MAX, 0) };
        assert_eq!(c_array, c_before);
        assert_eq!(rust_array, rust_before);

        unsafe {
            (c.arrfree)(c_array);
            (rust.arrfree)(rust_array);
        }
    }
}

fn arena_state(arena: &StringArena) -> (bool, usize, u8, u8) {
    (
        arena.storage.is_null(),
        arena.remaining,
        arena.block,
        arena.mode,
    )
}

fn compare_arenas(c: &Api, rust: &Api) {
    let mut c_arena = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut rust_arena = c_arena;
    let mut rng = Rng::new(0xa076_1d64_78bd_642f);

    for iteration in 0..512 {
        let length = match iteration {
            0 => 0,
            1 => 511,
            2 => 512,
            3 => 513,
            _ => rng.next() as usize % 4096,
        };
        let mut bytes = vec![b'x'; length];
        for byte in &mut bytes {
            *byte = b'a' + (rng.next() % 26) as u8;
        }
        let string = CString::new(bytes).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, string.as_ptr().cast_mut()) };
        assert_eq!(
            unsafe { CStr::from_ptr(c_result).to_bytes_with_nul() },
            unsafe { CStr::from_ptr(rust_result).to_bytes_with_nul() },
            "arena string mismatch at iteration {iteration}"
        );
        assert_eq!(
            arena_state(&c_arena),
            arena_state(&rust_arena),
            "arena state mismatch at iteration {iteration}"
        );
    }

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
    assert_eq!(arena_state(&c_arena), (true, 0, 0, 0));
    assert_eq!(arena_state(&c_arena), arena_state(&rust_arena));

    let mut c_max = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 22,
        mode: 7,
    };
    let mut rust_max = c_max;
    let small = CString::new("maximum-block-boundary").unwrap();
    unsafe {
        let c_result = (c.stralloc)(&mut c_max, small.as_ptr().cast_mut());
        let rust_result = (rust.stralloc)(&mut rust_max, small.as_ptr().cast_mut());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes_with_nul(),
            CStr::from_ptr(rust_result).to_bytes_with_nul()
        );
    }
    assert_eq!(c_max.block, 22);
    assert_eq!(arena_state(&c_max), arena_state(&rust_max));
    unsafe {
        (c.strreset)(&mut c_max);
        (rust.strreset)(&mut rust_max);
    }

    let over_max = CString::new(vec![b'z'; (1 << 20) + 1]).unwrap();
    let mut c_oversized = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut rust_oversized = c_oversized;
    unsafe {
        let c_result = (c.stralloc)(&mut c_oversized, over_max.as_ptr().cast_mut());
        let rust_result = (rust.stralloc)(&mut rust_oversized, over_max.as_ptr().cast_mut());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes_with_nul(),
            CStr::from_ptr(rust_result).to_bytes_with_nul()
        );
        assert_eq!(arena_state(&c_oversized), arena_state(&rust_oversized));
        (c.strreset)(&mut c_oversized);
        (rust.strreset)(&mut rust_oversized);
    }

    let mut c_empty = StringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut rust_empty = c_empty;
    unsafe {
        (c.strreset)(&mut c_empty);
        (rust.strreset)(&mut rust_empty);
    }
    assert_eq!(arena_state(&c_empty), arena_state(&rust_empty));
}

unsafe fn map_entries(map: *mut c_void) -> Vec<Entry> {
    let header = unsafe { &*map_header(map, size_of::<Entry>()) };
    let length = header.length - 1;
    unsafe { std::slice::from_raw_parts(map.cast::<Entry>(), length) }.to_vec()
}

unsafe fn set_entry_value(map: *mut c_void, value: u64) {
    let index = unsafe { (*map_header(map, size_of::<Entry>())).temp as usize };
    unsafe { (*map.cast::<Entry>().add(index)).value = value };
}

fn compare_binary_maps(c: &Api, rust: &Api) {
    const ELEMENT_SIZE: usize = size_of::<Entry>();
    unsafe {
        (c.hmfree)(ptr::null_mut(), ELEMENT_SIZE);
        (rust.hmfree)(ptr::null_mut(), ELEMENT_SIZE);
    }

    let mut c_no_table = unsafe { (c.hmdefault)(ptr::null_mut(), ELEMENT_SIZE) };
    let mut rust_no_table = unsafe { (rust.hmdefault)(ptr::null_mut(), ELEMENT_SIZE) };
    unsafe {
        let c_before = c_no_table;
        let rust_before = rust_no_table;
        c_no_table = (c.hmdefault)(c_no_table, ELEMENT_SIZE);
        rust_no_table = (rust.hmdefault)(rust_no_table, ELEMENT_SIZE);
        assert_eq!(c_no_table, c_before);
        assert_eq!(rust_no_table, rust_before);
        let c_header = *map_header(c_no_table, ELEMENT_SIZE);
        let rust_header = *map_header(rust_no_table, ELEMENT_SIZE);
        assert_eq!(c_header.length, 1);
        assert_eq!(c_header.length, rust_header.length);
        assert_eq!(c_header.capacity, rust_header.capacity);
        let mut key = 1234u64;
        let mut c_temp = 99;
        let mut rust_temp = 99;
        c_no_table = (c.hmget_ts)(
            c_no_table,
            ELEMENT_SIZE,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut c_temp,
            0,
        );
        rust_no_table = (rust.hmget_ts)(
            rust_no_table,
            ELEMENT_SIZE,
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);
        assert_eq!(
            (c.hmdel)(
                c_no_table,
                ELEMENT_SIZE,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            ),
            c_no_table
        );
        assert_eq!(
            (rust.hmdel)(
                rust_no_table,
                ELEMENT_SIZE,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            ),
            rust_no_table
        );
        assert_eq!((*map_header(c_no_table, ELEMENT_SIZE)).temp, 0);
        assert_eq!(
            (*map_header(c_no_table, ELEMENT_SIZE)).temp,
            (*map_header(rust_no_table, ELEMENT_SIZE)).temp
        );
        free_map(c, c_no_table, ELEMENT_SIZE);
        free_map(rust, rust_no_table, ELEMENT_SIZE);
    }

    for key_width in [1, 4, 8, 16] {
        unsafe {
            (c.rand_seed)(0x1020_3040_5060_7080);
            (rust.rand_seed)(0x1020_3040_5060_7080);
        }
        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        let mut rng = Rng::new(0xd1b5_4a32_d192_ed03 ^ key_width as u64);
        for _ in 0..128 {
            let mut key = [0u8; 16];
            rng.fill(&mut key);
            c_map = unsafe { (c.hmput)(c_map, key.len(), key.as_mut_ptr().cast(), key_width, 0) };
            rust_map =
                unsafe { (rust.hmput)(rust_map, key.len(), key.as_mut_ptr().cast(), key_width, 0) };
            let c_header = unsafe { *map_header(c_map, key.len()) };
            let rust_header = unsafe { *map_header(rust_map, key.len()) };
            assert_eq!(c_header.length, rust_header.length);
            assert_eq!(c_header.capacity, rust_header.capacity);
            assert_eq!(c_header.temp, rust_header.temp);
            let index = c_header.temp as usize;
            let c_key = unsafe {
                std::slice::from_raw_parts(c_map.cast::<u8>().add(index * key.len()), key_width)
            };
            let rust_key = unsafe {
                std::slice::from_raw_parts(rust_map.cast::<u8>().add(index * key.len()), key_width)
            };
            assert_eq!(c_key, rust_key);
            assert_eq!(c_key, &key[..key_width]);
        }
        unsafe {
            free_map(c, c_map, 16);
            free_map(rust, rust_map, 16);
        }
    }

    unsafe {
        (c.rand_seed)(0xfeed_face_cafe_beef);
        (rust.rand_seed)(0xfeed_face_cafe_beef);
    }
    let mut c_map = ptr::null_mut();
    let mut rust_map = ptr::null_mut();
    let mut keys = Vec::new();
    let mut rng = Rng::new(0x243f_6a88_85a3_08d3);

    for index in 0..320u64 {
        let mut key = rng.next() ^ index.rotate_left(17);
        keys.push(key);
        c_map = unsafe {
            (c.hmput)(
                c_map,
                ELEMENT_SIZE,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            )
        };
        rust_map = unsafe {
            (rust.hmput)(
                rust_map,
                ELEMENT_SIZE,
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            )
        };
        unsafe {
            set_entry_value(c_map, index.wrapping_mul(17));
            set_entry_value(rust_map, index.wrapping_mul(17));
            assert_eq!(map_entries(c_map), map_entries(rust_map));
        }
    }

    for (index, key) in keys.iter_mut().enumerate() {
        let c_result = unsafe {
            (c.hmget)(
                c_map,
                ELEMENT_SIZE,
                (key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            )
        };
        let rust_result = unsafe {
            (rust.hmget)(
                rust_map,
                ELEMENT_SIZE,
                (key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            )
        };
        c_map = c_result;
        rust_map = rust_result;
        let c_temp = unsafe { (*map_header(c_map, ELEMENT_SIZE)).temp };
        let rust_temp = unsafe { (*map_header(rust_map, ELEMENT_SIZE)).temp };
        assert_eq!(c_temp, rust_temp);
        assert!(c_temp >= 0, "inserted key missing at index {index}");
        let c_entry = unsafe { *c_map.cast::<Entry>().offset(c_temp) };
        let rust_entry = unsafe { *rust_map.cast::<Entry>().offset(rust_temp) };
        assert_eq!(c_entry, rust_entry);
    }

    let mut absent = 0x0bad_f00d_c001_d00du64;
    c_map = unsafe {
        (c.hmdel)(
            c_map,
            ELEMENT_SIZE,
            (&mut absent as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    rust_map = unsafe {
        (rust.hmdel)(
            rust_map,
            ELEMENT_SIZE,
            (&mut absent as *mut u64).cast(),
            size_of::<u64>(),
            0,
            0,
        )
    };
    assert_eq!(unsafe { (*map_header(c_map, ELEMENT_SIZE)).temp }, 0);
    assert_eq!(unsafe { (*map_header(c_map, ELEMENT_SIZE)).temp }, unsafe {
        (*map_header(rust_map, ELEMENT_SIZE)).temp
    });
    assert_eq!(unsafe { map_entries(c_map) }, unsafe {
        map_entries(rust_map)
    });

    for index in (0..keys.len()).step_by(3) {
        let key = &mut keys[index];
        c_map = unsafe {
            (c.hmdel)(
                c_map,
                ELEMENT_SIZE,
                (key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        };
        rust_map = unsafe {
            (rust.hmdel)(
                rust_map,
                ELEMENT_SIZE,
                (key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        };
        unsafe {
            assert_eq!((*map_header(c_map, ELEMENT_SIZE)).temp, 1);
            assert_eq!(map_entries(c_map), map_entries(rust_map));
        }
    }

    for index in 0..keys.len() {
        if index % 3 != 0 {
            let key = &mut keys[index];
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    ELEMENT_SIZE,
                    (key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    ELEMENT_SIZE,
                    (key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                    0,
                )
            };
            unsafe { assert_eq!(map_entries(c_map), map_entries(rust_map)) };
        }
    }
    assert_eq!(unsafe { map_entries(c_map) }, Vec::<Entry>::new());

    let mut missing = 0xdead_beef_dead_beefu64;
    c_map = unsafe {
        (c.hmget)(
            c_map,
            ELEMENT_SIZE,
            (&mut missing as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    rust_map = unsafe {
        (rust.hmget)(
            rust_map,
            ELEMENT_SIZE,
            (&mut missing as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    assert_eq!(unsafe { (*map_header(c_map, ELEMENT_SIZE)).temp }, -1);
    assert_eq!(unsafe { (*map_header(c_map, ELEMENT_SIZE)).temp }, unsafe {
        (*map_header(rust_map, ELEMENT_SIZE)).temp
    });
    unsafe {
        free_map(c, c_map, ELEMENT_SIZE);
        free_map(rust, rust_map, ELEMENT_SIZE);
    }
}

unsafe fn string_entries(map: *mut c_void) -> Vec<(Vec<u8>, c_int)> {
    let header = unsafe { &*map_header(map, size_of::<StringEntry>()) };
    let length = header.length - 1;
    let entries = unsafe { std::slice::from_raw_parts(map.cast::<StringEntry>(), length) };
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

fn compare_string_maps(c: &Api, rust: &Api) {
    for mode in 1..=3 {
        unsafe {
            (c.rand_seed)(0x0123_4567_89ab_cdef);
            (rust.rand_seed)(0x0123_4567_89ab_cdef);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<StringEntry>(), mode) };
        let mut rust_map = unsafe { (rust.shmode)(size_of::<StringEntry>(), mode) };
        let mut strings = Vec::new();

        for index in 0..160 {
            strings
                .push(CString::new(format!("key_{index:03}_{}", (index * 7919) % 104729)).unwrap());
            let key = strings.last().unwrap().as_ptr().cast_mut();
            c_map = unsafe {
                (c.hmput)(
                    c_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
            rust_map = unsafe {
                (rust.hmput)(
                    rust_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
            unsafe {
                let c_header = *map_header(c_map, size_of::<StringEntry>());
                let rust_header = *map_header(rust_map, size_of::<StringEntry>());
                assert_eq!(c_header.temp, rust_header.temp);
                assert_eq!(c_header.length, rust_header.length);
                assert_eq!(c_header.capacity, rust_header.capacity);
                (*c_map.cast::<StringEntry>().offset(c_header.temp)).value = index * 13;
                (*rust_map.cast::<StringEntry>().offset(rust_header.temp)).value = index * 13;
                assert_eq!(string_entries(c_map), string_entries(rust_map));
            }

            if index % 11 == 0 {
                c_map = unsafe {
                    (c.hmput)(
                        c_map,
                        size_of::<StringEntry>(),
                        key.cast(),
                        size_of::<*mut c_char>(),
                        1,
                    )
                };
                rust_map = unsafe {
                    (rust.hmput)(
                        rust_map,
                        size_of::<StringEntry>(),
                        key.cast(),
                        size_of::<*mut c_char>(),
                        1,
                    )
                };
                unsafe {
                    let c_temp = (*map_header(c_map, size_of::<StringEntry>())).temp;
                    let rust_temp = (*map_header(rust_map, size_of::<StringEntry>())).temp;
                    assert_eq!(c_temp, rust_temp);
                    (*c_map.cast::<StringEntry>().offset(c_temp)).value = -index;
                    (*rust_map.cast::<StringEntry>().offset(rust_temp)).value = -index;
                    assert_eq!(string_entries(c_map), string_entries(rust_map));
                }
            }
        }

        for (index, string) in strings.iter().enumerate() {
            let key = string.as_ptr().cast_mut();
            c_map = unsafe {
                (c.hmget)(
                    c_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
            rust_map = unsafe {
                (rust.hmget)(
                    rust_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                )
            };
            let c_temp = unsafe { (*map_header(c_map, size_of::<StringEntry>())).temp };
            let rust_temp = unsafe { (*map_header(rust_map, size_of::<StringEntry>())).temp };
            assert_eq!(
                c_temp, rust_temp,
                "string lookup mismatch mode={mode}, index={index}"
            );
        }

        let missing = CString::new("not-present").unwrap();
        c_map = unsafe {
            (c.hmget)(
                c_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        rust_map = unsafe {
            (rust.hmget)(
                rust_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            )
        };
        assert_eq!(
            unsafe { (*map_header(c_map, size_of::<StringEntry>())).temp },
            -1
        );
        assert_eq!(
            unsafe { (*map_header(c_map, size_of::<StringEntry>())).temp },
            unsafe { (*map_header(rust_map, size_of::<StringEntry>())).temp }
        );

        for index in (0..strings.len()).step_by(4) {
            let key = strings[index].as_ptr().cast_mut();
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                )
            };
            unsafe {
                assert_eq!(
                    (*map_header(c_map, size_of::<StringEntry>())).temp,
                    (*map_header(rust_map, size_of::<StringEntry>())).temp
                );
                assert_eq!(string_entries(c_map), string_entries(rust_map));
            }
        }

        unsafe {
            free_map(c, c_map, size_of::<StringEntry>());
            free_map(rust, rust_map, size_of::<StringEntry>());
        }
    }

    unsafe {
        (c.rand_seed)(77);
        (rust.rand_seed)(77);
    }
    let mut c_binary = unsafe { (c.shmode)(size_of::<Entry>(), 0) };
    let mut rust_binary = unsafe { (rust.shmode)(size_of::<Entry>(), 0) };
    let mut key = 0x7788_99aa_bbcc_ddeeu64;
    c_binary = unsafe {
        (c.hmput)(
            c_binary,
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    rust_binary = unsafe {
        (rust.hmput)(
            rust_binary,
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    unsafe {
        set_entry_value(c_binary, 91);
        set_entry_value(rust_binary, 91);
        assert_eq!(map_entries(c_binary), map_entries(rust_binary));
        free_map(c, c_binary, size_of::<Entry>());
        free_map(rust, rust_binary, size_of::<Entry>());
    }
}

fn compare_invalid_modes(c: &Api, rust: &Api) {
    let mut c_zero_key =
        unsafe { (c.hmput)(ptr::null_mut(), size_of::<u64>(), ptr::null_mut(), 0, 0) };
    let mut rust_zero_key =
        unsafe { (rust.hmput)(ptr::null_mut(), size_of::<u64>(), ptr::null_mut(), 0, 0) };
    c_zero_key = unsafe { (c.hmget)(c_zero_key, size_of::<u64>(), ptr::null_mut(), 0, 0) };
    rust_zero_key = unsafe { (rust.hmget)(rust_zero_key, size_of::<u64>(), ptr::null_mut(), 0, 0) };
    assert_eq!(
        unsafe { (*map_header(c_zero_key, size_of::<u64>())).temp },
        unsafe { (*map_header(rust_zero_key, size_of::<u64>())).temp }
    );
    unsafe {
        free_map(c, c_zero_key, size_of::<u64>());
        free_map(rust, rust_zero_key, size_of::<u64>());
    }

    let mut binary_key = 0x1234_5678_9abc_def0u64;
    let mut c_binary = unsafe {
        (c.hmput)(
            ptr::null_mut(),
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        )
    };
    let mut rust_binary = unsafe {
        (rust.hmput)(
            ptr::null_mut(),
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        )
    };
    unsafe {
        set_entry_value(c_binary, 42);
        set_entry_value(rust_binary, 42);
        c_binary = (c.hmget)(
            c_binary,
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        );
        rust_binary = (rust.hmget)(
            rust_binary,
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            -1,
        );
        assert_eq!(map_entries(c_binary), map_entries(rust_binary));
        assert_eq!(
            (*map_header(c_binary, size_of::<Entry>())).temp,
            (*map_header(rust_binary, size_of::<Entry>())).temp
        );
        free_map(c, c_binary, size_of::<Entry>());
        free_map(rust, rust_binary, size_of::<Entry>());
    }

    let string = CString::new("out-of-range-string-mode").unwrap();
    let mut c_string = unsafe {
        (c.hmput)(
            ptr::null_mut(),
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            2,
        )
    };
    let mut rust_string = unsafe {
        (rust.hmput)(
            ptr::null_mut(),
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            2,
        )
    };
    unsafe {
        let c_temp = (*map_header(c_string, size_of::<StringEntry>())).temp;
        let rust_temp = (*map_header(rust_string, size_of::<StringEntry>())).temp;
        (*c_string.cast::<StringEntry>().offset(c_temp)).value = 17;
        (*rust_string.cast::<StringEntry>().offset(rust_temp)).value = 17;
        c_string = (c.hmget)(
            c_string,
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            2,
        );
        rust_string = (rust.hmget)(
            rust_string,
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            2,
        );
        assert_eq!(string_entries(c_string), string_entries(rust_string));
        free_map(c, c_string, size_of::<StringEntry>());
        free_map(rust, rust_string, size_of::<StringEntry>());
    }

    let mut c_max_mode = unsafe {
        (c.hmput)(
            ptr::null_mut(),
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            c_int::MAX,
        )
    };
    let mut rust_max_mode = unsafe {
        (rust.hmput)(
            ptr::null_mut(),
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            c_int::MAX,
        )
    };
    c_max_mode = unsafe {
        (c.hmget)(
            c_max_mode,
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            c_int::MAX,
        )
    };
    rust_max_mode = unsafe {
        (rust.hmget)(
            rust_max_mode,
            size_of::<StringEntry>(),
            string.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            c_int::MAX,
        )
    };
    assert_eq!(
        unsafe { (*map_header(c_max_mode, size_of::<StringEntry>())).temp },
        unsafe { (*map_header(rust_max_mode, size_of::<StringEntry>())).temp }
    );
    unsafe {
        free_map(c, c_max_mode, size_of::<StringEntry>());
        free_map(rust, rust_max_mode, size_of::<StringEntry>());
    }

    let mut c_truncated = unsafe { (c.shmode)(size_of::<Entry>(), 260) };
    let mut rust_truncated = unsafe { (rust.shmode)(size_of::<Entry>(), 260) };
    c_truncated = unsafe {
        (c.hmput)(
            c_truncated,
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    rust_truncated = unsafe {
        (rust.hmput)(
            rust_truncated,
            size_of::<Entry>(),
            (&mut binary_key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        )
    };
    unsafe {
        set_entry_value(c_truncated, 99);
        set_entry_value(rust_truncated, 99);
        assert_eq!(map_entries(c_truncated), map_entries(rust_truncated));
        free_map(c, c_truncated, size_of::<Entry>());
        free_map(rust, rust_truncated, size_of::<Entry>());
    }
}

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

fn capture_stdout(function: StrDups, value: c_int) -> Vec<u8> {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved = dup(1);
        assert!(saved >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        close(fds[1]);
        function(value);
        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved, 1), 1);
        close(saved);

        let mut output = Vec::new();
        let mut buffer = [0u8; 256];
        loop {
            let count = read(fds[0], buffer.as_mut_ptr().cast(), buffer.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        close(fds[0]);
        output
    }
}

fn compare_helpers(c: &Api, rust: &Api) {
    for number in [c_int::MIN, -10_000, -1, 0, 1, 42, 1_000_000, c_int::MAX] {
        let c_value = unsafe { CStr::from_ptr((c.strkey)(number)) }
            .to_bytes_with_nul()
            .to_vec();
        let rust_value = unsafe { CStr::from_ptr((rust.strkey)(number)) }
            .to_bytes_with_nul()
            .to_vec();
        assert_eq!(c_value, rust_value, "strkey mismatch for {number}");
    }

    for number in [-1, 0, 1, 2, 127] {
        let c_output = capture_stdout(c.str_dups, number);
        let rust_output = capture_stdout(rust.str_dups, number);
        assert_eq!(
            c_output, rust_output,
            "str_dups stdout mismatch for {number}"
        );
        assert_eq!(c_output, format!("a {number}\n").as_bytes());
    }
}

fn child_status(function: impl FnOnce()) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            function();
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status
    }
}

fn assert_same_fatal(c_call: impl FnOnce(), rust_call: impl FnOnce(), name: &str) {
    let c_status = child_status(c_call);
    let rust_status = child_status(rust_call);
    let c_signal = c_status & 0x7f;
    let rust_signal = rust_status & 0x7f;
    assert_ne!(c_signal, 0, "C unexpectedly survived {name}");
    assert_eq!(
        c_signal, rust_signal,
        "termination signal mismatch for {name}: C status={c_status}, Rust status={rust_status}"
    );
}

fn compare_error_boundaries(c: &Api, rust: &Api) {
    unsafe {
        assert_eq!(
            (c.hmdel)(
                ptr::null_mut(),
                size_of::<Entry>(),
                ptr::null_mut(),
                0,
                0,
                0,
            ),
            ptr::null_mut()
        );
        assert_eq!(
            (rust.hmdel)(
                ptr::null_mut(),
                size_of::<Entry>(),
                ptr::null_mut(),
                0,
                0,
                0,
            ),
            ptr::null_mut()
        );

        let mut c_temp = 7;
        let mut rust_temp = 7;
        let c_map = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<Entry>(),
            ptr::null_mut(),
            0,
            &mut c_temp,
            0,
        );
        let rust_map = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<Entry>(),
            ptr::null_mut(),
            0,
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);
        free_map(c, c_map, size_of::<Entry>());
        free_map(rust, rust_map, size_of::<Entry>());
    }

    assert_same_fatal(
        || unsafe {
            (c.hash_bytes)(ptr::null_mut(), 1, 0);
        },
        || unsafe {
            (rust.hash_bytes)(ptr::null_mut(), 1, 0);
        },
        "hash_bytes(NULL, 1)",
    );
    assert_same_fatal(
        || unsafe {
            (c.hash_bytes)(ptr::null_mut(), usize::MAX, 0);
        },
        || unsafe {
            (rust.hash_bytes)(ptr::null_mut(), usize::MAX, 0);
        },
        "hash_bytes(NULL, SIZE_MAX)",
    );
    assert_same_fatal(
        || unsafe {
            (c.hash_string)(ptr::null_mut(), 0);
        },
        || unsafe {
            (rust.hash_string)(ptr::null_mut(), 0);
        },
        "hash_string(NULL)",
    );
    assert_same_fatal(
        || unsafe {
            (c.hmget_ts)(
                ptr::null_mut(),
                size_of::<Entry>(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
            );
        },
        || unsafe {
            (rust.hmget_ts)(
                ptr::null_mut(),
                size_of::<Entry>(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
            );
        },
        "hmget_key_ts(NULL temp)",
    );
    let string = CString::new("x").unwrap();
    assert_same_fatal(
        || unsafe {
            (c.stralloc)(ptr::null_mut(), string.as_ptr().cast_mut());
        },
        || unsafe {
            (rust.stralloc)(ptr::null_mut(), string.as_ptr().cast_mut());
        },
        "stralloc(NULL arena)",
    );
    assert_same_fatal(
        || unsafe {
            let mut arena = StringArena {
                storage: ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            (c.stralloc)(&mut arena, ptr::null_mut());
        },
        || unsafe {
            let mut arena = StringArena {
                storage: ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            (rust.stralloc)(&mut arena, ptr::null_mut());
        },
        "stralloc(NULL string)",
    );
    assert_same_fatal(
        || unsafe {
            (c.strreset)(ptr::null_mut());
        },
        || unsafe {
            (rust.strreset)(ptr::null_mut());
        },
        "strreset(NULL)",
    );
    assert_same_fatal(
        || unsafe {
            (c.arrfree)(ptr::null_mut());
        },
        || unsafe {
            (rust.arrfree)(ptr::null_mut());
        },
        "arrfree(NULL)",
    );
}

#[test]
fn ffi_surface_matches_c() {
    let (c_path, rust_path) = library_paths();
    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };

    compare_hashes(&c, &rust);
    compare_arrays(&c, &rust);
    compare_arenas(&c, &rust);
    compare_binary_maps(&c, &rust);
    compare_string_maps(&c, &rust);
    compare_invalid_modes(&c, &rust);
    compare_helpers(&c, &rust);
    compare_error_boundaries(&c, &rust);
}
