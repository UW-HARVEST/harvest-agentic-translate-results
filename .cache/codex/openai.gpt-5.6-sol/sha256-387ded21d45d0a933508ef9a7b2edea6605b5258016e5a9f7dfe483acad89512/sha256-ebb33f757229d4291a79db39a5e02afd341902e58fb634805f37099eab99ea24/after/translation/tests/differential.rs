#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

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
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
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
    string: StringArena,
    storage: *mut HashBucket,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct BinEntry {
    key: u64,
    value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StrEntry {
    key: *mut c_char,
    value: i64,
}

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            lib: Library::new(path).unwrap_or_else(|e| panic!("load {}: {e}", path.display())),
        }
    }

    unsafe fn arrgrow(
        &self,
        a: *mut c_void,
        elem: usize,
        add: usize,
        min: usize,
    ) -> *mut c_void {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void>(
                b"stbds_arrgrowf\0",
            )
            .unwrap();
        f(a, elem, add, min)
    }

    unsafe fn arrfree(&self, a: *mut c_void) {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_void)>(b"stbds_arrfreef\0")
            .unwrap();
        f(a)
    }

    unsafe fn rand_seed(&self, seed: usize) {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(usize)>(b"stbds_rand_seed\0")
            .unwrap();
        f(seed)
    }

    unsafe fn hash_string(&self, p: *mut c_char, seed: usize) -> usize {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_char, usize) -> usize>(b"stbds_hash_string\0")
            .unwrap();
        f(p, seed)
    }

    unsafe fn hash_bytes(&self, p: *mut c_void, len: usize, seed: usize) -> usize {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_void, usize, usize) -> usize>(b"stbds_hash_bytes\0")
            .unwrap();
        f(p, len, seed)
    }

    unsafe fn hmfree(&self, a: *mut c_void, elem: usize) {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_void, usize)>(b"stbds_hmfree_func\0")
            .unwrap();
        f(a, elem)
    }

    unsafe fn hmget_ts(
        &self,
        a: *mut c_void,
        elem: usize,
        key: *mut c_void,
        key_size: usize,
        temp: *mut isize,
        mode: c_int,
    ) -> *mut c_void {
        let f = self
            .lib
            .get::<
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    *mut isize,
                    c_int,
                ) -> *mut c_void,
            >(b"stbds_hmget_key_ts\0")
            .unwrap();
        f(a, elem, key, key_size, temp, mode)
    }

    unsafe fn hmget(
        &self,
        a: *mut c_void,
        elem: usize,
        key: *mut c_void,
        key_size: usize,
        mode: c_int,
    ) -> *mut c_void {
        let f = self
            .lib
            .get::<
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    c_int,
                ) -> *mut c_void,
            >(b"stbds_hmget_key\0")
            .unwrap();
        f(a, elem, key, key_size, mode)
    }

    unsafe fn hmdefault(&self, a: *mut c_void, elem: usize) -> *mut c_void {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>(
                b"stbds_hmput_default\0",
            )
            .unwrap();
        f(a, elem)
    }

    unsafe fn hmput(
        &self,
        a: *mut c_void,
        elem: usize,
        key: *mut c_void,
        key_size: usize,
        mode: c_int,
    ) -> *mut c_void {
        let f = self
            .lib
            .get::<
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    c_int,
                ) -> *mut c_void,
            >(b"stbds_hmput_key\0")
            .unwrap();
        f(a, elem, key, key_size, mode)
    }

    unsafe fn shmode(&self, elem: usize, mode: c_int) -> *mut c_void {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(usize, c_int) -> *mut c_void>(b"stbds_shmode_func\0")
            .unwrap();
        f(elem, mode)
    }

    unsafe fn hmdel(
        &self,
        a: *mut c_void,
        elem: usize,
        key: *mut c_void,
        key_size: usize,
        key_offset: usize,
        mode: c_int,
    ) -> *mut c_void {
        let f = self
            .lib
            .get::<
                unsafe extern "C" fn(
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    usize,
                    c_int,
                ) -> *mut c_void,
            >(b"stbds_hmdel_key\0")
            .unwrap();
        f(a, elem, key, key_size, key_offset, mode)
    }

    unsafe fn stralloc(&self, arena: *mut StringArena, string: *mut c_char) -> *mut c_char {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char>(
                b"stbds_stralloc\0",
            )
            .unwrap();
        f(arena, string)
    }

    unsafe fn strreset(&self, arena: *mut StringArena) {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(*mut StringArena)>(b"stbds_strreset\0")
            .unwrap();
        f(arena)
    }

    unsafe fn strkey(&self, n: c_int) -> *mut c_char {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(c_int) -> *mut c_char>(b"strkey\0")
            .unwrap();
        f(n)
    }

    unsafe fn sh_geti(&self, n: c_int) {
        let f = self
            .lib
            .get::<unsafe extern "C" fn(c_int)>(b"sh_geti\0")
            .unwrap();
        f(n)
    }
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_path() -> PathBuf {
    manifest().join("../c_src/build/libharvest-work-6tAMOr.so")
}

fn rust_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let preferred = manifest().join(format!("target/{profile}/libsh_geti_lib.so"));
    if preferred.exists() {
        preferred
    } else {
        manifest().join("target/release/libsh_geti_lib.so")
    }
}

unsafe fn header_from_array(a: *mut c_void) -> *mut ArrayHeader {
    a.cast::<ArrayHeader>().sub(1)
}

unsafe fn raw_from_map(map: *mut c_void, elem: usize) -> *mut c_void {
    map.cast::<u8>().sub(elem).cast()
}

unsafe fn map_header(map: *mut c_void, elem: usize) -> ArrayHeader {
    *header_from_array(raw_from_map(map, elem))
}

unsafe fn map_table(map: *mut c_void, elem: usize) -> Option<HashIndex> {
    let h = map_header(map, elem);
    if h.hash_table.is_null() {
        None
    } else {
        Some(*h.hash_table.cast::<HashIndex>())
    }
}

unsafe fn c_string_bytes(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p.cast::<u8>();
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
struct MapMeta {
    length: usize,
    capacity: usize,
    temp: isize,
    slot_count: usize,
    used: usize,
    tombstones: usize,
    seed: usize,
    string_mode: u8,
    arena_remaining: usize,
    arena_block: u8,
}

unsafe fn meta(map: *mut c_void, elem: usize) -> MapMeta {
    let h = map_header(map, elem);
    let t = map_table(map, elem);
    MapMeta {
        length: h.length,
        capacity: h.capacity,
        temp: h.temp,
        slot_count: t.map_or(0, |x| x.slot_count),
        used: t.map_or(0, |x| x.used_count),
        tombstones: t.map_or(0, |x| x.tombstone_count),
        seed: t.map_or(0, |x| x.seed),
        string_mode: t.map_or(0, |x| x.string.mode),
        arena_remaining: t.map_or(0, |x| x.string.remaining),
        arena_block: t.map_or(0, |x| x.string.block),
    }
}

unsafe fn bin_snapshot(map: *mut c_void) -> (MapMeta, Vec<(u64, i64)>) {
    let m = meta(map, size_of::<BinEntry>());
    let entries = (0..m.length.saturating_sub(1))
        .map(|i| {
            let e = *map.cast::<BinEntry>().add(i);
            (e.key, e.value)
        })
        .collect();
    (m, entries)
}

unsafe fn str_snapshot(map: *mut c_void) -> (MapMeta, Vec<(Vec<u8>, i64)>) {
    let m = meta(map, size_of::<StrEntry>());
    let entries = (0..m.length.saturating_sub(1))
        .map(|i| {
            let e = *map.cast::<StrEntry>().add(i);
            (c_string_bytes(e.key), e.value)
        })
        .collect();
    (m, entries)
}

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

unsafe fn compare_hashes(c: &Api, r: &Api) {
    let mut state = 0x82d3_1c4f_77aa_091bu64;
    for len in 0..160usize {
        for _ in 0..24 {
            let seed = next_random(&mut state) as usize;
            let mut bytes = vec![0u8; len.max(1)];
            for b in bytes.iter_mut().take(len) {
                *b = next_random(&mut state) as u8;
            }
            let p = if len == 0 && state & 1 == 0 {
                null_mut()
            } else {
                bytes.as_mut_ptr().cast()
            };
            assert_eq!(
                c.hash_bytes(p, len, seed),
                r.hash_bytes(p, len, seed),
                "hash bytes len={len} seed={seed:#x} bytes={:?}",
                &bytes[..len]
            );

            let string_len = len.min(96);
            let mut string = Vec::with_capacity(string_len + 1);
            for _ in 0..string_len {
                let mut b = next_random(&mut state) as u8;
                if b == 0 {
                    b = 0x80;
                }
                string.push(b);
            }
            string.push(0);
            assert_eq!(
                c.hash_string(string.as_mut_ptr().cast(), seed),
                r.hash_string(string.as_mut_ptr().cast(), seed),
                "hash string len={string_len} seed={seed:#x}"
            );
        }
    }
}

unsafe fn compare_arrays(c: &Api, r: &Api) {
    for &(elem, add, min) in &[
        (1, 0, 0),
        (1, 1, 0),
        (4, 0, 1),
        (8, 0, 4),
        (3, 9, 0),
        (16, 0, 33),
    ] {
        let ca = c.arrgrow(null_mut(), elem, add, min);
        let ra = r.arrgrow(null_mut(), elem, add, min);
        assert_eq!(ca.is_null(), ra.is_null());
        if !ca.is_null() {
            assert_eq!(*header_from_array(ca), *header_from_array(ra));
            c.arrfree(ca);
            r.arrfree(ra);
        }
    }

    let mut ca = c.arrgrow(null_mut(), 8, 0, 4);
    let mut ra = r.arrgrow(null_mut(), 8, 0, 4);
    (*header_from_array(ca)).length = 3;
    (*header_from_array(ra)).length = 3;
    for &(add, min) in &[(0, 3), (1, 0), (2, 0), (0, 40), (37, 0)] {
        ca = c.arrgrow(ca, 8, add, min);
        ra = r.arrgrow(ra, 8, add, min);
        assert_eq!(*header_from_array(ca), *header_from_array(ra));
    }
    c.arrfree(ca);
    r.arrfree(ra);
}

unsafe fn compare_arenas(c: &Api, r: &Api) {
    let mut ca = StringArena {
        storage: null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut ra = ca;
    let lengths = [
        0usize, 1, 7, 100, 400, 510, 511, 512, 513, 800, 2047, 4096, 600_000,
        1_048_576, 1_048_577,
    ];
    for &len in &lengths {
        let mut bytes = vec![b'x'; len];
        bytes.push(0);
        let cp = c.stralloc(&mut ca, bytes.as_mut_ptr().cast());
        let rp = r.stralloc(&mut ra, bytes.as_mut_ptr().cast());
        assert_eq!(c_string_bytes(cp), c_string_bytes(rp), "stralloc len={len}");
        assert_eq!(
            (ca.remaining, ca.block, ca.mode),
            (ra.remaining, ra.block, ra.mode),
            "arena state len={len}"
        );
    }
    c.strreset(&mut ca);
    r.strreset(&mut ra);
    assert_eq!(
        (ca.storage.is_null(), ca.remaining, ca.block, ca.mode),
        (ra.storage.is_null(), ra.remaining, ra.block, ra.mode)
    );
    c.strreset(&mut ca);
    r.strreset(&mut ra);

    let mut ca = StringArena {
        storage: null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    let mut ra = ca;
    for _ in 0..28 {
        let block_size = 512usize << ((ca.block as usize) >> 1);
        let data_len = block_size.min(1 << 20);
        let mut bytes = vec![b'g'; data_len];
        bytes.push(0);
        let cp = c.stralloc(&mut ca, bytes.as_mut_ptr().cast());
        let rp = r.stralloc(&mut ra, bytes.as_mut_ptr().cast());
        assert_eq!(c_string_bytes(cp), c_string_bytes(rp));
        assert_eq!(
            (ca.remaining, ca.block, ca.mode),
            (ra.remaining, ra.block, ra.mode)
        );
    }
    assert!(ca.block >= 22);
    assert_eq!(ca.block, ra.block);
    c.strreset(&mut ca);
    r.strreset(&mut ra);
}

unsafe fn compare_seed_extremes(c: &Api, r: &Api) {
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        c.rand_seed(seed);
        r.rand_seed(seed);
        let mut key = 0x8877_6655_4433_2211u64;
        let cm = c.hmput(
            null_mut(),
            size_of::<BinEntry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        );
        let rm = r.hmput(
            null_mut(),
            size_of::<BinEntry>(),
            (&mut key as *mut u64).cast(),
            size_of::<u64>(),
            0,
        );
        assert_eq!(meta(cm, size_of::<BinEntry>()), meta(rm, size_of::<BinEntry>()));
        assert_eq!(map_table(cm, size_of::<BinEntry>()).unwrap().seed, seed);
        c.hmfree(raw_from_map(cm, size_of::<BinEntry>()), size_of::<BinEntry>());
        r.hmfree(raw_from_map(rm, size_of::<BinEntry>()), size_of::<BinEntry>());
    }
}

unsafe fn compare_binary_key_sizes(c: &Api, r: &Api) {
    const ELEM: usize = 24;
    for key_size in [0usize, 1, 3, 4, 8, 15] {
        c.rand_seed(0x9999);
        r.rand_seed(0x9999);
        let mut cm = null_mut();
        let mut rm = null_mut();
        let mut key1 = [0x11u8; 16];
        let mut key2 = [0x22u8; 16];
        cm = c.hmput(cm, ELEM, key1.as_mut_ptr().cast(), key_size, 0);
        rm = r.hmput(rm, ELEM, key1.as_mut_ptr().cast(), key_size, 0);
        assert_eq!(meta(cm, ELEM), meta(rm, ELEM));
        std::ptr::write_bytes(cm.cast::<u8>().add(key_size), 0x5a, ELEM - key_size);
        std::ptr::write_bytes(rm.cast::<u8>().add(key_size), 0x5a, ELEM - key_size);
        assert_eq!(
            std::slice::from_raw_parts(cm.cast::<u8>(), ELEM),
            std::slice::from_raw_parts(rm.cast::<u8>(), ELEM)
        );
        cm = c.hmput(cm, ELEM, key2.as_mut_ptr().cast(), key_size, 0);
        rm = r.hmput(rm, ELEM, key2.as_mut_ptr().cast(), key_size, 0);
        assert_eq!(meta(cm, ELEM), meta(rm, ELEM), "key_size={key_size}");
        let mut ct = 99isize;
        let mut rt = 99isize;
        cm = c.hmget_ts(cm, ELEM, key1.as_mut_ptr().cast(), key_size, &mut ct, 0);
        rm = r.hmget_ts(rm, ELEM, key1.as_mut_ptr().cast(), key_size, &mut rt, 0);
        assert_eq!(ct, rt, "key_size={key_size}");
        c.hmfree(raw_from_map(cm, ELEM), ELEM);
        r.hmfree(raw_from_map(rm, ELEM), ELEM);
    }
}

unsafe fn put_bin(api: &Api, map: &mut *mut c_void, key: u64, value: i64, mode: c_int) {
    let mut key_copy = key;
    *map = api.hmput(
        *map,
        size_of::<BinEntry>(),
        (&mut key_copy as *mut u64).cast(),
        size_of::<u64>(),
        mode,
    );
    let index = map_header(*map, size_of::<BinEntry>()).temp;
    (*map.cast::<BinEntry>().offset(index)).value = value;
}

unsafe fn get_bin(api: &Api, map: &mut *mut c_void, key: u64, mode: c_int) -> isize {
    let mut key_copy = key;
    *map = api.hmget(
        *map,
        size_of::<BinEntry>(),
        (&mut key_copy as *mut u64).cast(),
        size_of::<u64>(),
        mode,
    );
    map_header(*map, size_of::<BinEntry>()).temp
}

unsafe fn del_bin(api: &Api, map: &mut *mut c_void, key: u64, mode: c_int) -> isize {
    let mut key_copy = key;
    *map = api.hmdel(
        *map,
        size_of::<BinEntry>(),
        (&mut key_copy as *mut u64).cast(),
        size_of::<u64>(),
        0,
        mode,
    );
    if map.is_null() {
        0
    } else {
        map_header(*map, size_of::<BinEntry>()).temp
    }
}

unsafe fn compare_binary_maps(c: &Api, r: &Api, mode: c_int) {
    c.rand_seed(0x1234_5678_9abc_def0);
    r.rand_seed(0x1234_5678_9abc_def0);
    let mut cm = null_mut();
    let mut rm = null_mut();

    cm = c.hmdefault(cm, size_of::<BinEntry>());
    rm = r.hmdefault(rm, size_of::<BinEntry>());
    (*cm.cast::<BinEntry>().offset(-1)).value = -77;
    (*rm.cast::<BinEntry>().offset(-1)).value = -77;
    assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
    let cm_before = cm;
    let rm_before = rm;
    cm = c.hmdefault(cm, size_of::<BinEntry>());
    rm = r.hmdefault(rm, size_of::<BinEntry>());
    assert_eq!(cm, cm_before);
    assert_eq!(rm, rm_before);
    assert_eq!(get_bin(c, &mut cm, 999, mode), -1);
    assert_eq!(get_bin(r, &mut rm, 999, mode), -1);

    let mut state = 0xbadc_0ffe_e0dd_f00du64;
    for step in 0..700 {
        let key = next_random(&mut state) % 180;
        match next_random(&mut state) % 4 {
            0 | 1 => {
                let value = next_random(&mut state) as i64;
                put_bin(c, &mut cm, key, value, mode);
                put_bin(r, &mut rm, key, value, mode);
            }
            2 => {
                assert_eq!(
                    get_bin(c, &mut cm, key, mode),
                    get_bin(r, &mut rm, key, mode),
                    "binary get step={step} key={key}"
                );
            }
            _ => {
                assert_eq!(
                    del_bin(c, &mut cm, key, mode),
                    del_bin(r, &mut rm, key, mode),
                    "binary delete step={step} key={key}"
                );
            }
        }
        assert_eq!(
            bin_snapshot(cm),
            bin_snapshot(rm),
            "binary state step={step} mode={mode}"
        );
    }
    c.hmfree(raw_from_map(cm, size_of::<BinEntry>()), size_of::<BinEntry>());
    r.hmfree(raw_from_map(rm, size_of::<BinEntry>()), size_of::<BinEntry>());
    c.hmfree(null_mut(), size_of::<BinEntry>());
    r.hmfree(null_mut(), size_of::<BinEntry>());
}

unsafe fn compare_map_transitions(c: &Api, r: &Api) {
    c.rand_seed(0x4242);
    r.rand_seed(0x4242);
    let mut cm = null_mut();
    let mut rm = null_mut();
    for key in 0..256u64 {
        put_bin(c, &mut cm, key, key as i64, 0);
        put_bin(r, &mut rm, key, key as i64, 0);
    }
    assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
    assert!(meta(cm, size_of::<BinEntry>()).slot_count >= 512);

    let mut saw_rebuild = false;
    let mut saw_shrink = false;
    let mut previous = meta(cm, size_of::<BinEntry>());
    for key in (0..256u64).step_by(2) {
        assert_eq!(del_bin(c, &mut cm, key, 0), del_bin(r, &mut rm, key, 0));
        let current = meta(cm, size_of::<BinEntry>());
        assert_eq!(current, meta(rm, size_of::<BinEntry>()));
        if current.slot_count == previous.slot_count && current.tombstones < previous.tombstones {
            saw_rebuild = true;
        }
        if current.slot_count < previous.slot_count {
            saw_shrink = true;
        }
        previous = current;
    }
    for key in (1..256u64).step_by(2).take(110) {
        assert_eq!(del_bin(c, &mut cm, key, 0), del_bin(r, &mut rm, key, 0));
        let current = meta(cm, size_of::<BinEntry>());
        assert_eq!(current, meta(rm, size_of::<BinEntry>()));
        if current.slot_count == previous.slot_count && current.tombstones < previous.tombstones {
            saw_rebuild = true;
        }
        if current.slot_count < previous.slot_count {
            saw_shrink = true;
        }
        previous = current;
    }
    assert!(saw_rebuild, "expected same-size tombstone rebuild");
    assert!(saw_shrink, "expected table shrink");
    c.hmfree(raw_from_map(cm, size_of::<BinEntry>()), size_of::<BinEntry>());
    r.hmfree(raw_from_map(rm, size_of::<BinEntry>()), size_of::<BinEntry>());
}

unsafe fn put_str(
    api: &Api,
    map: &mut *mut c_void,
    key: *mut c_char,
    value: i64,
    mode: c_int,
) {
    *map = api.hmput(
        *map,
        size_of::<StrEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        mode,
    );
    let index = map_header(*map, size_of::<StrEntry>()).temp;
    (*map.cast::<StrEntry>().offset(index)).value = value;
}

unsafe fn get_str(api: &Api, map: &mut *mut c_void, key: *mut c_char, mode: c_int) -> isize {
    *map = api.hmget(
        *map,
        size_of::<StrEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        mode,
    );
    map_header(*map, size_of::<StrEntry>()).temp
}

unsafe fn del_str(api: &Api, map: &mut *mut c_void, key: *mut c_char, mode: c_int) -> isize {
    *map = api.hmdel(
        *map,
        size_of::<StrEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        0,
        mode,
    );
    map_header(*map, size_of::<StrEntry>()).temp
}

unsafe fn compare_string_maps(c: &Api, r: &Api, ownership: c_int, call_mode: c_int) {
    c.rand_seed(0x3141_5926);
    r.rand_seed(0x3141_5926);
    let mut cm = if ownership == 1 {
        null_mut()
    } else {
        c.shmode(size_of::<StrEntry>(), ownership)
    };
    let mut rm = if ownership == 1 {
        null_mut()
    } else {
        r.shmode(size_of::<StrEntry>(), ownership)
    };
    let mut keys: Vec<CString> = (0..140)
        .map(|i| CString::new(format!("key_{i:03}_{}", (i * 7919) % 9973)).unwrap())
        .collect();
    let mut state = 0x5050_a1a1_7777_eeeeu64;

    for step in 0..650 {
        let k = (next_random(&mut state) % keys.len() as u64) as usize;
        let p = keys[k].as_ptr() as *mut c_char;
        match next_random(&mut state) % 4 {
            0 | 1 => {
                let value = next_random(&mut state) as i64;
                put_str(c, &mut cm, p, value, call_mode);
                put_str(r, &mut rm, p, value, call_mode);
            }
            2 => {
                assert_eq!(
                    get_str(c, &mut cm, p, call_mode),
                    get_str(r, &mut rm, p, call_mode),
                    "string get step={step}"
                );
            }
            _ => {
                assert_eq!(
                    del_str(c, &mut cm, p, call_mode),
                    del_str(r, &mut rm, p, call_mode),
                    "string delete step={step}"
                );
            }
        }
        assert_eq!(
            str_snapshot(cm),
            str_snapshot(rm),
            "string state step={step} ownership={ownership} mode={call_mode}"
        );
    }

    c.hmfree(raw_from_map(cm, size_of::<StrEntry>()), size_of::<StrEntry>());
    r.hmfree(raw_from_map(rm, size_of::<StrEntry>()), size_of::<StrEntry>());
    keys.clear();
}

unsafe fn compare_mode_four(c: &Api, r: &Api) {
    c.rand_seed(7);
    r.rand_seed(7);
    let mut cm = c.shmode(size_of::<StrEntry>(), 4);
    let mut rm = r.shmode(size_of::<StrEntry>(), 4);
    let key = CString::new("abcdefghijk").unwrap();
    cm = c.hmput(
        cm,
        size_of::<StrEntry>(),
        key.as_ptr() as *mut c_void,
        size_of::<*mut c_char>(),
        4,
    );
    rm = r.hmput(
        rm,
        size_of::<StrEntry>(),
        key.as_ptr() as *mut c_void,
        size_of::<*mut c_char>(),
        4,
    );
    let cb = std::slice::from_raw_parts(cm.cast::<u8>(), size_of::<StrEntry>()).to_vec();
    let rb = std::slice::from_raw_parts(rm.cast::<u8>(), size_of::<StrEntry>()).to_vec();
    assert_eq!(meta(cm, size_of::<StrEntry>()), meta(rm, size_of::<StrEntry>()));
    assert_eq!(cb, rb);
    c.hmfree(raw_from_map(cm, size_of::<StrEntry>()), size_of::<StrEntry>());
    r.hmfree(raw_from_map(rm, size_of::<StrEntry>()), size_of::<StrEntry>());
}

unsafe fn compare_get_ts(c: &Api, r: &Api) {
    let mut ct = 123isize;
    let mut rt = 123isize;
    let cm = c.hmget_ts(
        null_mut(),
        size_of::<BinEntry>(),
        null_mut(),
        size_of::<u64>(),
        &mut ct,
        0,
    );
    let rm = r.hmget_ts(
        null_mut(),
        size_of::<BinEntry>(),
        null_mut(),
        size_of::<u64>(),
        &mut rt,
        0,
    );
    assert_eq!(ct, rt);
    assert_eq!(bin_snapshot(cm), bin_snapshot(rm));
    c.hmfree(raw_from_map(cm, size_of::<BinEntry>()), size_of::<BinEntry>());
    r.hmfree(raw_from_map(rm, size_of::<BinEntry>()), size_of::<BinEntry>());

    let mut cm = c.hmdefault(null_mut(), size_of::<BinEntry>());
    let mut rm = r.hmdefault(null_mut(), size_of::<BinEntry>());
    let mut key = 44u64;
    ct = 123;
    rt = 123;
    let cm_same = c.hmget_ts(
        cm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut ct,
        0,
    );
    let rm_same = r.hmget_ts(
        rm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut rt,
        0,
    );
    assert_eq!(cm, cm_same);
    assert_eq!(rm, rm_same);
    assert_eq!(ct, rt);
    put_bin(c, &mut cm, key, 9, 0);
    put_bin(r, &mut rm, key, 9, 0);
    ct = -7;
    rt = -7;
    cm = c.hmget_ts(
        cm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut ct,
        0,
    );
    rm = r.hmget_ts(
        rm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut rt,
        0,
    );
    assert_eq!(ct, rt);
    assert_eq!(ct, 0);
    key = 45;
    cm = c.hmget_ts(
        cm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut ct,
        0,
    );
    rm = r.hmget_ts(
        rm,
        size_of::<BinEntry>(),
        (&mut key as *mut u64).cast(),
        size_of::<u64>(),
        &mut rt,
        0,
    );
    assert_eq!(ct, rt);
    assert_eq!(ct, -1);
    c.hmfree(raw_from_map(cm, size_of::<BinEntry>()), size_of::<BinEntry>());
    r.hmfree(raw_from_map(rm, size_of::<BinEntry>()), size_of::<BinEntry>());
}

unsafe fn compare_strkey(c: &Api, r: &Api) {
    for n in [c_int::MIN, -1000, -1, 0, 1, 99999, c_int::MAX] {
        let cb = c_string_bytes(c.strkey(n));
        let rb = c_string_bytes(r.strkey(n));
        assert_eq!(cb, rb, "strkey({n})");
    }
}

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut fds = [0; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    let saved = dup(1);
    assert!(saved >= 0);
    assert_eq!(fflush(null_mut()), 0);
    assert_eq!(dup2(fds[1], 1), 1);
    close(fds[1]);
    call();
    assert_eq!(fflush(null_mut()), 0);
    assert_eq!(dup2(saved, 1), 1);
    close(saved);
    let mut file = File::from_raw_fd(fds[0]);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    bytes
}

unsafe fn compare_driver(c: &Api, r: &Api) {
    for n in [-5, -1, 0, 1, 2, 7, 32, 129] {
        let co = capture_stdout(|| c.sh_geti(n));
        let ro = capture_stdout(|| r.sh_geti(n));
        assert_eq!(co, ro, "sh_geti stdout n={n}");
    }
}

#[test]
fn differential_valid_paths() {
    let _guard = SERIAL.lock().unwrap();
    unsafe {
        let c = Api::open(&c_path());
        let r = Api::open(&rust_path());
        eprintln!("hashes");
        compare_hashes(&c, &r);
        eprintln!("seeds");
        compare_seed_extremes(&c, &r);
        eprintln!("arrays");
        compare_arrays(&c, &r);
        eprintln!("arenas");
        compare_arenas(&c, &r);
        eprintln!("get_ts");
        compare_get_ts(&c, &r);
        eprintln!("key sizes");
        compare_binary_key_sizes(&c, &r);
        eprintln!("binary 0");
        compare_binary_maps(&c, &r, 0);
        eprintln!("binary -1");
        compare_binary_maps(&c, &r, -1);
        eprintln!("map transitions");
        compare_map_transitions(&c, &r);
        eprintln!("string default");
        compare_string_maps(&c, &r, 1, 1);
        eprintln!("string strdup");
        compare_string_maps(&c, &r, 2, 1);
        eprintln!("string arena");
        compare_string_maps(&c, &r, 3, 1);
        eprintln!("string mode 4");
        compare_mode_four(&c, &r);
        eprintln!("strkey");
        compare_strkey(&c, &r);
        eprintln!("driver");
        compare_driver(&c, &r);
    }
}

unsafe fn run_crash_case(api: &Api, case: &str) {
    match case {
        "hash_string_null" => {
            api.hash_string(null_mut(), 1);
        }
        "hash_bytes_null" => {
            api.hash_bytes(null_mut(), 1, 1);
        }
        "hmget_temp_null" => {
            api.hmget_ts(
                null_mut(),
                size_of::<BinEntry>(),
                null_mut(),
                size_of::<u64>(),
                null_mut(),
                0,
            );
        }
        "stralloc_arena_null" => {
            let s = CString::new("x").unwrap();
            api.stralloc(null_mut(), s.as_ptr() as *mut c_char);
        }
        "stralloc_string_null" => {
            let mut arena = StringArena {
                storage: null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            api.stralloc(&mut arena, null_mut());
        }
        "strreset_null" => api.strreset(null_mut()),
        "hash_bytes_oversized" => {
            let mut byte = 1u8;
            api.hash_bytes((&mut byte as *mut u8).cast(), usize::MAX, 1);
        }
        "arrfree_null" => api.arrfree(null_mut()),
        "hmput_key_null" => {
            api.hmput(null_mut(), size_of::<BinEntry>(), null_mut(), 1, 0);
        }
        "hmget_key_null" => {
            let mut key = 1u64;
            let map = api.hmput(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            api.hmget(map, size_of::<BinEntry>(), null_mut(), 1, 0);
        }
        "hmdel_key_null" => {
            let mut key = 1u64;
            let map = api.hmput(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            api.hmdel(map, size_of::<BinEntry>(), null_mut(), 1, 0, 0);
        }
        "zero_elem_map" => {
            let mut key = 1u8;
            let map = api.hmput(null_mut(), 0, (&mut key as *mut u8).cast(), 0, 0);
            api.hmget(map, 0, (&mut key as *mut u8).cast(), 0, 0);
        }
        "mode_four_lookup" => {
            let map = api.shmode(size_of::<StrEntry>(), 4);
            let key = CString::new("abcdefghijk").unwrap();
            let map = api.hmput(
                map,
                size_of::<StrEntry>(),
                key.as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                4,
            );
            api.hmget(
                map,
                size_of::<StrEntry>(),
                key.as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                4,
            );
        }
        "make_index_assert" => {
            let mut key = 1u64;
            let map = api.hmput(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            let raw = raw_from_map(map, size_of::<BinEntry>());
            let table = (*header_from_array(raw)).hash_table.cast::<HashIndex>();
            (*table).slot_count = 1;
            (*table).used_count_threshold = 0;
            let mut key2 = 2u64;
            api.hmput(
                map,
                size_of::<BinEntry>(),
                (&mut key2 as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
        }
        "delete_slot_assert" => {
            let mut selected = None;
            for seed in 0..64usize {
                api.rand_seed(seed);
                let mut key = 0x1000u64 + seed as u64;
                let map = api.hmput(
                    null_mut(),
                    size_of::<BinEntry>(),
                    (&mut key as *mut u64).cast(),
                    size_of::<u64>(),
                    0,
                );
                let table = map_table(map, size_of::<BinEntry>()).unwrap();
                let mut slot = None;
                for i in 0..table.slot_count {
                    let bucket = table.storage.add(i >> 3);
                    if (*bucket).index[i & 7] == 0 {
                        slot = Some(i);
                        break;
                    }
                }
                if slot.unwrap() > 0 {
                    selected = Some((map, key, slot.unwrap()));
                    break;
                }
                api.hmfree(raw_from_map(map, size_of::<BinEntry>()), size_of::<BinEntry>());
            }
            let (map, mut key, slot) = selected.unwrap();
            let raw = raw_from_map(map, size_of::<BinEntry>());
            let table = (*header_from_array(raw)).hash_table.cast::<HashIndex>();
            let target_bucket = (*table).storage.add(slot >> 3);
            let target_hash = (*target_bucket).hash[slot & 7];
            let filler = (target_hash ^ (1usize << (usize::BITS - 1))).max(2);
            for i in 0..slot {
                let bucket = (*table).storage.add(i >> 3);
                if (*bucket).hash[i & 7] == 0 {
                    (*bucket).hash[i & 7] = filler;
                    (*bucket).index[i & 7] = -2;
                }
            }
            (*table).slot_count = 1;
            api.hmdel(
                map,
                size_of::<BinEntry>(),
                (&mut key as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
        }
        "moved_key_missing_assert" => {
            let mut key1 = 11u64;
            let mut key2 = 22u64;
            let mut map = api.hmput(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key1 as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            map = api.hmput(
                map,
                size_of::<BinEntry>(),
                (&mut key2 as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            (*map.cast::<BinEntry>().add(1)).key = 0xdead_beef_dead_beef;
            api.hmdel(
                map,
                size_of::<BinEntry>(),
                (&mut key1 as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
        }
        "moved_index_assert" => {
            let mut key1 = 31u64;
            let mut key2 = 32u64;
            let mut map = api.hmput(
                null_mut(),
                size_of::<BinEntry>(),
                (&mut key1 as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            map = api.hmput(
                map,
                size_of::<BinEntry>(),
                (&mut key2 as *mut u64).cast(),
                size_of::<u64>(),
                0,
            );
            let table = map_table(map, size_of::<BinEntry>()).unwrap();
            for i in 0..table.slot_count {
                let bucket = table.storage.add(i >> 3);
                if (*bucket).index[i & 7] == 1 {
                    (*bucket).index[i & 7] = 0;
                    break;
                }
            }
            api.hmdel(
                map,
                size_of::<BinEntry>(),
                (&mut key1 as *mut u64).cast(),
                size_of::<u64>(),
                0,
                0,
            );
        }
        other => panic!("unknown crash case {other}"),
    }
}

#[test]
fn crash_worker() {
    let Ok(case) = std::env::var("DIFF_CRASH_CASE") else {
        return;
    };
    let which = std::env::var("DIFF_CRASH_LIB").unwrap();
    let path = if which == "c" { c_path() } else { rust_path() };
    unsafe {
        let api = Api::open(&path);
        run_crash_case(&api, &case);
    }
}

fn crash_status(which: &str, case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_worker", "--nocapture"])
        .env("DIFF_CRASH_LIB", which)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn differential_error_paths() {
    let _guard = SERIAL.lock().unwrap();
    for case in [
        "hash_string_null",
        "hash_bytes_null",
        "hmget_temp_null",
        "stralloc_arena_null",
        "stralloc_string_null",
        "strreset_null",
        "hash_bytes_oversized",
        "arrfree_null",
        "hmput_key_null",
        "hmget_key_null",
        "hmdel_key_null",
        "zero_elem_map",
        "mode_four_lookup",
        "make_index_assert",
        "delete_slot_assert",
        "moved_key_missing_assert",
        "moved_index_assert",
    ] {
        let cs = crash_status("c", case);
        let rs = crash_status("rust", case);
        assert_eq!(
            (cs.code(), cs.signal()),
            (rs.code(), rs.signal()),
            "fault behavior differs for {case}: C={cs:?}, Rust={rs:?}"
        );
    }
}

trait ExitSignal {
    fn signal(&self) -> Option<c_int>;
}

impl ExitSignal for std::process::ExitStatus {
    fn signal(&self) -> Option<c_int> {
        std::os::unix::process::ExitStatusExt::signal(self)
    }
}
