use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::Path;
use std::ptr::{NonNull, null_mut};

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
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ArrDel = unsafe extern "C" fn(c_int);

struct Api {
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
    arr_del: ArrDel,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap_or_else(|e| {
            panic!("failed to load {}: {e}", path.display());
        });
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let value = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|e| panic!("missing {} in {}: {e}", $name, path.display()));
                *value
            }};
        }
        Self {
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
            arr_del: symbol!("arr_del", ArrDel),
            _library: library,
        }
    }
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Pair {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringPair {
    key: *mut c_char,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Arena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Default for Arena {
    fn default() -> Self {
        Self {
            storage: null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

unsafe fn header(array: *mut c_void) -> *mut Header {
    unsafe { (array as *mut Header).sub(1) }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut Header {
    unsafe { header(raw_map(map, element_size)) }
}

unsafe fn set_array_len(array: *mut c_void, length: usize) {
    unsafe { (*header(array)).length = length };
}

unsafe fn array_state(array: *mut c_void, element_size: usize) -> (usize, usize, Vec<u8>) {
    if array.is_null() {
        return (0, 0, Vec::new());
    }
    let hdr = unsafe { &*header(array) };
    let bytes = unsafe {
        std::slice::from_raw_parts(array.cast::<u8>(), hdr.length * element_size).to_vec()
    };
    (hdr.length, hdr.capacity, bytes)
}

unsafe fn binary_state(map: *mut c_void) -> (usize, usize, isize, Vec<Pair>) {
    let hdr = unsafe { &*map_header(map, size_of::<Pair>()) };
    let len = hdr.length - 1;
    let entries = unsafe { std::slice::from_raw_parts(map.cast::<Pair>(), len).to_vec() };
    (len, hdr.capacity, hdr.temp, entries)
}

unsafe fn string_state(
    map: *mut c_void,
) -> (usize, usize, isize, Option<Vec<u8>>, Vec<(Vec<u8>, i64)>) {
    let hdr = unsafe { &*map_header(map, size_of::<StringPair>()) };
    let len = hdr.length - 1;
    let temp_key = if len == 0 || hdr.hash_table.is_null() {
        None
    } else {
        let pointer = unsafe { *(hdr.hash_table as *mut *mut c_char) };
        if pointer.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(pointer) }.to_bytes().to_vec())
        }
    };
    let mut entries = Vec::with_capacity(len);
    for pair in unsafe { std::slice::from_raw_parts(map.cast::<StringPair>(), len) } {
        let key = unsafe { CStr::from_ptr(pair.key) }.to_bytes().to_vec();
        entries.push((key, pair.value));
    }
    (len, hdr.capacity, hdr.temp, temp_key, entries)
}

unsafe fn put_binary(api: &Api, map: &mut *mut c_void, key: u64, value: i64, mode: c_int) {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmput)(
            *map,
            size_of::<Pair>(),
            (&mut key_copy as *mut u64).cast(),
            size_of::<u64>(),
            mode,
        )
    };
    let index = unsafe { (*map_header(*map, size_of::<Pair>())).temp };
    assert!(index >= 0);
    unsafe { (*map.cast::<Pair>().offset(index)).value = value };
}

unsafe fn get_binary_ts(api: &Api, map: &mut *mut c_void, key: u64, mode: c_int) -> (isize, i64) {
    let mut key_copy = key;
    let mut temp = 999;
    let result = unsafe {
        (api.hmget_ts)(
            *map,
            size_of::<Pair>(),
            (&mut key_copy as *mut u64).cast(),
            size_of::<u64>(),
            &mut temp,
            mode,
        )
    };
    *map = result;
    let value = if temp >= 0 {
        unsafe { (*map.cast::<Pair>().offset(temp)).value }
    } else {
        0
    };
    (temp, value)
}

unsafe fn get_binary(api: &Api, map: &mut *mut c_void, key: u64, mode: c_int) -> (isize, i64) {
    let mut key_copy = key;
    let result = unsafe {
        (api.hmget)(
            *map,
            size_of::<Pair>(),
            (&mut key_copy as *mut u64).cast(),
            size_of::<u64>(),
            mode,
        )
    };
    *map = result;
    let temp = unsafe { (*map_header(*map, size_of::<Pair>())).temp };
    let value = if temp >= 0 {
        unsafe { (*map.cast::<Pair>().offset(temp)).value }
    } else {
        0
    };
    (temp, value)
}

unsafe fn del_binary(api: &Api, map: &mut *mut c_void, key: u64, mode: c_int) -> isize {
    let mut key_copy = key;
    *map = unsafe {
        (api.hmdel)(
            *map,
            size_of::<Pair>(),
            (&mut key_copy as *mut u64).cast(),
            size_of::<u64>(),
            0,
            mode,
        )
    };
    unsafe { (*map_header(*map, size_of::<Pair>())).temp }
}

unsafe fn put_string(
    api: &Api,
    map: &mut *mut c_void,
    key: &CString,
    value: i64,
    hash_mode: c_int,
) {
    *map = unsafe {
        (api.hmput)(
            *map,
            size_of::<StringPair>(),
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            hash_mode,
        )
    };
    let index = unsafe { (*map_header(*map, size_of::<StringPair>())).temp };
    assert!(index >= 0);
    unsafe { (*map.cast::<StringPair>().offset(index)).value = value };
}

unsafe fn get_string_ts(
    api: &Api,
    map: *mut c_void,
    key: &CString,
    hash_mode: c_int,
) -> (isize, i64) {
    let mut temp = 999;
    let result = unsafe {
        (api.hmget_ts)(
            map,
            size_of::<StringPair>(),
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            &mut temp,
            hash_mode,
        )
    };
    assert_eq!(result, map);
    let value = if temp >= 0 {
        unsafe { (*map.cast::<StringPair>().offset(temp)).value }
    } else {
        0
    };
    (temp, value)
}

unsafe fn get_string(api: &Api, map: *mut c_void, key: &CString, hash_mode: c_int) -> (isize, i64) {
    let result = unsafe {
        (api.hmget)(
            map,
            size_of::<StringPair>(),
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            hash_mode,
        )
    };
    assert_eq!(result, map);
    let temp = unsafe { (*map_header(map, size_of::<StringPair>())).temp };
    let value = if temp >= 0 {
        unsafe { (*map.cast::<StringPair>().offset(temp)).value }
    } else {
        0
    };
    (temp, value)
}

unsafe fn del_string(api: &Api, map: &mut *mut c_void, key: &CString, hash_mode: c_int) -> isize {
    *map = unsafe {
        (api.hmdel)(
            *map,
            size_of::<StringPair>(),
            key.as_ptr().cast_mut().cast(),
            size_of::<*mut c_char>(),
            0,
            hash_mode,
        )
    };
    unsafe { (*map_header(*map, size_of::<StringPair>())).temp }
}

unsafe fn check_arrays(c: &Api, rust: &Api) {
    for element_size in [1usize, 4, 16] {
        let empty_c = unsafe { (c.arrgrow)(null_mut(), element_size, 0, 0) };
        let empty_r = unsafe { (rust.arrgrow)(null_mut(), element_size, 0, 0) };
        assert!(empty_c.is_null());
        assert!(empty_r.is_null());

        let mut ca = unsafe { (c.arrgrow)(null_mut(), element_size, 1, 0) };
        let mut ra = unsafe { (rust.arrgrow)(null_mut(), element_size, 1, 0) };
        assert_eq!(unsafe { array_state(ca, element_size) }, unsafe {
            array_state(ra, element_size)
        });
        assert_eq!(unsafe { (*header(ca)).capacity }, 4);

        unsafe {
            set_array_len(ca, 3);
            set_array_len(ra, 3);
        }
        for index in 0..3 * element_size {
            let byte = (index as u8)
                .wrapping_mul(37)
                .wrapping_add(element_size as u8);
            unsafe {
                *ca.cast::<u8>().add(index) = byte;
                *ra.cast::<u8>().add(index) = byte;
            }
        }

        let old_c = ca;
        let old_r = ra;
        ca = unsafe { (c.arrgrow)(ca, element_size, 0, 2) };
        ra = unsafe { (rust.arrgrow)(ra, element_size, 0, 2) };
        assert_eq!(ca, old_c);
        assert_eq!(ra, old_r);
        assert_eq!(unsafe { array_state(ca, element_size) }, unsafe {
            array_state(ra, element_size)
        });

        ca = unsafe { (c.arrgrow)(ca, element_size, 2, 0) };
        ra = unsafe { (rust.arrgrow)(ra, element_size, 2, 0) };
        assert_eq!(unsafe { (*header(ca)).capacity }, 8);
        assert_eq!(unsafe { array_state(ca, element_size) }, unsafe {
            array_state(ra, element_size)
        });

        ca = unsafe { (c.arrgrow)(ca, element_size, 0, 41) };
        ra = unsafe { (rust.arrgrow)(ra, element_size, 0, 41) };
        assert_eq!(unsafe { (*header(ca)).capacity }, 41);
        assert_eq!(unsafe { array_state(ca, element_size) }, unsafe {
            array_state(ra, element_size)
        });

        unsafe {
            (c.arrfree)(ca);
            (rust.arrfree)(ra);
        }
    }
}

unsafe fn check_hashes(c: &Api, rust: &Api) {
    let mut rng = Rng(0x4d59_5df4_d0f3_3173);
    for length in 0..=79usize {
        for _ in 0..40 {
            let seed = rng.next() as usize;
            let mut bytes = rng.bytes(length);
            let cp = if length == 0 {
                if seed & 1 == 0 {
                    null_mut()
                } else {
                    NonNull::<u8>::dangling().as_ptr().cast()
                }
            } else {
                bytes.as_mut_ptr().cast()
            };
            let rp = cp;
            assert_eq!(
                unsafe { (c.hash_bytes)(cp, length, seed) },
                unsafe { (rust.hash_bytes)(rp, length, seed) },
                "hash_bytes length={length} seed={seed:#x}"
            );
        }
    }

    let fixed = [
        vec![0],
        vec![b'a', 0],
        b"many bytes in a string\0".to_vec(),
        vec![0x80, 0],
        vec![0xff, 0x81, b'x', 0],
    ];
    for mut bytes in fixed {
        for _ in 0..100 {
            let seed = rng.next() as usize;
            assert_eq!(
                unsafe { (c.hash_string)(bytes.as_mut_ptr().cast(), seed) },
                unsafe { (rust.hash_string)(bytes.as_mut_ptr().cast(), seed) }
            );
        }
    }
    for _ in 0..1000 {
        let len = (rng.next() % 48) as usize;
        let mut bytes: Vec<u8> = (0..len)
            .map(|_| {
                let value = rng.next() as u8;
                if value == 0 { 1 } else { value }
            })
            .collect();
        bytes.push(0);
        let seed = rng.next() as usize;
        assert_eq!(
            unsafe { (c.hash_string)(bytes.as_mut_ptr().cast(), seed) },
            unsafe { (rust.hash_string)(bytes.as_mut_ptr().cast(), seed) }
        );
    }
}

unsafe fn check_default_and_null_paths(c: &Api, rust: &Api) {
    unsafe {
        (c.hmfree)(null_mut(), size_of::<Pair>());
        (rust.hmfree)(null_mut(), size_of::<Pair>());
    }
    let mut key = 17u64;
    assert!(
        unsafe {
            (c.hmdel)(
                null_mut(),
                size_of::<Pair>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        }
        .is_null()
    );
    assert!(
        unsafe {
            (rust.hmdel)(
                null_mut(),
                size_of::<Pair>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        }
        .is_null()
    );

    let cm = unsafe { (c.hmput_default)(null_mut(), size_of::<Pair>()) };
    let rm = unsafe { (rust.hmput_default)(null_mut(), size_of::<Pair>()) };
    assert_eq!(unsafe { binary_state(cm) }, unsafe { binary_state(rm) });
    assert_eq!(unsafe { binary_state(cm).3 }, Vec::<Pair>::new());
    assert_eq!(unsafe { *cm.cast::<Pair>().offset(-1) }, Pair::default());
    assert_eq!(unsafe { *rm.cast::<Pair>().offset(-1) }, Pair::default());
    assert_eq!(unsafe { (c.hmput_default)(cm, size_of::<Pair>()) }, cm);
    assert_eq!(unsafe { (rust.hmput_default)(rm, size_of::<Pair>()) }, rm);

    let mut temp_c = 12;
    let mut temp_r = 12;
    let mut missing = 777u64;
    let cm = unsafe {
        (c.hmget_ts)(
            cm,
            size_of::<Pair>(),
            (&mut missing as *mut u64).cast(),
            size_of::<u64>(),
            &mut temp_c,
            0,
        )
    };
    let rm = unsafe {
        (rust.hmget_ts)(
            rm,
            size_of::<Pair>(),
            (&mut missing as *mut u64).cast(),
            size_of::<u64>(),
            &mut temp_r,
            0,
        )
    };
    assert_eq!((temp_c, temp_r), (-1, -1));
    assert_eq!(unsafe { binary_state(cm) }, unsafe { binary_state(rm) });

    assert_eq!(
        unsafe {
            (c.hmdel)(
                cm,
                size_of::<Pair>(),
                (&mut missing as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        },
        cm
    );
    assert_eq!(
        unsafe {
            (rust.hmdel)(
                rm,
                size_of::<Pair>(),
                (&mut missing as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            )
        },
        rm
    );
    assert_eq!(unsafe { (*map_header(cm, size_of::<Pair>())).temp }, 0);
    assert_eq!(unsafe { (*map_header(rm, size_of::<Pair>())).temp }, 0);
    unsafe {
        (c.hmfree)(raw_map(cm, size_of::<Pair>()), size_of::<Pair>());
        (rust.hmfree)(raw_map(rm, size_of::<Pair>()), size_of::<Pair>());
    }

    let mut tc = 5;
    let mut tr = 5;
    let cm = unsafe {
        (c.hmget_ts)(
            null_mut(),
            size_of::<Pair>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut tc,
            0,
        )
    };
    let rm = unsafe {
        (rust.hmget_ts)(
            null_mut(),
            size_of::<Pair>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut tr,
            0,
        )
    };
    assert_eq!((tc, tr), (-1, -1));
    assert_eq!(unsafe { binary_state(cm) }, unsafe { binary_state(rm) });
    unsafe {
        (c.hmfree)(raw_map(cm, size_of::<Pair>()), size_of::<Pair>());
        (rust.hmfree)(raw_map(rm, size_of::<Pair>()), size_of::<Pair>());
    }
}

unsafe fn check_binary_maps(c: &Api, rust: &Api) {
    for storage_mode in [0, -1, 256] {
        let mut cm = unsafe { (c.shmode)(size_of::<Pair>(), storage_mode) };
        let mut rm = unsafe { (rust.shmode)(size_of::<Pair>(), storage_mode) };
        unsafe {
            put_binary(c, &mut cm, 0x1122_3344_5566_7788, -91, 0);
            put_binary(rust, &mut rm, 0x1122_3344_5566_7788, -91, 0);
        }
        assert_eq!(unsafe { binary_state(cm) }, unsafe { binary_state(rm) });
        unsafe {
            (c.hmfree)(raw_map(cm, size_of::<Pair>()), size_of::<Pair>());
            (rust.hmfree)(raw_map(rm, size_of::<Pair>()), size_of::<Pair>());
        }
    }

    for mode in [0, -7] {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            unsafe {
                (c.rand_seed)(seed);
                (rust.rand_seed)(seed);
            }
            let mut cm = null_mut();
            let mut rm = null_mut();
            let mut present = std::collections::BTreeMap::<u64, i64>::new();
            let mut rng = Rng(seed as u64 ^ 0xa076_1d64_78bd_642f);

            for step in 0..700 {
                let key = rng.next() % 173;
                match rng.next() % 5 {
                    0 | 1 | 2 => {
                        let value = rng.next() as i64;
                        unsafe {
                            put_binary(c, &mut cm, key, value, mode);
                            put_binary(rust, &mut rm, key, value, mode);
                        }
                        present.insert(key, value);
                    }
                    3 => {
                        let c_result = unsafe { get_binary_ts(c, &mut cm, key, mode) };
                        let r_result = unsafe { get_binary_ts(rust, &mut rm, key, mode) };
                        assert_eq!(c_result, r_result, "TS get step={step} mode={mode}");
                        assert_eq!(c_result.0 >= 0, present.contains_key(&key));
                        if let Some(value) = present.get(&key) {
                            assert_eq!(c_result.1, *value);
                        }
                        let c_result = unsafe { get_binary(c, &mut cm, key, mode) };
                        let r_result = unsafe { get_binary(rust, &mut rm, key, mode) };
                        assert_eq!(c_result, r_result, "get step={step} mode={mode}");
                    }
                    _ => {
                        let expected = present.remove(&key).is_some() as isize;
                        let cd = if cm.is_null() {
                            0
                        } else {
                            unsafe { del_binary(c, &mut cm, key, mode) }
                        };
                        let rd = if rm.is_null() {
                            0
                        } else {
                            unsafe { del_binary(rust, &mut rm, key, mode) }
                        };
                        assert_eq!((cd, rd), (expected, expected));
                    }
                }
                if !cm.is_null() {
                    assert_eq!(
                        unsafe { binary_state(cm) },
                        unsafe { binary_state(rm) },
                        "binary map state step={step} mode={mode} seed={seed:#x}"
                    );
                } else {
                    assert!(rm.is_null());
                }
            }

            for (&key, &value) in &present {
                let cr = unsafe { get_binary_ts(c, &mut cm, key, mode) };
                let rr = unsafe { get_binary_ts(rust, &mut rm, key, mode) };
                assert_eq!(cr, rr);
                assert_eq!(cr.1, value);
            }
            if !cm.is_null() {
                unsafe {
                    (c.hmfree)(raw_map(cm, size_of::<Pair>()), size_of::<Pair>());
                    (rust.hmfree)(raw_map(rm, size_of::<Pair>()), size_of::<Pair>());
                }
            }
        }
    }
}

unsafe fn check_string_maps(c: &Api, rust: &Api) {
    let keys: Vec<CString> = (0..120)
        .map(|index| CString::new(format!("key_{index:03}_{}", index * 7919)).unwrap())
        .collect();

    for storage_mode in [1, 2, 3, 257] {
        unsafe {
            (c.rand_seed)(0x8877_6655);
            (rust.rand_seed)(0x8877_6655);
        }
        let mut cm = unsafe { (c.shmode)(size_of::<StringPair>(), storage_mode) };
        let mut rm = unsafe { (rust.shmode)(size_of::<StringPair>(), storage_mode) };
        assert_eq!(unsafe { string_state(cm) }, unsafe { string_state(rm) });

        for (index, key) in keys.iter().enumerate() {
            let value = (index as i64).wrapping_mul(-912_367);
            unsafe {
                put_string(c, &mut cm, key, value, 1);
                put_string(rust, &mut rm, key, value, 1);
            }
            assert_eq!(
                unsafe { string_state(cm) },
                unsafe { string_state(rm) },
                "string put mode={storage_mode} index={index}"
            );
        }

        for index in (0..keys.len()).step_by(7) {
            let value = index as i64 * 33 + 4;
            unsafe {
                put_string(c, &mut cm, &keys[index], value, 9);
                put_string(rust, &mut rm, &keys[index], value, 9);
            }
        }
        for (index, key) in keys.iter().enumerate() {
            let cr = unsafe { get_string_ts(c, cm, key, 1) };
            let rr = unsafe { get_string_ts(rust, rm, key, 1) };
            assert_eq!(cr, rr, "string get mode={storage_mode} index={index}");
            let cr = unsafe { get_string(c, cm, key, 1) };
            let rr = unsafe { get_string(rust, rm, key, 1) };
            assert_eq!(
                cr, rr,
                "string non-TS get mode={storage_mode} index={index}"
            );
        }
        let absent = CString::new("absent-key").unwrap();
        assert_eq!(unsafe { get_string_ts(c, cm, &absent, 5) }, unsafe {
            get_string_ts(rust, rm, &absent, 5)
        });

        for index in (0..keys.len()).step_by(3) {
            let cd = unsafe { del_string(c, &mut cm, &keys[index], 1) };
            let rd = unsafe { del_string(rust, &mut rm, &keys[index], 1) };
            assert_eq!((cd, rd), (1, 1));
            assert_eq!(
                unsafe { string_state(cm) },
                unsafe { string_state(rm) },
                "string delete mode={storage_mode} index={index}"
            );
        }
        assert_eq!(unsafe { del_string(c, &mut cm, &absent, 1) }, unsafe {
            del_string(rust, &mut rm, &absent, 1)
        });
        unsafe {
            (c.hmfree)(
                raw_map(cm, size_of::<StringPair>()),
                size_of::<StringPair>(),
            );
            (rust.hmfree)(
                raw_map(rm, size_of::<StringPair>()),
                size_of::<StringPair>(),
            );
        }
    }
}

unsafe fn check_arenas(c: &Api, rust: &Api) {
    let mut ca = Arena::default();
    let mut ra = Arena::default();
    unsafe {
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
    }
    assert_eq!(
        (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
        (ra.storage.is_null(), ra.remaining, ra.block, ra.mode)
    );

    let mut inputs = vec![
        CString::new("").unwrap(),
        CString::new("x").unwrap(),
        CString::new("a".repeat(510)).unwrap(),
        CString::new("b".repeat(511)).unwrap(),
        CString::new("c".repeat(512)).unwrap(),
        CString::new("d".repeat(1500)).unwrap(),
    ];
    for index in 0..900 {
        inputs.push(CString::new(format!("small_{index:04}")).unwrap());
    }
    for input in &inputs {
        let cp = unsafe { (c.stralloc)(&mut ca, input.as_ptr().cast_mut()) };
        let rp = unsafe { (rust.stralloc)(&mut ra, input.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(cp) }, input.as_c_str());
        assert_eq!(unsafe { CStr::from_ptr(rp) }, input.as_c_str());
        assert_eq!(
            (ca.remaining, ca.block, ca.mode),
            (ra.remaining, ra.block, ra.mode)
        );
    }
    unsafe {
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
    }
    assert_eq!(
        (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
        (true, 0, 0, 0)
    );
    assert_eq!(
        (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
        (true, 0, 0, 0)
    );

    for _ in 0..24 {
        let block_size = 512usize << ((ca.block as usize) >> 1);
        let length = block_size.min(1 << 20) + 17;
        let input = CString::new(vec![b'z'; length]).unwrap();
        let cp = unsafe { (c.stralloc)(&mut ca, input.as_ptr().cast_mut()) };
        let rp = unsafe { (rust.stralloc)(&mut ra, input.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(cp) }.to_bytes().len(), length);
        assert_eq!(unsafe { CStr::from_ptr(rp) }.to_bytes().len(), length);
        assert_eq!(
            (ca.remaining, ca.block, ca.mode),
            (ra.remaining, ra.block, ra.mode)
        );
    }
    assert_eq!(ca.block, ra.block);
    unsafe {
        (c.strreset)(&mut ca);
        (rust.strreset)(&mut ra);
    }
}

unsafe fn check_small_exports(c: &Api, rust: &Api) {
    for value in [c_int::MIN, -1_000_000, -1, 0, 1, 42, 1_000_000, c_int::MAX] {
        let cs = unsafe { CStr::from_ptr((c.strkey)(value)) }
            .to_bytes()
            .to_vec();
        let rs = unsafe { CStr::from_ptr((rust.strkey)(value)) }
            .to_bytes()
            .to_vec();
        assert_eq!(cs, rs);
        assert_eq!(cs, format!("test_{value}").as_bytes());
        unsafe {
            (c.arr_del)(value);
            (rust.arr_del)(value);
        }
    }
}

#[test]
fn all_exported_surfaces_match() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = root.join("../c_src/build/libharvest-work-Wqkjij.so");
    let rust_path = root.join("target/release/libarr_del_lib.so");
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust release library: {}",
        rust_path.display()
    );

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    unsafe {
        check_arrays(&c, &rust);
        check_hashes(&c, &rust);
        check_default_and_null_paths(&c, &rust);
        check_binary_maps(&c, &rust);
        check_string_maps(&c, &rust);
        check_arenas(&c, &rust);
        check_small_exports(&c, &rust);
    }
}
