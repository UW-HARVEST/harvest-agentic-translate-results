use libloading::Library;
use std::env;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::ptr;

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
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type StrDups = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
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

impl Default for Arena {
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
struct Pair {
    key: u32,
    value: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct OffsetPair {
    hash_key: u32,
    key_at_offset: u32,
    value: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct HashBucket {
    hash: [usize; 8],
    index: [isize; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    string: Arena,
    storage: *mut HashBucket,
}

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        Self {
            library: unsafe { Library::new(path).unwrap() },
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.library.get::<T>(name).unwrap() }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        (
            Api::load(root.join("../c_src/build/libharvest-work-OCFKN1.so")),
            Api::load(root.join("target/release/libstr_dups_lib.so")),
        )
    }
}

unsafe fn header(raw: *mut c_void) -> *mut Header {
    unsafe { raw.cast::<u8>().sub(size_of::<Header>()).cast() }
}

unsafe fn map_raw(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_len(map: *mut c_void, element_size: usize) -> usize {
    unsafe { (*header(map_raw(map, element_size))).length - 1 }
}

unsafe fn map_temp(map: *mut c_void, element_size: usize) -> isize {
    unsafe { (*header(map_raw(map, element_size))).temp }
}

unsafe fn table(map: *mut c_void, element_size: usize) -> *mut HashIndex {
    unsafe { (*header(map_raw(map, element_size))).hash_table.cast() }
}

unsafe fn pair_snapshot(map: *mut c_void) -> (Vec<Pair>, isize, usize, usize) {
    if map.is_null() {
        return (Vec::new(), 0, 0, 0);
    }
    let len = unsafe { map_len(map, size_of::<Pair>()) };
    let entries = unsafe { std::slice::from_raw_parts(map.cast::<Pair>(), len) }.to_vec();
    let raw = unsafe { map_raw(map, size_of::<Pair>()) };
    let table = unsafe { (*header(raw)).hash_table.cast::<HashIndex>() };
    let (slots, tombstones) = if table.is_null() {
        (0, 0)
    } else {
        unsafe { ((*table).slot_count, (*table).tombstone_count) }
    };
    (entries, unsafe { (*header(raw)).temp }, slots, tombstones)
}

unsafe fn raw_map_snapshot(
    map: *mut c_void,
    element_size: usize,
) -> (Vec<u8>, isize, usize, usize) {
    let len = unsafe { map_len(map, element_size) };
    let bytes =
        unsafe { std::slice::from_raw_parts(map.cast::<u8>(), len * element_size) }.to_vec();
    let table = unsafe { table(map, element_size) };
    let (slots, tombstones) = if table.is_null() {
        (0, 0)
    } else {
        unsafe { ((*table).slot_count, (*table).tombstone_count) }
    };
    (
        bytes,
        unsafe { map_temp(map, element_size) },
        slots,
        tombstones,
    )
}

unsafe fn set_pair_value(map: *mut c_void, value: u32) {
    let index = unsafe { map_temp(map, size_of::<Pair>()) } as usize;
    unsafe { (*map.cast::<Pair>().add(index)).value = value };
}

unsafe fn string_snapshot(map: *mut c_void) -> (Vec<(Vec<u8>, c_int)>, isize, usize, usize) {
    let len = unsafe { map_len(map, size_of::<StringEntry>()) };
    let mut entries = Vec::with_capacity(len);
    for index in 0..len {
        let entry = unsafe { *map.cast::<StringEntry>().add(index) };
        entries.push((
            unsafe { CStr::from_ptr(entry.key) }.to_bytes().to_vec(),
            entry.value,
        ));
    }
    let table = unsafe { table(map, size_of::<StringEntry>()) };
    (
        entries,
        unsafe { map_temp(map, size_of::<StringEntry>()) },
        unsafe { (*table).slot_count },
        unsafe { (*table).tombstone_count },
    )
}

unsafe fn set_string_value(map: *mut c_void, value: c_int) {
    let index = unsafe { map_temp(map, size_of::<StringEntry>()) } as usize;
    unsafe { (*map.cast::<StringEntry>().add(index)).value = value };
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

unsafe fn capture_stdout(function: StrDups, number: c_int) -> Vec<u8> {
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    let mut fds = [0; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { dup2(fds[1], 1) }, 1);
    unsafe { function(number) };
    unsafe { fflush(ptr::null_mut()) };
    assert_eq!(unsafe { dup2(saved, 1) }, 1);
    unsafe {
        close(saved);
        close(fds[1]);
    }

    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = unsafe { read(fds[0], buffer.as_mut_ptr().cast(), buffer.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    unsafe { close(fds[0]) };
    output
}

fn assert_child_failed(status: ExitStatus, label: &str) {
    assert!(!status.success(), "{label} unexpectedly succeeded");
}

fn run_crash_child(which: &str, case: &str) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_crash_child")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIB", which)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn differential_valid_and_error_surfaces() {
    let (c, rust) = libraries();
    unsafe {
        let c_seed: RandSeed = c.symbol(b"stbds_rand_seed");
        let r_seed: RandSeed = rust.symbol(b"stbds_rand_seed");
        let c_hash_string: HashString = c.symbol(b"stbds_hash_string");
        let r_hash_string: HashString = rust.symbol(b"stbds_hash_string");
        let c_hash_bytes: HashBytes = c.symbol(b"stbds_hash_bytes");
        let r_hash_bytes: HashBytes = rust.symbol(b"stbds_hash_bytes");

        // CONFIGS 1-9: seed and both hash functions, every tail/full-block shape.
        let mut rng = Rng(0x8f3d_91a2_5c77_e4b1);
        for seed in [0, 1, 0x3141_5926, usize::MAX] {
            c_seed(seed);
            r_seed(seed);
            for len in 0..96 {
                let mut bytes = rng.bytes(len);
                let c_value = c_hash_bytes(
                    if len == 0 {
                        ptr::null_mut()
                    } else {
                        bytes.as_mut_ptr().cast()
                    },
                    len,
                    seed,
                );
                let r_value = r_hash_bytes(
                    if len == 0 {
                        ptr::null_mut()
                    } else {
                        bytes.as_mut_ptr().cast()
                    },
                    len,
                    seed,
                );
                assert_eq!(c_value, r_value, "hash_bytes len={len} seed={seed:#x}");
            }

            for len in 0..64 {
                let bytes: Vec<u8> = (0..len)
                    .map(|_| {
                        let byte = rng.next() as u8;
                        if byte == 0 { 0x80 } else { byte }
                    })
                    .collect();
                let string = CString::new(bytes).unwrap();
                assert_eq!(
                    c_hash_string(string.as_ptr().cast_mut(), seed),
                    r_hash_string(string.as_ptr().cast_mut(), seed),
                    "hash_string len={len} seed={seed:#x}"
                );
            }
        }

        // CONFIGS 10-15: array allocation, no-growth, doubling, direct growth, widths.
        let c_grow: ArrGrow = c.symbol(b"stbds_arrgrowf");
        let r_grow: ArrGrow = rust.symbol(b"stbds_arrgrowf");
        let c_free: ArrFree = c.symbol(b"stbds_arrfreef");
        let r_free: ArrFree = rust.symbol(b"stbds_arrfreef");
        for width in [1usize, 4, 8, 24] {
            assert!(c_grow(ptr::null_mut(), width, 0, 0).is_null());
            assert!(r_grow(ptr::null_mut(), width, 0, 0).is_null());
            let mut ca = c_grow(ptr::null_mut(), width, 1, 0);
            let mut ra = r_grow(ptr::null_mut(), width, 1, 0);
            assert_eq!((*header(ca)).capacity, 4);
            assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
            assert_eq!((*header(ca)).length, (*header(ra)).length);

            for index in 0..width * 4 {
                *ca.cast::<u8>().add(index) = index as u8 ^ 0x5a;
                *ra.cast::<u8>().add(index) = index as u8 ^ 0x5a;
            }
            (*header(ca)).length = 3;
            (*header(ra)).length = 3;
            let old_ca = ca;
            let old_ra = ra;
            ca = c_grow(ca, width, 0, 4);
            ra = r_grow(ra, width, 0, 4);
            assert_eq!(ca, old_ca);
            assert_eq!(ra, old_ra);

            ca = c_grow(ca, width, 2, 0);
            ra = r_grow(ra, width, 2, 0);
            assert_eq!((*header(ca)).capacity, 8);
            assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
            assert_eq!(
                std::slice::from_raw_parts(ca.cast::<u8>(), width * 3),
                std::slice::from_raw_parts(ra.cast::<u8>(), width * 3)
            );

            ca = c_grow(ca, width, 0, 37);
            ra = r_grow(ra, width, 0, 37);
            assert_eq!((*header(ca)).capacity, 37);
            assert_eq!((*header(ca)).capacity, (*header(ra)).capacity);
            c_free(ca);
            r_free(ra);
        }

        // CONFIGS 16-25, 32-38, 40 and ERRORS 1-9, 28: low-level binary maps.
        let c_get_ts: HmGetTs = c.symbol(b"stbds_hmget_key_ts");
        let r_get_ts: HmGetTs = rust.symbol(b"stbds_hmget_key_ts");
        let c_get: HmGet = c.symbol(b"stbds_hmget_key");
        let r_get: HmGet = rust.symbol(b"stbds_hmget_key");
        let c_default: HmPutDefault = c.symbol(b"stbds_hmput_default");
        let r_default: HmPutDefault = rust.symbol(b"stbds_hmput_default");
        let c_put: HmPut = c.symbol(b"stbds_hmput_key");
        let r_put: HmPut = rust.symbol(b"stbds_hmput_key");
        let c_del: HmDel = c.symbol(b"stbds_hmdel_key");
        let r_del: HmDel = rust.symbol(b"stbds_hmdel_key");
        let c_hmfree: HmFree = c.symbol(b"stbds_hmfree_func");
        let r_hmfree: HmFree = rust.symbol(b"stbds_hmfree_func");

        let cm_default = c_default(ptr::null_mut(), size_of::<Pair>());
        let rm_default = r_default(ptr::null_mut(), size_of::<Pair>());
        assert_eq!(map_len(cm_default, size_of::<Pair>()), 0);
        assert_eq!(map_len(rm_default, size_of::<Pair>()), 0);
        assert_eq!(
            *map_raw(cm_default, size_of::<Pair>()).cast::<Pair>(),
            Pair { key: 0, value: 0 }
        );
        assert_eq!(
            *map_raw(rm_default, size_of::<Pair>()).cast::<Pair>(),
            Pair { key: 0, value: 0 }
        );
        c_hmfree(map_raw(cm_default, size_of::<Pair>()), size_of::<Pair>());
        r_hmfree(map_raw(rm_default, size_of::<Pair>()), size_of::<Pair>());

        c_hmfree(ptr::null_mut(), size_of::<Pair>());
        r_hmfree(ptr::null_mut(), size_of::<Pair>());
        assert!(c_del(ptr::null_mut(), size_of::<Pair>(), ptr::null_mut(), 0, 0, 0).is_null());
        assert!(r_del(ptr::null_mut(), size_of::<Pair>(), ptr::null_mut(), 0, 0, 0).is_null());

        let mut key = 7u32;
        let mut ct = 99isize;
        let mut rt = 99isize;
        let mut cm = c_get_ts(
            ptr::null_mut(),
            size_of::<Pair>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut ct,
            0,
        );
        let mut rm = r_get_ts(
            ptr::null_mut(),
            size_of::<Pair>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            &mut rt,
            0,
        );
        assert_eq!((ct, rt), (-1, -1));
        assert_eq!(map_len(cm, size_of::<Pair>()), 0);
        assert_eq!(map_len(rm, size_of::<Pair>()), 0);
        let original_cm = cm;
        let original_rm = rm;
        cm = c_default(cm, size_of::<Pair>());
        rm = r_default(rm, size_of::<Pair>());
        assert_eq!(cm, original_cm);
        assert_eq!(rm, original_rm);
        cm = c_del(
            cm,
            size_of::<Pair>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            0,
            0,
        );
        rm = r_del(
            rm,
            size_of::<Pair>(),
            (&mut key as *mut u32).cast(),
            size_of::<u32>(),
            0,
            0,
        );
        assert_eq!(map_temp(cm, size_of::<Pair>()), 0);
        assert_eq!(map_temp(rm, size_of::<Pair>()), 0);
        c_hmfree(map_raw(cm, size_of::<Pair>()), size_of::<Pair>());
        r_hmfree(map_raw(rm, size_of::<Pair>()), size_of::<Pair>());

        c_seed(0x1234_5678);
        r_seed(0x1234_5678);
        let mut cm: *mut c_void = ptr::null_mut();
        let mut rm: *mut c_void = ptr::null_mut();
        let mut model_rng = Rng(0x1133_5577_99bb_ddff);
        for operation in 0..600 {
            let mut operation_key = (model_rng.next() % 73) as u32;
            match model_rng.next() % 5 {
                0 | 1 | 2 => {
                    cm = c_put(
                        cm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        if operation % 2 == 0 { 0 } else { -17 },
                    );
                    rm = r_put(
                        rm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        if operation % 2 == 0 { 0 } else { -17 },
                    );
                    let value = model_rng.next() as u32;
                    set_pair_value(cm, value);
                    set_pair_value(rm, value);
                }
                3 => {
                    let c_result = c_get(
                        cm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        0,
                    );
                    let r_result = r_get(
                        rm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        0,
                    );
                    assert_eq!(c_result.is_null(), r_result.is_null());
                    cm = c_result;
                    rm = r_result;
                }
                _ => {
                    cm = c_del(
                        cm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        0,
                        0,
                    );
                    rm = r_del(
                        rm,
                        size_of::<Pair>(),
                        (&mut operation_key as *mut u32).cast(),
                        size_of::<u32>(),
                        0,
                        0,
                    );
                }
            }
            assert_eq!(
                pair_snapshot(cm),
                pair_snapshot(rm),
                "map operation {operation}"
            );
        }
        c_hmfree(map_raw(cm, size_of::<Pair>()), size_of::<Pair>());
        r_hmfree(map_raw(rm, size_of::<Pair>()), size_of::<Pair>());

        // Deterministic found/missing lookup and 1-/8-byte key widths.
        for key_size in [1usize, 8] {
            let mut key_bytes = [0xabu8; 8];
            let mut missing_bytes = [0xcdu8; 8];
            let mut cm: *mut c_void = ptr::null_mut();
            let mut rm: *mut c_void = ptr::null_mut();
            cm = c_put(cm, key_size * 2, key_bytes.as_mut_ptr().cast(), key_size, 0);
            rm = r_put(rm, key_size * 2, key_bytes.as_mut_ptr().cast(), key_size, 0);
            let mut ct = 77isize;
            let mut rt = 77isize;
            cm = c_get_ts(
                cm,
                key_size * 2,
                key_bytes.as_mut_ptr().cast(),
                key_size,
                &mut ct,
                0,
            );
            rm = r_get_ts(
                rm,
                key_size * 2,
                key_bytes.as_mut_ptr().cast(),
                key_size,
                &mut rt,
                0,
            );
            assert_eq!((ct, rt), (0, 0));
            cm = c_get_ts(
                cm,
                key_size * 2,
                missing_bytes.as_mut_ptr().cast(),
                key_size,
                &mut ct,
                0,
            );
            rm = r_get_ts(
                rm,
                key_size * 2,
                missing_bytes.as_mut_ptr().cast(),
                key_size,
                &mut rt,
                0,
            );
            assert_eq!((ct, rt), (-1, -1));
            assert_eq!(
                raw_map_snapshot(cm, key_size * 2),
                raw_map_snapshot(rm, key_size * 2)
            );
            c_hmfree(map_raw(cm, key_size * 2), key_size * 2);
            r_hmfree(map_raw(rm, key_size * 2), key_size * 2);
        }

        // Force same-size tombstone rebuild and then a shrink after growth.
        for (insert_count, delete_count, expected_slots, expected_tombstones) in [
            (10u32, 4u32, 16usize, 0usize),
            (30u32, 15u32, 32usize, 0usize),
        ] {
            c_seed(0x7777 + insert_count as usize);
            r_seed(0x7777 + insert_count as usize);
            let mut cm: *mut c_void = ptr::null_mut();
            let mut rm: *mut c_void = ptr::null_mut();
            for mut key in 0..insert_count {
                cm = c_put(
                    cm,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                );
                rm = r_put(
                    rm,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                );
                set_pair_value(cm, key);
                set_pair_value(rm, key);
            }
            for mut key in 0..delete_count {
                cm = c_del(
                    cm,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                );
                rm = r_del(
                    rm,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                );
            }
            assert_eq!(pair_snapshot(cm), pair_snapshot(rm));
            assert_eq!((*table(cm, size_of::<Pair>())).slot_count, expected_slots);
            assert_eq!(
                (*table(cm, size_of::<Pair>())).tombstone_count,
                expected_tombstones
            );
            c_hmfree(map_raw(cm, size_of::<Pair>()), size_of::<Pair>());
            r_hmfree(map_raw(rm, size_of::<Pair>()), size_of::<Pair>());
        }

        // Explicit non-final compaction and nonzero key-offset deletion.
        c_seed(0x8877);
        r_seed(0x8877);
        let mut cm: *mut c_void = ptr::null_mut();
        let mut rm: *mut c_void = ptr::null_mut();
        for value in 0..20u32 {
            let mut entry = OffsetPair {
                hash_key: value,
                key_at_offset: value,
                value: value ^ 0x55aa,
            };
            cm = c_put(
                cm,
                size_of::<OffsetPair>(),
                (&mut entry.hash_key as *mut u32).cast(),
                size_of::<u32>(),
                0,
            );
            rm = r_put(
                rm,
                size_of::<OffsetPair>(),
                (&mut entry.hash_key as *mut u32).cast(),
                size_of::<u32>(),
                0,
            );
            let ci = map_temp(cm, size_of::<OffsetPair>()) as usize;
            let ri = map_temp(rm, size_of::<OffsetPair>()) as usize;
            *cm.cast::<OffsetPair>().add(ci) = entry;
            *rm.cast::<OffsetPair>().add(ri) = entry;
        }
        let mut delete_key = 3u32;
        cm = c_del(
            cm,
            size_of::<OffsetPair>(),
            (&mut delete_key as *mut u32).cast(),
            size_of::<u32>(),
            size_of::<u32>(),
            0,
        );
        rm = r_del(
            rm,
            size_of::<OffsetPair>(),
            (&mut delete_key as *mut u32).cast(),
            size_of::<u32>(),
            size_of::<u32>(),
            0,
        );
        let clen = map_len(cm, size_of::<OffsetPair>());
        let rlen = map_len(rm, size_of::<OffsetPair>());
        assert_eq!(clen, rlen);
        assert_eq!(
            std::slice::from_raw_parts(cm.cast::<OffsetPair>(), clen)
                .iter()
                .map(|entry| (entry.hash_key, entry.key_at_offset, entry.value))
                .collect::<Vec<_>>(),
            std::slice::from_raw_parts(rm.cast::<OffsetPair>(), rlen)
                .iter()
                .map(|entry| (entry.hash_key, entry.key_at_offset, entry.value))
                .collect::<Vec<_>>()
        );
        c_hmfree(
            map_raw(cm, size_of::<OffsetPair>()),
            size_of::<OffsetPair>(),
        );
        r_hmfree(
            map_raw(rm, size_of::<OffsetPair>()),
            size_of::<OffsetPair>(),
        );

        // CONFIGS 26-31, 39-40: all string ownership and out-of-range modes.
        let c_shmode: ShMode = c.symbol(b"stbds_shmode_func");
        let r_shmode: ShMode = rust.symbol(b"stbds_shmode_func");
        for seed in [0usize, 1] {
            c_seed(seed);
            r_seed(seed);
            let empty = CString::new("").unwrap();
            let key = empty.as_ptr().cast_mut();
            let mut csm = c_shmode(size_of::<StringEntry>(), 1);
            let mut rsm = r_shmode(size_of::<StringEntry>(), 1);
            assert_eq!((*table(csm, size_of::<StringEntry>())).seed, seed);
            assert_eq!((*table(rsm, size_of::<StringEntry>())).seed, seed);
            csm = c_put(
                csm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                1,
            );
            rsm = r_put(
                rsm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                1,
            );
            let mut ct = -2isize;
            let mut rt = -2isize;
            csm = c_get_ts(
                csm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                &mut ct,
                1,
            );
            rsm = r_get_ts(
                rsm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                &mut rt,
                1,
            );
            assert_eq!((ct, rt), (0, 0));
            assert_eq!(string_snapshot(csm), string_snapshot(rsm));
            c_hmfree(
                map_raw(csm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            r_hmfree(
                map_raw(rsm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }

        for mode in [1, 2, 3] {
            c_seed(0x4444 + mode as usize);
            r_seed(0x4444 + mode as usize);
            let mut csm = c_shmode(size_of::<StringEntry>(), mode);
            let mut rsm = r_shmode(size_of::<StringEntry>(), mode);
            let mut strings = Vec::new();
            for index in 0..80 {
                let bytes = match index % 5 {
                    0 => Vec::new(),
                    1 => vec![b'a' + (index % 26) as u8],
                    2 => format!("shared-prefix-{index:03}").into_bytes(),
                    3 => vec![0x80 | index as u8, b'x', b'y'],
                    _ => vec![b'z'; 300 + index],
                };
                strings.push(CString::new(bytes).unwrap());
                let key = strings.last_mut().unwrap().as_ptr().cast_mut();
                csm = c_put(
                    csm,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                );
                rsm = r_put(
                    rsm,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    1,
                );
                set_string_value(csm, index as c_int);
                set_string_value(rsm, index as c_int);
                if index == 0 {
                    if mode == 1 {
                        assert_eq!((*csm.cast::<StringEntry>()).key, key);
                        assert_eq!((*rsm.cast::<StringEntry>()).key, key);
                    } else {
                        assert_ne!((*csm.cast::<StringEntry>()).key, key);
                        assert_ne!((*rsm.cast::<StringEntry>()).key, key);
                    }
                }
                assert_eq!(string_snapshot(csm), string_snapshot(rsm));
            }

            for index in (0..80).step_by(3) {
                let key = strings[index].as_ptr().cast_mut();
                csm = c_del(
                    csm,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                rsm = r_del(
                    rsm,
                    size_of::<StringEntry>(),
                    key.cast(),
                    size_of::<*mut c_char>(),
                    0,
                    1,
                );
                assert_eq!(string_snapshot(csm), string_snapshot(rsm));
            }
            c_hmfree(
                map_raw(csm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            r_hmfree(
                map_raw(rsm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }

        // Out-of-range stored string modes take the C switch default: hash the
        // input as a string but copy key_size raw bytes into the key field.
        for mode in [4, 255] {
            let bytes = CString::new("abcdefgh-out-of-range").unwrap();
            let key = bytes.as_ptr().cast_mut();
            let mut csm = c_shmode(size_of::<StringEntry>(), mode);
            let mut rsm = r_shmode(size_of::<StringEntry>(), mode);
            csm = c_put(
                csm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                1,
            );
            rsm = r_put(
                rsm,
                size_of::<StringEntry>(),
                key.cast(),
                size_of::<*mut c_char>(),
                1,
            );
            set_string_value(csm, 77);
            set_string_value(rsm, 77);
            let ce = *csm.cast::<StringEntry>();
            let re = *rsm.cast::<StringEntry>();
            assert_eq!(ce.key as usize, re.key as usize);
            assert_eq!(ce.value, re.value);
            c_hmfree(
                map_raw(csm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
            r_hmfree(
                map_raw(rsm, size_of::<StringEntry>()),
                size_of::<StringEntry>(),
            );
        }

        // Mode 0 and negative modes use binary hashing/copy branches.
        for mode in [0, -9] {
            let mut binary_key = 0x1234_5678u32;
            let mut cm = c_shmode(size_of::<Pair>(), mode);
            let mut rm = r_shmode(size_of::<Pair>(), mode);
            cm = c_put(
                cm,
                size_of::<Pair>(),
                (&mut binary_key as *mut u32).cast(),
                size_of::<u32>(),
                mode,
            );
            rm = r_put(
                rm,
                size_of::<Pair>(),
                (&mut binary_key as *mut u32).cast(),
                size_of::<u32>(),
                mode,
            );
            set_pair_value(cm, 99);
            set_pair_value(rm, 99);
            assert_eq!(pair_snapshot(cm), pair_snapshot(rm));
            c_hmfree(map_raw(cm, size_of::<Pair>()), size_of::<Pair>());
            r_hmfree(map_raw(rm, size_of::<Pair>()), size_of::<Pair>());
        }

        // CONFIGS 41-47: string arena short/reuse/growth/oversized/max/reset.
        let c_alloc: StrAlloc = c.symbol(b"stbds_stralloc");
        let r_alloc: StrAlloc = rust.symbol(b"stbds_stralloc");
        let c_reset: StrReset = c.symbol(b"stbds_strreset");
        let r_reset: StrReset = rust.symbol(b"stbds_strreset");
        let mut ca = Arena::default();
        let mut ra = Arena::default();
        for len in [0usize, 1, 7, 200, 400, 513, 20, 1100, 17, 5000] {
            let string = CString::new(vec![b'q'; len]).unwrap();
            let cp = c_alloc(&mut ca, string.as_ptr().cast_mut());
            let rp = r_alloc(&mut ra, string.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!(
                (ca.remaining, ca.block, ca.mode),
                (ra.remaining, ra.block, ra.mode)
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
        c_reset(&mut ca);
        r_reset(&mut ra);

        let oversized = CString::new(vec![b'o'; 513]).unwrap();
        let cp = c_alloc(&mut ca, oversized.as_ptr().cast_mut());
        let rp = r_alloc(&mut ra, oversized.as_ptr().cast_mut());
        assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        c_reset(&mut ca);
        r_reset(&mut ra);

        // Drive the block counter to its cap with dedicated allocations.
        for _ in 0..24 {
            let block_size = 512usize << ((ca.block as usize) >> 1);
            let len = block_size.saturating_add(1).min((1 << 20) + 1);
            let string = CString::new(vec![b'm'; len]).unwrap();
            let cp = c_alloc(&mut ca, string.as_ptr().cast_mut());
            let rp = r_alloc(&mut ra, string.as_ptr().cast_mut());
            assert_eq!(CStr::from_ptr(cp).to_bytes(), CStr::from_ptr(rp).to_bytes());
            assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
        }
        assert_eq!(ca.block, ra.block);
        c_reset(&mut ca);
        r_reset(&mut ra);

        // CONFIG 48: static formatting buffer.
        let c_strkey: StrKey = c.symbol(b"strkey");
        let r_strkey: StrKey = rust.symbol(b"strkey");
        for number in [c_int::MIN, -100, -1, 0, 1, 42, c_int::MAX] {
            assert_eq!(
                CStr::from_ptr(c_strkey(number)).to_bytes(),
                CStr::from_ptr(r_strkey(number)).to_bytes()
            );
        }

        // CONFIGS 49-51 and the str_dups assertions: capture actual C stdout.
        let c_dups: StrDups = c.symbol(b"str_dups");
        let r_dups: StrDups = rust.symbol(b"str_dups");
        for number in [-3, 0, 1, 12, 2_000] {
            let c_output = capture_stdout(c_dups, number);
            let r_output = capture_stdout(r_dups, number);
            assert_eq!(c_output, r_output, "str_dups({number})");
            assert_eq!(c_output, format!("a {number}\n").as_bytes());
        }
    }

    // Generic null-pointer boundaries are undefined by C, but both translations
    // must reject by process termination rather than silently returning.
    for case in [
        "hash_string_null",
        "hash_bytes_null",
        "arrfree_null",
        "map_key_null",
        "temp_null",
        "stralloc_arena_null",
        "stralloc_string_null",
        "strreset_null",
        "arrgrow_oversized",
        "assert_hash_threshold",
        "assert_moved_missing",
        "assert_moved_index",
    ] {
        let c_status = run_crash_child("c", case);
        let rust_status = run_crash_child("rust", case);
        if case == "arrgrow_oversized" {
            assert_eq!(
                c_status.success(),
                rust_status.success(),
                "oversized allocation termination differs"
            );
        } else {
            assert_child_failed(c_status, &format!("C {case}"));
            assert_child_failed(rust_status, &format!("Rust {case}"));
        }
    }
}

#[test]
fn ffi_crash_child() {
    let Ok(which) = env::var("DIFF_CRASH_LIB") else {
        return;
    };
    let case = env::var("DIFF_CRASH_CASE").unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = if which == "c" {
        root.join("../c_src/build/libharvest-work-OCFKN1.so")
    } else {
        root.join("target/release/libstr_dups_lib.so")
    };
    unsafe {
        let api = Api::load(path);
        match case.as_str() {
            "hash_string_null" => {
                let function: HashString = api.symbol(b"stbds_hash_string");
                function(ptr::null_mut(), 0);
            }
            "hash_bytes_null" => {
                let function: HashBytes = api.symbol(b"stbds_hash_bytes");
                function(ptr::null_mut(), 1, 0);
            }
            "arrfree_null" => {
                let function: ArrFree = api.symbol(b"stbds_arrfreef");
                function(ptr::null_mut());
            }
            "map_key_null" => {
                let function: HmPut = api.symbol(b"stbds_hmput_key");
                let map = function(
                    ptr::null_mut(),
                    size_of::<Pair>(),
                    ptr::null_mut(),
                    size_of::<u32>(),
                    0,
                );
                let free: HmFree = api.symbol(b"stbds_hmfree_func");
                free(map_raw(map, size_of::<Pair>()), size_of::<Pair>());
            }
            "temp_null" => {
                let function: HmGetTs = api.symbol(b"stbds_hmget_key_ts");
                let mut key = 1u32;
                function(
                    ptr::null_mut(),
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    ptr::null_mut(),
                    0,
                );
            }
            "stralloc_arena_null" => {
                let function: StrAlloc = api.symbol(b"stbds_stralloc");
                let string = CString::new("x").unwrap();
                function(ptr::null_mut(), string.as_ptr().cast_mut());
            }
            "stralloc_string_null" => {
                let function: StrAlloc = api.symbol(b"stbds_stralloc");
                let mut arena = Arena::default();
                function(&mut arena, ptr::null_mut());
            }
            "strreset_null" => {
                let function: StrReset = api.symbol(b"stbds_strreset");
                function(ptr::null_mut());
            }
            "arrgrow_oversized" => {
                let function: ArrGrow = api.symbol(b"stbds_arrgrowf");
                let free: ArrFree = api.symbol(b"stbds_arrfreef");
                let array = function(ptr::null_mut(), usize::MAX, 1, 0);
                if !array.is_null() {
                    free(array);
                }
            }
            "assert_hash_threshold" => {
                let mode: ShMode = api.symbol(b"stbds_shmode_func");
                let put: HmPut = api.symbol(b"stbds_hmput_key");
                let map = mode(size_of::<Pair>(), 0);
                let hash_table = table(map, size_of::<Pair>());
                (*hash_table).slot_count = 1;
                (*hash_table).used_count = 0;
                (*hash_table).used_count_threshold = 0;
                let mut key = 1u32;
                put(
                    map,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                );
            }
            "assert_moved_missing" | "assert_moved_index" => {
                let put: HmPut = api.symbol(b"stbds_hmput_key");
                let delete: HmDel = api.symbol(b"stbds_hmdel_key");
                let mut map = ptr::null_mut();
                for mut key in [1u32, 2u32] {
                    map = put(
                        map,
                        size_of::<Pair>(),
                        (&mut key as *mut u32).cast(),
                        size_of::<u32>(),
                        0,
                    );
                }
                let hash_table = table(map, size_of::<Pair>());
                let buckets = std::slice::from_raw_parts_mut(
                    (*hash_table).storage,
                    (*hash_table).slot_count / 8,
                );
                let mut changed = false;
                for bucket in buckets {
                    for slot in 0..8 {
                        if bucket.index[slot] == 1 {
                            if case == "assert_moved_missing" {
                                bucket.hash[slot] = usize::MAX;
                            } else {
                                bucket.index[slot] = 0;
                            }
                            changed = true;
                        }
                    }
                }
                assert!(changed);
                let mut key = 1u32;
                delete(
                    map,
                    size_of::<Pair>(),
                    (&mut key as *mut u32).cast(),
                    size_of::<u32>(),
                    0,
                    0,
                );
            }
            _ => panic!("unknown crash case"),
        }
    }
}
