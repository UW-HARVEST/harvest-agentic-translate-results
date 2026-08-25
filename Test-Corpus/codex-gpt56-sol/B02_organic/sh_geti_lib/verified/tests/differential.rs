use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::path::PathBuf;
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
type ShGeti = unsafe extern "C" fn(c_int);

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

struct Api {
    _library: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    seed: RandSeed,
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
    sh_geti: ShGeti,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = unsafe { Library::new(path).unwrap() };
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()).unwrap() }
            };
        }
        Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            seed: symbol!("stbds_rand_seed", RandSeed),
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
            sh_geti: symbol!("sh_geti", ShGeti),
            _library: library,
        }
    }
}

fn libraries() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        (
            Api::load(root.join("c_src/build/libtranslated_rust.so")),
            Api::load(root.join("target/release/libsh_geti_lib.so")),
        )
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

unsafe fn header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe { array.cast::<ArrayHeader>().sub(1) }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe { header(raw_map(map, element_size)) }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= state.wrapping_shl(13);
    *state ^= state.wrapping_shr(7);
    *state ^= state.wrapping_shl(17);
    *state
}

#[test]
fn symbols_load_from_both_shared_objects() {
    let _guard = test_guard();
    let _ = libraries();
}

#[test]
fn hashes_match_for_all_shapes_and_random_values() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    let seeds = [
        0,
        1,
        0x3141_5926,
        0xdead_beef,
        usize::MAX,
        0x0123_4567_89ab_cdef,
    ];

    for &seed in &seeds {
        let mut empty = b"\0".to_vec();
        assert_eq!(
            unsafe { (c.hash_string)(empty.as_mut_ptr().cast(), seed) },
            unsafe { (rust.hash_string)(empty.as_mut_ptr().cast(), seed) }
        );
        for byte in 1u8..=255 {
            let mut string = vec![byte, 0];
            assert_eq!(
                unsafe { (c.hash_string)(string.as_mut_ptr().cast(), seed) },
                unsafe { (rust.hash_string)(string.as_mut_ptr().cast(), seed) },
                "one-byte string {byte:#x}, seed {seed:#x}"
            );
        }
        assert_eq!(
            unsafe { (c.hash_bytes)(ptr::null_mut(), 0, seed) },
            unsafe { (rust.hash_bytes)(ptr::null_mut(), 0, seed) }
        );
    }

    let mut state = 0x8d26_3f91_d17a_4b35;
    let mut bytes = vec![0u8; 256];
    for item in &mut bytes {
        *item = next_random(&mut state) as u8;
    }
    for length in 0..=bytes.len() {
        for iteration in 0..32 {
            for item in &mut bytes[..length] {
                *item = next_random(&mut state) as u8;
            }
            let seed = next_random(&mut state) as usize;
            let c_hash = unsafe { (c.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed) };
            let rust_hash = unsafe { (rust.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed) };
            assert_eq!(c_hash, rust_hash, "length {length}, iteration {iteration}");
        }
    }

    for length in 1..=96 {
        for iteration in 0..24 {
            let mut bytes = Vec::with_capacity(length + 1);
            while bytes.len() < length {
                let byte = next_random(&mut state) as u8;
                if byte != 0 {
                    bytes.push(byte);
                }
            }
            bytes.push(0);
            let seed = next_random(&mut state) as usize;
            assert_eq!(
                unsafe { (c.hash_string)(bytes.as_mut_ptr().cast(), seed) },
                unsafe { (rust.hash_string)(bytes.as_mut_ptr().cast(), seed) },
                "string length {length}, iteration {iteration}"
            );
        }
    }
}

unsafe fn exercise_array(api: &Api, element_size: usize) -> Vec<(usize, usize, Vec<u8>)> {
    let mut snapshots = Vec::new();
    let mut array = ptr::null_mut();
    for minimum in 0..=3 {
        array = unsafe { (api.arrgrow)(array, element_size, 0, minimum) };
        if array.is_null() {
            snapshots.push((0, 0, Vec::new()));
            continue;
        }
        snapshots.push((
            unsafe { (*header(array)).length },
            unsafe { (*header(array)).capacity },
            Vec::new(),
        ));
    }

    let initial_capacity = unsafe { (*header(array)).capacity };
    let byte_count = initial_capacity * element_size;
    for index in 0..byte_count {
        unsafe { *array.cast::<u8>().add(index) = index.wrapping_mul(37) as u8 };
    }
    unsafe { (*header(array)).length = initial_capacity.saturating_sub(1) };

    array = unsafe { (api.arrgrow)(array, element_size, 0, initial_capacity) };
    snapshots.push((
        unsafe { (*header(array)).length },
        unsafe { (*header(array)).capacity },
        unsafe { std::slice::from_raw_parts(array.cast::<u8>(), byte_count).to_vec() },
    ));
    array = unsafe { (api.arrgrow)(array, element_size, 2, 0) };
    snapshots.push((
        unsafe { (*header(array)).length },
        unsafe { (*header(array)).capacity },
        unsafe { std::slice::from_raw_parts(array.cast::<u8>(), byte_count).to_vec() },
    ));
    array = unsafe { (api.arrgrow)(array, element_size, 0, 29) };
    snapshots.push((
        unsafe { (*header(array)).length },
        unsafe { (*header(array)).capacity },
        unsafe { std::slice::from_raw_parts(array.cast::<u8>(), byte_count).to_vec() },
    ));
    unsafe { (api.arrfree)(array) };
    snapshots
}

#[test]
fn arrays_and_default_map_match() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    for element_size in [1, 4, 16] {
        assert_eq!(
            unsafe { exercise_array(&c, element_size) },
            unsafe { exercise_array(&rust, element_size) },
            "element size {element_size}"
        );
    }

    for api in [&c, &rust] {
        unsafe { (api.hmfree)(ptr::null_mut(), 16) };
    }
    let mut maps = [ptr::null_mut(), ptr::null_mut()];
    for (map, api) in maps.iter_mut().zip([&c, &rust]) {
        *map = unsafe { (api.hmdefault)(*map, 16) };
        let raw = unsafe { raw_map(*map, 16) };
        assert_eq!(unsafe { (*header(raw)).length }, 1);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(raw.cast::<u8>(), 16) },
            &[0; 16]
        );
        unsafe { *raw.cast::<u8>().add(7) = 0xa5 };
        let same = unsafe { (api.hmdefault)(*map, 16) };
        assert_eq!(same, *map);
        assert_eq!(unsafe { *raw.cast::<u8>().add(7) }, 0xa5);
    }
    unsafe {
        (c.hmfree)(raw_map(maps[0], 16), 16);
        (rust.hmfree)(raw_map(maps[1], 16), 16);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BinarySnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<u8>,
}

unsafe fn binary_snapshot(map: *mut c_void, element_size: usize) -> BinarySnapshot {
    let header = unsafe { &*map_header(map, element_size) };
    let length = header.length - 1;
    BinarySnapshot {
        length,
        capacity: header.capacity,
        temp: header.temp,
        entries: unsafe {
            std::slice::from_raw_parts(map.cast::<u8>(), length * element_size).to_vec()
        },
    }
}

unsafe fn put_binary(
    api: &Api,
    map: &mut *mut c_void,
    element_size: usize,
    key: &mut [u8],
    value: u64,
) {
    *map = unsafe { (api.hmput)(*map, element_size, key.as_mut_ptr().cast(), key.len(), 0) };
    let index = unsafe { (*map_header(*map, element_size)).temp as usize };
    let entry = unsafe {
        std::slice::from_raw_parts_mut(map.cast::<u8>().add(index * element_size), element_size)
    };
    entry[..key.len()].copy_from_slice(key);
    for (offset, byte) in entry[key.len()..].iter_mut().enumerate() {
        *byte = value.rotate_left((offset * 7) as u32) as u8;
    }
}

unsafe fn exercise_binary_map(api: &Api, key_size: usize) -> Vec<BinarySnapshot> {
    let element_size = key_size + 8;
    let mut map = unsafe { (api.hmdefault)(ptr::null_mut(), element_size) };
    let mut snapshots = Vec::new();
    let mut state = 0x43a5_1ce7_d90b_268f;

    for index in 0..180u64 {
        let random = next_random(&mut state);
        let mut key = vec![0; key_size];
        for (offset, byte) in key.iter_mut().enumerate() {
            *byte = random.rotate_left((offset * 9) as u32) as u8;
        }
        key[0] = index as u8;
        if key_size > 1 {
            key[1] = (index >> 8) as u8;
        }
        unsafe { put_binary(api, &mut map, element_size, &mut key, index * 101) };
        if matches!(index, 0 | 5 | 6 | 11 | 24 | 47 | 95 | 179) {
            snapshots.push(unsafe { binary_snapshot(map, element_size) });
        }
    }

    let mut existing = vec![0; key_size];
    existing[0] = 17;
    if key_size > 1 {
        existing[1] = 0;
    }
    // Locate a definitely present key through the ordered entry bytes.
    existing.copy_from_slice(unsafe {
        std::slice::from_raw_parts(map.cast::<u8>().add(17 * element_size), key_size)
    });
    unsafe { put_binary(api, &mut map, element_size, &mut existing, 0xfeed_face) };
    snapshots.push(unsafe { binary_snapshot(map, element_size) });

    let mut temp = 123;
    let found = unsafe {
        (api.hmget_ts)(
            map,
            element_size,
            existing.as_mut_ptr().cast(),
            key_size,
            &mut temp,
            0,
        )
    };
    assert_eq!(found, map);
    assert_eq!(temp, 17);
    map = unsafe { (api.hmget)(map, element_size, existing.as_mut_ptr().cast(), key_size, 0) };
    assert_eq!(unsafe { (*map_header(map, element_size)).temp }, 17);

    let mut missing = vec![0xff; key_size];
    missing[0] = 0xfa;
    map = unsafe {
        (api.hmdel)(
            map,
            element_size,
            missing.as_mut_ptr().cast(),
            key_size,
            0,
            0,
        )
    };
    assert_eq!(unsafe { (*map_header(map, element_size)).temp }, 0);
    snapshots.push(unsafe { binary_snapshot(map, element_size) });

    for deletion in [179usize, 3, 17, 61, 100] {
        let length = unsafe { (*map_header(map, element_size)).length - 1 };
        let index = deletion.min(length - 1);
        let mut key = unsafe {
            std::slice::from_raw_parts(map.cast::<u8>().add(index * element_size), key_size)
                .to_vec()
        };
        map = unsafe { (api.hmdel)(map, element_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
        assert_eq!(unsafe { (*map_header(map, element_size)).temp }, 1);
        snapshots.push(unsafe { binary_snapshot(map, element_size) });
    }

    let mut inserted = vec![0x5a; key_size];
    unsafe { put_binary(api, &mut map, element_size, &mut inserted, 0x1234_5678) };
    snapshots.push(unsafe { binary_snapshot(map, element_size) });

    while unsafe { (*map_header(map, element_size)).length } > 2 {
        let mut key = unsafe { std::slice::from_raw_parts(map.cast::<u8>(), key_size).to_vec() };
        map = unsafe { (api.hmdel)(map, element_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
        let length = unsafe { (*map_header(map, element_size)).length - 1 };
        if matches!(length, 128 | 64 | 32 | 16 | 8 | 4 | 1) {
            snapshots.push(unsafe { binary_snapshot(map, element_size) });
        }
    }

    unsafe { (api.hmfree)(raw_map(map, element_size), element_size) };
    snapshots
}

#[test]
fn binary_maps_match_across_growth_lookup_delete_rebuild_and_shrink() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    for key_size in [1, 4, 8, 16] {
        unsafe {
            (c.seed)(0x1234_5678_9abc_def0);
            (rust.seed)(0x1234_5678_9abc_def0);
        }
        assert_eq!(
            unsafe { exercise_binary_map(&c, key_size) },
            unsafe { exercise_binary_map(&rust, key_size) },
            "key size {key_size}"
        );
    }

    let mut key = 7u64.to_ne_bytes();
    for api in [&c, &rust] {
        let mut temp = 99;
        let map = unsafe {
            (api.hmget_ts)(
                ptr::null_mut(),
                16,
                key.as_mut_ptr().cast(),
                key.len(),
                &mut temp,
                0,
            )
        };
        assert_eq!(temp, -1);
        assert_eq!(unsafe { (*map_header(map, 16)).length }, 1);
        assert!(unsafe { (*map_header(map, 16)).hash_table }.is_null());

        let map = unsafe { (api.hmget)(map, 16, key.as_mut_ptr().cast(), key.len(), 0) };
        assert_eq!(unsafe { (*map_header(map, 16)).temp }, -1);

        let same = unsafe { (api.hmdel)(map, 16, key.as_mut_ptr().cast(), key.len(), 0, 0) };
        assert_eq!(same, map);
        assert_eq!(unsafe { (*map_header(map, 16)).temp }, 0);
        unsafe { (api.hmfree)(raw_map(map, 16), 16) };
        assert!(
            unsafe {
                (api.hmdel)(
                    ptr::null_mut(),
                    16,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    0,
                    0,
                )
            }
            .is_null()
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

#[derive(Debug, PartialEq, Eq)]
struct StringSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    entries: Vec<(Vec<u8>, c_int)>,
}

unsafe fn string_snapshot(map: *mut StringEntry) -> StringSnapshot {
    let header = unsafe { &*map_header(map.cast(), size_of::<StringEntry>()) };
    let length = header.length - 1;
    let mut entries = Vec::with_capacity(length);
    for index in 0..length {
        let entry = unsafe { *map.add(index) };
        entries.push((
            unsafe { CStr::from_ptr(entry.key).to_bytes().to_vec() },
            entry.value,
        ));
    }
    StringSnapshot {
        length,
        capacity: header.capacity,
        temp: header.temp,
        entries,
    }
}

unsafe fn exercise_string_map(api: &Api, mode: c_int) -> Vec<StringSnapshot> {
    let element_size = size_of::<StringEntry>();
    let mut map = unsafe { (api.shmode)(element_size, mode).cast::<StringEntry>() };
    map = unsafe { (api.hmdefault)(map.cast(), element_size).cast() };
    unsafe { (*map.offset(-1)).value = -77 };
    let mut snapshots = vec![unsafe { string_snapshot(map) }];
    let mut keys = Vec::new();

    for index in 0..96 {
        let text = if index == 55 {
            format!("key_{index}_{}", "x".repeat(900))
        } else {
            format!("key_{index}_{}", (index * 7919) % 104729)
        };
        keys.push(CString::new(text).unwrap());
        let key = keys.last_mut().unwrap().as_ptr() as *mut c_char;
        map = unsafe {
            (api.hmput)(
                map.cast(),
                element_size,
                key.cast(),
                size_of::<*mut c_char>(),
                1,
            )
            .cast()
        };
        let item = unsafe { (*map_header(map.cast(), element_size)).temp };
        unsafe { (*map.offset(item)).value = index * 13 - 5 };
        if matches!(index, 0 | 5 | 6 | 24 | 47 | 95) {
            snapshots.push(unsafe { string_snapshot(map) });
        }
    }

    for index in 0..96 {
        let mut temp = 777;
        let same = unsafe {
            (api.hmget_ts)(
                map.cast(),
                element_size,
                keys[index].as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                &mut temp,
                1,
            )
            .cast::<StringEntry>()
        };
        assert_eq!(same, map);
        assert_eq!(
            unsafe { (*map.offset(temp)).value },
            index as c_int * 13 - 5
        );
    }

    let absent = CString::new("absent").unwrap();
    map = unsafe {
        (api.hmget)(
            map.cast(),
            element_size,
            absent.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            1,
        )
        .cast()
    };
    assert_eq!(unsafe { (*map_header(map.cast(), element_size)).temp }, -1);
    assert_eq!(unsafe { (*map.offset(-1)).value }, -77);

    for index in (0..72).step_by(2) {
        map = unsafe {
            (api.hmdel)(
                map.cast(),
                element_size,
                keys[index].as_ptr() as *mut c_void,
                size_of::<*mut c_char>(),
                0,
                1,
            )
            .cast()
        };
        if index % 12 == 0 {
            snapshots.push(unsafe { string_snapshot(map) });
        }
    }
    map = unsafe {
        (api.hmdel)(
            map.cast(),
            element_size,
            absent.as_ptr() as *mut c_void,
            size_of::<*mut c_char>(),
            0,
            1,
        )
        .cast()
    };
    assert_eq!(unsafe { (*map_header(map.cast(), element_size)).temp }, 0);
    snapshots.push(unsafe { string_snapshot(map) });
    unsafe { (api.hmfree)(raw_map(map.cast(), element_size), element_size) };
    snapshots
}

#[test]
fn string_modes_and_out_of_range_modes_match() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    for mode in [1, 2, 3] {
        unsafe {
            (c.seed)(0x44aa_9317);
            (rust.seed)(0x44aa_9317);
        }
        assert_eq!(
            unsafe { exercise_string_map(&c, mode) },
            unsafe { exercise_string_map(&rust, mode) },
            "string mode {mode}"
        );
    }

    for mode in [c_int::MIN, -1, 0, 1, 2, 3, 4, 255, 256, c_int::MAX] {
        for api in [&c, &rust] {
            let map = unsafe { (api.shmode)(size_of::<StringEntry>(), mode) };
            let header = unsafe { &*map_header(map, size_of::<StringEntry>()) };
            assert_eq!(header.length, 1);
            assert_eq!(header.capacity, 4);
            unsafe {
                (api.hmfree)(
                    raw_map(map, size_of::<StringEntry>()),
                    size_of::<StringEntry>(),
                )
            };
        }
    }
}

unsafe fn exercise_arena(api: &Api) -> Vec<(Vec<u8>, usize, u8, u8)> {
    let mut arena = StringArena::default();
    let mut snapshots = Vec::new();
    let lengths = [
        0usize, 1, 2, 7, 31, 255, 510, 511, 512, 513, 700, 1023, 4097, 65_537, 1_100_000,
    ];
    for (index, &length) in lengths.iter().enumerate() {
        let byte = b'a' + index as u8 % 26;
        let mut input = vec![byte; length];
        input.push(0);
        let result = unsafe { (api.stralloc)(&mut arena, input.as_mut_ptr().cast::<c_char>()) };
        snapshots.push((
            unsafe { CStr::from_ptr(result).to_bytes().to_vec() },
            arena.remaining,
            arena.block,
            arena.mode,
        ));
    }

    // Force fresh regular blocks repeatedly until the block-size cap branch is reached.
    while arena.block < 22 {
        let required = arena.remaining + 1;
        let mut input = vec![b'z'; required];
        input.push(0);
        let result = unsafe { (api.stralloc)(&mut arena, input.as_mut_ptr().cast::<c_char>()) };
        assert_eq!(unsafe { CStr::from_ptr(result).to_bytes().len() }, required);
        snapshots.push((
            vec![b'z'; required],
            arena.remaining,
            arena.block,
            arena.mode,
        ));
    }
    let required = arena.remaining + 1;
    let mut input = vec![b'q'; required];
    input.push(0);
    unsafe { (api.stralloc)(&mut arena, input.as_mut_ptr().cast::<c_char>()) };
    assert_eq!(arena.block, 22);
    snapshots.push((
        vec![b'q'; required],
        arena.remaining,
        arena.block,
        arena.mode,
    ));
    unsafe { (api.strreset)(&mut arena) };
    snapshots.push((Vec::new(), arena.remaining, arena.block, arena.mode));
    assert!(arena.storage.is_null());

    unsafe { (api.strreset)(&mut arena) };
    assert!(arena.storage.is_null());
    snapshots
}

#[test]
fn string_arena_and_strkey_match() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    assert_eq!(unsafe { exercise_arena(&c) }, unsafe {
        exercise_arena(&rust)
    });

    for value in [c_int::MIN, -1_000_000, -1, 0, 1, 42, 1_000_000, c_int::MAX] {
        let c_value = unsafe { CStr::from_ptr((c.strkey)(value)).to_bytes().to_vec() };
        let rust_value = unsafe { CStr::from_ptr((rust.strkey)(value)).to_bytes().to_vec() };
        assert_eq!(c_value, rust_value, "strkey({value})");
    }
}

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn tmpfile() -> *mut c_void;
    fn fileno(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fseek(stream: *mut c_void, offset: i64, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> i64;
    fn fread(buffer: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fclose(stream: *mut c_void) -> c_int;
}

unsafe fn capture_stdout(function: ShGeti, argument: c_int) -> Vec<u8> {
    unsafe { fflush(ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    let file = unsafe { tmpfile() };
    assert!(!file.is_null());
    let file_fd = unsafe { fileno(file) };
    assert!(file_fd >= 0);
    assert_eq!(unsafe { dup2(file_fd, 1) }, 1);
    unsafe { function(argument) };
    unsafe { fflush(ptr::null_mut()) };
    assert_eq!(unsafe { dup2(saved, 1) }, 1);
    unsafe { close(saved) };

    let length = unsafe { ftell(file) };
    assert!(length >= 0);
    assert_eq!(unsafe { fseek(file, 0, 0) }, 0);
    let mut output = vec![0; length as usize];
    if !output.is_empty() {
        assert_eq!(
            unsafe { fread(output.as_mut_ptr().cast(), 1, output.len(), file) },
            output.len()
        );
    }
    unsafe { fclose(file) };
    output
}

#[test]
fn sh_geti_matches_through_exported_wrapper() {
    let _guard = test_guard();
    let (c, rust) = libraries();
    let mut arguments = vec![-100, -1, 0, 1, 2, 3, 4, 5, 16, 31, 64, 127];
    let mut state = 0xa18c_f093_75d2_4be1;
    for _ in 0..24 {
        arguments.push((next_random(&mut state) % 160) as c_int);
    }
    for argument in arguments {
        assert_eq!(
            unsafe { capture_stdout(c.sh_geti, argument) },
            unsafe { capture_stdout(rust.sh_geti, argument) },
            "sh_geti({argument})"
        );
    }
}
