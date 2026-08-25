#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::ptr::{self, null_mut};
use std::sync::atomic::{AtomicU64, Ordering};

const C_LIBRARY: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIBRARY: &str = "target/release/libstr_dups_lib.so";
const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;
const BUCKET_LENGTH: usize = 8;

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmPutDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type RandSeed = unsafe extern "C" fn(usize);
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut StringArena);
type StrDups = unsafe extern "C" fn(c_int);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;

struct Api {
    library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        Self {
            library: Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *self.library.get::<T>(name).unwrap()
    }

    unsafe fn arrgrow(
        &self,
        array: *mut c_void,
        element_size: usize,
        add_length: usize,
        minimum_capacity: usize,
    ) -> *mut c_void {
        self.symbol::<ArrGrow>(b"stbds_arrgrowf\0")(
            array,
            element_size,
            add_length,
            minimum_capacity,
        )
    }

    unsafe fn arrfree(&self, array: *mut c_void) {
        self.symbol::<ArrFree>(b"stbds_arrfreef\0")(array)
    }

    unsafe fn hash_bytes(&self, bytes: *mut c_void, length: usize, seed: usize) -> usize {
        self.symbol::<HashBytes>(b"stbds_hash_bytes\0")(bytes, length, seed)
    }

    unsafe fn hash_string(&self, string: *mut c_char, seed: usize) -> usize {
        self.symbol::<HashString>(b"stbds_hash_string\0")(string, seed)
    }

    unsafe fn hmget(
        &self,
        map: *mut c_void,
        element_size: usize,
        key: *mut c_void,
        key_size: usize,
        mode: c_int,
    ) -> *mut c_void {
        self.symbol::<HmGet>(b"stbds_hmget_key\0")(map, element_size, key, key_size, mode)
    }

    unsafe fn hmget_ts(
        &self,
        map: *mut c_void,
        element_size: usize,
        key: *mut c_void,
        key_size: usize,
        temporary: *mut isize,
        mode: c_int,
    ) -> *mut c_void {
        self.symbol::<HmGetTs>(b"stbds_hmget_key_ts\0")(
            map,
            element_size,
            key,
            key_size,
            temporary,
            mode,
        )
    }

    unsafe fn hmput_default(&self, map: *mut c_void, element_size: usize) -> *mut c_void {
        self.symbol::<HmPutDefault>(b"stbds_hmput_default\0")(map, element_size)
    }

    unsafe fn hmput(
        &self,
        map: *mut c_void,
        element_size: usize,
        key: *mut c_void,
        key_size: usize,
        mode: c_int,
    ) -> *mut c_void {
        self.symbol::<HmPut>(b"stbds_hmput_key\0")(map, element_size, key, key_size, mode)
    }

    unsafe fn hmdel(
        &self,
        map: *mut c_void,
        element_size: usize,
        key: *mut c_void,
        key_size: usize,
        key_offset: usize,
        mode: c_int,
    ) -> *mut c_void {
        self.symbol::<HmDel>(b"stbds_hmdel_key\0")(
            map,
            element_size,
            key,
            key_size,
            key_offset,
            mode,
        )
    }

    unsafe fn hmfree(&self, raw_array: *mut c_void, element_size: usize) {
        self.symbol::<HmFree>(b"stbds_hmfree_func\0")(raw_array, element_size)
    }

    unsafe fn rand_seed(&self, seed: usize) {
        self.symbol::<RandSeed>(b"stbds_rand_seed\0")(seed)
    }

    unsafe fn shmode(&self, element_size: usize, mode: c_int) -> *mut c_void {
        self.symbol::<ShMode>(b"stbds_shmode_func\0")(element_size, mode)
    }

    unsafe fn stralloc(&self, arena: *mut StringArena, string: *mut c_char) -> *mut c_char {
        self.symbol::<StrAlloc>(b"stbds_stralloc\0")(arena, string)
    }

    unsafe fn strreset(&self, arena: *mut StringArena) {
        self.symbol::<StrReset>(b"stbds_strreset\0")(arena)
    }

    unsafe fn strkey(&self, number: c_int) -> *mut c_char {
        self.symbol::<StrKey>(b"strkey\0")(number)
    }

    unsafe fn str_dups(&self, number: c_int) {
        self.symbol::<StrDups>(b"str_dups\0")(number)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
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

#[repr(C)]
struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
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
    storage: *mut HashBucket,
}

#[derive(Debug, PartialEq, Eq)]
struct MapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    payload: Vec<u8>,
    table_scalars: Option<Vec<usize>>,
    hashes: Vec<usize>,
    indices: Vec<isize>,
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

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.next() as u8).collect()
    }
}

fn library_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    (array as *mut ArrayHeader).sub(1)
}

unsafe fn raw_array(map: *mut c_void, element_size: usize) -> *mut c_void {
    (map as *mut u8).sub(element_size).cast()
}

unsafe fn map_snapshot(map: *mut c_void, element_size: usize) -> MapSnapshot {
    let raw = raw_array(map, element_size);
    let array_header = &*header(raw);
    let payload =
        std::slice::from_raw_parts(raw.cast::<u8>(), array_header.length * element_size).to_vec();
    let table = array_header.hash_table.cast::<HashIndex>();
    if table.is_null() {
        return MapSnapshot {
            length: array_header.length,
            capacity: array_header.capacity,
            temp: array_header.temp,
            payload,
            table_scalars: None,
            hashes: Vec::new(),
            indices: Vec::new(),
        };
    }

    let table = &*table;
    let mut hashes = Vec::with_capacity(table.slot_count);
    let mut indices = Vec::with_capacity(table.slot_count);
    for bucket_index in 0..(table.slot_count / BUCKET_LENGTH) {
        let bucket = &*table.storage.add(bucket_index);
        hashes.extend_from_slice(&bucket.hash);
        indices.extend_from_slice(&bucket.index);
    }
    MapSnapshot {
        length: array_header.length,
        capacity: array_header.capacity,
        temp: array_header.temp,
        payload,
        table_scalars: Some(vec![
            table.slot_count,
            table.used_count,
            table.used_count_threshold,
            table.used_count_shrink_threshold,
            table.tombstone_count,
            table.tombstone_count_threshold,
            table.seed,
            table.slot_count_log2,
            table.string.remaining,
            table.string.block as usize,
            table.string.mode as usize,
        ]),
        hashes,
        indices,
    }
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    api.hmfree(raw_array(map, element_size), element_size);
}

unsafe fn exercise_arrays(api: &Api) -> Vec<(usize, usize, isize, Vec<u8>, bool)> {
    let mut observations = Vec::new();
    let mut rng = Rng(0x4d59_5df4_d0f3_3173);
    for element_size in [1, 2, 3, 4, 8, 16, 31] {
        let added = api.arrgrow(null_mut(), element_size, 5, 1);
        observations.push((
            (*header(added)).length,
            (*header(added)).capacity,
            (*header(added)).temp,
            Vec::new(),
            false,
        ));
        api.arrfree(added);
        for requested in 0..12 {
            let mut array = api.arrgrow(null_mut(), element_size, 0, requested);
            if array.is_null() {
                observations.push((0, 0, 0, Vec::new(), true));
                continue;
            }
            let initial_capacity = (*header(array)).capacity;
            let length = requested.min(initial_capacity);
            (*header(array)).length = length;
            let payload = rng.bytes(length * element_size);
            ptr::copy_nonoverlapping(payload.as_ptr(), array.cast::<u8>(), payload.len());

            let unchanged = api.arrgrow(array, element_size, 0, initial_capacity) == array;
            array = api.arrgrow(array, element_size, 1, initial_capacity + requested + 1);
            let result_header = &*header(array);
            let retained = std::slice::from_raw_parts(array.cast::<u8>(), payload.len()).to_vec();
            observations.push((
                result_header.length,
                result_header.capacity,
                result_header.temp,
                retained,
                unchanged,
            ));
            api.arrfree(array);
        }
    }
    observations
}

unsafe fn exercise_hashes(api: &Api) -> Vec<usize> {
    let mut rng = Rng(0x243f_6a88_85a3_08d3);
    let mut output = Vec::new();
    for length in 0..96 {
        for _ in 0..24 {
            let mut bytes = rng.bytes(length);
            let seed = rng.next() as usize;
            let pointer = if length == 0 && seed & 1 == 0 {
                null_mut()
            } else {
                bytes.as_mut_ptr().cast()
            };
            output.push(api.hash_bytes(pointer, length, seed));

            bytes.retain(|byte| *byte != 0);
            bytes.push(0);
            output.push(api.hash_string(bytes.as_mut_ptr().cast(), seed));
        }
    }
    output
}

unsafe fn write_binary_value(map: *mut c_void, value: u64) {
    let raw = raw_array(map, 16);
    let index = (*header(raw)).temp as usize;
    *map.cast::<u64>().add(index * 2 + 1) = value;
}

unsafe fn exercise_binary_map(api: &Api) -> Vec<MapSnapshot> {
    const ELEMENT_SIZE: usize = 16;
    let mut output = Vec::new();
    let seed = 0x1319_8a2e_0370_7344usize;
    api.rand_seed(seed);

    let mut missing = 11u64;
    let mut temporary = 99isize;
    let empty_from_ts = api.hmget_ts(
        null_mut(),
        ELEMENT_SIZE,
        ptr::addr_of_mut!(missing).cast(),
        size_of::<u64>(),
        ptr::addr_of_mut!(temporary),
        HM_BINARY,
    );
    assert_eq!(temporary, -1);
    output.push(map_snapshot(empty_from_ts, ELEMENT_SIZE));
    free_map(api, empty_from_ts, ELEMENT_SIZE);

    let empty_from_get = api.hmget(
        null_mut(),
        ELEMENT_SIZE,
        ptr::addr_of_mut!(missing).cast(),
        size_of::<u64>(),
        HM_BINARY,
    );
    output.push(map_snapshot(empty_from_get, ELEMENT_SIZE));
    free_map(api, empty_from_get, ELEMENT_SIZE);

    let mut map = api.hmput_default(null_mut(), ELEMENT_SIZE);
    output.push(map_snapshot(map, ELEMENT_SIZE));
    assert_eq!(
        api.hmput_default(map, ELEMENT_SIZE),
        map,
        "existing default must be retained"
    );
    map = api.hmget(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(missing).cast(),
        size_of::<u64>(),
        HM_BINARY,
    );
    output.push(map_snapshot(map, ELEMENT_SIZE));

    let mut keys = Vec::new();
    let mut rng = Rng(0xa409_3822_299f_31d0);
    while keys.len() < 96 {
        let key = rng.next();
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    for (index, key) in keys.iter_mut().enumerate() {
        map = api.hmput(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        write_binary_value(map, (index as u64).wrapping_mul(17));
        if index < 12 || index % 7 == 0 {
            output.push(map_snapshot(map, ELEMENT_SIZE));
        }
    }

    for index in (0..keys.len()).step_by(5) {
        map = api.hmput(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(&mut keys[index]).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        write_binary_value(map, 0xf000_0000 + index as u64);
        output.push(map_snapshot(map, ELEMENT_SIZE));
    }

    for index in (0..keys.len()).step_by(3) {
        temporary = 777;
        map = api.hmget_ts(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(&mut keys[index]).cast(),
            size_of::<u64>(),
            ptr::addr_of_mut!(temporary),
            HM_BINARY,
        );
        assert!(temporary >= 0);
        map = api.hmget(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(&mut keys[index]).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        output.push(map_snapshot(map, ELEMENT_SIZE));
    }

    temporary = 777;
    map = api.hmget_ts(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(missing).cast(),
        size_of::<u64>(),
        ptr::addr_of_mut!(temporary),
        HM_BINARY,
    );
    assert_eq!(temporary, -1);
    output.push(map_snapshot(map, ELEMENT_SIZE));

    map = api.hmdel(
        map,
        ELEMENT_SIZE,
        ptr::from_mut(keys.last_mut().unwrap()).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
    output.push(map_snapshot(map, ELEMENT_SIZE));
    map = api.hmdel(
        map,
        ELEMENT_SIZE,
        ptr::from_mut(&mut keys[0]).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
    output.push(map_snapshot(map, ELEMENT_SIZE));
    map = api.hmdel(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(missing).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
    output.push(map_snapshot(map, ELEMENT_SIZE));

    for index in 1..80 {
        map = api.hmdel(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(&mut keys[index]).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        if index % 4 == 0 {
            output.push(map_snapshot(map, ELEMENT_SIZE));
        }
    }
    free_map(api, map, ELEMENT_SIZE);

    api.hmfree(null_mut(), ELEMENT_SIZE);
    output
}

unsafe fn collision_keys(api: &Api, seed: usize) -> Vec<u64> {
    let mut keys = Vec::new();
    let mut candidate = 0u64;
    while keys.len() < 4 {
        let hash = api.hash_bytes(ptr::addr_of_mut!(candidate).cast(), size_of::<u64>(), seed);
        if hash & 7 == 7 {
            keys.push(candidate);
        }
        candidate += 1;
    }
    keys
}

unsafe fn exercise_wrapped_probe_and_offsets(api: &Api) -> Vec<MapSnapshot> {
    const ELEMENT_SIZE: usize = 24;
    let seed = 0x082e_fa98_ec4e_6c89usize;
    api.rand_seed(seed);
    let mut keys = collision_keys(api, seed);
    let mut map = null_mut();
    let mut output = Vec::new();
    for (index, key) in keys.iter_mut().take(3).enumerate() {
        map = api.hmput(
            map,
            ELEMENT_SIZE,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        let raw = raw_array(map, ELEMENT_SIZE);
        let entry = (map as *mut u8).add((*header(raw)).temp as usize * ELEMENT_SIZE);
        *entry.add(8).cast::<u64>() = *key;
        *entry.add(16).cast::<u64>() = index as u64;
        output.push(map_snapshot(map, ELEMENT_SIZE));
    }
    let mut absent = keys[3];
    let mut temporary = 44;
    map = api.hmget_ts(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(absent).cast(),
        size_of::<u64>(),
        ptr::addr_of_mut!(temporary),
        HM_BINARY,
    );
    assert_eq!(temporary, -1);
    output.push(map_snapshot(map, ELEMENT_SIZE));

    let mut delete_key = keys[1];
    map = api.hmdel(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(delete_key).cast(),
        size_of::<u64>(),
        8,
        HM_BINARY,
    );
    output.push(map_snapshot(map, ELEMENT_SIZE));
    map = api.hmput(
        map,
        ELEMENT_SIZE,
        ptr::addr_of_mut!(absent).cast(),
        size_of::<u64>(),
        HM_BINARY,
    );
    let raw = raw_array(map, ELEMENT_SIZE);
    let entry = (map as *mut u8).add((*header(raw)).temp as usize * ELEMENT_SIZE);
    *entry.add(8).cast::<u64>() = absent;
    *entry.add(16).cast::<u64>() = 99;
    output.push(map_snapshot(map, ELEMENT_SIZE));
    free_map(api, map, ELEMENT_SIZE);

    for element_size in [8usize, 16, 24] {
        api.rand_seed(seed);
        let mut key = 42u64;
        let map = api.hmput(
            null_mut(),
            element_size,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>().min(element_size),
            HM_BINARY,
        );
        free_map(api, map, element_size);
    }

    api.rand_seed(seed);
    let mut key = 1u64;
    let map = api.hmput(null_mut(), 8, ptr::addr_of_mut!(key).cast(), 0, HM_BINARY);
    assert_eq!((*header(raw_array(map, 8))).length, 2);
    free_map(api, map, 8);
    output
}

unsafe fn exercise_seed_width_and_rebuild_shapes(api: &Api) -> Vec<MapSnapshot> {
    let mut output = Vec::new();
    let mut rng = Rng(0x6a09_e667_f3bc_c909);
    let mut seeds = vec![0, 1, usize::MAX];
    for _ in 0..32 {
        seeds.push(rng.next() as usize);
    }
    for seed in seeds {
        api.rand_seed(seed);
        let mut key = rng.next();
        let map = api.hmput(
            null_mut(),
            16,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        write_binary_value(map, seed as u64);
        output.push(map_snapshot(map, 16));
        free_map(api, map, 16);
    }

    for key_size in [1usize, 2, 4, 8, 16] {
        api.rand_seed(0x510e_527f_ade6_82d1);
        let mut key = [0u8; 16];
        for byte in &mut key {
            *byte = rng.next() as u8;
        }
        let map = api.hmput(null_mut(), 24, key.as_mut_ptr().cast(), key_size, HM_BINARY);
        let raw = raw_array(map, 24);
        let entry = (map as *mut u8).add((*header(raw)).temp as usize * 24);
        ptr::write_bytes(entry.add(key_size), 0, 24 - key_size);
        output.push(map_snapshot(map, 24));
        free_map(api, map, 24);
    }

    api.rand_seed(0x1f83_d9ab_fb41_bd6b);
    let mut map = null_mut();
    let mut keys: Vec<u64> = (0..24).map(|index| index * 17 + 3).collect();
    for (index, key) in keys.iter_mut().enumerate() {
        map = api.hmput(
            map,
            16,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        write_binary_value(map, index as u64);
    }
    let grown_slot_count =
        (*(*header(raw_array(map, 16))).hash_table.cast::<HashIndex>()).slot_count;
    for key in keys.iter_mut().take(18) {
        map = api.hmdel(
            map,
            16,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        output.push(map_snapshot(map, 16));
    }
    let shrunken_slot_count =
        (*(*header(raw_array(map, 16))).hash_table.cast::<HashIndex>()).slot_count;
    assert!(shrunken_slot_count < grown_slot_count);
    free_map(api, map, 16);

    api.rand_seed(0x5be0_cd19_137e_2179);
    let mut map = null_mut();
    let mut rebuild_keys = [101u64, 102, 103, 104, 105, 106];
    for key in &mut rebuild_keys {
        map = api.hmput(
            map,
            16,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        write_binary_value(map, *key);
    }
    for key in rebuild_keys.iter_mut().take(2) {
        map = api.hmdel(
            map,
            16,
            ptr::from_mut(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        output.push(map_snapshot(map, 16));
    }
    let table = &*(*header(raw_array(map, 16))).hash_table.cast::<HashIndex>();
    assert_eq!(table.slot_count, 8);
    assert_eq!(table.tombstone_count, 0);
    free_map(api, map, 16);
    output
}

#[derive(Debug, PartialEq, Eq)]
struct StringMapObservation {
    header: (usize, usize, isize),
    entries: Vec<(Vec<u8>, i64)>,
    source_pointer_retained: bool,
    table: Vec<usize>,
}

unsafe fn string_map_observation(
    map: *mut c_void,
    source_pointer: *mut c_char,
) -> StringMapObservation {
    let raw = raw_array(map, 16);
    let array_header = &*header(raw);
    let table = &*array_header.hash_table.cast::<HashIndex>();
    let mut entries = Vec::new();
    for index in 0..array_header.length - 1 {
        let entry = (map as *mut u8).add(index * 16);
        let key = *entry.cast::<*mut c_char>();
        let value = *entry.add(8).cast::<i64>();
        entries.push((CStr::from_ptr(key).to_bytes().to_vec(), value));
    }
    StringMapObservation {
        header: (
            array_header.length,
            array_header.capacity,
            array_header.temp,
        ),
        entries,
        source_pointer_retained: *map.cast::<*mut c_char>() == source_pointer,
        table: vec![
            table.slot_count,
            table.used_count,
            table.tombstone_count,
            table.seed,
            table.string.remaining,
            table.string.block as usize,
            table.string.mode as usize,
        ],
    }
}

unsafe fn exercise_string_maps(api: &Api) -> Vec<StringMapObservation> {
    let mut output = Vec::new();
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        api.rand_seed(0x4528_21e6_38d0_1377);
        let mut map = api.shmode(16, mode);
        let mut sources = vec![
            CString::new("").unwrap(),
            CString::new("a").unwrap(),
            CString::new("alphabet").unwrap(),
            CString::new("a long string key with spaces").unwrap(),
        ];
        let first_source = sources[0].as_ptr() as *mut c_char;
        for (index, source) in sources.iter_mut().enumerate() {
            map = api.hmput(
                map,
                16,
                source.as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                HM_STRING,
            );
            let raw = raw_array(map, 16);
            let entry = (map as *mut u8).add((*header(raw)).temp as usize * 16);
            *entry.add(8).cast::<i64>() = index as i64 * 101;
            output.push(string_map_observation(map, first_source));
        }

        let replacement = CString::new("alphabet").unwrap();
        map = api.hmput(
            map,
            16,
            replacement.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            HM_STRING,
        );
        let raw = raw_array(map, 16);
        let entry = (map as *mut u8).add((*header(raw)).temp as usize * 16);
        *entry.add(8).cast::<i64>() = 999;
        output.push(string_map_observation(map, first_source));

        let mut temporary = 77;
        map = api.hmget_ts(
            map,
            16,
            sources[1].as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            ptr::addr_of_mut!(temporary),
            HM_STRING,
        );
        assert!(temporary >= 0);
        let missing = CString::new("missing").unwrap();
        map = api.hmget(
            map,
            16,
            missing.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            HM_STRING,
        );
        assert_eq!((*header(raw_array(map, 16))).temp, -1);
        output.push(string_map_observation(map, first_source));

        map = api.hmdel(
            map,
            16,
            sources[1].as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            0,
            HM_STRING,
        );
        output.push(string_map_observation(map, first_source));
        free_map(api, map, 16);
    }

    for mode in [
        -257, -1, SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 4, 257, 258, 259,
    ] {
        api.rand_seed(123);
        let map = api.shmode(16, mode);
        let raw = raw_array(map, 16);
        let actual = (*(*header(raw)).hash_table.cast::<HashIndex>()).string.mode;
        assert_eq!(actual, mode as u8);
        free_map(api, map, 16);
    }

    api.rand_seed(456);
    let mut binary_key = 0x0102_0304_0506_0708u64;
    let binary_map = api.hmput(
        null_mut(),
        16,
        ptr::addr_of_mut!(binary_key).cast(),
        size_of::<u64>(),
        -1,
    );
    free_map(api, binary_map, 16);

    api.rand_seed(789);
    let string = CString::new("mode-two").unwrap();
    let mut string_map = api.hmput(
        null_mut(),
        16,
        string.as_ptr() as *mut c_void,
        size_of::<*mut c_char>(),
        2,
    );
    string_map = api.hmdel(
        string_map,
        16,
        string.as_ptr() as *mut c_void,
        size_of::<*mut c_char>(),
        0,
        2,
    );
    free_map(api, string_map, 16);
    output
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaObservation {
    returned: Vec<u8>,
    remaining: usize,
    block: u8,
    mode: u8,
}

unsafe fn exercise_arena(api: &Api) -> Vec<ArenaObservation> {
    let mut output = Vec::new();
    let mut arena: StringArena = std::mem::zeroed();
    let mut rng = Rng(0xbe54_66cf_34e9_0c6c);
    let mut inputs = vec![
        Vec::new(),
        vec![b'a'],
        vec![b'b'; 510],
        vec![b'c'; 511],
        vec![b'd'; 512],
        vec![b'e'; 513],
        vec![b'f'; 2000],
    ];
    for _ in 0..128 {
        let length = (rng.next() % 900) as usize;
        let mut bytes = rng.bytes(length);
        for byte in &mut bytes {
            if *byte == 0 {
                *byte = 1;
            }
        }
        inputs.push(bytes);
    }

    for bytes in inputs {
        let string = CString::new(bytes.clone()).unwrap();
        let returned = api.stralloc(ptr::addr_of_mut!(arena), string.as_ptr() as *mut c_char);
        output.push(ArenaObservation {
            returned: CStr::from_ptr(returned).to_bytes().to_vec(),
            remaining: arena.remaining,
            block: arena.block,
            mode: arena.mode,
        });
        assert_eq!(CStr::from_ptr(returned).to_bytes(), bytes);
    }
    api.strreset(ptr::addr_of_mut!(arena));
    output.push(ArenaObservation {
        returned: Vec::new(),
        remaining: arena.remaining,
        block: arena.block,
        mode: arena.mode,
    });
    api.strreset(ptr::addr_of_mut!(arena));

    let mut maximum_arena: StringArena = std::mem::zeroed();
    for _ in 0..30 {
        let block_size = (512usize << (maximum_arena.block >> 1)).min(1 << 20);
        let string = CString::new(vec![b'x'; block_size]).unwrap();
        let returned = api.stralloc(
            ptr::addr_of_mut!(maximum_arena),
            string.as_ptr() as *mut c_char,
        );
        output.push(ArenaObservation {
            returned: CStr::from_ptr(returned).to_bytes().to_vec(),
            remaining: maximum_arena.remaining,
            block: maximum_arena.block,
            mode: maximum_arena.mode,
        });
    }
    assert_eq!(maximum_arena.block, 22);
    api.strreset(ptr::addr_of_mut!(maximum_arena));
    output
}

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "translated-rust-stdout-{}-{}",
        std::process::id(),
        counter
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    fflush(null_mut());
    let saved = dup(1);
    assert!(saved >= 0);
    assert_eq!(dup2(file.as_raw_fd(), 1), 1);
    call();
    fflush(null_mut());
    assert_eq!(dup2(saved, 1), 1);
    close(saved);
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    drop(file);
    remove_file(path).unwrap();
    output
}

unsafe fn exercise_public_wrappers(api: &Api) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<bool>) {
    let mut keys = Vec::new();
    let mut same_pointer = Vec::new();
    let mut previous: *mut c_char = null_mut();
    for number in [c_int::MIN, -1000, -1, 0, 1, 999, c_int::MAX] {
        let pointer = api.strkey(number);
        keys.push(CStr::from_ptr(pointer).to_bytes().to_vec());
        if !previous.is_null() {
            same_pointer.push(pointer == previous);
        }
        previous = pointer;
    }

    let mut outputs = Vec::new();
    let mut rng = Rng(0x3bd3_9e10_cb0e_f593);
    let mut counts = vec![-100, -1, 0, 1, 2, 510, 511, 512, 513, 1024, 2048];
    for _ in 0..32 {
        counts.push((rng.next() % 3000) as c_int);
    }
    for count in counts {
        outputs.push(capture_stdout(|| api.str_dups(count)));
    }
    (keys, outputs, same_pointer)
}

fn child_status(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_terminating_helper")
        .arg("--nocapture")
        .env("TRANSLATED_RUST_HELPER_LIBRARY", library)
        .env("TRANSLATED_RUST_HELPER_CASE", case)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
}

fn assert_same_termination(case: &str) {
    let c_status = child_status(&library_path(C_LIBRARY), case);
    let rust_status = child_status(&library_path(RUST_LIBRARY), case);
    assert!(!c_status.success(), "C unexpectedly accepted {case}");
    assert!(!rust_status.success(), "Rust unexpectedly accepted {case}");
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "different terminating signal for {case}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
fn differential_surface() {
    let c_path = library_path(C_LIBRARY);
    let rust_path = library_path(RUST_LIBRARY);
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    unsafe {
        let c = Api::load(&c_path);
        let rust = Api::load(&rust_path);

        assert_eq!(exercise_arrays(&c), exercise_arrays(&rust));
        assert_eq!(exercise_hashes(&c), exercise_hashes(&rust));
        assert_eq!(exercise_binary_map(&c), exercise_binary_map(&rust));
        assert_eq!(
            exercise_wrapped_probe_and_offsets(&c),
            exercise_wrapped_probe_and_offsets(&rust)
        );
        assert_eq!(
            exercise_seed_width_and_rebuild_shapes(&c),
            exercise_seed_width_and_rebuild_shapes(&rust)
        );
        assert_eq!(exercise_string_maps(&c), exercise_string_maps(&rust));
        assert_eq!(exercise_arena(&c), exercise_arena(&rust));
        assert_eq!(
            exercise_public_wrappers(&c),
            exercise_public_wrappers(&rust)
        );

        c.hmfree(null_mut(), 16);
        rust.hmfree(null_mut(), 16);
        assert!(
            c.hmdel(null_mut(), 16, null_mut(), 8, 0, HM_BINARY)
                .is_null()
        );
        assert!(
            rust.hmdel(null_mut(), 16, null_mut(), 8, 0, HM_BINARY)
                .is_null()
        );

        let c_default = c.hmput_default(null_mut(), 16);
        let rust_default = rust.hmput_default(null_mut(), 16);
        let mut key = 1u64;
        let c_default = c.hmdel(
            c_default,
            16,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        let rust_default = rust.hmdel(
            rust_default,
            16,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        assert_eq!(map_snapshot(c_default, 16), map_snapshot(rust_default, 16));
        free_map(&c, c_default, 16);
        free_map(&rust, rust_default, 16);

        let c_zero = c.hmget(
            null_mut(),
            0,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        let rust_zero = rust.hmget(
            null_mut(),
            0,
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        assert_eq!(
            ((*header(c_zero)).length, (*header(c_zero)).temp),
            ((*header(rust_zero)).length, (*header(rust_zero)).temp)
        );
        free_map(&c, c_zero, 0);
        free_map(&rust, rust_zero, 0);

        let c_oversized = c.arrgrow(null_mut(), usize::MAX, 0, 1);
        let rust_oversized = rust.arrgrow(null_mut(), usize::MAX, 0, 1);
        assert_eq!(
            (
                (*header(c_oversized)).length,
                (*header(c_oversized)).capacity,
                (*header(c_oversized)).temp,
            ),
            (
                (*header(rust_oversized)).length,
                (*header(rust_oversized)).capacity,
                (*header(rust_oversized)).temp,
            )
        );
        c.arrfree(c_oversized);
        rust.arrfree(rust_oversized);
    }

    for case in [
        "hash-string-null",
        "hash-bytes-null",
        "hash-bytes-null-oversized",
        "arrfree-null",
        "stralloc-arena-null",
        "stralloc-string-null",
        "strreset-null",
        "hmget-key-null",
        "hmget-temp-null",
        "hmput-key-null",
        "hmdel-key-null",
        "assert-hash-index-threshold",
        "assert-delete-slot-range",
        "assert-delete-moved-missing",
        "assert-delete-moved-index",
    ] {
        assert_same_termination(case);
    }
}

#[test]
fn ffi_terminating_helper() {
    let Some(path) = std::env::var_os("TRANSLATED_RUST_HELPER_LIBRARY") else {
        return;
    };
    let case = std::env::var("TRANSLATED_RUST_HELPER_CASE").unwrap();
    unsafe {
        let api = Api::load(Path::new(&path));
        match case.as_str() {
            "hash-string-null" => {
                api.hash_string(null_mut(), 0);
            }
            "hash-bytes-null" => {
                api.hash_bytes(null_mut(), 1, 0);
            }
            "hash-bytes-null-oversized" => {
                api.hash_bytes(null_mut(), usize::MAX, 0);
            }
            "arrfree-null" => {
                api.arrfree(null_mut());
            }
            "stralloc-arena-null" => {
                let string = CString::new("x").unwrap();
                api.stralloc(null_mut(), string.as_ptr() as *mut c_char);
            }
            "stralloc-string-null" => {
                let mut arena: StringArena = std::mem::zeroed();
                api.stralloc(ptr::addr_of_mut!(arena), null_mut());
            }
            "strreset-null" => {
                api.strreset(null_mut());
            }
            "hmget-key-null" => {
                api.rand_seed(40);
                let mut key = 1u64;
                let map = api.hmput(
                    null_mut(),
                    16,
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
                api.hmget(map, 16, null_mut(), size_of::<u64>(), HM_BINARY);
            }
            "hmget-temp-null" => {
                let mut key = 1u64;
                api.hmget_ts(
                    null_mut(),
                    16,
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    null_mut(),
                    HM_BINARY,
                );
            }
            "hmput-key-null" => {
                api.rand_seed(41);
                api.hmput(null_mut(), 16, null_mut(), size_of::<u64>(), HM_BINARY);
            }
            "hmdel-key-null" => {
                api.rand_seed(42);
                let mut key = 1u64;
                let map = api.hmput(
                    null_mut(),
                    16,
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
                api.hmdel(map, 16, null_mut(), size_of::<u64>(), 0, HM_BINARY);
            }
            "assert-hash-index-threshold" => {
                api.rand_seed(1);
                let mut first = 1u64;
                let map = api.hmput(
                    null_mut(),
                    16,
                    ptr::addr_of_mut!(first).cast(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
                let table = (*header(raw_array(map, 16))).hash_table.cast::<HashIndex>();
                (*table).slot_count = 0;
                (*table).used_count = 0;
                (*table).used_count_threshold = 0;
                let mut second = 2u64;
                api.hmput(
                    map,
                    16,
                    ptr::addr_of_mut!(second).cast(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            }
            "assert-delete-slot-range" => {
                let seed = 0x1122_3344usize;
                api.rand_seed(seed);
                let mut key = collision_keys(&api, seed)[0];
                let map = api.hmput(
                    null_mut(),
                    16,
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
                let table = (*header(raw_array(map, 16))).hash_table.cast::<HashIndex>();
                let bucket = &mut *(*table).storage;
                let hash = api.hash_bytes(ptr::addr_of_mut!(key).cast(), size_of::<u64>(), seed);
                for index in 0..7 {
                    bucket.hash[index] = 1;
                    bucket.index[index] = -2;
                }
                bucket.hash[7] = hash.max(2);
                bucket.index[7] = 0;
                (*table).slot_count = 1;
                (*table).slot_count_log2 = 0;
                api.hmdel(
                    map,
                    16,
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    0,
                    HM_BINARY,
                );
            }
            "assert-delete-moved-missing" => {
                api.rand_seed(2);
                let mut keys = [10u64, 20u64];
                let mut map = null_mut();
                for key in &mut keys {
                    map = api.hmput(
                        map,
                        16,
                        ptr::from_mut(key).cast(),
                        size_of::<u64>(),
                        HM_BINARY,
                    );
                }
                *map.cast::<u64>().add(2) = 30;
                api.hmdel(
                    map,
                    16,
                    ptr::addr_of_mut!(keys[0]).cast(),
                    size_of::<u64>(),
                    0,
                    HM_BINARY,
                );
            }
            "assert-delete-moved-index" => {
                api.rand_seed(3);
                let mut keys = [10u64, 20u64, 30u64];
                let mut map = null_mut();
                for key in &mut keys {
                    map = api.hmput(
                        map,
                        16,
                        ptr::from_mut(key).cast(),
                        size_of::<u64>(),
                        HM_BINARY,
                    );
                }
                *map.cast::<u64>().add(2) = keys[2];
                let table = &mut *(*header(raw_array(map, 16))).hash_table.cast::<HashIndex>();
                for bucket_index in 0..table.slot_count / BUCKET_LENGTH {
                    let bucket = &mut *table.storage.add(bucket_index);
                    for index in 0..BUCKET_LENGTH {
                        if bucket.index[index] == 2 {
                            bucket.index[index] = 1;
                        }
                    }
                }
                api.hmdel(
                    map,
                    16,
                    ptr::addr_of_mut!(keys[0]).cast(),
                    size_of::<u64>(),
                    0,
                    HM_BINARY,
                );
            }
            _ => panic!("unknown helper case {case}"),
        }
    }
}
