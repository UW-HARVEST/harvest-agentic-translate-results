use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
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
type HmGeti = unsafe extern "C" fn(c_int);

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
    hm_geti: HmGeti,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }.unwrap_or_else(
                    |error| panic!("missing {} in {}: {error}", $name, path.display()),
                )
            };
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
            hm_geti: symbol!("hm_geti", HmGeti),
            _library: library,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-XMiVLi.so"),
        crate_dir.join("target/release/libhm_geti_lib.so"),
    )
}

unsafe fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing release Rust library: {}",
        rust_path.display()
    );
    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
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
    fn zeroed() -> Self {
        Self {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }

    fn state(&self) -> (bool, usize, u8, u8) {
        (
            !self.storage.is_null(),
            self.remaining,
            self.block,
            self.mode,
        )
    }
}

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

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { (array as *mut u8).sub(size_of::<ArrayHeader>()) as *mut ArrayHeader }
}

unsafe fn raw_map(map: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(elem_size) as *mut c_void }
}

unsafe fn map_header(map: *mut c_void, elem_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, elem_size)) }
}

unsafe fn free_map(api: &Api, map: *mut c_void, elem_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_map(map, elem_size), elem_size) };
    }
}

#[test]
fn hashes_match_for_all_tail_widths_and_random_values() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let mut rng = Rng::new(0x8e91_54ca_f031_77d2);

    for len in 0..=71usize {
        for _ in 0..64 {
            let mut bytes = vec![0u8; len];
            rng.fill(&mut bytes);
            let pointer = if len == 0 {
                null_mut()
            } else {
                bytes.as_mut_ptr() as *mut c_void
            };
            let seed = rng.next_u64() as usize;
            assert_eq!(
                unsafe { (c.hash_bytes)(pointer, len, seed) },
                unsafe { (rust.hash_bytes)(pointer, len, seed) },
                "byte hash mismatch for len={len}, seed={seed:#x}"
            );
        }
    }

    for len in [0usize, 1, 2, 7, 8, 31, 255] {
        for _ in 0..64 {
            let mut bytes = vec![b'a'; len + 1];
            for byte in &mut bytes[..len] {
                let candidate = rng.next_u64() as u8;
                *byte = if candidate == 0 { 0x80 } else { candidate };
            }
            bytes[len] = 0;
            let seed = rng.next_u64() as usize;
            assert_eq!(
                unsafe { (c.hash_string)(bytes.as_mut_ptr() as *mut c_char, seed) },
                unsafe { (rust.hash_string)(bytes.as_mut_ptr() as *mut c_char, seed) },
                "string hash mismatch for len={len}, seed={seed:#x}"
            );
        }
    }
}

#[test]
fn array_growth_matches_capacity_and_preserves_bytes() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let mut rng = Rng::new(0xba5e_6a11_0c00_ffee);

    for elem_size in [1usize, 2, 4, 8, 13, 32] {
        for &(add_len, min_cap) in &[(0, 0), (0, 1), (0, 3), (5, 0), (3, 17)] {
            let c_array = unsafe { (c.arrgrow)(null_mut(), elem_size, add_len, min_cap) };
            let r_array = unsafe { (rust.arrgrow)(null_mut(), elem_size, add_len, min_cap) };
            assert_eq!(c_array.is_null(), r_array.is_null());
            if c_array.is_null() {
                continue;
            }
            let c_header = unsafe { *header(c_array) };
            let r_header = unsafe { *header(r_array) };
            assert_eq!(c_header.length, r_header.length);
            assert_eq!(c_header.capacity, r_header.capacity);
            assert_eq!(c_header.temp, r_header.temp);
            assert_eq!(c_header.hash_table.is_null(), r_header.hash_table.is_null());
            unsafe {
                (c.arrfree)(c_array);
                (rust.arrfree)(r_array);
            }
        }

        for _ in 0..64 {
            let initial_cap = 4 + (rng.next_u64() as usize % 60);
            let mut c_array = unsafe { (c.arrgrow)(null_mut(), elem_size, 0, initial_cap) };
            let mut r_array = unsafe { (rust.arrgrow)(null_mut(), elem_size, 0, initial_cap) };
            let actual_cap = unsafe { (*header(c_array)).capacity };
            assert_eq!(actual_cap, unsafe { (*header(r_array)).capacity });
            let length = rng.next_u64() as usize % (actual_cap + 1);
            unsafe {
                (*header(c_array)).length = length;
                (*header(r_array)).length = length;
            }
            let mut contents = vec![0u8; length * elem_size];
            rng.fill(&mut contents);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    contents.as_ptr(),
                    c_array as *mut u8,
                    contents.len(),
                );
                std::ptr::copy_nonoverlapping(
                    contents.as_ptr(),
                    r_array as *mut u8,
                    contents.len(),
                );
            }

            let request = match rng.next_u64() % 3 {
                0 => actual_cap,
                1 => actual_cap + 1,
                _ => actual_cap.saturating_mul(3).saturating_add(7),
            };
            let old_c = c_array;
            let old_r = r_array;
            c_array = unsafe { (c.arrgrow)(c_array, elem_size, 0, request) };
            r_array = unsafe { (rust.arrgrow)(r_array, elem_size, 0, request) };
            assert_eq!(unsafe { (*header(c_array)).capacity }, unsafe {
                (*header(r_array)).capacity
            });
            assert_eq!(unsafe { (*header(c_array)).length }, unsafe {
                (*header(r_array)).length
            });
            if request <= actual_cap {
                assert_eq!(old_c, c_array);
                assert_eq!(old_r, r_array);
            }
            let c_bytes =
                unsafe { std::slice::from_raw_parts(c_array as *const u8, contents.len()) };
            let r_bytes =
                unsafe { std::slice::from_raw_parts(r_array as *const u8, contents.len()) };
            assert_eq!(c_bytes, contents);
            assert_eq!(r_bytes, contents);
            unsafe {
                (c.arrfree)(c_array);
                (rust.arrfree)(r_array);
            }
        }
    }
}

#[test]
fn string_arena_matches_normal_dedicated_and_max_block_paths() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let mut c_arena = StringArena::zeroed();
    let mut r_arena = StringArena::zeroed();

    for len in [0usize, 1, 31, 255, 300, 600, 4096, 1_048_577] {
        let input = CString::new(vec![b'x'; len]).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, input.as_ptr() as *mut c_char) };
        let r_result = unsafe { (rust.stralloc)(&mut r_arena, input.as_ptr() as *mut c_char) };
        assert_eq!(unsafe { CStr::from_ptr(c_result).to_bytes() }, unsafe {
            CStr::from_ptr(r_result).to_bytes()
        });
        assert_eq!(c_arena.state(), r_arena.state());
    }

    for _ in 0..28 {
        let block_size = (512usize << ((c_arena.block as usize) >> 1)).min(1 << 20);
        let input = CString::new(vec![b'z'; block_size - 1]).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, input.as_ptr() as *mut c_char) };
        let r_result = unsafe { (rust.stralloc)(&mut r_arena, input.as_ptr() as *mut c_char) };
        assert_eq!(unsafe { CStr::from_ptr(c_result).to_bytes() }, unsafe {
            CStr::from_ptr(r_result).to_bytes()
        });
        assert_eq!(c_arena.state(), r_arena.state());
    }
    assert!(
        c_arena.block >= 22,
        "arena did not reach maximum block size"
    );

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
    }
    assert_eq!(c_arena.state(), (false, 0, 0, 0));
    assert_eq!(r_arena.state(), c_arena.state());
    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
    }
    assert_eq!(r_arena.state(), c_arena.state());
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BinEntry {
    key: u64,
    value: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct BinSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<BinEntry>,
}

unsafe fn bin_snapshot(map: *mut BinEntry) -> BinSnapshot {
    let h = unsafe { &*map_header(map as *mut c_void, size_of::<BinEntry>()) };
    BinSnapshot {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        entries: unsafe { std::slice::from_raw_parts(map, h.length.saturating_sub(1)).to_vec() },
    }
}

unsafe fn bin_put(api: &Api, map: &mut *mut BinEntry, key: u64, value: i64) {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmput)(
            *map as *mut c_void,
            size_of::<BinEntry>(),
            &mut key_copy as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
        ) as *mut BinEntry
    };
    let index = unsafe { (*map_header(*map as *mut c_void, size_of::<BinEntry>())).temp };
    unsafe { (*map.offset(index)).value = value };
}

unsafe fn bin_get(api: &Api, map: &mut *mut BinEntry, key: u64, thread_safe: bool) -> isize {
    let mut key_copy = key;
    if thread_safe {
        let mut temp = isize::MIN;
        *map = unsafe {
            (api.hmget_ts)(
                *map as *mut c_void,
                size_of::<BinEntry>(),
                &mut key_copy as *mut u64 as *mut c_void,
                size_of::<u64>(),
                &mut temp,
                0,
            ) as *mut BinEntry
        };
        temp
    } else {
        *map = unsafe {
            (api.hmget)(
                *map as *mut c_void,
                size_of::<BinEntry>(),
                &mut key_copy as *mut u64 as *mut c_void,
                size_of::<u64>(),
                0,
            ) as *mut BinEntry
        };
        unsafe { (*map_header(*map as *mut c_void, size_of::<BinEntry>())).temp }
    }
}

unsafe fn bin_del(api: &Api, map: &mut *mut BinEntry, key: u64) -> isize {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmdel)(
            *map as *mut c_void,
            size_of::<BinEntry>(),
            &mut key_copy as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
            0,
        ) as *mut BinEntry
    };
    if map.is_null() {
        0
    } else {
        unsafe { (*map_header(*map as *mut c_void, size_of::<BinEntry>())).temp }
    }
}

#[test]
fn binary_hash_maps_match_randomized_end_to_end_sequences() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let seeds = [0usize, 1, usize::MAX, 0x3141_5926];

    for seed in seeds {
        unsafe {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
        }
        let mut c_map: *mut BinEntry = null_mut();
        let mut r_map: *mut BinEntry = null_mut();

        c_map =
            unsafe { (c.hmdefault)(c_map as *mut c_void, size_of::<BinEntry>()) as *mut BinEntry };
        r_map = unsafe {
            (rust.hmdefault)(r_map as *mut c_void, size_of::<BinEntry>()) as *mut BinEntry
        };
        unsafe {
            (*c_map.offset(-1)).value = -991;
            (*r_map.offset(-1)).value = -991;
        }
        assert_eq!(unsafe { bin_snapshot(c_map) }, unsafe {
            bin_snapshot(r_map)
        });
        let old_c = c_map;
        let old_r = r_map;
        c_map =
            unsafe { (c.hmdefault)(c_map as *mut c_void, size_of::<BinEntry>()) as *mut BinEntry };
        r_map = unsafe {
            (rust.hmdefault)(r_map as *mut c_void, size_of::<BinEntry>()) as *mut BinEntry
        };
        assert_eq!(old_c, c_map);
        assert_eq!(old_r, r_map);

        let mut rng = Rng::new(seed as u64 ^ 0xf00d_cafe_1234_5678);
        for iteration in 0..600 {
            let key = rng.next_u64() % 173;
            match rng.next_u64() % 4 {
                0 | 1 => {
                    let value = rng.next_u64() as i64;
                    unsafe {
                        bin_put(&c, &mut c_map, key, value);
                        bin_put(&rust, &mut r_map, key, value);
                    }
                }
                2 => {
                    let c_index = unsafe { bin_get(&c, &mut c_map, key, iteration % 2 == 0) };
                    let r_index = unsafe { bin_get(&rust, &mut r_map, key, iteration % 2 == 0) };
                    assert_eq!(c_index, r_index);
                }
                _ => {
                    assert_eq!(unsafe { bin_del(&c, &mut c_map, key) }, unsafe {
                        bin_del(&rust, &mut r_map, key)
                    });
                }
            }
            assert_eq!(
                unsafe { bin_snapshot(c_map) },
                unsafe { bin_snapshot(r_map) },
                "binary map diverged at seed={seed:#x}, iteration={iteration}"
            );
        }
        unsafe {
            free_map(&c, c_map as *mut c_void, size_of::<BinEntry>());
            free_map(&rust, r_map as *mut c_void, size_of::<BinEntry>());
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WideEntry {
    key: [u8; 13],
    value: u64,
}

#[test]
fn binary_key_widths_and_empty_key_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let mut rng = Rng::new(0x44ef_7712_cdb9_021a);

    for key_size in [0usize, 1, 4, 8, 13] {
        let mut c_map: *mut WideEntry = null_mut();
        let mut r_map: *mut WideEntry = null_mut();
        for iteration in 0..96 {
            let mut key = [0u8; 13];
            rng.fill(&mut key);
            let c_key = key;
            let r_key = key;
            c_map = unsafe {
                (c.hmput)(
                    c_map as *mut c_void,
                    size_of::<WideEntry>(),
                    c_key.as_ptr() as *mut c_void,
                    key_size,
                    0,
                ) as *mut WideEntry
            };
            r_map = unsafe {
                (rust.hmput)(
                    r_map as *mut c_void,
                    size_of::<WideEntry>(),
                    r_key.as_ptr() as *mut c_void,
                    key_size,
                    0,
                ) as *mut WideEntry
            };
            let c_temp =
                unsafe { (*map_header(c_map as *mut c_void, size_of::<WideEntry>())).temp };
            let r_temp =
                unsafe { (*map_header(r_map as *mut c_void, size_of::<WideEntry>())).temp };
            assert_eq!(c_temp, r_temp);
            unsafe {
                (*c_map.offset(c_temp)).value = iteration;
                (*r_map.offset(r_temp)).value = iteration;
            }

            let mut c_found = isize::MIN;
            let mut r_found = isize::MIN;
            unsafe {
                c_map = (c.hmget_ts)(
                    c_map as *mut c_void,
                    size_of::<WideEntry>(),
                    c_key.as_ptr() as *mut c_void,
                    key_size,
                    &mut c_found,
                    0,
                ) as *mut WideEntry;
                r_map = (rust.hmget_ts)(
                    r_map as *mut c_void,
                    size_of::<WideEntry>(),
                    r_key.as_ptr() as *mut c_void,
                    key_size,
                    &mut r_found,
                    0,
                ) as *mut WideEntry;
            }
            assert_eq!(c_found, r_found);
            let c_h = unsafe { &*map_header(c_map as *mut c_void, size_of::<WideEntry>()) };
            let r_h = unsafe { &*map_header(r_map as *mut c_void, size_of::<WideEntry>()) };
            assert_eq!(
                (c_h.length, c_h.capacity, c_h.temp),
                (r_h.length, r_h.capacity, r_h.temp)
            );
        }
        unsafe {
            free_map(&c, c_map as *mut c_void, size_of::<WideEntry>());
            free_map(&rust, r_map as *mut c_void, size_of::<WideEntry>());
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct StringSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<(Vec<u8>, i64)>,
}

unsafe fn string_snapshot(map: *mut StringEntry) -> StringSnapshot {
    let h = unsafe { &*map_header(map as *mut c_void, size_of::<StringEntry>()) };
    let mut entries = Vec::new();
    for index in 0..h.length.saturating_sub(1) {
        let entry = unsafe { *map.add(index) };
        entries.push((
            unsafe { CStr::from_ptr(entry.key).to_bytes().to_vec() },
            entry.value,
        ));
    }
    StringSnapshot {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        entries,
    }
}

unsafe fn string_put(api: &Api, map: &mut *mut StringEntry, key: &CStr, value: i64) {
    *map = unsafe {
        (api.hmput)(
            *map as *mut c_void,
            size_of::<StringEntry>(),
            key.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            1,
        ) as *mut StringEntry
    };
    let index = unsafe { (*map_header(*map as *mut c_void, size_of::<StringEntry>())).temp };
    unsafe { (*map.offset(index)).value = value };
}

unsafe fn string_get(api: &Api, map: &mut *mut StringEntry, key: &CStr) -> isize {
    let mut temp = isize::MIN;
    *map = unsafe {
        (api.hmget_ts)(
            *map as *mut c_void,
            size_of::<StringEntry>(),
            key.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            &mut temp,
            1,
        ) as *mut StringEntry
    };
    temp
}

unsafe fn string_del(api: &Api, map: &mut *mut StringEntry, key: &CStr) -> isize {
    *map = unsafe {
        (api.hmdel)(
            *map as *mut c_void,
            size_of::<StringEntry>(),
            key.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            0,
            1,
        ) as *mut StringEntry
    };
    unsafe { (*map_header(*map as *mut c_void, size_of::<StringEntry>())).temp }
}

#[test]
fn string_maps_match_borrowed_strdup_and_arena_modes() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };

    for mode in [1, 2, 3] {
        unsafe {
            (c.rand_seed)(0x1234_5678);
            (rust.rand_seed)(0x1234_5678);
        }
        let mut c_map = unsafe { (c.shmode)(size_of::<StringEntry>(), mode) as *mut StringEntry };
        let mut r_map =
            unsafe { (rust.shmode)(size_of::<StringEntry>(), mode) as *mut StringEntry };
        let mut retained = Vec::new();

        for iteration in 0..180 {
            let bytes = match iteration % 4 {
                0 => Vec::new(),
                1 => vec![b'a' + (iteration % 26) as u8],
                2 => format!("key_{iteration:04}_{}", "x".repeat(iteration % 71)).into_bytes(),
                _ => vec![0x80 + (iteration % 100) as u8, b'_', b'k'],
            };
            let input = CString::new(bytes).unwrap();
            unsafe {
                string_put(&c, &mut c_map, &input, iteration as i64 * 17);
                string_put(&rust, &mut r_map, &input, iteration as i64 * 17);
            }
            if mode == 1 {
                retained.push(input);
            }
            assert_eq!(
                unsafe { string_snapshot(c_map) },
                unsafe { string_snapshot(r_map) },
                "string map diverged in mode={mode}, insertion={iteration}"
            );
        }

        for iteration in 0..220 {
            let input =
                CString::new(format!("key_{iteration:04}_{}", "x".repeat(iteration % 71))).unwrap();
            assert_eq!(unsafe { string_get(&c, &mut c_map, &input) }, unsafe {
                string_get(&rust, &mut r_map, &input)
            });
            if iteration % 3 == 0 {
                assert_eq!(unsafe { string_del(&c, &mut c_map, &input) }, unsafe {
                    string_del(&rust, &mut r_map, &input)
                });
            }
        }
        assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
            string_snapshot(r_map)
        });
        unsafe {
            free_map(&c, c_map as *mut c_void, size_of::<StringEntry>());
            free_map(&rust, r_map as *mut c_void, size_of::<StringEntry>());
        }
        drop(retained);
    }
}

#[test]
fn rejection_sentinels_and_mode_boundaries_match() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let mut key = 77u64;

    let c_deleted = unsafe {
        (c.hmdel)(
            null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
            0,
        )
    };
    let r_deleted = unsafe {
        (rust.hmdel)(
            null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            0,
            0,
        )
    };
    assert!(c_deleted.is_null() && r_deleted.is_null());

    let mut c_temp = isize::MIN;
    let mut r_temp = isize::MIN;
    let mut c_map = unsafe {
        (c.hmget_ts)(
            null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut c_temp,
            0,
        ) as *mut BinEntry
    };
    let mut r_map = unsafe {
        (rust.hmget_ts)(
            null_mut(),
            size_of::<BinEntry>(),
            &mut key as *mut u64 as *mut c_void,
            size_of::<u64>(),
            &mut r_temp,
            0,
        ) as *mut BinEntry
    };
    assert_eq!(c_temp, -1);
    assert_eq!(r_temp, c_temp);
    assert_eq!(unsafe { bin_snapshot(c_map) }, unsafe {
        bin_snapshot(r_map)
    });

    let old_c = c_map;
    let old_r = r_map;
    assert_eq!(unsafe { bin_del(&c, &mut c_map, key) }, unsafe {
        bin_del(&rust, &mut r_map, key)
    });
    assert_eq!(c_map, old_c);
    assert_eq!(r_map, old_r);
    assert_eq!(unsafe { bin_snapshot(c_map) }, unsafe {
        bin_snapshot(r_map)
    });
    unsafe {
        free_map(&c, c_map as *mut c_void, size_of::<BinEntry>());
        free_map(&rust, r_map as *mut c_void, size_of::<BinEntry>());
    }

    for mode in [-1, 0, 1, 2, 3, 4, c_int::MAX] {
        let mut c_mode_map = unsafe { (c.shmode)(size_of::<WideEntry>(), mode) as *mut WideEntry };
        let mut r_mode_map =
            unsafe { (rust.shmode)(size_of::<WideEntry>(), mode) as *mut WideEntry };
        let mut key_bytes = [0x31u8; 13];
        key_bytes[12] = 0;
        c_mode_map = unsafe {
            (c.hmput)(
                c_mode_map as *mut c_void,
                size_of::<WideEntry>(),
                key_bytes.as_ptr() as *mut c_void,
                13,
                if mode >= 1 { 1 } else { 0 },
            ) as *mut WideEntry
        };
        r_mode_map = unsafe {
            (rust.hmput)(
                r_mode_map as *mut c_void,
                size_of::<WideEntry>(),
                key_bytes.as_ptr() as *mut c_void,
                13,
                if mode >= 1 { 1 } else { 0 },
            ) as *mut WideEntry
        };
        let c_h = unsafe { &*map_header(c_mode_map as *mut c_void, size_of::<WideEntry>()) };
        let r_h = unsafe { &*map_header(r_mode_map as *mut c_void, size_of::<WideEntry>()) };
        assert_eq!(
            (c_h.length, c_h.capacity, c_h.temp),
            (r_h.length, r_h.capacity, r_h.temp)
        );
        unsafe {
            free_map(&c, c_mode_map as *mut c_void, size_of::<WideEntry>());
            free_map(&rust, r_mode_map as *mut c_void, size_of::<WideEntry>());
        }
    }
}

#[test]
fn convenience_exports_match_boundaries_and_stress_paths() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = unsafe { apis() };
    for number in [c_int::MIN, -1_000_000, -1, 0, 1, 9, 10, 999_999, c_int::MAX] {
        let c_value = unsafe { CStr::from_ptr((c.strkey)(number)).to_bytes().to_vec() };
        let r_value = unsafe { CStr::from_ptr((rust.strkey)(number)).to_bytes().to_vec() };
        assert_eq!(c_value, r_value, "strkey mismatch for {number}");
    }

    for number in [-100, -1, 0, 1, 2, 7, 32, 257, 4096] {
        unsafe {
            (c.hm_geti)(number);
            (rust.hm_geti)(number);
        }
    }

    unsafe {
        (c.hmfree)(null_mut(), size_of::<BinEntry>());
        (rust.hmfree)(null_mut(), size_of::<BinEntry>());
    }
}
