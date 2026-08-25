use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::{offset_of, size_of};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

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
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type Helxo = unsafe extern "C" fn(c_char);

struct Api {
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
    helxo: Helxo,
    _library: Library,
}

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(&path) }
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
            helxo: symbol!("helxo", Helxo),
            _library: library,
        }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("c_src/build/libtranslated_rust.so");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let rust_path = root.join(format!("target/{profile}/libhelxo_lib.so"));
    assert!(c_path.exists(), "build the C shared object first");
    assert!(
        rust_path.exists(),
        "Rust cdylib missing at {}",
        rust_path.display()
    );
    unsafe { (Api::load(c_path), Api::load(rust_path)) }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Arena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: u64,
}

unsafe fn array_header(data: *mut c_void) -> *mut Header {
    data.cast::<Header>().wrapping_sub(1)
}

unsafe fn hash_raw(hash: *mut c_void, elem_size: usize) -> *mut c_void {
    hash.cast::<u8>().wrapping_sub(elem_size).cast()
}

unsafe fn hash_header(hash: *mut c_void, elem_size: usize) -> *mut Header {
    unsafe { array_header(hash_raw(hash, elem_size)) }
}

fn header_shape(header: Header) -> (usize, usize, bool, isize) {
    (
        header.length,
        header.capacity,
        !header.hash_table.is_null(),
        header.temp,
    )
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x8d26_7a45_1f90_c3be)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(8) {
            let random = self.next_u64().to_ne_bytes();
            chunk.copy_from_slice(&random[..chunk.len()]);
        }
    }
}

unsafe fn array_snapshot(data: *mut c_void, bytes: usize) -> (Header, Vec<u8>) {
    let header = unsafe { *array_header(data) };
    let content = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), bytes) }.to_vec();
    (header, content)
}

unsafe fn binary_snapshot(hash: *mut c_void, elem_size: usize) -> (Header, Vec<Vec<u8>>) {
    let header = unsafe { *hash_header(hash, elem_size) };
    let count = header.length.saturating_sub(1);
    let entries = (0..count)
        .map(|index| {
            unsafe {
                std::slice::from_raw_parts(hash.cast::<u8>().add(index * elem_size), elem_size)
            }
            .to_vec()
        })
        .collect();
    (header, entries)
}

unsafe fn string_snapshot(hash: *mut c_void) -> (Header, Vec<(Vec<u8>, u64)>) {
    let elem_size = size_of::<StringEntry>();
    let header = unsafe { *hash_header(hash, elem_size) };
    let count = header.length.saturating_sub(1);
    let entries = (0..count)
        .map(|index| {
            let entry = unsafe { *hash.cast::<StringEntry>().add(index) };
            (
                unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
                entry.value,
            )
        })
        .collect();
    (header, entries)
}

unsafe fn put_binary(
    api: &Api,
    hash: *mut c_void,
    key: &mut [u8],
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let elem_size = key.len() + size_of::<u64>();
    let hash = unsafe { (api.hmput)(hash, elem_size, key.as_mut_ptr().cast(), key.len(), mode) };
    let index = unsafe { (*hash_header(hash, elem_size)).temp as usize };
    unsafe {
        ptr::write_unaligned(
            hash.cast::<u8>()
                .add(index * elem_size + key.len())
                .cast::<u64>(),
            value,
        )
    };
    hash
}

unsafe fn put_string(
    api: &Api,
    hash: *mut c_void,
    key: &CString,
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let elem_size = size_of::<StringEntry>();
    let hash = unsafe {
        (api.hmput)(
            hash,
            elem_size,
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let index = unsafe { (*hash_header(hash, elem_size)).temp as usize };
    unsafe { (*hash.cast::<StringEntry>().add(index)).value = value };
    hash
}

fn assert_header_equal(c: Header, rust: Header) {
    assert_eq!(header_shape(c), header_shape(rust));
}

#[test]
fn arrays_cover_configs_1_through_6_and_52() {
    let _guard = serial();
    let (c, rust) = libraries();
    let mut rng = Rng::new();
    unsafe {
        for elem_size in [1usize, 2, 4, 8, 13] {
            let c_null = (c.arrgrow)(ptr::null_mut(), elem_size, 0, 0);
            let r_null = (rust.arrgrow)(ptr::null_mut(), elem_size, 0, 0);
            assert!(c_null.is_null());
            assert!(r_null.is_null());

            for min_cap in 1..=3 {
                let c_ptr = (c.arrgrow)(ptr::null_mut(), elem_size, 0, min_cap);
                let r_ptr = (rust.arrgrow)(ptr::null_mut(), elem_size, 0, min_cap);
                let (c_header, _) = array_snapshot(c_ptr, 0);
                let (r_header, _) = array_snapshot(r_ptr, 0);
                assert_header_equal(c_header, r_header);
                (c.arrfree)(c_ptr);
                (rust.arrfree)(r_ptr);
            }

            for add_len in [1usize, 2, 5, 17] {
                let mut c_ptr = (c.arrgrow)(ptr::null_mut(), elem_size, add_len, 0);
                let mut r_ptr = (rust.arrgrow)(ptr::null_mut(), elem_size, add_len, 0);
                let bytes = elem_size * add_len;
                let mut content = vec![0u8; bytes];
                rng.fill(&mut content);
                ptr::copy_nonoverlapping(content.as_ptr(), c_ptr.cast(), bytes);
                ptr::copy_nonoverlapping(content.as_ptr(), r_ptr.cast(), bytes);
                (*array_header(c_ptr)).length = add_len;
                (*array_header(r_ptr)).length = add_len;

                let old_c = c_ptr;
                let old_r = r_ptr;
                c_ptr = (c.arrgrow)(c_ptr, elem_size, 0, add_len);
                r_ptr = (rust.arrgrow)(r_ptr, elem_size, 0, add_len);
                assert_eq!(c_ptr, old_c);
                assert_eq!(r_ptr, old_r);

                for growth_kind in 0..3 {
                    let capacity = (*array_header(c_ptr)).capacity;
                    let request = match growth_kind {
                        0 => capacity + 1,
                        1 => capacity * 2,
                        _ => capacity * 3 + 1,
                    };
                    c_ptr = (c.arrgrow)(c_ptr, elem_size, 0, request);
                    r_ptr = (rust.arrgrow)(r_ptr, elem_size, 0, request);
                    let (ch, cb) = array_snapshot(c_ptr, bytes);
                    let (rh, rb) = array_snapshot(r_ptr, bytes);
                    assert_header_equal(ch, rh);
                    assert_eq!(cb, rb);
                    assert_eq!(cb, content);
                }
                (c.arrfree)(c_ptr);
                (rust.arrfree)(r_ptr);
            }
        }
    }
}

#[test]
fn hashes_cover_configs_7_through_12_and_error_19() {
    let _guard = serial();
    let (c, rust) = libraries();
    let mut rng = Rng::new();
    let seeds = [0, 1, usize::MAX, 0x3141_5926, rng.next_u64() as usize];

    unsafe {
        for seed in seeds {
            let empty = CString::new("").unwrap();
            assert_eq!(
                (c.hash_string)(empty.as_ptr().cast_mut(), seed),
                (rust.hash_string)(empty.as_ptr().cast_mut(), seed)
            );
            assert_eq!(
                (c.hash_bytes)(ptr::null_mut(), 0, seed),
                (rust.hash_bytes)(ptr::null_mut(), 0, seed)
            );

            for len in 0..96usize {
                for _ in 0..32 {
                    let mut bytes = vec![0u8; len];
                    rng.fill(&mut bytes);
                    let c_hash = (c.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed);
                    let rust_hash = (rust.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed);
                    assert_eq!(
                        c_hash, rust_hash,
                        "byte hash mismatch len={len} seed={seed}"
                    );
                }
            }

            for len in [1usize, 2, 7, 8, 9, 15, 16, 31, 63, 127] {
                for _ in 0..32 {
                    let mut bytes = vec![0u8; len];
                    rng.fill(&mut bytes);
                    for byte in &mut bytes {
                        if *byte == 0 {
                            *byte = 0x80;
                        }
                    }
                    let string = CString::new(bytes).unwrap();
                    let c_hash = (c.hash_string)(string.as_ptr().cast_mut(), seed);
                    let rust_hash = (rust.hash_string)(string.as_ptr().cast_mut(), seed);
                    assert_eq!(c_hash, rust_hash, "string hash mismatch len={len}");
                }
            }
        }
    }
}

unsafe fn compare_binary_maps(c_hash: *mut c_void, rust_hash: *mut c_void, elem_size: usize) {
    let (ch, ce) = unsafe { binary_snapshot(c_hash, elem_size) };
    let (rh, re) = unsafe { binary_snapshot(rust_hash, elem_size) };
    assert_header_equal(ch, rh);
    assert_eq!(ce, re);
}

unsafe fn compare_string_maps(c_hash: *mut c_void, rust_hash: *mut c_void) {
    let (ch, ce) = unsafe { string_snapshot(c_hash) };
    let (rh, re) = unsafe { string_snapshot(rust_hash) };
    assert_header_equal(ch, rh);
    assert_eq!(ce, re);
}

#[test]
fn binary_maps_cover_configs_13_through_30_and_errors_1_through_16() {
    let _guard = serial();
    let (c, rust) = libraries();
    let mut rng = Rng::new();

    unsafe {
        // Null/default/no-table paths.
        for elem_size in [9usize, 10, 12, 16, 24] {
            let mut c_hash = (c.hmput_default)(ptr::null_mut(), elem_size);
            let mut r_hash = (rust.hmput_default)(ptr::null_mut(), elem_size);
            compare_binary_maps(c_hash, r_hash, elem_size);
            assert_eq!(
                std::slice::from_raw_parts(hash_raw(c_hash, elem_size).cast::<u8>(), elem_size),
                std::slice::from_raw_parts(hash_raw(r_hash, elem_size).cast::<u8>(), elem_size)
            );

            let old_c = c_hash;
            let old_r = r_hash;
            c_hash = (c.hmput_default)(c_hash, elem_size);
            r_hash = (rust.hmput_default)(r_hash, elem_size);
            assert_eq!(c_hash, old_c);
            assert_eq!(r_hash, old_r);

            let mut key = vec![0x5au8; elem_size - 8];
            let mut ct = 99isize;
            let mut rt = 99isize;
            assert_eq!(
                (c.hmget_ts)(
                    c_hash,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    &mut ct,
                    0
                ),
                c_hash
            );
            assert_eq!(
                (rust.hmget_ts)(
                    r_hash,
                    elem_size,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    &mut rt,
                    0
                ),
                r_hash
            );
            assert_eq!((ct, rt), (-1, -1));

            let c_get = (c.hmget)(c_hash, elem_size, key.as_mut_ptr().cast(), key.len(), 0);
            let r_get = (rust.hmget)(r_hash, elem_size, key.as_mut_ptr().cast(), key.len(), 0);
            assert_eq!((*hash_header(c_get, elem_size)).temp, -1);
            assert_eq!((*hash_header(r_get, elem_size)).temp, -1);

            assert_eq!(
                (c.hmdel)(c_hash, elem_size, key.as_mut_ptr().cast(), key.len(), 0, 0),
                c_hash
            );
            assert_eq!(
                (rust.hmdel)(r_hash, elem_size, key.as_mut_ptr().cast(), key.len(), 0, 0),
                r_hash
            );
            assert_eq!((*hash_header(c_hash, elem_size)).temp, 0);
            assert_eq!((*hash_header(r_hash, elem_size)).temp, 0);
            (c.hmfree)(hash_raw(c_hash, elem_size), elem_size);
            (rust.hmfree)(hash_raw(r_hash, elem_size), elem_size);
        }

        let mut key = [7u8; 8];
        let mut ct = 0isize;
        let mut rt = 0isize;
        let c_created = (c.hmget_ts)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, &mut ct, 0);
        let r_created =
            (rust.hmget_ts)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, &mut rt, 0);
        assert_eq!((ct, rt), (-1, -1));
        compare_binary_maps(c_created, r_created, 16);
        (c.hmfree)(hash_raw(c_created, 16), 16);
        (rust.hmfree)(hash_raw(r_created, 16), 16);

        let c_created = (c.hmget)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, 0);
        let r_created = (rust.hmget)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, 0);
        assert_eq!((*hash_header(c_created, 16)).temp, -1);
        assert_eq!((*hash_header(r_created, 16)).temp, -1);
        (c.hmfree)(hash_raw(c_created, 16), 16);
        (rust.hmfree)(hash_raw(r_created, 16), 16);

        assert!((c.hmdel)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, 0, 0).is_null());
        assert!((rust.hmdel)(ptr::null_mut(), 16, key.as_mut_ptr().cast(), 8, 0, 0).is_null());
        (c.hmfree)(ptr::null_mut(), 16);
        (rust.hmfree)(ptr::null_mut(), 16);

        // Width, seed, update, lookup, growth, collision, and deletion paths.
        for key_size in [1usize, 2, 4, 8, 16] {
            let elem_size = key_size + 8;
            for seed in [0usize, 1, usize::MAX, rng.next_u64() as usize] {
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
                let mut c_hash = ptr::null_mut();
                let mut r_hash = ptr::null_mut();
                let mut keys = Vec::new();

                for index in 0..80u64 {
                    let mut key = vec![0u8; key_size];
                    rng.fill(&mut key);
                    let unique = index.to_le_bytes();
                    let unique_len = key_size.min(unique.len());
                    key[..unique_len].copy_from_slice(&unique[..unique_len]);
                    c_hash = put_binary(&c, c_hash, &mut key, index ^ seed as u64, 0);
                    r_hash = put_binary(&rust, r_hash, &mut key, index ^ seed as u64, 0);
                    keys.push(key);
                    compare_binary_maps(c_hash, r_hash, elem_size);
                }

                for index in (0..keys.len()).step_by(7) {
                    let value = rng.next_u64();
                    c_hash = put_binary(&c, c_hash, &mut keys[index], value, 0);
                    r_hash = put_binary(&rust, r_hash, &mut keys[index], value, 0);
                    compare_binary_maps(c_hash, r_hash, elem_size);
                }

                for index in (0..keys.len()).step_by(5) {
                    let mut ct = -9isize;
                    let mut rt = -9isize;
                    c_hash = (c.hmget_ts)(
                        c_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        &mut ct,
                        0,
                    );
                    r_hash = (rust.hmget_ts)(
                        r_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        &mut rt,
                        0,
                    );
                    assert_eq!(ct, rt);
                    assert!(ct >= 0);
                }

                for _ in 0..32 {
                    let mut missing = vec![0u8; key_size];
                    rng.fill(&mut missing);
                    let mut ct = -9isize;
                    let mut rt = -9isize;
                    c_hash = (c.hmget_ts)(
                        c_hash,
                        elem_size,
                        missing.as_mut_ptr().cast(),
                        key_size,
                        &mut ct,
                        0,
                    );
                    r_hash = (rust.hmget_ts)(
                        r_hash,
                        elem_size,
                        missing.as_mut_ptr().cast(),
                        key_size,
                        &mut rt,
                        0,
                    );
                    assert_eq!(ct, rt);
                }

                let mut absent = vec![0xff; key_size];
                c_hash = (c.hmdel)(
                    c_hash,
                    elem_size,
                    absent.as_mut_ptr().cast(),
                    key_size,
                    0,
                    0,
                );
                r_hash = (rust.hmdel)(
                    r_hash,
                    elem_size,
                    absent.as_mut_ptr().cast(),
                    key_size,
                    0,
                    0,
                );
                compare_binary_maps(c_hash, r_hash, elem_size);

                // Non-final and final deletion, then tombstone reuse/rebuild/shrink.
                for index in (0..keys.len()).step_by(2) {
                    c_hash = (c.hmdel)(
                        c_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        0,
                        0,
                    );
                    r_hash = (rust.hmdel)(
                        r_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        0,
                        0,
                    );
                    compare_binary_maps(c_hash, r_hash, elem_size);
                }
                for index in (1..50).step_by(2) {
                    c_hash = (c.hmdel)(
                        c_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        0,
                        0,
                    );
                    r_hash = (rust.hmdel)(
                        r_hash,
                        elem_size,
                        keys[index].as_mut_ptr().cast(),
                        key_size,
                        0,
                        0,
                    );
                    compare_binary_maps(c_hash, r_hash, elem_size);
                }
                for index in 0..24u64 {
                    let mut replacement = vec![0u8; key_size];
                    rng.fill(&mut replacement);
                    let unique = (128 + index).to_le_bytes();
                    let unique_len = key_size.min(unique.len());
                    replacement[..unique_len].copy_from_slice(&unique[..unique_len]);
                    c_hash = put_binary(&c, c_hash, &mut replacement, index, 0);
                    r_hash = put_binary(&rust, r_hash, &mut replacement, index, 0);
                    compare_binary_maps(c_hash, r_hash, elem_size);
                }

                (c.hmfree)(hash_raw(c_hash, elem_size), elem_size);
                (rust.hmfree)(hash_raw(r_hash, elem_size), elem_size);
            }
        }
    }
}

#[test]
fn string_maps_cover_configs_31_through_42_49_and_error_18() {
    let _guard = serial();
    let (c, rust) = libraries();
    let elem_size = size_of::<StringEntry>();

    unsafe {
        // SH_NONE reaches the binary/default switch arm.
        let mut c_binary = (c.shmode)(16, 0);
        let mut r_binary = (rust.shmode)(16, 0);
        for value in 0..24u64 {
            let mut key = value.rotate_left(17).to_ne_bytes().to_vec();
            c_binary = put_binary(&c, c_binary, &mut key, !value, 0);
            r_binary = put_binary(&rust, r_binary, &mut key, !value, 0);
            compare_binary_maps(c_binary, r_binary, 16);
        }
        (c.hmfree)(hash_raw(c_binary, 16), 16);
        (rust.hmfree)(hash_raw(r_binary, 16), 16);

        for storage_mode in [1, 2, 3] {
            (c.rand_seed)(0x1234_5678);
            (rust.rand_seed)(0x1234_5678);
            let mut c_hash = (c.shmode)(elem_size, storage_mode);
            let mut r_hash = (rust.shmode)(elem_size, storage_mode);
            let mut keys: Vec<CString> = (0..48)
                .map(|index| {
                    let text = if index == 0 {
                        String::new()
                    } else if index == 47 {
                        format!("long-key-{index}-{}", "x".repeat(700))
                    } else {
                        format!("key-{index:03}-{}", (index * 7919) % 104729)
                    };
                    CString::new(text).unwrap()
                })
                .collect();

            for (index, key) in keys.iter().enumerate() {
                c_hash = put_string(&c, c_hash, key, index as u64, 1);
                r_hash = put_string(&rust, r_hash, key, index as u64, 1);
                compare_string_maps(c_hash, r_hash);
            }

            let duplicate = CString::new(keys[9].to_bytes()).unwrap();
            c_hash = put_string(&c, c_hash, &duplicate, u64::MAX, 1);
            r_hash = put_string(&rust, r_hash, &duplicate, u64::MAX, 1);
            compare_string_maps(c_hash, r_hash);

            for index in (0..keys.len()).step_by(6) {
                let mut ct = -99isize;
                let mut rt = -99isize;
                c_hash = (c.hmget_ts)(
                    c_hash,
                    elem_size,
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut ct,
                    1,
                );
                r_hash = (rust.hmget_ts)(
                    r_hash,
                    elem_size,
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut rt,
                    1,
                );
                assert_eq!(ct, rt);
            }

            let missing = CString::new("not-present").unwrap();
            c_hash = (c.hmget)(
                c_hash,
                elem_size,
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            r_hash = (rust.hmget)(
                r_hash,
                elem_size,
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            assert_eq!((*hash_header(c_hash, elem_size)).temp, -1);
            assert_eq!((*hash_header(r_hash, elem_size)).temp, -1);

            // Missing, non-final, and final deletions in every string storage mode.
            c_hash = (c.hmdel)(
                c_hash,
                elem_size,
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            );
            r_hash = (rust.hmdel)(
                r_hash,
                elem_size,
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            );
            compare_string_maps(c_hash, r_hash);
            for index in [7usize, 47, 3, 41, 0] {
                c_hash = (c.hmdel)(
                    c_hash,
                    elem_size,
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                r_hash = (rust.hmdel)(
                    r_hash,
                    elem_size,
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                compare_string_maps(c_hash, r_hash);
            }

            (c.hmfree)(hash_raw(c_hash, elem_size), elem_size);
            (rust.hmfree)(hash_raw(r_hash, elem_size), elem_size);
            keys.clear();
        }

        // A null direct string map selects borrowed/default mode.
        let direct = CString::new("direct").unwrap();
        let c_direct = put_string(&c, ptr::null_mut(), &direct, 17, 1);
        let r_direct = put_string(&rust, ptr::null_mut(), &direct, 17, 1);
        compare_string_maps(c_direct, r_direct);
        (c.hmfree)(hash_raw(c_direct, elem_size), elem_size);
        (rust.hmfree)(hash_raw(r_direct, elem_size), elem_size);

        // Out-of-range modes use raw comparisons and shmode's unsigned-byte cast.
        let mut c_negative = (c.shmode)(16, -1);
        let mut r_negative = (rust.shmode)(16, -1);
        let mut binary_key = 0xfeed_face_cafe_beefu64.to_ne_bytes().to_vec();
        c_negative = put_binary(&c, c_negative, &mut binary_key, 1, -1);
        r_negative = put_binary(&rust, r_negative, &mut binary_key, 1, -1);
        compare_binary_maps(c_negative, r_negative, 16);
        (c.hmfree)(hash_raw(c_negative, 16), 16);
        (rust.hmfree)(hash_raw(r_negative, 16), 16);

        let high_mode_key = CString::new("mode-257").unwrap();
        let c_high = (c.shmode)(elem_size, 257);
        let r_high = (rust.shmode)(elem_size, 257);
        let c_high = put_string(&c, c_high, &high_mode_key, 257, 257);
        let r_high = put_string(&rust, r_high, &high_mode_key, 257, 257);
        compare_string_maps(c_high, r_high);
        (c.hmfree)(hash_raw(c_high, elem_size), elem_size);
        (rust.hmfree)(hash_raw(r_high, elem_size), elem_size);
    }
}

fn arena_shape(arena: Arena) -> (bool, usize, u8, u8) {
    (
        !arena.storage.is_null(),
        arena.remaining,
        arena.block,
        arena.mode,
    )
}

#[test]
fn string_arenas_cover_configs_43_through_48_and_error_17() {
    let _guard = serial();
    let (c, rust) = libraries();
    assert_eq!(offset_of!(Arena, remaining), size_of::<*mut c_void>());

    unsafe {
        let mut c_arena: Arena = std::mem::zeroed();
        let mut r_arena: Arena = std::mem::zeroed();
        let sizes = [
            0usize, 1, 7, 31, 255, 500, 511, 512, 513, 700, 1023, 1024, 4097,
        ];
        for (index, len) in sizes.into_iter().enumerate() {
            let byte = b'a' + (index % 26) as u8;
            let string = CString::new(vec![byte; len]).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut());
            let r_result = (rust.stralloc)(&mut r_arena, string.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(CStr::from_ptr(c_result).to_bytes(), string.as_bytes());
            assert_eq!(arena_shape(c_arena), arena_shape(r_arena));
        }
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert_eq!(arena_shape(c_arena), (false, 0, 0, 0));
        assert_eq!(arena_shape(c_arena), arena_shape(r_arena));

        // Exercise each block-growth exponent, including the 1 MiB cap.
        for block in 0u8..=24 {
            let mut c_arena = Arena {
                storage: ptr::null_mut(),
                remaining: 0,
                block,
                mode: 0x5a,
            };
            let mut r_arena = c_arena;
            let string = CString::new(format!("block-{block}")).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut());
            let r_result = (rust.stralloc)(&mut r_arena, string.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(arena_shape(c_arena), arena_shape(r_arena));
            (c.strreset)(&mut c_arena);
            (rust.strreset)(&mut r_arena);
        }

        // Empty reset is an explicit public no-content shape.
        let mut c_empty: Arena = std::mem::zeroed();
        let mut r_empty: Arena = std::mem::zeroed();
        (c.strreset)(&mut c_empty);
        (rust.strreset)(&mut r_empty);
        assert_eq!(arena_shape(c_empty), arena_shape(r_empty));
    }
}

#[test]
fn strkey_covers_config_50() {
    let _guard = serial();
    let (c, rust) = libraries();
    unsafe {
        for number in [c_int::MIN, -1_000_000, -1, 0, 1, 1_000_000, c_int::MAX] {
            let c_first = (c.strkey)(number);
            let r_first = (rust.strkey)(number);
            assert_eq!(CStr::from_ptr(c_first), CStr::from_ptr(r_first));
            assert_eq!(
                CStr::from_ptr(c_first).to_str().unwrap(),
                format!("test_{number}")
            );
            let c_address = c_first;
            let r_address = r_first;
            assert_eq!((c.strkey)(number.wrapping_add(1)), c_address);
            assert_eq!((rust.strkey)(number.wrapping_add(1)), r_address);
            assert_eq!(CStr::from_ptr(c_address), CStr::from_ptr(r_address));
        }
    }
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [-1, -1];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    unsafe { fflush(ptr::null_mut()) };
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(fds[1], 1) }, 1);
    unsafe { close(fds[1]) };

    call();

    unsafe { fflush(ptr::null_mut()) };
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    unsafe { close(saved_stdout) };

    let mut output = Vec::new();
    loop {
        let mut chunk = [0u8; 256];
        let count = unsafe { read(fds[0], chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    unsafe { close(fds[0]) };
    output
}

#[test]
fn helxo_covers_config_51() {
    let _guard = serial();
    let (c, rust) = libraries();
    unsafe {
        for letter in [0i8, b'A' as i8, 0x7f, -1, -128] {
            let c_output = capture_stdout(|| (c.helxo)(letter as c_char));
            let rust_output = capture_stdout(|| (rust.helxo)(letter as c_char));
            assert_eq!(c_output, rust_output, "helxo mismatch for {letter}");
            assert!(c_output.starts_with(b"bob h\nsally e\nfred l\njen "));
            assert!(c_output.ends_with(b"doug o\n"));
        }
    }
}
