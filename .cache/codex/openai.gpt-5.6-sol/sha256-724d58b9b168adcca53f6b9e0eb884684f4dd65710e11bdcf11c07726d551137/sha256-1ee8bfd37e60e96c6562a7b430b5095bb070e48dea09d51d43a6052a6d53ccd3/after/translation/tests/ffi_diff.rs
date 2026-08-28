use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
type IntPut = unsafe extern "C" fn(c_int);

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
    hmput_default: HmPutDefault,
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

        unsafe fn symbol<T: Copy>(library: &Library, name: &[u8]) -> T {
            *unsafe { library.get::<T>(name) }.unwrap_or_else(|error| {
                panic!("missing {:?}: {error}", CStr::from_bytes_with_nul(name))
            })
        }

        Self {
            arrgrow: unsafe { symbol(&library, b"stbds_arrgrowf\0") },
            arrfree: unsafe { symbol(&library, b"stbds_arrfreef\0") },
            rand_seed: unsafe { symbol(&library, b"stbds_rand_seed\0") },
            hash_bytes: unsafe { symbol(&library, b"stbds_hash_bytes\0") },
            hash_string: unsafe { symbol(&library, b"stbds_hash_string\0") },
            hmfree: unsafe { symbol(&library, b"stbds_hmfree_func\0") },
            hmget: unsafe { symbol(&library, b"stbds_hmget_key\0") },
            hmget_ts: unsafe { symbol(&library, b"stbds_hmget_key_ts\0") },
            hmput_default: unsafe { symbol(&library, b"stbds_hmput_default\0") },
            hmput: unsafe { symbol(&library, b"stbds_hmput_key\0") },
            hmdel: unsafe { symbol(&library, b"stbds_hmdel_key\0") },
            shmode: unsafe { symbol(&library, b"stbds_shmode_func\0") },
            stralloc: unsafe { symbol(&library, b"stbds_stralloc\0") },
            strreset: unsafe { symbol(&library, b"stbds_strreset\0") },
            strkey: unsafe { symbol(&library, b"strkey\0") },
            intput: unsafe { symbol(&library, b"intput\0") },
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

impl Default for StringArena {
    fn default() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IntEntry {
    key: i32,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArraySnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StringSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<(Vec<u8>, i32)>,
}

#[derive(Clone)]
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

    fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u64() as u8;
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-lca2wP.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libintput_lib.so")
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
}

unsafe fn raw_from_hash(hash_array: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { hash_array.cast::<u8>().sub(elem_size).cast() }
}

unsafe fn snapshot_array(array: *mut c_void, byte_len: usize) -> ArraySnapshot {
    let metadata = unsafe { *header(array) };
    ArraySnapshot {
        length: metadata.length,
        capacity: metadata.capacity,
        temp: metadata.temp,
        bytes: unsafe { std::slice::from_raw_parts(array.cast::<u8>(), byte_len) }.to_vec(),
    }
}

unsafe fn snapshot_map(hash_array: *mut c_void, elem_size: usize) -> ArraySnapshot {
    let raw = unsafe { raw_from_hash(hash_array, elem_size) };
    let metadata = unsafe { *header(raw) };
    let entries = metadata.length.saturating_sub(1);
    ArraySnapshot {
        length: metadata.length,
        capacity: metadata.capacity,
        temp: metadata.temp,
        bytes: unsafe { std::slice::from_raw_parts(hash_array.cast::<u8>(), entries * elem_size) }
            .to_vec(),
    }
}

unsafe fn snapshot_string_map(hash_array: *mut c_void) -> StringSnapshot {
    let elem_size = size_of::<StringEntry>();
    let raw = unsafe { raw_from_hash(hash_array, elem_size) };
    let metadata = unsafe { *header(raw) };
    let mut entries = Vec::new();
    for index in 0..metadata.length.saturating_sub(1) {
        let entry = unsafe { *hash_array.cast::<StringEntry>().add(index) };
        entries.push((
            unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
            entry.value,
        ));
    }
    StringSnapshot {
        length: metadata.length,
        capacity: metadata.capacity,
        temp: metadata.temp,
        entries,
    }
}

unsafe fn put_int(api: &Api, map: &mut *mut c_void, key: i32, value: i32, mode: c_int) {
    let mut key_arg = key;
    *map = unsafe {
        (api.hmput)(
            *map,
            size_of::<IntEntry>(),
            ptr::addr_of_mut!(key_arg).cast(),
            size_of::<i32>(),
            mode,
        )
    };
    let raw = unsafe { raw_from_hash(*map, size_of::<IntEntry>()) };
    let index = unsafe { (*header(raw)).temp } as usize;
    unsafe {
        (*map.cast::<IntEntry>().add(index)).key = key;
        (*map.cast::<IntEntry>().add(index)).value = value;
    }
}

unsafe fn put_string(api: &Api, map: &mut *mut c_void, key: *mut c_char, value: i32, mode: c_int) {
    *map = unsafe {
        (api.hmput)(
            *map,
            size_of::<StringEntry>(),
            key.cast(),
            size_of::<*mut c_char>(),
            mode,
        )
    };
    let raw = unsafe { raw_from_hash(*map, size_of::<StringEntry>()) };
    let index = unsafe { (*header(raw)).temp } as usize;
    unsafe {
        (*map.cast::<StringEntry>().add(index)).value = value;
    }
}

unsafe fn free_map(api: &Api, map: *mut c_void, elem_size: usize) {
    if !map.is_null() {
        unsafe { (api.hmfree)(raw_from_hash(map, elem_size), elem_size) };
    }
}

fn load_pair() -> (Api, Api) {
    assert!(c_library_path().is_file(), "C shared library was not built");
    assert!(
        rust_library_path().is_file(),
        "Rust release shared library was not built"
    );
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

#[test]
fn symbols_load_through_both_dynamic_libraries() {
    let _guard = test_lock();
    let (_c, _rust) = load_pair();
}

#[test]
fn arrays_match_all_capacity_branches_and_element_widths() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x41a7_5eed_1234_5678);

    for &elem_size in &[1usize, 4, 8, 16] {
        for case in 0..64 {
            let add_len = if case == 0 {
                0
            } else {
                1 + rng.next_usize() % 31
            };
            let min_cap = if case % 2 == 0 {
                rng.next_usize() % (add_len + 1)
            } else {
                add_len + rng.next_usize() % 31
            };
            let c_array = unsafe { (c.arrgrow)(ptr::null_mut(), elem_size, add_len, min_cap) };
            let rust_array =
                unsafe { (rust.arrgrow)(ptr::null_mut(), elem_size, add_len, min_cap) };

            if add_len == 0 && min_cap == 0 {
                assert!(c_array.is_null());
                assert!(rust_array.is_null());
                continue;
            }
            let expected_capacity = add_len.max(min_cap).max(4);
            assert_eq!(unsafe { (*header(c_array)).capacity }, expected_capacity);
            assert_eq!(unsafe { snapshot_array(c_array, 0) }, unsafe {
                snapshot_array(rust_array, 0)
            });

            let capacity = unsafe { (*header(c_array)).capacity };
            let data_len = capacity * elem_size;
            let mut bytes = vec![0u8; data_len];
            rng.fill(&mut bytes);
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), c_array.cast(), data_len);
                ptr::copy_nonoverlapping(bytes.as_ptr(), rust_array.cast(), data_len);
                (*header(c_array)).length = capacity / 2;
                (*header(rust_array)).length = capacity / 2;
            }

            let c_same = unsafe { (c.arrgrow)(c_array, elem_size, 0, capacity) };
            let rust_same = unsafe { (rust.arrgrow)(rust_array, elem_size, 0, capacity) };
            assert_eq!(c_same, c_array);
            assert_eq!(rust_same, rust_array);
            assert_eq!(unsafe { snapshot_array(c_same, data_len) }, unsafe {
                snapshot_array(rust_same, data_len)
            });

            let doubled_request = capacity + 1;
            let c_doubled = unsafe { (c.arrgrow)(c_same, elem_size, 0, doubled_request) };
            let rust_doubled = unsafe { (rust.arrgrow)(rust_same, elem_size, 0, doubled_request) };
            assert_eq!(unsafe { (*header(c_doubled)).capacity }, capacity * 2);
            assert_eq!(unsafe { snapshot_array(c_doubled, data_len) }, unsafe {
                snapshot_array(rust_doubled, data_len)
            });

            let large_request = capacity * 4 + 3;
            let c_large = unsafe { (c.arrgrow)(c_doubled, elem_size, 0, large_request) };
            let rust_large = unsafe { (rust.arrgrow)(rust_doubled, elem_size, 0, large_request) };
            assert_eq!(unsafe { (*header(c_large)).capacity }, large_request);
            assert_eq!(unsafe { snapshot_array(c_large, data_len) }, unsafe {
                snapshot_array(rust_large, data_len)
            });

            unsafe {
                (c.arrfree)(c_large);
                (rust.arrfree)(rust_large);
            }
        }
    }
}

#[test]
fn hash_functions_match_every_tail_length_and_random_values() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x5a17_b17e_cafe_f00d);

    for len in 0..128usize {
        for _ in 0..32 {
            let seed = rng.next_usize();
            let mut bytes = vec![0u8; len];
            rng.fill(&mut bytes);
            let pointer = if bytes.is_empty() {
                ptr::null_mut()
            } else {
                bytes.as_mut_ptr().cast()
            };
            assert_eq!(
                unsafe { (c.hash_bytes)(pointer, len, seed) },
                unsafe { (rust.hash_bytes)(pointer, len, seed) },
                "byte hash differs at len={len}, remainder={}",
                len % 8
            );
        }
    }

    for len in 0..96usize {
        for _ in 0..32 {
            let seed = rng.next_usize();
            let mut bytes = vec![0u8; len];
            for byte in &mut bytes {
                *byte = 1 + (rng.next_u64() % 255) as u8;
            }
            if len > 0 && len % 3 == 0 {
                bytes[len / 2] |= 0x80;
            }
            let string = CString::new(bytes).unwrap();
            assert_eq!(
                unsafe { (c.hash_string)(string.as_ptr().cast_mut(), seed) },
                unsafe { (rust.hash_string)(string.as_ptr().cast_mut(), seed) },
                "string hash differs at len={len}"
            );
        }
    }
}

#[test]
fn oversized_and_zero_size_array_arithmetic_matches() {
    let _guard = test_lock();
    let (c, rust) = load_pair();

    let c_zero = unsafe { (c.arrgrow)(ptr::null_mut(), 0, 0, 1) };
    let rust_zero = unsafe { (rust.arrgrow)(ptr::null_mut(), 0, 0, 1) };
    assert_eq!(unsafe { snapshot_array(c_zero, 0) }, unsafe {
        snapshot_array(rust_zero, 0)
    });
    unsafe {
        (c.arrfree)(c_zero);
        (rust.arrfree)(rust_zero);
    }

    let c_wrapped = unsafe { (c.arrgrow)(ptr::null_mut(), usize::MAX, 2, 0) };
    let rust_wrapped = unsafe { (rust.arrgrow)(ptr::null_mut(), usize::MAX, 2, 0) };
    assert_eq!(unsafe { snapshot_array(c_wrapped, 0) }, unsafe {
        snapshot_array(rust_wrapped, 0)
    });
    unsafe {
        (c.arrfree)(c_wrapped);
        (rust.arrfree)(rust_wrapped);
    }
}

#[test]
fn binary_maps_match_creation_lookup_update_growth_and_deletion() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let elem_size = size_of::<IntEntry>();

    let mut c_default = unsafe { (c.hmput_default)(ptr::null_mut(), elem_size) };
    let mut rust_default = unsafe { (rust.hmput_default)(ptr::null_mut(), elem_size) };
    assert_eq!(unsafe { snapshot_map(c_default, elem_size) }, unsafe {
        snapshot_map(rust_default, elem_size)
    });
    let c_default_again = unsafe { (c.hmput_default)(c_default, elem_size) };
    let rust_default_again = unsafe { (rust.hmput_default)(rust_default, elem_size) };
    assert_eq!(c_default_again, c_default);
    assert_eq!(rust_default_again, rust_default);

    let mut key = 123i32;
    let mut c_temp = 99isize;
    let mut rust_temp = 99isize;
    c_default = unsafe {
        (c.hmget_ts)(
            c_default,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut c_temp,
            0,
        )
    };
    rust_default = unsafe {
        (rust.hmget_ts)(
            rust_default,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut rust_temp,
            0,
        )
    };
    assert_eq!((c_temp, rust_temp), (-1, -1));
    assert_eq!(unsafe { snapshot_map(c_default, elem_size) }, unsafe {
        snapshot_map(rust_default, elem_size)
    });
    unsafe {
        free_map(&c, c_default, elem_size);
        free_map(&rust, rust_default, elem_size);
    }

    for mode in [0, -1] {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 0x9e37_79b9_7f4a_7c15] {
            unsafe {
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
            }
            let mut c_map = ptr::null_mut();
            let mut rust_map = ptr::null_mut();
            let mut rng = Rng::new(seed as u64 ^ 0xb1a4_7e55);
            let mut keys = Vec::new();

            for index in 0..384i32 {
                let key = (rng.next_u64() as i32).wrapping_add(index.wrapping_mul(7919));
                let value = rng.next_u64() as i32;
                keys.push(key);
                unsafe {
                    put_int(&c, &mut c_map, key, value, mode);
                    put_int(&rust, &mut rust_map, key, value, mode);
                }
                assert_eq!(
                    unsafe { snapshot_map(c_map, elem_size) },
                    unsafe { snapshot_map(rust_map, elem_size) },
                    "insert mismatch for seed={seed}, index={index}"
                );
            }

            for (index, key) in keys.iter().copied().enumerate().step_by(7) {
                let value = (index as i32).wrapping_mul(-17);
                unsafe {
                    put_int(&c, &mut c_map, key, value, mode);
                    put_int(&rust, &mut rust_map, key, value, mode);
                }
                assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
                    snapshot_map(rust_map, elem_size)
                });
            }

            for key in keys.iter().copied().step_by(11) {
                let mut c_lookup_temp = isize::MIN;
                let mut rust_lookup_temp = isize::MIN;
                let mut key_arg = key;
                c_map = unsafe {
                    (c.hmget_ts)(
                        c_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        &mut c_lookup_temp,
                        mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmget_ts)(
                        rust_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        &mut rust_lookup_temp,
                        mode,
                    )
                };
                assert_eq!(c_lookup_temp, rust_lookup_temp);
                assert!(c_lookup_temp >= 0);

                c_map = unsafe {
                    (c.hmget)(
                        c_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmget)(
                        rust_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        mode,
                    )
                };
                assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
                    snapshot_map(rust_map, elem_size)
                });
            }

            for absent in [i32::MIN, i32::MAX, 0x55aa_33cci32] {
                let mut key_arg = absent;
                let mut c_lookup_temp = 44;
                let mut rust_lookup_temp = 44;
                unsafe {
                    (c.hmget_ts)(
                        c_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        &mut c_lookup_temp,
                        mode,
                    );
                    (rust.hmget_ts)(
                        rust_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        &mut rust_lookup_temp,
                        mode,
                    );
                }
                assert_eq!((c_lookup_temp, rust_lookup_temp), (-1, -1));
            }

            let mut missing = 0x2468_1357i32;
            c_map = unsafe {
                (c.hmdel)(
                    c_map,
                    elem_size,
                    ptr::addr_of_mut!(missing).cast(),
                    size_of::<i32>(),
                    0,
                    mode,
                )
            };
            rust_map = unsafe {
                (rust.hmdel)(
                    rust_map,
                    elem_size,
                    ptr::addr_of_mut!(missing).cast(),
                    size_of::<i32>(),
                    0,
                    mode,
                )
            };
            assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
                snapshot_map(rust_map, elem_size)
            });

            for key in keys.iter().copied().take(300) {
                let mut key_arg = key;
                c_map = unsafe {
                    (c.hmdel)(
                        c_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmdel)(
                        rust_map,
                        elem_size,
                        ptr::addr_of_mut!(key_arg).cast(),
                        size_of::<i32>(),
                        0,
                        mode,
                    )
                };
                assert_eq!(
                    unsafe { snapshot_map(c_map, elem_size) },
                    unsafe { snapshot_map(rust_map, elem_size) },
                    "delete mismatch for key={key}"
                );
            }

            unsafe {
                free_map(&c, c_map, elem_size);
                free_map(&rust, rust_map, elem_size);
            }
        }
    }
}

#[test]
fn binary_key_widths_and_null_map_sentinels_match() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x600d_f00d);

    for &key_size in &[0usize, 1, 4, 8, 16] {
        let elem_size = key_size + 8;
        let mut c_map = ptr::null_mut();
        let mut rust_map = ptr::null_mut();
        for _ in 0..128 {
            let mut key = vec![0u8; key_size];
            rng.fill(&mut key);
            let key_pointer = if key.is_empty() {
                ptr::null_mut()
            } else {
                key.as_mut_ptr().cast()
            };
            c_map = unsafe { (c.hmput)(c_map, elem_size, key_pointer, key_size, 0) };
            rust_map = unsafe { (rust.hmput)(rust_map, elem_size, key_pointer, key_size, 0) };
            let c_raw = unsafe { raw_from_hash(c_map, elem_size) };
            let rust_raw = unsafe { raw_from_hash(rust_map, elem_size) };
            let c_index = unsafe { (*header(c_raw)).temp } as usize;
            let rust_index = unsafe { (*header(rust_raw)).temp } as usize;
            assert_eq!(c_index, rust_index);
            let mut record = vec![0u8; elem_size];
            record[..key_size].copy_from_slice(&key);
            rng.fill(&mut record[key_size..]);
            unsafe {
                ptr::copy_nonoverlapping(
                    record.as_ptr(),
                    c_map.cast::<u8>().add(c_index * elem_size),
                    elem_size,
                );
                ptr::copy_nonoverlapping(
                    record.as_ptr(),
                    rust_map.cast::<u8>().add(rust_index * elem_size),
                    elem_size,
                );
            }
        }
        assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
            snapshot_map(rust_map, elem_size)
        });
        unsafe {
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }
    }

    let mut key = 7i32;
    let mut c_temp = 5isize;
    let mut rust_temp = 5isize;
    let c_map = unsafe {
        (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<IntEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut c_temp,
            0,
        )
    };
    let rust_map = unsafe {
        (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<IntEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            &mut rust_temp,
            0,
        )
    };
    assert_eq!((c_temp, rust_temp), (-1, -1));
    assert_eq!(
        unsafe { snapshot_map(c_map, size_of::<IntEntry>()) },
        unsafe { snapshot_map(rust_map, size_of::<IntEntry>()) }
    );
    unsafe {
        free_map(&c, c_map, size_of::<IntEntry>());
        free_map(&rust, rust_map, size_of::<IntEntry>());
        assert!(
            (c.hmdel)(
                ptr::null_mut(),
                size_of::<IntEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<i32>(),
                0,
                0
            )
            .is_null()
        );
        assert!(
            (rust.hmdel)(
                ptr::null_mut(),
                size_of::<IntEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<i32>(),
                0,
                0
            )
            .is_null()
        );
        (c.hmfree)(ptr::null_mut(), size_of::<IntEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<IntEntry>());
    }
}

#[test]
fn string_maps_match_all_storage_modes_and_operations() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let elem_size = size_of::<StringEntry>();

    for storage_mode in [1, 2, 3] {
        for comparison_mode in [1, 2, i32::MAX] {
            unsafe {
                (c.rand_seed)(0x51a1_6eed + storage_mode as usize);
                (rust.rand_seed)(0x51a1_6eed + storage_mode as usize);
            }
            let mut c_map = unsafe { (c.shmode)(elem_size, storage_mode) };
            let mut rust_map = unsafe { (rust.shmode)(elem_size, storage_mode) };
            let strings: Vec<CString> = (0..192)
                .map(|index| CString::new(format!("key_{storage_mode}_{index:04}")).unwrap())
                .collect();

            for (index, string) in strings.iter().enumerate() {
                unsafe {
                    put_string(
                        &c,
                        &mut c_map,
                        string.as_ptr().cast_mut(),
                        index as i32 * 13,
                        comparison_mode,
                    );
                    put_string(
                        &rust,
                        &mut rust_map,
                        string.as_ptr().cast_mut(),
                        index as i32 * 13,
                        comparison_mode,
                    );
                }
                assert_eq!(
                    unsafe { snapshot_string_map(c_map) },
                    unsafe { snapshot_string_map(rust_map) },
                    "string insert differs in storage mode {storage_mode}"
                );
            }

            for (index, string) in strings.iter().enumerate().step_by(9) {
                unsafe {
                    put_string(
                        &c,
                        &mut c_map,
                        string.as_ptr().cast_mut(),
                        -(index as i32),
                        comparison_mode,
                    );
                    put_string(
                        &rust,
                        &mut rust_map,
                        string.as_ptr().cast_mut(),
                        -(index as i32),
                        comparison_mode,
                    );
                }
            }
            assert_eq!(unsafe { snapshot_string_map(c_map) }, unsafe {
                snapshot_string_map(rust_map)
            });

            for string in strings.iter().step_by(13) {
                let mut c_temp = isize::MIN;
                let mut rust_temp = isize::MIN;
                c_map = unsafe {
                    (c.hmget_ts)(
                        c_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut c_temp,
                        comparison_mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmget_ts)(
                        rust_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        &mut rust_temp,
                        comparison_mode,
                    )
                };
                assert_eq!(c_temp, rust_temp);
                assert!(c_temp >= 0);

                c_map = unsafe {
                    (c.hmget)(
                        c_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        comparison_mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmget)(
                        rust_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        comparison_mode,
                    )
                };
                assert_eq!(unsafe { snapshot_string_map(c_map) }, unsafe {
                    snapshot_string_map(rust_map)
                });
            }

            let absent = CString::new("not_present").unwrap();
            let mut c_temp = 77isize;
            let mut rust_temp = 77isize;
            unsafe {
                (c.hmget_ts)(
                    c_map,
                    elem_size,
                    absent.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    comparison_mode,
                );
                (rust.hmget_ts)(
                    rust_map,
                    elem_size,
                    absent.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut rust_temp,
                    comparison_mode,
                );
            }
            assert_eq!((c_temp, rust_temp), (-1, -1));

            let delete_keys: Vec<&CString> = if comparison_mode == 1 {
                strings.iter().take(150).collect()
            } else {
                strings.iter().rev().take(24).collect()
            };
            for string in delete_keys {
                c_map = unsafe {
                    (c.hmdel)(
                        c_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        comparison_mode,
                    )
                };
                rust_map = unsafe {
                    (rust.hmdel)(
                        rust_map,
                        elem_size,
                        string.as_ptr().cast_mut().cast(),
                        size_of::<*mut c_char>(),
                        0,
                        comparison_mode,
                    )
                };
                assert_eq!(unsafe { snapshot_string_map(c_map) }, unsafe {
                    snapshot_string_map(rust_map)
                });
            }

            unsafe {
                free_map(&c, c_map, elem_size);
                free_map(&rust, rust_map, elem_size);
            }
        }
    }
}

#[test]
fn shmode_none_and_out_of_range_modes_match() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let elem_size = size_of::<IntEntry>();

    let mut c_map = unsafe { (c.shmode)(elem_size, 0) };
    let mut rust_map = unsafe { (rust.shmode)(elem_size, 0) };
    for key in -64i32..64 {
        unsafe {
            put_int(&c, &mut c_map, key, key.wrapping_mul(31), 0);
            put_int(&rust, &mut rust_map, key, key.wrapping_mul(31), 0);
        }
    }
    assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
        snapshot_map(rust_map, elem_size)
    });
    unsafe {
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);
    }

    for mode in [-1, 4, i32::MAX] {
        let c_map = unsafe { (c.shmode)(elem_size, mode) };
        let rust_map = unsafe { (rust.shmode)(elem_size, mode) };
        assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
            snapshot_map(rust_map, elem_size)
        });
        unsafe {
            free_map(&c, c_map, elem_size);
            free_map(&rust, rust_map, elem_size);
        }
    }
}

#[test]
fn string_arenas_match_short_large_and_max_block_paths() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let mut c_arena = StringArena::default();
    let mut rust_arena = StringArena::default();
    let mut rng = Rng::new(0xa2e4_9a11);

    let mut lengths = vec![0usize, 1, 7, 63, 127, 255, 510, 511, 512, 513, 777, 2048];
    for _ in 0..64 {
        lengths.push(rng.next_usize() % 4096);
    }
    for len in lengths {
        let bytes: Vec<u8> = (0..len)
            .map(|_| b'a' + (rng.next_u64() % 26) as u8)
            .collect();
        let string = CString::new(bytes.clone()).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, string.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(c_result) }.to_bytes(), bytes);
        assert_eq!(unsafe { CStr::from_ptr(c_result) }, unsafe {
            CStr::from_ptr(rust_result)
        });
        assert_eq!(c_arena.remaining, rust_arena.remaining);
        assert_eq!(c_arena.block, rust_arena.block);
        assert_eq!(c_arena.mode, rust_arena.mode);
    }

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
    assert!(c_arena.storage.is_null());
    assert!(rust_arena.storage.is_null());
    assert_eq!(
        (c_arena.remaining, c_arena.block, c_arena.mode),
        (rust_arena.remaining, rust_arena.block, rust_arena.mode)
    );

    // Repeated near-capacity allocations advance the exponent until the
    // source's 1 MiB cap branch stops incrementing the block state.
    for exponent_step in 0..24 {
        let len = (512usize << (exponent_step / 2).min(11)).min(1 << 20);
        let string = CString::new(vec![b'x'; len.saturating_sub(1)]).unwrap();
        let c_result = unsafe { (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut()) };
        let rust_result = unsafe { (rust.stralloc)(&mut rust_arena, string.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(c_result) }, unsafe {
            CStr::from_ptr(rust_result)
        });
        assert_eq!(c_arena.remaining, rust_arena.remaining);
        assert_eq!(c_arena.block, rust_arena.block);
    }

    let oversized = CString::new(vec![b'z'; (1 << 20) + 37]).unwrap();
    let c_large = unsafe { (c.stralloc)(&mut c_arena, oversized.as_ptr().cast_mut()) };
    let rust_large = unsafe { (rust.stralloc)(&mut rust_arena, oversized.as_ptr().cast_mut()) };
    assert_eq!(unsafe { CStr::from_ptr(c_large) }, unsafe {
        CStr::from_ptr(rust_large)
    });
    assert_eq!(c_arena.remaining, rust_arena.remaining);
    assert_eq!(c_arena.block, rust_arena.block);

    unsafe {
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut rust_arena);
    }
    assert!(c_arena.storage.is_null());
    assert!(rust_arena.storage.is_null());
}

#[test]
fn strkey_and_intput_valid_domain_match() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let mut rng = Rng::new(0x1a2b_3c4d);
    let mut values = vec![i32::MIN, -1, 0, 1, 9, 11, i32::MAX];
    values.extend((0..512).map(|_| rng.next_u64() as i32));

    for value in values {
        let c_result = unsafe { CStr::from_ptr((c.strkey)(value)) }
            .to_bytes()
            .to_vec();
        let rust_result = unsafe { CStr::from_ptr((rust.strkey)(value)) }
            .to_bytes()
            .to_vec();
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, format!("test_{value}").as_bytes());
    }

    for _ in 0..512 {
        let mut value = rng.next_u64() as i32;
        if value == 9 || value == 11 {
            value = value.wrapping_add(1);
        }
        unsafe {
            (c.intput)(value);
            (rust.intput)(value);
        }
    }
}

#[test]
fn source_sentinel_error_paths_match_exactly() {
    let _guard = test_lock();
    let (c, rust) = load_pair();
    let elem_size = size_of::<IntEntry>();
    let mut key = 0x1357_2468i32;

    let mut c_no_table = unsafe { (c.hmput_default)(ptr::null_mut(), elem_size) };
    let mut rust_no_table = unsafe { (rust.hmput_default)(ptr::null_mut(), elem_size) };
    let c_before = c_no_table;
    let rust_before = rust_no_table;
    c_no_table = unsafe {
        (c.hmdel)(
            c_no_table,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        )
    };
    rust_no_table = unsafe {
        (rust.hmdel)(
            rust_no_table,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        )
    };
    assert_eq!(c_no_table, c_before);
    assert_eq!(rust_no_table, rust_before);
    assert_eq!(unsafe { snapshot_map(c_no_table, elem_size) }, unsafe {
        snapshot_map(rust_no_table, elem_size)
    });
    assert_eq!(
        unsafe { (*header(raw_from_hash(c_no_table, elem_size))).temp },
        0
    );
    unsafe {
        free_map(&c, c_no_table, elem_size);
        free_map(&rust, rust_no_table, elem_size);
    }

    let mut c_map = ptr::null_mut();
    let mut rust_map = ptr::null_mut();
    unsafe {
        put_int(&c, &mut c_map, 1, 10, 0);
        put_int(&rust, &mut rust_map, 1, 10, 0);
    }
    let c_before = c_map;
    let rust_before = rust_map;
    c_map = unsafe {
        (c.hmdel)(
            c_map,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        )
    };
    rust_map = unsafe {
        (rust.hmdel)(
            rust_map,
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
            0,
        )
    };
    assert_eq!(c_map, c_before);
    assert_eq!(rust_map, rust_before);
    assert_eq!(
        unsafe { (*header(raw_from_hash(c_map, elem_size))).temp },
        0
    );
    assert_eq!(unsafe { snapshot_map(c_map, elem_size) }, unsafe {
        snapshot_map(rust_map, elem_size)
    });
    unsafe {
        free_map(&c, c_map, elem_size);
        free_map(&rust, rust_map, elem_size);
    }

    let c_from_get = unsafe {
        (c.hmget)(
            ptr::null_mut(),
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
        )
    };
    let rust_from_get = unsafe {
        (rust.hmget)(
            ptr::null_mut(),
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<i32>(),
            0,
        )
    };
    assert_eq!(unsafe { snapshot_map(c_from_get, elem_size) }, unsafe {
        snapshot_map(rust_from_get, elem_size)
    });
    assert_eq!(
        unsafe { (*header(raw_from_hash(c_from_get, elem_size))).temp },
        -1
    );
    unsafe {
        free_map(&c, c_from_get, elem_size);
        free_map(&rust, rust_from_get, elem_size);
    }
}

fn run_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("child_probe")
        .arg("--nocapture")
        .env("FFI_CHILD_LIBRARY", library)
        .env("FFI_CHILD_CASE", case)
        .output()
        .unwrap_or_else(|error| panic!("failed to run child probe {case}: {error}"))
        .status
}

#[cfg(unix)]
fn assert_same_termination(case: &str, c_status: ExitStatus, rust_status: ExitStatus) {
    use std::os::unix::process::ExitStatusExt;

    assert!(!c_status.success(), "C probe {case} unexpectedly succeeded");
    assert!(
        !rust_status.success(),
        "Rust probe {case} unexpectedly succeeded"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "termination signal differs for {case}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
fn process_terminating_error_boundaries_match() {
    let _guard = test_lock();
    for case in [
        "intput_9",
        "intput_11",
        "hash_string_null",
        "hash_bytes_null_positive",
        "hash_bytes_null_oversized",
        "arrfree_null",
        "hmget_temp_null",
        "hmget_existing_null_key",
        "hmdel_existing_null_key",
        "hmput_binary_null_key",
        "stralloc_null_string",
        "stralloc_null_arena",
        "strreset_null",
        "hmput_string_null",
        "hmdel_out_of_range_mode",
    ] {
        let c_status = run_child(&c_library_path(), case);
        let rust_status = run_child(&rust_library_path(), case);
        #[cfg(unix)]
        assert_same_termination(case, c_status, rust_status);
    }
}

#[test]
fn child_probe() {
    let Ok(library_path) = env::var("FFI_CHILD_LIBRARY") else {
        return;
    };
    let case = env::var("FFI_CHILD_CASE").expect("child case");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let mut key = 7i32;
    let mut arena = StringArena::default();
    let string = CString::new("child").unwrap();

    unsafe {
        match case.as_str() {
            "intput_9" => (api.intput)(9),
            "intput_11" => (api.intput)(11),
            "hash_string_null" => {
                (api.hash_string)(ptr::null_mut(), 0);
            }
            "hash_bytes_null_positive" => {
                (api.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hash_bytes_null_oversized" => {
                (api.hash_bytes)(ptr::null_mut(), usize::MAX, 0);
            }
            "arrfree_null" => {
                (api.arrfree)(ptr::null_mut());
            }
            "hmget_temp_null" => {
                (api.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<IntEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<i32>(),
                    ptr::null_mut(),
                    0,
                );
            }
            "hmget_existing_null_key" => {
                let mut map = ptr::null_mut();
                put_int(&api, &mut map, 7, 9, 0);
                (api.hmget)(
                    map,
                    size_of::<IntEntry>(),
                    ptr::null_mut(),
                    size_of::<i32>(),
                    0,
                );
            }
            "hmdel_existing_null_key" => {
                let mut map = ptr::null_mut();
                put_int(&api, &mut map, 7, 9, 0);
                (api.hmdel)(
                    map,
                    size_of::<IntEntry>(),
                    ptr::null_mut(),
                    size_of::<i32>(),
                    0,
                    0,
                );
            }
            "hmput_binary_null_key" => {
                (api.hmput)(
                    ptr::null_mut(),
                    size_of::<IntEntry>(),
                    ptr::null_mut(),
                    size_of::<i32>(),
                    0,
                );
            }
            "stralloc_null_string" => {
                (api.stralloc)(&mut arena, ptr::null_mut());
            }
            "stralloc_null_arena" => {
                (api.stralloc)(ptr::null_mut(), string.as_ptr().cast_mut());
            }
            "strreset_null" => {
                (api.strreset)(ptr::null_mut());
            }
            "hmput_string_null" => {
                (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    1,
                );
            }
            "hmdel_out_of_range_mode" => {
                let strings = [
                    CString::new("first").unwrap(),
                    CString::new("second").unwrap(),
                ];
                let mut map = (api.shmode)(size_of::<StringEntry>(), 1);
                put_string(&api, &mut map, strings[0].as_ptr().cast_mut(), 1, 2);
                put_string(&api, &mut map, strings[1].as_ptr().cast_mut(), 2, 2);
                (api.hmdel)(
                    map,
                    size_of::<StringEntry>(),
                    strings[0].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    2,
                );
            }
            other => panic!("unknown child case {other}"),
        }
    }

    // Keep these live across calls that receive their pointers.
    std::hint::black_box(string);
}
