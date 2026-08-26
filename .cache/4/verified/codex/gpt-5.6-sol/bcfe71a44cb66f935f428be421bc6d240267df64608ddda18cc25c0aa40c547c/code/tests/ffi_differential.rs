use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
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
type IntPut = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_string: HashString,
    hash_bytes: HashBytes,
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
    intput: IntPut,
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
            hash_string: symbol!("stbds_hash_string", HashString),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
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
            intput: symbol!("intput", IntPut),
            _library: library,
        }
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
#[derive(Clone, Copy, Debug)]
struct StringArena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: i64,
}

const BINARY_ENTRY_SIZE: usize = 24;
const BINARY_VALUE_OFFSET: usize = 16;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_DYLIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target/debug/libintput_lib.so"))
}

unsafe fn load_pair() -> (Api, Api) {
    (unsafe { Api::load(&c_library_path()) }, unsafe {
        Api::load(&rust_library_path())
    })
}

fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
}

unsafe fn map_raw(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> ArrayHeader {
    unsafe { *header(map_raw(map, element_size)) }
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(map_raw(map, element_size), element_size) };
    }
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

fn assert_status_equivalent(c: ExitStatus, rust: ExitStatus) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(c.signal(), rust.signal(), "C={c:?}, Rust={rust:?}");
    }
    assert_eq!(c.success(), rust.success(), "C={c:?}, Rust={rust:?}");
}

#[test]
fn all_exported_symbols_load_from_both_shared_objects() {
    let _guard = serial();
    let (_c, _rust) = unsafe { load_pair() };
}

#[test]
fn hashes_match_for_all_tail_shapes_and_random_values() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
    let seeds = [0, 1, 0x3141_5926, usize::MAX, 0xfeed_face_dead_beef];

    for &seed in &seeds {
        let c_hash = unsafe { (c.hash_bytes)(ptr::null_mut(), 0, seed) };
        let rust_hash = unsafe { (rust.hash_bytes)(ptr::null_mut(), 0, seed) };
        assert_eq!(c_hash, rust_hash, "null/empty bytes, seed={seed:#x}");

        for length in 0..=79 {
            for iteration in 0..64 {
                let mut bytes = vec![0; length];
                rng.fill(&mut bytes);
                if iteration % 3 == 0 {
                    for byte in &mut bytes {
                        *byte |= 0x80;
                    }
                }
                let data = if bytes.is_empty() {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                let c_hash = unsafe { (c.hash_bytes)(data, length, seed) };
                let rust_hash = unsafe { (rust.hash_bytes)(data, length, seed) };
                assert_eq!(
                    c_hash, rust_hash,
                    "byte hash length={length}, iteration={iteration}, seed={seed:#x}"
                );
            }
        }

        for length in [0, 1, 2, 7, 8, 31, 255] {
            for iteration in 0..64 {
                let mut bytes = vec![b'a'; length + 1];
                for byte in &mut bytes[..length] {
                    let candidate = rng.next_u64() as u8;
                    *byte = if candidate == 0 { 0x80 } else { candidate };
                }
                bytes[length] = 0;
                let pointer = bytes.as_mut_ptr().cast::<c_char>();
                let c_hash = unsafe { (c.hash_string)(pointer, seed) };
                let rust_hash = unsafe { (rust.hash_string)(pointer, seed) };
                assert_eq!(
                    c_hash, rust_hash,
                    "string hash length={length}, iteration={iteration}, seed={seed:#x}"
                );
            }
        }
    }
}

#[test]
fn array_growth_capacity_and_contents_match() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        let mut c_array: *mut c_void = ptr::null_mut();
        let mut rust_array: *mut c_void = ptr::null_mut();
        let requests = [
            (0, 0),
            (1, 0),
            (0, 3),
            (0, 4),
            (1, 0),
            (0, 7),
            (9, 0),
            (0, 40),
            (80, 0),
        ];

        for (step, &(add_length, minimum_capacity)) in requests.iter().enumerate() {
            let old_c = c_array;
            let old_rust = rust_array;
            let old_capacity = if c_array.is_null() {
                0
            } else {
                (*header(c_array)).capacity
            };
            let old_length = if c_array.is_null() {
                0
            } else {
                (*header(c_array)).length
            };
            c_array = (c.arrgrow)(c_array, size_of::<u64>(), add_length, minimum_capacity);
            rust_array = (rust.arrgrow)(rust_array, size_of::<u64>(), add_length, minimum_capacity);
            assert_eq!(c_array.is_null(), rust_array.is_null(), "step {step}");
            if c_array.is_null() {
                continue;
            }
            let c_header = *header(c_array);
            let rust_header = *header(rust_array);
            assert_eq!(c_header.length, rust_header.length, "step {step}");
            assert_eq!(c_header.capacity, rust_header.capacity, "step {step}");
            assert_eq!(c_header.temp, rust_header.temp, "step {step}");
            assert_eq!(
                c_header.hash_table.is_null(),
                rust_header.hash_table.is_null(),
                "step {step}"
            );
            let required = old_length.wrapping_add(add_length).max(minimum_capacity);
            if required <= old_capacity && !old_c.is_null() {
                assert_eq!(c_array, old_c, "C no-growth step {step}");
                assert_eq!(rust_array, old_rust, "Rust no-growth step {step}");
            }

            let initialized = c_header.length.min(c_header.capacity);
            for index in 0..initialized {
                assert_eq!(
                    *c_array.cast::<u64>().add(index),
                    *rust_array.cast::<u64>().add(index),
                    "step={step}, index={index}"
                );
            }
            if add_length > 0 {
                let new_length = c_header.length.saturating_add(add_length);
                if new_length <= c_header.capacity {
                    for index in c_header.length..new_length {
                        *c_array.cast::<u64>().add(index) = 0xa5a5_0000_0000_0000 | index as u64;
                        *rust_array.cast::<u64>().add(index) = 0xa5a5_0000_0000_0000 | index as u64;
                    }
                    (*header(c_array)).length = new_length;
                    (*header(rust_array)).length = new_length;
                }
            }
        }

        (c.arrfree)(c_array);
        (rust.arrfree)(rust_array);
    }
}

unsafe fn set_binary_entry(map: *mut c_void, index: isize, key: &[u8], value: u64) {
    assert!(index >= 0);
    assert!(key.len() <= BINARY_VALUE_OFFSET);
    let entry = unsafe { map.cast::<u8>().add(index as usize * BINARY_ENTRY_SIZE) };
    unsafe {
        ptr::write_bytes(entry, 0, BINARY_ENTRY_SIZE);
        ptr::copy_nonoverlapping(key.as_ptr(), entry, key.len());
        entry.add(BINARY_VALUE_OFFSET).cast::<u64>().write(value);
    }
}

unsafe fn binary_put(
    api: &Api,
    map: &mut *mut c_void,
    key: &mut [u8],
    value: u64,
    mode: c_int,
) -> isize {
    *map = unsafe {
        (api.hmput)(
            *map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            mode,
        )
    };
    let temporary = unsafe { map_header(*map, BINARY_ENTRY_SIZE).temp };
    unsafe { set_binary_entry(*map, temporary, key, value) };
    temporary
}

unsafe fn binary_get(
    api: &Api,
    map: &mut *mut c_void,
    key: &mut [u8],
    mode: c_int,
) -> (isize, Option<u64>) {
    *map = unsafe {
        (api.hmget)(
            *map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            mode,
        )
    };
    let temporary = unsafe { map_header(*map, BINARY_ENTRY_SIZE).temp };
    let value = if temporary < 0 {
        None
    } else {
        let pointer = unsafe {
            (*map)
                .cast::<u8>()
                .add(temporary as usize * BINARY_ENTRY_SIZE + BINARY_VALUE_OFFSET)
                .cast::<u64>()
        };
        Some(unsafe { *pointer })
    };
    (temporary, value)
}

unsafe fn binary_delete(api: &Api, map: &mut *mut c_void, key: &mut [u8], mode: c_int) -> isize {
    *map = unsafe {
        (api.hmdel)(
            *map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            mode,
        )
    };
    if (*map).is_null() {
        0
    } else {
        unsafe { map_header(*map, BINARY_ENTRY_SIZE).temp }
    }
}

unsafe fn assert_binary_maps_equal(c_map: *mut c_void, rust_map: *mut c_void) {
    assert_eq!(c_map.is_null(), rust_map.is_null());
    if c_map.is_null() {
        return;
    }
    let c_header = unsafe { map_header(c_map, BINARY_ENTRY_SIZE) };
    let rust_header = unsafe { map_header(rust_map, BINARY_ENTRY_SIZE) };
    assert_eq!(c_header.length, rust_header.length);
    assert_eq!(c_header.capacity, rust_header.capacity);
    assert_eq!(c_header.temp, rust_header.temp);
    assert_eq!(
        c_header.hash_table.is_null(),
        rust_header.hash_table.is_null()
    );
    let byte_length = (c_header.length - 1) * BINARY_ENTRY_SIZE;
    let c_bytes = unsafe { std::slice::from_raw_parts(c_map.cast::<u8>(), byte_length) };
    let rust_bytes = unsafe { std::slice::from_raw_parts(rust_map.cast::<u8>(), byte_length) };
    assert_eq!(c_bytes, rust_bytes);
}

fn key_bytes(value: u64, key_size: usize) -> Vec<u8> {
    let mut key = vec![0; key_size];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = value.rotate_left((index * 7) as u32) as u8;
    }
    key
}

#[test]
fn map_defaults_absent_lookups_and_null_operations_match() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        (c.hmfree)(ptr::null_mut(), BINARY_ENTRY_SIZE);
        (rust.hmfree)(ptr::null_mut(), BINARY_ENTRY_SIZE);

        let mut key = key_bytes(0x1234, 8);
        let c_deleted = (c.hmdel)(
            ptr::null_mut(),
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            0,
        );
        let rust_deleted = (rust.hmdel)(
            ptr::null_mut(),
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            0,
        );
        assert!(c_deleted.is_null());
        assert!(rust_deleted.is_null());

        let mut c_temp = 123;
        let mut rust_temp = 123;
        let mut c_map = (c.hmget_ts)(
            ptr::null_mut(),
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut c_temp,
            0,
        );
        let mut rust_map = (rust.hmget_ts)(
            ptr::null_mut(),
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);
        assert_binary_maps_equal(c_map, rust_map);

        let c_before = c_map;
        let rust_before = rust_map;
        c_map = (c.hmdefault)(c_map, BINARY_ENTRY_SIZE);
        rust_map = (rust.hmdefault)(rust_map, BINARY_ENTRY_SIZE);
        assert_eq!(c_map == c_before, rust_map == rust_before);
        assert_binary_maps_equal(c_map, rust_map);

        c_temp = 123;
        rust_temp = 123;
        c_map = (c.hmget_ts)(
            c_map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut c_temp,
            0,
        );
        rust_map = (rust.hmget_ts)(
            rust_map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            &mut rust_temp,
            0,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(c_temp, rust_temp);

        let c_result = (c.hmdel)(
            c_map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            0,
        );
        let rust_result = (rust.hmdel)(
            rust_map,
            BINARY_ENTRY_SIZE,
            key.as_mut_ptr().cast(),
            key.len(),
            0,
            0,
        );
        assert_eq!(c_result == c_map, rust_result == rust_map);
        c_map = c_result;
        rust_map = rust_result;
        assert_binary_maps_equal(c_map, rust_map);

        free_map(&c, c_map, BINARY_ENTRY_SIZE);
        free_map(&rust, rust_map, BINARY_ENTRY_SIZE);
    }
}

#[test]
fn randomized_binary_maps_match_through_growth_updates_and_deletes() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    unsafe {
        for &key_size in &[1, 2, 4, 8, 13] {
            for &seed in &[0, 1, 0x3141_5926, usize::MAX] {
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
                let mut c_map = ptr::null_mut();
                let mut rust_map = ptr::null_mut();

                for index in 0..96u64 {
                    let key_value = rng.next_u64() ^ index.wrapping_mul(0x9e37_79b9);
                    let value = rng.next_u64();
                    let mut c_key = key_bytes(key_value, key_size);
                    let mut rust_key = c_key.clone();
                    let c_index = binary_put(&c, &mut c_map, &mut c_key, value, 0);
                    let rust_index = binary_put(&rust, &mut rust_map, &mut rust_key, value, 0);
                    assert_eq!(c_index, rust_index);
                    assert_binary_maps_equal(c_map, rust_map);

                    if index % 7 == 0 {
                        let replacement = !value;
                        let c_index = binary_put(&c, &mut c_map, &mut c_key, replacement, 0);
                        let rust_index =
                            binary_put(&rust, &mut rust_map, &mut rust_key, replacement, 0);
                        assert_eq!(c_index, rust_index);
                        assert_binary_maps_equal(c_map, rust_map);
                    }
                }

                for index in 0..128u64 {
                    let key_value = if index < 96 {
                        // Reconstruct only a shape; randomized absent and present
                        // probes are both covered by the subsequent operation mix.
                        index.wrapping_mul(0x9e37_79b9)
                    } else {
                        rng.next_u64()
                    };
                    let mut c_key = key_bytes(key_value, key_size);
                    let mut rust_key = c_key.clone();
                    let (c_index, c_value) = binary_get(&c, &mut c_map, &mut c_key, 0);
                    let (rust_index, rust_value) =
                        binary_get(&rust, &mut rust_map, &mut rust_key, 0);
                    assert_eq!((c_index, c_value), (rust_index, rust_value));
                }

                let mut missing_c = vec![0xff; key_size];
                let mut missing_rust = missing_c.clone();
                let c_deleted = binary_delete(&c, &mut c_map, &mut missing_c, 0);
                let rust_deleted = binary_delete(&rust, &mut rust_map, &mut missing_rust, 0);
                assert_eq!(c_deleted, rust_deleted);
                assert_binary_maps_equal(c_map, rust_map);

                // Delete current entries from both final and non-final positions.
                for _ in 0..72 {
                    let header_value = map_header(c_map, BINARY_ENTRY_SIZE);
                    if header_value.length <= 1 {
                        break;
                    }
                    let live_count = header_value.length - 1;
                    let selected = (rng.next_u64() as usize) % live_count;
                    let key = std::slice::from_raw_parts(
                        c_map.cast::<u8>().add(selected * BINARY_ENTRY_SIZE),
                        key_size,
                    )
                    .to_vec();
                    let mut c_key = key.clone();
                    let mut rust_key = key;
                    let c_deleted = binary_delete(&c, &mut c_map, &mut c_key, 0);
                    let rust_deleted = binary_delete(&rust, &mut rust_map, &mut rust_key, 0);
                    assert_eq!(c_deleted, 1);
                    assert_eq!(c_deleted, rust_deleted);
                    assert_binary_maps_equal(c_map, rust_map);
                }

                // Reinsert after deletes to exercise tombstone reuse.
                for index in 0..32u64 {
                    let mut c_key = key_bytes(0xf000_0000 + index, key_size);
                    let mut rust_key = c_key.clone();
                    let c_index = binary_put(&c, &mut c_map, &mut c_key, index, 0);
                    let rust_index = binary_put(&rust, &mut rust_map, &mut rust_key, index, 0);
                    assert_eq!(c_index, rust_index);
                    assert_binary_maps_equal(c_map, rust_map);
                }

                free_map(&c, c_map, BINARY_ENTRY_SIZE);
                free_map(&rust, rust_map, BINARY_ENTRY_SIZE);
            }
        }
    }
}

#[test]
fn binary_and_out_of_range_modes_follow_the_c_integer_rules() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        for &mode in &[c_int::MIN, -7, -1, 0] {
            (c.rand_seed)(77);
            (rust.rand_seed)(77);
            let mut c_map = ptr::null_mut();
            let mut rust_map = ptr::null_mut();
            for value in 0..24u64 {
                let mut c_key = key_bytes(value, 8);
                let mut rust_key = c_key.clone();
                binary_put(&c, &mut c_map, &mut c_key, value + 100, mode);
                binary_put(&rust, &mut rust_map, &mut rust_key, value + 100, mode);
            }
            assert_binary_maps_equal(c_map, rust_map);
            free_map(&c, c_map, BINARY_ENTRY_SIZE);
            free_map(&rust, rust_map, BINARY_ENTRY_SIZE);
        }

        // A positive out-of-range mode on a null map is normalized by C to
        // STBDS_SH_DEFAULT because mode >= STBDS_HM_STRING.
        for &mode in &[1, 4, c_int::MAX] {
            let mut c_key = b"out-of-range\0".to_vec();
            let mut rust_key = c_key.clone();
            let mut c_map = (c.hmput)(
                ptr::null_mut(),
                size_of::<StringEntry>(),
                c_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            let mut rust_map = (rust.hmput)(
                ptr::null_mut(),
                size_of::<StringEntry>(),
                rust_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            let c_index = map_header(c_map, size_of::<StringEntry>()).temp;
            let rust_index = map_header(rust_map, size_of::<StringEntry>()).temp;
            assert_eq!(c_index, rust_index);
            (*c_map.cast::<StringEntry>().offset(c_index)).value = 91;
            (*rust_map.cast::<StringEntry>().offset(rust_index)).value = 91;

            c_map = (c.hmget)(
                c_map,
                size_of::<StringEntry>(),
                c_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            rust_map = (rust.hmget)(
                rust_map,
                size_of::<StringEntry>(),
                rust_key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                mode,
            );
            assert_eq!(
                map_header(c_map, size_of::<StringEntry>()).temp,
                map_header(rust_map, size_of::<StringEntry>()).temp
            );
            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, rust_map, size_of::<StringEntry>());
        }
    }
}

unsafe fn string_put(
    api: &Api,
    map: &mut *mut c_void,
    key: &mut [u8],
    value: i64,
    mode: c_int,
) -> isize {
    *map = unsafe {
        (api.hmput)(
            *map,
            size_of::<StringEntry>(),
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let temporary = unsafe { map_header(*map, size_of::<StringEntry>()).temp };
    unsafe { (*map.cast::<StringEntry>().offset(temporary)).value = value };
    temporary
}

unsafe fn string_get(
    api: &Api,
    map: &mut *mut c_void,
    key: &mut [u8],
    mode: c_int,
) -> (isize, Option<i64>) {
    *map = unsafe {
        (api.hmget)(
            *map,
            size_of::<StringEntry>(),
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let temporary = unsafe { map_header(*map, size_of::<StringEntry>()).temp };
    let value = if temporary < 0 {
        None
    } else {
        Some(unsafe { (*map.cast::<StringEntry>().offset(temporary)).value })
    };
    (temporary, value)
}

unsafe fn string_delete(api: &Api, map: &mut *mut c_void, key: &mut [u8], mode: c_int) -> isize {
    *map = unsafe {
        (api.hmdel)(
            *map,
            size_of::<StringEntry>(),
            key.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            0,
            mode,
        )
    };
    unsafe { map_header(*map, size_of::<StringEntry>()).temp }
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, i64)> {
    let metadata = unsafe { map_header(map, size_of::<StringEntry>()) };
    (0..metadata.length - 1)
        .map(|index| {
            let entry = unsafe { *map.cast::<StringEntry>().add(index) };
            let key = unsafe { CStr::from_ptr(entry.key) }
                .to_bytes_with_nul()
                .to_vec();
            (key, entry.value)
        })
        .collect()
}

unsafe fn assert_string_maps_equal(c_map: *mut c_void, rust_map: *mut c_void) {
    let c_header = unsafe { map_header(c_map, size_of::<StringEntry>()) };
    let rust_header = unsafe { map_header(rust_map, size_of::<StringEntry>()) };
    assert_eq!(c_header.length, rust_header.length);
    assert_eq!(c_header.capacity, rust_header.capacity);
    assert_eq!(c_header.temp, rust_header.temp);
    assert_eq!(
        c_header.hash_table.is_null(),
        rust_header.hash_table.is_null()
    );
    assert_eq!(unsafe { string_snapshot(c_map) }, unsafe {
        string_snapshot(rust_map)
    });
}

fn string_bytes(index: usize, length: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(length + 1);
    for position in 0..length {
        bytes.push(b'a' + ((index * 17 + position * 13) % 26) as u8);
    }
    bytes.extend_from_slice(format!("_{index}").as_bytes());
    bytes.push(0);
    bytes
}

#[test]
fn all_string_storage_modes_match_for_growth_lookup_update_and_delete() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        for &storage_mode in &[1, 2, 3] {
            (c.rand_seed)(0x1234_5678);
            (rust.rand_seed)(0x1234_5678);
            let mut c_map = (c.shmode)(size_of::<StringEntry>(), storage_mode);
            let mut rust_map = (rust.shmode)(size_of::<StringEntry>(), storage_mode);
            assert_string_maps_equal(c_map, rust_map);

            let mut sources = Vec::new();
            let mut originals = Vec::new();
            for index in 0..48 {
                let length = match index {
                    0 => 0,
                    1 => 1,
                    2 => 511,
                    3 => 512,
                    4 => 513,
                    5 => 2048,
                    _ => (index * 19) % 97,
                };
                let key = string_bytes(index, length);
                originals.push(key.clone());
                sources.push(key);
                let source = sources.last_mut().unwrap();
                let c_index = string_put(&c, &mut c_map, source, index as i64 * 37, 1);
                let rust_index = string_put(&rust, &mut rust_map, source, index as i64 * 37, 1);
                assert_eq!(c_index, rust_index);
                assert_string_maps_equal(c_map, rust_map);

                if storage_mode == 2 && source.len() > 1 {
                    source[0] = if source[0] == b'z' { b'y' } else { b'z' };
                    assert_string_maps_equal(c_map, rust_map);
                }
            }

            for &index in &[0, 1, 5, 17, 31, 47] {
                let mut c_key = originals[index].clone();
                let mut rust_key = c_key.clone();
                let c_result = string_get(&c, &mut c_map, &mut c_key, 1);
                let rust_result = string_get(&rust, &mut rust_map, &mut rust_key, 1);
                assert_eq!(c_result, rust_result);
                assert_eq!(c_result.1, Some(index as i64 * 37));
            }

            let mut c_missing = b"definitely_missing\0".to_vec();
            let mut rust_missing = c_missing.clone();
            assert_eq!(
                string_get(&c, &mut c_map, &mut c_missing, 1),
                string_get(&rust, &mut rust_map, &mut rust_missing, 1)
            );

            let mut c_duplicate = originals[17].clone();
            let mut rust_duplicate = c_duplicate.clone();
            assert_eq!(
                string_put(&c, &mut c_map, &mut c_duplicate, -991, 1),
                string_put(&rust, &mut rust_map, &mut rust_duplicate, -991, 1)
            );
            assert_string_maps_equal(c_map, rust_map);

            for &index in &[17, 47, 0, 23] {
                let mut c_key = originals[index].clone();
                let mut rust_key = c_key.clone();
                assert_eq!(
                    string_delete(&c, &mut c_map, &mut c_key, 1),
                    string_delete(&rust, &mut rust_map, &mut rust_key, 1)
                );
                assert_string_maps_equal(c_map, rust_map);
            }

            let mut c_missing = b"still_missing\0".to_vec();
            let mut rust_missing = c_missing.clone();
            assert_eq!(
                string_delete(&c, &mut c_map, &mut c_missing, 1),
                string_delete(&rust, &mut rust_map, &mut rust_missing, 1)
            );
            assert_string_maps_equal(c_map, rust_map);

            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, rust_map, size_of::<StringEntry>());
        }

        // Out-of-range shmode values are retained as unsigned char. A first
        // insertion takes the switch default branch in both implementations.
        for &mode in &[0, 4, 255, 256, -1] {
            let mut key = b"single\0".to_vec();
            let c_map = (c.shmode)(size_of::<StringEntry>(), mode);
            let rust_map = (rust.shmode)(size_of::<StringEntry>(), mode);
            let c_map = (c.hmput)(
                c_map,
                size_of::<StringEntry>(),
                key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            let rust_map = (rust.hmput)(
                rust_map,
                size_of::<StringEntry>(),
                key.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            let c_header = map_header(c_map, size_of::<StringEntry>());
            let rust_header = map_header(rust_map, size_of::<StringEntry>());
            assert_eq!(c_header.length, rust_header.length);
            assert_eq!(c_header.capacity, rust_header.capacity);
            assert_eq!(c_header.temp, rust_header.temp);
            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, rust_map, size_of::<StringEntry>());
        }
    }
}

#[test]
fn string_arena_allocations_and_reset_match_at_all_thresholds() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        let mut c_arena = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut rust_arena = c_arena;

        for (index, &length) in [
            0usize,
            1,
            7,
            500,
            511,
            512,
            513,
            1023,
            1024,
            4096,
            (1 << 20) - 1,
            1 << 20,
            (1 << 20) + 1,
        ]
        .iter()
        .enumerate()
        {
            let mut value = string_bytes(index + 100, length);
            let c_result = (c.stralloc)(&mut c_arena, value.as_mut_ptr().cast());
            let rust_result = (rust.stralloc)(&mut rust_arena, value.as_mut_ptr().cast());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes_with_nul(),
                CStr::from_ptr(rust_result).to_bytes_with_nul(),
                "length={length}"
            );
            assert_eq!(c_arena.remaining, rust_arena.remaining, "length={length}");
            assert_eq!(c_arena.block, rust_arena.block, "length={length}");
            assert_eq!(c_arena.mode, rust_arena.mode, "length={length}");
            assert_eq!(
                c_arena.storage.is_null(),
                rust_arena.storage.is_null(),
                "length={length}"
            );
        }

        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
        assert_eq!(c_arena.remaining, rust_arena.remaining);
        assert_eq!(c_arena.block, rust_arena.block);
        assert_eq!(c_arena.mode, rust_arena.mode);
        assert_eq!(c_arena.storage.is_null(), rust_arena.storage.is_null());

        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
        assert!(c_arena.storage.is_null());
        assert!(rust_arena.storage.is_null());
    }
}

#[test]
fn formatted_keys_and_valid_intput_calls_match() {
    let _guard = serial();
    let (c, rust) = unsafe { load_pair() };

    unsafe {
        for value in [
            c_int::MIN,
            -1_000_000,
            -1,
            0,
            1,
            9,
            11,
            1_000_000,
            c_int::MAX,
        ] {
            let c_value = CStr::from_ptr((c.strkey)(value))
                .to_bytes_with_nul()
                .to_vec();
            let rust_value = CStr::from_ptr((rust.strkey)(value))
                .to_bytes_with_nul()
                .to_vec();
            assert_eq!(c_value, rust_value, "value={value}");
        }

        let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
        for _ in 0..256 {
            let value = rng.next_u64() as c_int;
            if value != 9 && value != 11 {
                (c.intput)(value);
                (rust.intput)(value);
            }
        }
    }
}

fn run_child(library: &Path, child_test: &str, operation: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(child_test)
        .arg("--nocapture")
        .env("DIFF_CHILD", "1")
        .env("DIFF_LIBRARY", library)
        .env("DIFF_OPERATION", operation)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
}

#[test]
fn ffi_boundary_child() {
    if std::env::var_os("DIFF_CHILD").is_none() {
        return;
    }
    let _guard = serial();
    let path = PathBuf::from(std::env::var_os("DIFF_LIBRARY").unwrap());
    let operation = std::env::var("DIFF_OPERATION").unwrap();
    let api = unsafe { Api::load(&path) };
    unsafe {
        match operation.as_str() {
            "hash-bytes-null" => {
                (api.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hash-string-null" => {
                (api.hash_string)(ptr::null_mut(), 0);
            }
            "hash-bytes-oversized" => {
                let mut byte = 0u8;
                (api.hash_bytes)(ptr::addr_of_mut!(byte).cast(), usize::MAX, 0);
            }
            "arrfree-null" => {
                (api.arrfree)(ptr::null_mut());
            }
            "strreset-null" => {
                (api.strreset)(ptr::null_mut());
            }
            _ => panic!("unknown child operation {operation}"),
        }
    }
}

#[test]
fn invalid_pointer_boundaries_reject_equivalently() {
    let _guard = serial();
    for operation in [
        "hash-bytes-null",
        "hash-string-null",
        "hash-bytes-oversized",
        "arrfree-null",
        "strreset-null",
    ] {
        let c_status = run_child(&c_library_path(), "ffi_boundary_child", operation);
        let rust_status = run_child(&rust_library_path(), "ffi_boundary_child", operation);
        assert_status_equivalent(c_status, rust_status);
        assert!(!c_status.success(), "{operation} unexpectedly succeeded");
    }
}

#[test]
fn intput_abort_child() {
    if std::env::var_os("DIFF_CHILD").is_none() {
        return;
    }
    let _guard = serial();
    let path = PathBuf::from(std::env::var_os("DIFF_LIBRARY").unwrap());
    let value: c_int = std::env::var("DIFF_OPERATION").unwrap().parse().unwrap();
    let api = unsafe { Api::load(&path) };
    unsafe { (api.intput)(value) };
}

#[test]
fn intput_duplicate_key_assertions_reject_equivalently() {
    let _guard = serial();
    for value in ["9", "11"] {
        let c_status = run_child(&c_library_path(), "intput_abort_child", value);
        let rust_status = run_child(&rust_library_path(), "intput_abort_child", value);
        assert_status_equivalent(c_status, rust_status);
        assert!(
            !c_status.success(),
            "intput({value}) unexpectedly succeeded"
        );
    }
}
