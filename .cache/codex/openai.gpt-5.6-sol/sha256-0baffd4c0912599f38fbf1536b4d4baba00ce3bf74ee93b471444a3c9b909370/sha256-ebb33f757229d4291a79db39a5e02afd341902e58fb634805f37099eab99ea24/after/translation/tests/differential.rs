use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

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
    storage: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Entry {
    key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct StringEntry {
    key: *mut c_char,
    value: u64,
}

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type RandSeed = unsafe extern "C" fn(usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
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

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        Self {
            lib: unsafe { Library::new(path).unwrap() },
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        unsafe { *self.lib.get::<T>(name).unwrap() }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = root
        .parent()
        .unwrap()
        .join("c_src/build/libharvest-work-3isVJH.so");
    let rust = root.join("target/release/libarr_push_lib.so");
    assert!(c.exists(), "missing C library: {}", c.display());
    assert!(rust.exists(), "missing Rust library: {}", rust.display());
    unsafe { (Api::load(c), Api::load(rust)) }
}

unsafe fn header(array: *mut c_void) -> ArrayHeader {
    unsafe { *((array as *mut u8).sub(size_of::<ArrayHeader>()) as *const ArrayHeader) }
}

unsafe fn hash_raw(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { (map as *mut u8).sub(element_size) as *mut c_void }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> ArrayHeader {
    unsafe { header(hash_raw(map, element_size)) }
}

unsafe fn map_seed(map: *mut c_void, element_size: usize) -> usize {
    let h = unsafe { map_header(map, element_size) };
    unsafe { (*(h.hash_table as *const HashIndex)).seed }
}

unsafe fn entry_slice(map: *mut c_void) -> Vec<Entry> {
    let h = unsafe { map_header(map, size_of::<Entry>()) };
    let count = h.length.saturating_sub(1);
    unsafe { std::slice::from_raw_parts(map as *const Entry, count).to_vec() }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn phase_a_dynamic_symbols_are_loadable() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    let symbols: &[&[u8]] = &[
        b"arr_push\0",
        b"stbds_arrfreef\0",
        b"stbds_arrgrowf\0",
        b"stbds_hash_bytes\0",
        b"stbds_hash_string\0",
        b"stbds_hmdel_key\0",
        b"stbds_hmfree_func\0",
        b"stbds_hmget_key\0",
        b"stbds_hmget_key_ts\0",
        b"stbds_hmput_default\0",
        b"stbds_hmput_key\0",
        b"stbds_rand_seed\0",
        b"stbds_shmode_func\0",
        b"stbds_stralloc\0",
        b"stbds_strreset\0",
        b"strkey\0",
    ];
    for name in symbols {
        unsafe {
            let _: *mut c_void = *c.lib.get(name).unwrap();
            let _: *mut c_void = *rust.lib.get(name).unwrap();
        }
    }
}

#[test]
fn hashes_match_for_all_source_shapes() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_hash_bytes: HashBytes = c.symbol(b"stbds_hash_bytes\0");
        let r_hash_bytes: HashBytes = rust.symbol(b"stbds_hash_bytes\0");
        let c_hash_string: HashString = c.symbol(b"stbds_hash_string\0");
        let r_hash_string: HashString = rust.symbol(b"stbds_hash_string\0");

        for &seed in &[0, 1, 0x3141_5926, usize::MAX] {
            assert_eq!(
                c_hash_bytes(ptr::null_mut(), 0, seed),
                r_hash_bytes(ptr::null_mut(), 0, seed)
            );
        }

        let mut random = 0x6a09_e667_f3bc_c909u64;
        for len in 0..=79usize {
            for _ in 0..64 {
                let mut bytes = vec![0u8; len.max(1)];
                for byte in &mut bytes {
                    *byte = next_random(&mut random) as u8;
                }
                let seed = next_random(&mut random) as usize;
                assert_eq!(
                    c_hash_bytes(bytes.as_mut_ptr().cast(), len, seed),
                    r_hash_bytes(bytes.as_mut_ptr().cast(), len, seed),
                    "byte hash mismatch len={len} seed={seed:#x}"
                );
            }
        }

        let strings = [
            CString::new("").unwrap(),
            CString::new("a").unwrap(),
            CString::new("short").unwrap(),
            CString::new("a much longer source-derived string shape").unwrap(),
            CString::new(vec![0x80, 0x81, 0xfe, 0xff]).unwrap(),
        ];
        for string in &strings {
            for &seed in &[0, 1, 0x3141_5926, usize::MAX] {
                assert_eq!(
                    c_hash_string(string.as_ptr().cast_mut(), seed),
                    r_hash_string(string.as_ptr().cast_mut(), seed),
                    "string hash mismatch {:?} seed={seed:#x}",
                    string
                );
            }
        }
    }
}

#[test]
fn array_growth_and_free_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_grow: ArrGrow = c.symbol(b"stbds_arrgrowf\0");
        let r_grow: ArrGrow = rust.symbol(b"stbds_arrgrowf\0");
        let c_free: ArrFree = c.symbol(b"stbds_arrfreef\0");
        let r_free: ArrFree = rust.symbol(b"stbds_arrfreef\0");

        for &element_size in &[1usize, 4, 16] {
            for add_len in 0..=5 {
                for min_capacity in 0..=9 {
                    let ca = c_grow(ptr::null_mut(), element_size, add_len, min_capacity);
                    let ra = r_grow(ptr::null_mut(), element_size, add_len, min_capacity);
                    assert_eq!(ca.is_null(), ra.is_null());
                    if ca.is_null() {
                        continue;
                    }
                    let ch = header(ca);
                    let rh = header(ra);
                    assert_eq!(
                        (ch.length, ch.capacity, ch.temp),
                        (rh.length, rh.capacity, rh.temp)
                    );
                    assert!(ch.hash_table.is_null() && rh.hash_table.is_null());

                    let fill = element_size * ch.capacity;
                    for index in 0..fill {
                        *(ca as *mut u8).add(index) = index.wrapping_mul(17) as u8;
                        *(ra as *mut u8).add(index) = index.wrapping_mul(17) as u8;
                    }

                    let ca2 = c_grow(ca, element_size, 0, ch.capacity);
                    let ra2 = r_grow(ra, element_size, 0, rh.capacity);
                    assert_eq!(ca2, ca);
                    assert_eq!(ra2, ra);

                    let ca3 = c_grow(ca2, element_size, ch.capacity + 1, 0);
                    let ra3 = r_grow(ra2, element_size, rh.capacity + 1, 0);
                    let ch3 = header(ca3);
                    let rh3 = header(ra3);
                    assert_eq!(
                        (ch3.length, ch3.capacity, ch3.temp),
                        (rh3.length, rh3.capacity, rh3.temp)
                    );
                    assert_eq!(
                        std::slice::from_raw_parts(ca3 as *const u8, fill),
                        std::slice::from_raw_parts(ra3 as *const u8, fill)
                    );

                    let requested = ch3.capacity * 3 + 1;
                    let ca4 = c_grow(ca3, element_size, 0, requested);
                    let ra4 = r_grow(ra3, element_size, 0, requested);
                    assert_eq!(header(ca4).capacity, header(ra4).capacity);
                    c_free(ca4);
                    r_free(ra4);
                }
            }
        }
    }
}

#[test]
fn simple_exports_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_strkey: StrKey = c.symbol(b"strkey\0");
        let r_strkey: StrKey = rust.symbol(b"strkey\0");
        for value in [c_int::MIN, -1000, -1, 0, 1, 49, 50, 51, 1000, c_int::MAX] {
            let expected = CStr::from_ptr(c_strkey(value)).to_bytes().to_vec();
            let actual = CStr::from_ptr(r_strkey(value)).to_bytes().to_vec();
            assert_eq!(expected, actual);
        }

        let c_arr_push: ArrPush = c.symbol(b"arr_push\0");
        let r_arr_push: ArrPush = rust.symbol(b"arr_push\0");
        for value in [-100, -1, 0, 1, 49, 50, 51, 99, 100, 251] {
            c_arr_push(value);
            r_arr_push(value);
        }
    }
}

unsafe fn comparable_map_header(
    map: *mut c_void,
    element_size: usize,
) -> (usize, usize, isize, bool) {
    let h = unsafe { map_header(map, element_size) };
    (h.length, h.capacity, h.temp, !h.hash_table.is_null())
}

unsafe fn set_entry_value(map: *mut c_void, value: u64) {
    let h = unsafe { map_header(map, size_of::<Entry>()) };
    assert!(h.temp >= 0);
    unsafe {
        (*(map as *mut Entry).add(h.temp as usize)).value = value;
    }
}

unsafe fn put_binary(
    put: HmPut,
    map: *mut c_void,
    key: &mut u64,
    value: u64,
    mode: c_int,
) -> *mut c_void {
    let result = unsafe {
        put(
            map,
            size_of::<Entry>(),
            (key as *mut u64).cast(),
            size_of::<u64>(),
            mode,
        )
    };
    unsafe { set_entry_value(result, value) };
    result
}

unsafe fn get_binary(get: HmGetTs, map: *mut c_void, key: &mut u64, mode: c_int) -> isize {
    let mut temp = 12345isize;
    let result = unsafe {
        get(
            map,
            size_of::<Entry>(),
            (key as *mut u64).cast(),
            size_of::<u64>(),
            &mut temp,
            mode,
        )
    };
    assert_eq!(result, map);
    temp
}

unsafe fn sorted_entries(map: *mut c_void) -> Vec<Entry> {
    let mut entries = unsafe { entry_slice(map) };
    entries.sort_by_key(|entry| entry.key);
    entries
}

#[test]
fn map_null_default_and_boundary_paths_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_get_ts: HmGetTs = c.symbol(b"stbds_hmget_key_ts\0");
        let r_get_ts: HmGetTs = rust.symbol(b"stbds_hmget_key_ts\0");
        let c_get: HmGet = c.symbol(b"stbds_hmget_key\0");
        let r_get: HmGet = rust.symbol(b"stbds_hmget_key\0");
        let c_default: HmPutDefault = c.symbol(b"stbds_hmput_default\0");
        let r_default: HmPutDefault = rust.symbol(b"stbds_hmput_default\0");
        let c_del: HmDel = c.symbol(b"stbds_hmdel_key\0");
        let r_del: HmDel = rust.symbol(b"stbds_hmdel_key\0");
        let c_free: HmFree = c.symbol(b"stbds_hmfree_func\0");
        let r_free: HmFree = rust.symbol(b"stbds_hmfree_func\0");

        c_free(ptr::null_mut(), size_of::<Entry>());
        r_free(ptr::null_mut(), size_of::<Entry>());

        let mut key = 0x1234_5678_9abc_def0u64;
        assert!(
            c_del(
                ptr::null_mut(),
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0
            )
            .is_null()
        );
        assert!(
            r_del(
                ptr::null_mut(),
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0
            )
            .is_null()
        );

        let mut ct = 99isize;
        let mut rt = 99isize;
        let cm = c_get_ts(
            ptr::null_mut(),
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut ct,
            0,
        );
        let rm = r_get_ts(
            ptr::null_mut(),
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            &mut rt,
            0,
        );
        assert_eq!(ct, rt);
        assert_eq!(ct, -1);
        assert_eq!(
            comparable_map_header(cm, size_of::<Entry>()),
            comparable_map_header(rm, size_of::<Entry>())
        );
        assert_eq!(
            std::slice::from_raw_parts(
                hash_raw(cm, size_of::<Entry>()) as *const u8,
                size_of::<Entry>()
            ),
            std::slice::from_raw_parts(
                hash_raw(rm, size_of::<Entry>()) as *const u8,
                size_of::<Entry>()
            )
        );

        let cm = c_get(
            cm,
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        );
        let rm = r_get(
            rm,
            size_of::<Entry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        );
        assert_eq!(
            comparable_map_header(cm, size_of::<Entry>()),
            comparable_map_header(rm, size_of::<Entry>())
        );
        assert_eq!(map_header(cm, size_of::<Entry>()).temp, -1);

        let cm2 = c_default(cm, size_of::<Entry>());
        let rm2 = r_default(rm, size_of::<Entry>());
        assert_eq!(cm2, cm);
        assert_eq!(rm2, rm);
        assert_eq!(
            comparable_map_header(cm2, size_of::<Entry>()),
            comparable_map_header(rm2, size_of::<Entry>())
        );
        c_free(hash_raw(cm2, size_of::<Entry>()), size_of::<Entry>());
        r_free(hash_raw(rm2, size_of::<Entry>()), size_of::<Entry>());

        let cm = c_default(ptr::null_mut(), size_of::<Entry>());
        let rm = r_default(ptr::null_mut(), size_of::<Entry>());
        assert_eq!(
            comparable_map_header(cm, size_of::<Entry>()),
            comparable_map_header(rm, size_of::<Entry>())
        );
        let cm2 = c_default(cm, size_of::<Entry>());
        let rm2 = r_default(rm, size_of::<Entry>());
        assert_eq!(cm2, cm);
        assert_eq!(rm2, rm);
        c_free(hash_raw(cm2, size_of::<Entry>()), size_of::<Entry>());
        r_free(hash_raw(rm2, size_of::<Entry>()), size_of::<Entry>());
    }
}

#[test]
fn randomized_binary_map_state_machine_matches() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_seed: RandSeed = c.symbol(b"stbds_rand_seed\0");
        let r_seed: RandSeed = rust.symbol(b"stbds_rand_seed\0");
        let c_put: HmPut = c.symbol(b"stbds_hmput_key\0");
        let r_put: HmPut = rust.symbol(b"stbds_hmput_key\0");
        let c_get: HmGetTs = c.symbol(b"stbds_hmget_key_ts\0");
        let r_get: HmGetTs = rust.symbol(b"stbds_hmget_key_ts\0");
        let c_get_direct: HmGet = c.symbol(b"stbds_hmget_key\0");
        let r_get_direct: HmGet = rust.symbol(b"stbds_hmget_key\0");
        let c_del: HmDel = c.symbol(b"stbds_hmdel_key\0");
        let r_del: HmDel = rust.symbol(b"stbds_hmdel_key\0");
        let c_free: HmFree = c.symbol(b"stbds_hmfree_func\0");
        let r_free: HmFree = rust.symbol(b"stbds_hmfree_func\0");

        for &seed in &[0usize, 1, 0x3141_5926, usize::MAX] {
            c_seed(seed);
            r_seed(seed);
            let mut cm = ptr::null_mut();
            let mut rm = ptr::null_mut();
            let mut random = 0x243f_6a88_85a3_08d3u64 ^ seed as u64;

            for step in 0..600usize {
                let mut key = next_random(&mut random) % 96;
                match next_random(&mut random) % 4 {
                    0 | 1 => {
                        let value = next_random(&mut random);
                        let mode = if step % 11 == 0 { -7 } else { 0 };
                        cm = put_binary(c_put, cm, &mut key, value, mode);
                        rm = put_binary(r_put, rm, &mut key, value, mode);
                        assert_eq!(map_seed(cm, size_of::<Entry>()), seed);
                        assert_eq!(map_seed(rm, size_of::<Entry>()), seed);
                        if step % 17 == 0 {
                            cm = c_get_direct(
                                cm,
                                size_of::<Entry>(),
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                0,
                            );
                            rm = r_get_direct(
                                rm,
                                size_of::<Entry>(),
                                (&mut key as *mut u64).cast(),
                                size_of::<u64>(),
                                0,
                            );
                            assert_eq!(
                                map_header(cm, size_of::<Entry>()).temp,
                                map_header(rm, size_of::<Entry>()).temp
                            );
                        }
                    }
                    2 => {
                        if !cm.is_null() {
                            let ci = get_binary(c_get, cm, &mut key, 0);
                            let ri = get_binary(r_get, rm, &mut key, 0);
                            assert_eq!(ci, ri, "lookup mismatch step={step} key={key}");
                            if ci >= 0 {
                                assert_eq!(
                                    *(cm as *const Entry).add(ci as usize),
                                    *(rm as *const Entry).add(ri as usize)
                                );
                            }
                        }
                    }
                    _ => {
                        cm = c_del(
                            cm,
                            size_of::<Entry>(),
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                            0,
                        );
                        rm = r_del(
                            rm,
                            size_of::<Entry>(),
                            (&mut key as *mut u64).cast(),
                            size_of::<u64>(),
                            0,
                            0,
                        );
                    }
                }
                assert_eq!(cm.is_null(), rm.is_null());
                if !cm.is_null() {
                    assert_eq!(
                        comparable_map_header(cm, size_of::<Entry>()),
                        comparable_map_header(rm, size_of::<Entry>()),
                        "header mismatch seed={seed} step={step}"
                    );
                    assert_eq!(
                        sorted_entries(cm),
                        sorted_entries(rm),
                        "entry mismatch seed={seed} step={step}"
                    );
                }
            }

            if !cm.is_null() {
                c_free(hash_raw(cm, size_of::<Entry>()), size_of::<Entry>());
                r_free(hash_raw(rm, size_of::<Entry>()), size_of::<Entry>());
            }
        }
    }
}

#[test]
fn binary_map_growth_rebuild_shrink_and_key_sizes_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_seed: RandSeed = c.symbol(b"stbds_rand_seed\0");
        let r_seed: RandSeed = rust.symbol(b"stbds_rand_seed\0");
        let c_put: HmPut = c.symbol(b"stbds_hmput_key\0");
        let r_put: HmPut = rust.symbol(b"stbds_hmput_key\0");
        let c_del: HmDel = c.symbol(b"stbds_hmdel_key\0");
        let r_del: HmDel = rust.symbol(b"stbds_hmdel_key\0");
        let c_free: HmFree = c.symbol(b"stbds_hmfree_func\0");
        let r_free: HmFree = rust.symbol(b"stbds_hmfree_func\0");

        c_seed(0xfeed_face);
        r_seed(0xfeed_face);
        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        for key_value in 0..80u64 {
            let mut key = key_value;
            cm = put_binary(c_put, cm, &mut key, key_value * 3, 0);
            rm = put_binary(r_put, rm, &mut key, key_value * 3, 0);
        }
        for key_value in 0..68u64 {
            let mut key = key_value;
            cm = c_del(
                cm,
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
            rm = r_del(
                rm,
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
            assert_eq!(
                comparable_map_header(cm, size_of::<Entry>()),
                comparable_map_header(rm, size_of::<Entry>())
            );
            assert_eq!(sorted_entries(cm), sorted_entries(rm));
        }
        c_free(hash_raw(cm, size_of::<Entry>()), size_of::<Entry>());
        r_free(hash_raw(rm, size_of::<Entry>()), size_of::<Entry>());

        for &key_size in &[1usize, 4, 8, 16] {
            c_seed(123);
            r_seed(123);
            let element_size = key_size + 8;
            let mut cm: *mut c_void = ptr::null_mut();
            let mut rm: *mut c_void = ptr::null_mut();
            let mut random = 0x1319_8a2e_0370_7344u64;
            for _ in 0..48 {
                let mut key = [0u8; 16];
                for byte in &mut key[..key_size] {
                    *byte = next_random(&mut random) as u8;
                }
                cm = c_put(cm, element_size, key.as_mut_ptr().cast(), key_size, 0);
                rm = r_put(rm, element_size, key.as_mut_ptr().cast(), key_size, 0);
                let ch = map_header(cm, element_size);
                let rh = map_header(rm, element_size);
                assert_eq!(
                    (ch.length, ch.capacity, ch.temp),
                    (rh.length, rh.capacity, rh.temp)
                );
                let index = ch.temp as usize;
                let value = next_random(&mut random).to_ne_bytes();
                ptr::copy_nonoverlapping(
                    value.as_ptr(),
                    (cm as *mut u8).add(index * element_size + key_size),
                    8,
                );
                ptr::copy_nonoverlapping(
                    value.as_ptr(),
                    (rm as *mut u8).add(index * element_size + key_size),
                    8,
                );
            }
            let ch = map_header(cm, element_size);
            let rh = map_header(rm, element_size);
            assert_eq!(
                (ch.length, ch.capacity, ch.temp),
                (rh.length, rh.capacity, rh.temp)
            );
            assert_eq!(
                std::slice::from_raw_parts(cm as *const u8, (ch.length - 1) * element_size),
                std::slice::from_raw_parts(rm as *const u8, (rh.length - 1) * element_size)
            );
            c_free(hash_raw(cm, element_size), element_size);
            r_free(hash_raw(rm, element_size), element_size);
        }
    }
}

unsafe fn string_entries(map: *mut c_void) -> Vec<(Vec<u8>, u64)> {
    let h = unsafe { map_header(map, size_of::<StringEntry>()) };
    let mut result = Vec::new();
    for index in 0..h.length.saturating_sub(1) {
        let entry = unsafe { *(map as *const StringEntry).add(index) };
        let key = unsafe { CStr::from_ptr(entry.key).to_bytes().to_vec() };
        result.push((key, entry.value));
    }
    result.sort();
    result
}

#[test]
fn string_map_ownership_modes_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_seed: RandSeed = c.symbol(b"stbds_rand_seed\0");
        let r_seed: RandSeed = rust.symbol(b"stbds_rand_seed\0");
        let c_put: HmPut = c.symbol(b"stbds_hmput_key\0");
        let r_put: HmPut = rust.symbol(b"stbds_hmput_key\0");
        let c_get: HmGetTs = c.symbol(b"stbds_hmget_key_ts\0");
        let r_get: HmGetTs = rust.symbol(b"stbds_hmget_key_ts\0");
        let c_del: HmDel = c.symbol(b"stbds_hmdel_key\0");
        let r_del: HmDel = rust.symbol(b"stbds_hmdel_key\0");
        let c_free: HmFree = c.symbol(b"stbds_hmfree_func\0");
        let r_free: HmFree = rust.symbol(b"stbds_hmfree_func\0");
        let c_mode: ShMode = c.symbol(b"stbds_shmode_func\0");
        let r_mode: ShMode = rust.symbol(b"stbds_shmode_func\0");

        for ownership_mode in [1, 2, 3] {
            c_seed(0x1234);
            r_seed(0x1234);
            let mut cm = if ownership_mode == 1 {
                ptr::null_mut()
            } else {
                c_mode(size_of::<StringEntry>(), ownership_mode)
            };
            let mut rm = if ownership_mode == 1 {
                ptr::null_mut()
            } else {
                r_mode(size_of::<StringEntry>(), ownership_mode)
            };
            let keys: Vec<CString> = (0..40)
                .map(|index| CString::new(format!("key_{ownership_mode}_{index:03}")).unwrap())
                .collect();

            for (index, key) in keys.iter().enumerate() {
                cm = c_put(
                    cm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    1,
                );
                rm = r_put(
                    rm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    1,
                );
                let ch = map_header(cm, size_of::<StringEntry>());
                let rh = map_header(rm, size_of::<StringEntry>());
                assert_eq!(
                    (ch.length, ch.capacity, ch.temp),
                    (rh.length, rh.capacity, rh.temp)
                );
                (*(cm as *mut StringEntry).add(ch.temp as usize)).value = index as u64 * 17;
                (*(rm as *mut StringEntry).add(rh.temp as usize)).value = index as u64 * 17;
            }

            let first_c_pointer = (*(cm as *const StringEntry)).key;
            let first_r_pointer = (*(rm as *const StringEntry)).key;
            if ownership_mode == 1 {
                assert_eq!(first_c_pointer, keys[0].as_ptr().cast_mut());
                assert_eq!(first_r_pointer, keys[0].as_ptr().cast_mut());
            } else {
                assert_ne!(first_c_pointer, keys[0].as_ptr().cast_mut());
                assert_ne!(first_r_pointer, keys[0].as_ptr().cast_mut());
            }
            cm = c_put(
                cm,
                size_of::<StringEntry>(),
                keys[0].as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            rm = r_put(
                rm,
                size_of::<StringEntry>(),
                keys[0].as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                1,
            );
            let ch = map_header(cm, size_of::<StringEntry>());
            let rh = map_header(rm, size_of::<StringEntry>());
            assert_eq!(ch.temp, rh.temp);
            assert_eq!(
                (*(cm as *const StringEntry).add(ch.temp as usize)).key,
                first_c_pointer
            );
            assert_eq!(
                (*(rm as *const StringEntry).add(rh.temp as usize)).key,
                first_r_pointer
            );

            for (index, key) in keys.iter().enumerate() {
                let mut ci = -99isize;
                let mut ri = -99isize;
                c_get(
                    cm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut ci,
                    2,
                );
                r_get(
                    rm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut ri,
                    99,
                );
                assert_eq!(ci, ri);
                assert_eq!(
                    (*(cm as *const StringEntry).add(ci as usize)).value,
                    index as u64 * 17
                );
            }
            assert_eq!(string_entries(cm), string_entries(rm));

            for index in (0..30).step_by(2) {
                let key = &keys[index];
                cm = c_del(
                    cm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                rm = r_del(
                    rm,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                assert_eq!(string_entries(cm), string_entries(rm));
            }

            let missing = CString::new("not_present").unwrap();
            let cm2 = c_del(
                cm,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            );
            let rm2 = r_del(
                rm,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                1,
            );
            assert_eq!(cm2, cm);
            assert_eq!(rm2, rm);
            c_free(
                hash_raw(cm2, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            r_free(
                hash_raw(rm2, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }

        for mode in [0, 4, 255, 256, -1] {
            let mut cm = c_mode(size_of::<Entry>(), mode);
            let mut rm = r_mode(size_of::<Entry>(), mode);
            let mut key = 0x8877_6655_4433_2211u64;
            cm = put_binary(c_put, cm, &mut key, 0xabcdef, 0);
            rm = put_binary(r_put, rm, &mut key, 0xabcdef, 0);
            let mut ci = -99;
            let mut ri = -99;
            c_get(
                cm,
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                &mut ci,
                0,
            );
            r_get(
                rm,
                size_of::<Entry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                &mut ri,
                0,
            );
            assert_eq!(ci, ri);
            assert_eq!(sorted_entries(cm), sorted_entries(rm));
            assert_eq!(
                comparable_map_header(cm, size_of::<Entry>()),
                comparable_map_header(rm, size_of::<Entry>())
            );
            c_free(hash_raw(cm, size_of::<Entry>()), size_of::<Entry>());
            r_free(hash_raw(rm, size_of::<Entry>()), size_of::<Entry>());
        }
    }
}

#[test]
fn string_arena_boundaries_and_reset_match() {
    let _guard = SERIAL.lock().unwrap();
    let (c, rust) = libraries();
    unsafe {
        let c_alloc: StrAlloc = c.symbol(b"stbds_stralloc\0");
        let r_alloc: StrAlloc = rust.symbol(b"stbds_stralloc\0");
        let c_reset: StrReset = c.symbol(b"stbds_strreset\0");
        let r_reset: StrReset = rust.symbol(b"stbds_strreset\0");

        let mut ca = StringArena {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let mut ra = ca;
        c_reset(&mut ca);
        r_reset(&mut ra);
        assert_eq!(
            (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
            (true, 0, 0, 0)
        );
        assert_eq!(
            (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
            (true, 0, 0, 0)
        );

        let mut shapes = vec![0usize, 1, 15, 127, 510, 511, 512, 700, 1023, 1024];
        shapes.extend(std::iter::repeat_n(73usize, 30));
        shapes.push((1 << 20) + 17);
        for (index, chars) in shapes.into_iter().enumerate() {
            let byte = b'a' + (index % 26) as u8;
            let string = CString::new(vec![byte; chars]).unwrap();
            let cp = c_alloc(&mut ca, string.as_ptr().cast_mut());
            let rp = r_alloc(&mut ra, string.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(
                (ca.remaining, ca.block, ca.mode, ca.storage.is_null()),
                (ra.remaining, ra.block, ra.mode, ra.storage.is_null()),
                "arena state mismatch at shape {chars}"
            );
        }
        c_reset(&mut ca);
        r_reset(&mut ra);
        assert_eq!(
            (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
            (true, 0, 0, 0)
        );
        assert_eq!(
            (ra.storage.is_null(), ra.remaining, ra.block, ra.mode),
            (true, 0, 0, 0)
        );

        for chars in [511usize, 512] {
            let mut ca = StringArena {
                storage: ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            let mut ra = ca;
            let string = CString::new(vec![b'z'; chars]).unwrap();
            let cp = c_alloc(&mut ca, string.as_ptr().cast_mut());
            let rp = r_alloc(&mut ra, string.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(
                (ca.remaining, ca.block, ca.storage.is_null()),
                (ra.remaining, ra.block, ra.storage.is_null())
            );
            c_reset(&mut ca);
            r_reset(&mut ra);
        }
    }
}
