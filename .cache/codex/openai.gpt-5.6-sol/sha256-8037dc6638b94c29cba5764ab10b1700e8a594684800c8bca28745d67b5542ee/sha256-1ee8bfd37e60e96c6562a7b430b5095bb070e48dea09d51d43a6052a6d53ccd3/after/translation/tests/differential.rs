use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;
use std::sync::Mutex;

const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;
const BUCKET_LENGTH: usize = 8;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

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
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct HashBucket {
    hash: [usize; BUCKET_LENGTH],
    index: [isize; BUCKET_LENGTH],
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
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

type Arrgrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type Arrfree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type Hmfree = unsafe extern "C" fn(*mut c_void, usize);
type HmgetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type Hmget = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmputDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type Hmput = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type Shmode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type Hmdel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type Stralloc = unsafe extern "C" fn(*mut StringArena, *mut c_char) -> *mut c_char;
type Strreset = unsafe extern "C" fn(*mut StringArena);
type Strkey = unsafe extern "C" fn(c_int) -> *mut c_char;
type ShPuts = unsafe extern "C" fn(c_int);

struct Api {
    _library: Library,
    arrgrow: Arrgrow,
    arrfree: Arrfree,
    rand_seed: RandSeed,
    hash_string: HashString,
    hash_bytes: HashBytes,
    hmfree: Hmfree,
    hmget_ts: HmgetTs,
    hmget: Hmget,
    hmput_default: HmputDefault,
    hmput: Hmput,
    shmode: Shmode,
    hmdel: Hmdel,
    stralloc: Stralloc,
    strreset: Strreset,
    strkey: Strkey,
    sh_puts: ShPuts,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! load {
            ($name:literal, $ty:ty) => {{
                let symbol = unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *symbol
            }};
        }
        Self {
            arrgrow: load!("stbds_arrgrowf", Arrgrow),
            arrfree: load!("stbds_arrfreef", Arrfree),
            rand_seed: load!("stbds_rand_seed", RandSeed),
            hash_string: load!("stbds_hash_string", HashString),
            hash_bytes: load!("stbds_hash_bytes", HashBytes),
            hmfree: load!("stbds_hmfree_func", Hmfree),
            hmget_ts: load!("stbds_hmget_key_ts", HmgetTs),
            hmget: load!("stbds_hmget_key", Hmget),
            hmput_default: load!("stbds_hmput_default", HmputDefault),
            hmput: load!("stbds_hmput_key", Hmput),
            shmode: load!("stbds_shmode_func", Shmode),
            hmdel: load!("stbds_hmdel_key", Hmdel),
            stralloc: load!("stbds_stralloc", Stralloc),
            strreset: load!("stbds_strreset", Strreset),
            strkey: load!("strkey", Strkey),
            sh_puts: load!("sh_puts", ShPuts),
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("../c_src/build")
        .join("libharvest-work-YuvvZb.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libsh_puts_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}; run cargo build --release",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

unsafe fn array_header(array: *mut c_void) -> *mut ArrayHeader {
    unsafe {
        array
            .cast::<u8>()
            .sub(size_of::<ArrayHeader>())
            .cast::<ArrayHeader>()
    }
}

unsafe fn raw_map(map: *mut c_void, element_size: usize) -> *mut c_void {
    unsafe { map.cast::<u8>().sub(element_size).cast() }
}

unsafe fn map_header(map: *mut c_void, element_size: usize) -> *mut ArrayHeader {
    unsafe { array_header(raw_map(map, element_size)) }
}

#[derive(Debug, PartialEq, Eq)]
struct TableSnapshot {
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string_remaining: usize,
    string_block: u8,
    string_mode: u8,
    has_string_storage: bool,
    buckets: Vec<([usize; BUCKET_LENGTH], [isize; BUCKET_LENGTH])>,
}

#[derive(Debug, PartialEq, Eq)]
struct MapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    table: Option<TableSnapshot>,
    entries: Vec<Vec<u8>>,
}

unsafe fn snapshot_map(map: *mut c_void, element_size: usize) -> MapSnapshot {
    let header = unsafe { &*map_header(map, element_size) };
    let public_length = header.length - 1;
    let entries = (0..public_length)
        .map(|index| unsafe {
            std::slice::from_raw_parts(map.cast::<u8>().add(index * element_size), element_size)
                .to_vec()
        })
        .collect();
    let table = if header.hash_table.is_null() {
        None
    } else {
        let table = unsafe { &*header.hash_table.cast::<HashIndex>() };
        let buckets = (0..table.slot_count / BUCKET_LENGTH)
            .map(|index| unsafe {
                let bucket = &*table.storage.add(index);
                (bucket.hash, bucket.index)
            })
            .collect();
        Some(TableSnapshot {
            slot_count: table.slot_count,
            used_count: table.used_count,
            used_count_threshold: table.used_count_threshold,
            used_count_shrink_threshold: table.used_count_shrink_threshold,
            tombstone_count: table.tombstone_count,
            tombstone_count_threshold: table.tombstone_count_threshold,
            seed: table.seed,
            slot_count_log2: table.slot_count_log2,
            string_remaining: table.string.remaining,
            string_block: table.string.block,
            string_mode: table.string.mode,
            has_string_storage: !table.string.storage.is_null(),
            buckets,
        })
    };
    MapSnapshot {
        length: public_length,
        capacity: header.capacity,
        temp: header.temp,
        table,
        entries,
    }
}

unsafe fn write_binary_value(map: *mut c_void, element_size: usize, key: &[u8], value: u64) {
    let index = unsafe { (*map_header(map, element_size)).temp as usize };
    let entry = unsafe { map.cast::<u8>().add(index * element_size) };
    unsafe {
        ptr::write_bytes(entry, 0, element_size);
        ptr::copy_nonoverlapping(key.as_ptr(), entry, key.len());
        ptr::write_unaligned(entry.add(element_size - size_of::<u64>()).cast(), value);
    }
}

unsafe fn free_map(api: &Api, map: *mut c_void, element_size: usize) {
    unsafe { (api.hmfree)(raw_map(map, element_size), element_size) };
}

#[derive(Clone, Copy)]
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

unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().unwrap();
    let path = std::env::temp_dir().join(format!(
        "translation-differential-{}-{}.out",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    unsafe {
        fflush(ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0);
        assert_eq!(dup2(file.as_raw_fd(), 1), 1);
        call();
        fflush(ptr::null_mut());
        assert_eq!(dup2(saved, 1), 1);
        assert_eq!(close(saved), 0);
    }
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut output = Vec::new();
    file.read_to_end(&mut output).unwrap();
    drop(file);
    std::fs::remove_file(path).unwrap();
    output
}

fn c_string(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0));
    let mut result = bytes.to_vec();
    result.push(0);
    result
}

fn status_shape(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

#[derive(Debug, PartialEq, Eq)]
struct ArrayObservation {
    length: usize,
    capacity: usize,
    temp: isize,
    pointer_preserved: bool,
    prefix: Vec<u8>,
}

unsafe fn observe_array(
    api: &Api,
    element_size: usize,
    add_len: usize,
    min_capacity: usize,
    initial: Option<(usize, usize, Vec<u8>)>,
) -> ArrayObservation {
    let mut array = ptr::null_mut();
    if let Some((length, capacity, prefix)) = &initial {
        array = unsafe { (api.arrgrow)(ptr::null_mut(), element_size, 0, *capacity) };
        unsafe {
            (*array_header(array)).length = *length;
            ptr::copy_nonoverlapping(prefix.as_ptr(), array.cast(), prefix.len());
        }
    }
    let old = array;
    array = unsafe { (api.arrgrow)(array, element_size, add_len, min_capacity) };
    if array.is_null() {
        return ArrayObservation {
            length: 0,
            capacity: 0,
            temp: 0,
            pointer_preserved: old.is_null(),
            prefix: Vec::new(),
        };
    }
    let header = unsafe { *array_header(array) };
    let prefix_len = initial
        .as_ref()
        .map(|(_, _, bytes)| bytes.len())
        .unwrap_or(0);
    let prefix = unsafe { std::slice::from_raw_parts(array.cast::<u8>(), prefix_len).to_vec() };
    let observation = ArrayObservation {
        length: header.length,
        capacity: header.capacity,
        temp: header.temp,
        pointer_preserved: old.is_null() || old == array,
        prefix,
    };
    unsafe { (api.arrfree)(array) };
    observation
}

const BINARY_ELEMENT_SIZE: usize = 24;

unsafe fn put_binary(api: &Api, map: &mut *mut c_void, key: &[u8], value: u64, mode: c_int) {
    *map = unsafe {
        (api.hmput)(
            *map,
            BINARY_ELEMENT_SIZE,
            key.as_ptr().cast_mut().cast(),
            key.len(),
            mode,
        )
    };
    unsafe { write_binary_value(*map, BINARY_ELEMENT_SIZE, key, value) };
}

unsafe fn get_temp(
    api: &Api,
    map: *mut c_void,
    element_size: usize,
    key: &[u8],
    mode: c_int,
) -> isize {
    let mut temp = 12345;
    let returned = unsafe {
        (api.hmget_ts)(
            map,
            element_size,
            key.as_ptr().cast_mut().cast(),
            key.len(),
            &mut temp,
            mode,
        )
    };
    assert!(!returned.is_null());
    temp
}

#[derive(Debug, PartialEq, Eq)]
struct BinaryScenario {
    stages: Vec<MapSnapshot>,
    present_temps: Vec<isize>,
    absent_temp: isize,
}

unsafe fn run_binary_scenario(api: &Api, seed: usize, keys: &[Vec<u8>]) -> BinaryScenario {
    unsafe { (api.rand_seed)(seed) };
    let mut map = ptr::null_mut();
    let mut stages = Vec::new();
    for (index, key) in keys.iter().enumerate() {
        unsafe { put_binary(api, &mut map, key, index as u64 * 17 + 3, HM_BINARY) };
        stages.push(unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) });
    }

    let mut present_temps = Vec::new();
    for key in keys {
        let temp = unsafe { get_temp(api, map, BINARY_ELEMENT_SIZE, key, HM_BINARY) };
        map = unsafe {
            (api.hmget)(
                map,
                BINARY_ELEMENT_SIZE,
                key.as_ptr().cast_mut().cast(),
                key.len(),
                HM_BINARY,
            )
        };
        assert_eq!(
            unsafe { (*map_header(map, BINARY_ELEMENT_SIZE)).temp },
            temp
        );
        present_temps.push(temp);
    }
    let absent = vec![0xa5; 13];
    let absent_temp = unsafe { get_temp(api, map, BINARY_ELEMENT_SIZE, &absent, HM_BINARY) };

    if let Some(key) = keys.first() {
        unsafe { put_binary(api, &mut map, key, 0xfeed_beef, HM_BINARY) };
        stages.push(unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) });
    }

    unsafe { free_map(api, map, BINARY_ELEMENT_SIZE) };
    BinaryScenario {
        stages,
        present_temps,
        absent_temp,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct DeleteScenario {
    stages: Vec<MapSnapshot>,
    deletion_results: Vec<isize>,
}

unsafe fn run_delete_scenario(api: &Api, seed: usize, key_count: usize) -> DeleteScenario {
    unsafe { (api.rand_seed)(seed) };
    let mut map = ptr::null_mut();
    let keys: Vec<Vec<u8>> = (0..key_count)
        .map(|index| (index as u64).to_le_bytes().to_vec())
        .collect();
    for (index, key) in keys.iter().enumerate() {
        unsafe { put_binary(api, &mut map, key, index as u64 + 100, HM_BINARY) };
    }
    let mut stages = vec![unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) }];
    let mut deletion_results = Vec::new();

    let absent = usize::MAX.to_le_bytes();
    map = unsafe {
        (api.hmdel)(
            map,
            BINARY_ELEMENT_SIZE,
            absent.as_ptr().cast_mut().cast(),
            absent.len(),
            0,
            HM_BINARY,
        )
    };
    deletion_results.push(unsafe { (*map_header(map, BINARY_ELEMENT_SIZE)).temp });
    stages.push(unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) });

    let deletion_order: Vec<usize> = if key_count > 12 {
        (0..key_count).collect()
    } else {
        vec![key_count - 1, 0]
    };
    for index in deletion_order {
        let key = &keys[index];
        map = unsafe {
            (api.hmdel)(
                map,
                BINARY_ELEMENT_SIZE,
                key.as_ptr().cast_mut().cast(),
                key.len(),
                0,
                HM_BINARY,
            )
        };
        deletion_results.push(unsafe { (*map_header(map, BINARY_ELEMENT_SIZE)).temp });
        stages.push(unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) });
    }
    unsafe { free_map(api, map, BINARY_ELEMENT_SIZE) };
    DeleteScenario {
        stages,
        deletion_results,
    }
}

unsafe fn put_string(
    api: &Api,
    map: &mut *mut c_void,
    source: &mut [u8],
    value: c_int,
    operation_mode: c_int,
) {
    let element_size = size_of::<StringEntry>();
    *map = unsafe {
        (api.hmput)(
            *map,
            element_size,
            source.as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            operation_mode,
        )
    };
    let header = unsafe { &mut *map_header(*map, element_size) };
    let table = unsafe { &mut *header.hash_table.cast::<HashIndex>() };
    let entry = unsafe { map.cast::<StringEntry>().add(header.temp as usize) };
    unsafe {
        *entry = StringEntry {
            key: source.as_mut_ptr().cast(),
            value,
        };
        (*entry).key = table.temp_key;
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StringMapSnapshot {
    length: usize,
    capacity: usize,
    temp: isize,
    table: TableSnapshot,
    entries: Vec<(Vec<u8>, c_int, bool)>,
}

unsafe fn snapshot_string_map(
    map: *mut c_void,
    source_pointers: &[*mut c_char],
) -> StringMapSnapshot {
    let element_size = size_of::<StringEntry>();
    let base = unsafe { snapshot_map(map, element_size) };
    let entries = (0..base.length)
        .map(|index| unsafe {
            let entry = &*map.cast::<StringEntry>().add(index);
            (
                CStr::from_ptr(entry.key).to_bytes().to_vec(),
                entry.value,
                source_pointers.contains(&entry.key),
            )
        })
        .collect();
    StringMapSnapshot {
        length: base.length,
        capacity: base.capacity,
        temp: base.temp,
        table: base.table.unwrap(),
        entries,
    }
}

unsafe fn run_string_scenario(
    api: &Api,
    storage_mode: Option<c_int>,
    operation_mode: c_int,
    seed: usize,
) -> StringScenario {
    unsafe { (api.rand_seed)(seed) };
    let element_size = size_of::<StringEntry>();
    let mut sources = vec![
        c_string(b""),
        c_string(b"a"),
        c_string(b"alpha"),
        c_string(b"a much longer key"),
    ];
    let source_pointers: Vec<*mut c_char> = sources
        .iter_mut()
        .map(|source| source.as_mut_ptr().cast())
        .collect();
    let mut map = match storage_mode {
        Some(mode) => unsafe { (api.shmode)(element_size, mode) },
        None => ptr::null_mut(),
    };
    let mut stages = Vec::new();
    for (index, source) in sources.iter_mut().enumerate() {
        unsafe { put_string(api, &mut map, source, index as c_int * 11, operation_mode) };
        stages.push(unsafe { snapshot_string_map(map, &source_pointers) });
    }
    unsafe { put_string(api, &mut map, &mut sources[1], 991, operation_mode) };
    stages.push(unsafe { snapshot_string_map(map, &source_pointers) });

    let mut lookup_temps = Vec::new();
    for source in &mut sources {
        let mut temp = 777;
        map = unsafe {
            (api.hmget_ts)(
                map,
                element_size,
                source.as_mut_ptr().cast(),
                size_of::<*mut c_char>(),
                &mut temp,
                operation_mode,
            )
        };
        lookup_temps.push(temp);
    }
    let missing = c_string(b"missing");
    lookup_temps.push(unsafe { get_temp(api, map, element_size, &missing, operation_mode) });

    if lookup_temps[..sources.len()].iter().any(|temp| *temp < 0) {
        return StringScenario {
            stages,
            lookup_temps,
            cleanup_skipped: true,
        };
    }

    map = unsafe {
        (api.hmdel)(
            map,
            element_size,
            sources[0].as_mut_ptr().cast(),
            size_of::<*mut c_char>(),
            0,
            operation_mode,
        )
    };
    stages.push(unsafe { snapshot_string_map(map, &source_pointers) });
    unsafe { free_map(api, map, element_size) };
    StringScenario {
        stages,
        lookup_temps,
        cleanup_skipped: false,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StringScenario {
    stages: Vec<StringMapSnapshot>,
    lookup_temps: Vec<isize>,
    cleanup_skipped: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaObservation {
    returned: Vec<Vec<u8>>,
    remaining: usize,
    block: u8,
    mode: u8,
    block_count: usize,
}

unsafe fn arena_block_count(arena: &StringArena) -> usize {
    let mut count = 0;
    let mut block = arena.storage;
    while !block.is_null() {
        count += 1;
        block = unsafe { (*block).next };
    }
    count
}

unsafe fn run_arena_scenario(api: &Api, strings: &mut [Vec<u8>]) -> ArenaObservation {
    let mut arena: StringArena = unsafe { std::mem::zeroed() };
    let mut returned = Vec::new();
    for string in strings {
        let output = unsafe { (api.stralloc)(&mut arena, string.as_mut_ptr().cast()) };
        returned.push(unsafe { CStr::from_ptr(output).to_bytes().to_vec() });
    }
    let observation = ArenaObservation {
        returned,
        remaining: arena.remaining,
        block: arena.block,
        mode: arena.mode,
        block_count: unsafe { arena_block_count(&arena) },
    };
    unsafe { (api.strreset)(&mut arena) };
    assert!(arena.storage.is_null());
    assert_eq!(arena.remaining, 0);
    assert_eq!(arena.block, 0);
    assert_eq!(arena.mode, 0);
    observation
}

fn run_boundary_case(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "boundary_child", "--nocapture"])
        .env("DIFFERENTIAL_CHILD_LIBRARY", library)
        .env("DIFFERENTIAL_CHILD_CASE", case)
        .output()
        .unwrap()
        .status
}

unsafe fn run_wrapped_missing(
    api: &Api,
    seed: usize,
    inserted: &[Vec<u8>],
    missing: &[u8],
) -> (isize, MapSnapshot) {
    unsafe { (api.rand_seed)(seed) };
    let mut map = ptr::null_mut();
    for (index, key) in inserted.iter().enumerate() {
        unsafe { put_binary(api, &mut map, key, index as u64, HM_BINARY) };
    }
    let temp = unsafe { get_temp(api, map, BINARY_ELEMENT_SIZE, missing, HM_BINARY) };
    let snapshot = unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) };
    unsafe { free_map(api, map, BINARY_ELEMENT_SIZE) };
    (temp, snapshot)
}

unsafe fn run_raw_mode_scenario(
    api: &Api,
    storage_mode: c_int,
    operation_mode: c_int,
) -> MapSnapshot {
    let mut map = unsafe { (api.shmode)(BINARY_ELEMENT_SIZE, storage_mode) };
    let mut key = [0x41_u8; 16];
    key[15] = 0;
    map = unsafe {
        (api.hmput)(
            map,
            BINARY_ELEMENT_SIZE,
            key.as_mut_ptr().cast(),
            8,
            operation_mode,
        )
    };
    unsafe { write_binary_value(map, BINARY_ELEMENT_SIZE, &key[..8], 123) };
    let snapshot = unsafe { snapshot_map(map, BINARY_ELEMENT_SIZE) };
    unsafe { free_map(api, map, BINARY_ELEMENT_SIZE) };
    snapshot
}

#[test]
fn differential_surface() {
    let (c, rust) = load_apis();

    unsafe {
        let array_cases = [
            (3, 0, 0, None),
            (5, 1, 0, None),
            (7, 3, 0, None),
            (2, 1, 17, None),
            (3, 0, 4, Some((2, 4, vec![1, 2, 3, 4, 5, 6]))),
            (3, 3, 0, Some((2, 4, vec![7, 8, 9, 10, 11, 12]))),
            (3, 0, 20, Some((2, 4, vec![13, 14, 15, 16, 17, 18]))),
        ];
        for (case_index, (element_size, add_len, min_capacity, initial)) in
            array_cases.into_iter().enumerate()
        {
            let c_result = observe_array(&c, element_size, add_len, min_capacity, initial.clone());
            let rust_result = observe_array(&rust, element_size, add_len, min_capacity, initial);
            assert_eq!(c_result, rust_result, "array case {case_index}");
        }

        let mut array_rng = Rng::new(0x72f0_3a51_99c4_11e7);
        for _ in 0..128 {
            let element_size = (array_rng.next_u64() as usize % 31) + 1;
            let capacity = (array_rng.next_u64() as usize % 61) + 4;
            let length = array_rng.next_u64() as usize % (capacity + 1);
            let mut prefix = vec![0_u8; length * element_size];
            array_rng.fill(&mut prefix);
            let add_len = array_rng.next_u64() as usize % 65;
            let min_capacity = array_rng.next_u64() as usize % 129;
            let initial = Some((length, capacity, prefix));
            assert_eq!(
                observe_array(&c, element_size, add_len, min_capacity, initial.clone()),
                observe_array(&rust, element_size, add_len, min_capacity, initial)
            );
        }

        (c.hmfree)(ptr::null_mut(), BINARY_ELEMENT_SIZE);
        (rust.hmfree)(ptr::null_mut(), BINARY_ELEMENT_SIZE);

        let mut rng = Rng::new(0xd1ff_e2e5_1234_5678);
        for length in 0..=96 {
            for _ in 0..32 {
                let seed = rng.next_u64() as usize;
                let mut bytes = vec![0_u8; length];
                rng.fill(&mut bytes);
                let c_pointer = if length == 0 && seed & 1 == 0 {
                    ptr::null_mut()
                } else {
                    bytes.as_mut_ptr().cast()
                };
                let rust_pointer = c_pointer;
                assert_eq!(
                    (c.hash_bytes)(c_pointer, length, seed),
                    (rust.hash_bytes)(rust_pointer, length, seed),
                    "hash_bytes length={length} seed={seed:#x}"
                );
            }
        }

        for length in 0..=64 {
            for _ in 0..32 {
                let seed = rng.next_u64() as usize;
                let mut bytes = vec![1_u8; length];
                rng.fill(&mut bytes);
                for byte in &mut bytes {
                    if *byte == 0 {
                        *byte = 0x80;
                    }
                }
                let mut string = c_string(&bytes);
                assert_eq!(
                    (c.hash_string)(string.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(string.as_mut_ptr().cast(), seed),
                    "hash_string length={length} seed={seed:#x}"
                );
            }
        }

        for seed in [0, 1, 0x3141_5926, usize::MAX] {
            let keys: Vec<Vec<u8>> = (0..64)
                .map(|index| {
                    let mut key = vec![0_u8; 13];
                    key[..8].copy_from_slice(&(index as u64).to_le_bytes());
                    rng.fill(&mut key[8..]);
                    key
                })
                .collect();
            assert_eq!(
                run_binary_scenario(&c, seed, &keys),
                run_binary_scenario(&rust, seed, &keys)
            );
        }

        for key_size in [0, 1, 4, 13] {
            let keys: Vec<Vec<u8>> = (0..12)
                .map(|index| vec![index as u8 + 1; key_size])
                .collect();
            assert_eq!(
                run_binary_scenario(&c, 0x4433_2211, &keys),
                run_binary_scenario(&rust, 0x4433_2211, &keys)
            );
        }

        let default_observation = |api: &Api| {
            let mut map = (api.hmput_default)(ptr::null_mut(), BINARY_ELEMENT_SIZE);
            let first = snapshot_map(map, BINARY_ELEMENT_SIZE);
            let old = map;
            map = (api.hmput_default)(map, BINARY_ELEMENT_SIZE);
            let second = snapshot_map(map, BINARY_ELEMENT_SIZE);
            let preserved = old == map;
            free_map(api, map, BINARY_ELEMENT_SIZE);
            (first, second, preserved)
        };
        assert_eq!(default_observation(&c), default_observation(&rust));

        let null_get_observation = |api: &Api, mode: c_int| {
            let mut temp = 99;
            let map = (api.hmget_ts)(
                ptr::null_mut(),
                BINARY_ELEMENT_SIZE,
                ptr::null_mut(),
                0,
                &mut temp,
                mode,
            );
            let snapshot = snapshot_map(map, BINARY_ELEMENT_SIZE);
            free_map(api, map, BINARY_ELEMENT_SIZE);
            (temp, snapshot)
        };
        for mode in [HM_BINARY, HM_STRING, -1, 4, c_int::MAX] {
            assert_eq!(
                null_get_observation(&c, mode),
                null_get_observation(&rust, mode)
            );
        }

        let tableless_observation = |api: &Api| {
            let mut key = 42_u64.to_le_bytes();
            let map = (api.hmput_default)(ptr::null_mut(), BINARY_ELEMENT_SIZE);
            let mut temp = 10;
            let returned = (api.hmget_ts)(
                map,
                BINARY_ELEMENT_SIZE,
                key.as_mut_ptr().cast(),
                key.len(),
                &mut temp,
                HM_BINARY,
            );
            assert_eq!(returned, map);
            let returned = (api.hmget)(
                map,
                BINARY_ELEMENT_SIZE,
                key.as_mut_ptr().cast(),
                key.len(),
                HM_BINARY,
            );
            assert_eq!(returned, map);
            let after_get = snapshot_map(map, BINARY_ELEMENT_SIZE);
            let returned = (api.hmdel)(
                map,
                BINARY_ELEMENT_SIZE,
                key.as_mut_ptr().cast(),
                key.len(),
                0,
                HM_BINARY,
            );
            assert_eq!(returned, map);
            let after_delete = snapshot_map(map, BINARY_ELEMENT_SIZE);
            free_map(api, map, BINARY_ELEMENT_SIZE);
            (temp, after_get, after_delete)
        };
        assert_eq!(tableless_observation(&c), tableless_observation(&rust));

        let seed = 0x9911_7733_usize;
        let mut position_six = Vec::new();
        let mut position_seven = None;
        for number in 0_u64..100_000 {
            let mut key = number.to_le_bytes();
            let c_hash = (c.hash_bytes)(key.as_mut_ptr().cast(), key.len(), seed);
            let rust_hash = (rust.hash_bytes)(key.as_mut_ptr().cast(), key.len(), seed);
            assert_eq!(c_hash, rust_hash);
            let adjusted = if c_hash < 2 { c_hash + 2 } else { c_hash };
            match adjusted & 7 {
                6 if position_six.len() < 2 => position_six.push(key.to_vec()),
                7 if position_seven.is_none() => position_seven = Some(key.to_vec()),
                _ => {}
            }
            if position_six.len() == 2 && position_seven.is_some() {
                break;
            }
        }
        assert_eq!(position_six.len(), 2);
        let inserted = vec![position_six[0].clone(), position_seven.unwrap()];
        assert_eq!(
            run_wrapped_missing(&c, seed, &inserted, &position_six[1]),
            run_wrapped_missing(&rust, seed, &inserted, &position_six[1])
        );

        for count in [6, 48] {
            for seed in [0, 1, 0x1357_9bdf, usize::MAX] {
                assert_eq!(
                    run_delete_scenario(&c, seed, count),
                    run_delete_scenario(&rust, seed, count)
                );
            }
        }

        let tombstone_reuse = |api: &Api| {
            (api.rand_seed)(0x2468_ace0);
            let mut map = ptr::null_mut();
            let keys: Vec<Vec<u8>> = (0_u64..6)
                .map(|value| value.to_le_bytes().to_vec())
                .collect();
            for (index, key) in keys.iter().enumerate() {
                put_binary(api, &mut map, key, index as u64, HM_BINARY);
            }
            map = (api.hmdel)(
                map,
                BINARY_ELEMENT_SIZE,
                keys[0].as_ptr().cast_mut().cast(),
                keys[0].len(),
                0,
                HM_BINARY,
            );
            let deleted = snapshot_map(map, BINARY_ELEMENT_SIZE);
            put_binary(api, &mut map, &keys[0], 999, HM_BINARY);
            let reinserted = snapshot_map(map, BINARY_ELEMENT_SIZE);
            free_map(api, map, BINARY_ELEMENT_SIZE);
            (deleted, reinserted)
        };
        assert_eq!(tombstone_reuse(&c), tombstone_reuse(&rust));

        for seed in [
            0,
            1,
            0xa1b2_c3d4,
            0x5a5a_1357,
            usize::MAX,
            0x0123_4567_89ab_cdef,
        ] {
            for (storage_mode, operation_mode) in [
                (None, HM_STRING),
                (Some(SH_DEFAULT), HM_STRING),
                (Some(SH_STRDUP), HM_STRING),
                (Some(SH_ARENA), HM_STRING),
            ] {
                assert_eq!(
                    run_string_scenario(&c, storage_mode, operation_mode, seed),
                    run_string_scenario(&rust, storage_mode, operation_mode, seed)
                );
            }
        }

        let empty_table = |api: &Api, mode: c_int| {
            let map = (api.shmode)(BINARY_ELEMENT_SIZE, mode);
            let snapshot = snapshot_map(map, BINARY_ELEMENT_SIZE);
            free_map(api, map, BINARY_ELEMENT_SIZE);
            snapshot
        };
        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            assert_eq!(empty_table(&c, mode), empty_table(&rust, mode));
        }

        for (storage_mode, operation_mode) in [
            (SH_NONE, HM_BINARY),
            (-1, -1),
            (4, 4),
            (c_int::MAX, c_int::MAX),
        ] {
            assert_eq!(
                run_raw_mode_scenario(&c, storage_mode, operation_mode),
                run_raw_mode_scenario(&rust, storage_mode, operation_mode)
            );
        }

        let mut short_strings = vec![
            c_string(b""),
            c_string(b"x"),
            c_string(&vec![b'a'; 100]),
            c_string(&vec![b'b'; 400]),
        ];
        assert_eq!(
            run_arena_scenario(&c, &mut short_strings.clone()),
            run_arena_scenario(&rust, &mut short_strings)
        );

        let mut dedicated_strings = vec![
            c_string(&vec![b'c'; 600]),
            c_string(&vec![b'd'; 700]),
            c_string(b"tail"),
        ];
        assert_eq!(
            run_arena_scenario(&c, &mut dedicated_strings.clone()),
            run_arena_scenario(&rust, &mut dedicated_strings)
        );

        let mut growth_strings = Vec::new();
        let mut block = 0_u8;
        for _ in 0..26 {
            let block_size = 512_usize << (usize::from(block) >> 1);
            let bounded_size = block_size.min(1 << 20);
            let mut payload = vec![1_u8; bounded_size - 1];
            rng.fill(&mut payload);
            for byte in &mut payload {
                if *byte == 0 {
                    *byte = 1;
                }
            }
            growth_strings.push(c_string(&payload));
            if block_size < 1 << 20 {
                block = block.wrapping_add(1);
            }
        }
        assert_eq!(
            run_arena_scenario(&c, &mut growth_strings.clone()),
            run_arena_scenario(&rust, &mut growth_strings)
        );

        (c.strreset)(&mut std::mem::zeroed());
        (rust.strreset)(&mut std::mem::zeroed());

        for number in [c_int::MIN, -12345, -1, 0, 1, 12345, c_int::MAX] {
            let c_pointer = (c.strkey)(number);
            let rust_pointer = (rust.strkey)(number);
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                CStr::from_ptr(rust_pointer).to_bytes()
            );
            assert_eq!(
                CStr::from_ptr(c_pointer).to_bytes(),
                format!("test_{number}").as_bytes()
            );
            assert_eq!(c_pointer, (c.strkey)(number.wrapping_add(1)));
            assert_eq!(rust_pointer, (rust.strkey)(number.wrapping_add(1)));
        }
        for _ in 0..128 {
            let number = rng.next_u64() as c_int;
            assert_eq!(
                CStr::from_ptr((c.strkey)(number)).to_bytes(),
                CStr::from_ptr((rust.strkey)(number)).to_bytes()
            );
        }

        let mut sh_puts_inputs = vec![-7, 0, 1, 32, 2_000];
        sh_puts_inputs.extend((0..24).map(|_| (rng.next_u64() % 3_000) as c_int));
        for number in sh_puts_inputs {
            let c_output = capture_stdout(|| (c.sh_puts)(number));
            let rust_output = capture_stdout(|| (rust.sh_puts)(number));
            assert_eq!(c_output, rust_output, "sh_puts({number})");
            assert_eq!(c_output, format!("a {number}\n").as_bytes());
        }
    }

    let normal_cases = [
        "hmfree_null",
        "hmdel_null",
        "hash_bytes_null_zero",
        "arrgrow_zero_element",
        "hmget_zero_element",
        "hmput_zero_lengths",
    ];
    for case in normal_cases {
        let c_status = run_boundary_case(&c_library_path(), case);
        let rust_status = run_boundary_case(&rust_library_path(), case);
        assert_eq!(status_shape(c_status), (Some(0), None), "{case} C");
        assert_eq!(status_shape(rust_status), (Some(0), None), "{case} Rust");
    }

    let invalid_cases = [
        "arrfree_null",
        "hash_string_null",
        "hash_bytes_null_one",
        "hmget_null_temp",
        "strreset_null",
        "stralloc_null_arena",
        "stralloc_null_string",
        "hmput_string_null",
        "hmget_binary_null_key",
        "hmget_string_null_key",
        "hmdel_binary_null_key",
        "hmdel_string_null_key",
        "hmget_oversized_key",
        "hmput_oversized_key",
        "hmget_oversized_element",
        "hmdel_moved_key_missing",
        "hmdel_moved_index_wrong",
        "arrgrow_oversized",
        "hash_bytes_oversized",
    ];
    for case in invalid_cases {
        let c_status = run_boundary_case(&c_library_path(), case);
        let rust_status = run_boundary_case(&rust_library_path(), case);
        assert_eq!(
            status_shape(c_status),
            status_shape(rust_status),
            "boundary behavior differs for {case}"
        );
        if matches!(case, "hmdel_moved_key_missing" | "hmdel_moved_index_wrong") {
            assert_eq!(status_shape(c_status), (None, Some(6)), "{case}");
        }
    }
}

#[test]
fn boundary_child() {
    let Some(path) = std::env::var_os("DIFFERENTIAL_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFFERENTIAL_CHILD_CASE").unwrap();
    let api = unsafe { Api::load(Path::new(&path)) };
    unsafe {
        match case.as_str() {
            "hmfree_null" => (api.hmfree)(ptr::null_mut(), BINARY_ELEMENT_SIZE),
            "hmdel_null" => {
                let result = (api.hmdel)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    ptr::null_mut(),
                    0,
                    0,
                    HM_BINARY,
                );
                assert!(result.is_null());
            }
            "hash_bytes_null_zero" => {
                let _ = (api.hash_bytes)(ptr::null_mut(), 0, 17);
            }
            "arrgrow_zero_element" => {
                let array = (api.arrgrow)(ptr::null_mut(), 0, 1, 0);
                assert!(!array.is_null());
                (api.arrfree)(array);
            }
            "hmget_zero_element" => {
                let mut temp = 0;
                let map =
                    (api.hmget_ts)(ptr::null_mut(), 0, ptr::null_mut(), 0, &mut temp, HM_BINARY);
                assert_eq!(temp, -1);
                (api.hmfree)(map, 0);
            }
            "hmput_zero_lengths" => {
                let mut byte = 1_u8;
                let map = (api.hmput)(
                    ptr::null_mut(),
                    0,
                    (&mut byte as *mut u8).cast(),
                    0,
                    HM_BINARY,
                );
                (api.hmfree)(map, 0);
            }
            "arrfree_null" => (api.arrfree)(ptr::null_mut()),
            "hash_string_null" => {
                let _ = (api.hash_string)(ptr::null_mut(), 0);
            }
            "hash_bytes_null_one" => {
                let _ = (api.hash_bytes)(ptr::null_mut(), 1, 0);
            }
            "hmget_null_temp" => {
                let _ = (api.hmget_ts)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    HM_BINARY,
                );
            }
            "strreset_null" => (api.strreset)(ptr::null_mut()),
            "stralloc_null_arena" => {
                let mut string = c_string(b"x");
                let _ = (api.stralloc)(ptr::null_mut(), string.as_mut_ptr().cast());
            }
            "stralloc_null_string" => {
                let mut arena: StringArena = std::mem::zeroed();
                let _ = (api.stralloc)(&mut arena, ptr::null_mut());
            }
            "hmput_string_null" => {
                let _ = (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            }
            "hmget_binary_null_key" => {
                let mut key = 7_u64.to_le_bytes();
                let map = (api.hmput)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    HM_BINARY,
                );
                let _ = (api.hmget)(
                    map,
                    BINARY_ELEMENT_SIZE,
                    ptr::null_mut(),
                    key.len(),
                    HM_BINARY,
                );
            }
            "hmget_string_null_key" => {
                let mut key = c_string(b"valid");
                let map = (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    key.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                let _ = (api.hmget)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            }
            "hmdel_binary_null_key" => {
                let mut key = 7_u64.to_le_bytes();
                let map = (api.hmput)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    HM_BINARY,
                );
                let _ = (api.hmdel)(
                    map,
                    BINARY_ELEMENT_SIZE,
                    ptr::null_mut(),
                    key.len(),
                    0,
                    HM_BINARY,
                );
            }
            "hmdel_string_null_key" => {
                let mut key = c_string(b"valid");
                let map = (api.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    key.as_mut_ptr().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                let _ = (api.hmdel)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                );
            }
            "hmget_oversized_key" => {
                let mut key = 7_u64.to_le_bytes();
                let map = (api.hmput)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    key.as_mut_ptr().cast(),
                    key.len(),
                    HM_BINARY,
                );
                let _ = (api.hmget)(
                    map,
                    BINARY_ELEMENT_SIZE,
                    key.as_mut_ptr().cast(),
                    usize::MAX,
                    HM_BINARY,
                );
            }
            "hmput_oversized_key" => {
                let mut byte = 1_u8;
                let _ = (api.hmput)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    (&mut byte as *mut u8).cast(),
                    usize::MAX,
                    HM_BINARY,
                );
            }
            "hmget_oversized_element" => {
                let mut temp = 0;
                let _ = (api.hmget_ts)(
                    ptr::null_mut(),
                    usize::MAX,
                    ptr::null_mut(),
                    0,
                    &mut temp,
                    HM_BINARY,
                );
            }
            "hmdel_moved_key_missing" | "hmdel_moved_index_wrong" => {
                let mut first = 11_u64.to_le_bytes();
                let mut second = 22_u64.to_le_bytes();
                let mut map = (api.hmput)(
                    ptr::null_mut(),
                    BINARY_ELEMENT_SIZE,
                    first.as_mut_ptr().cast(),
                    first.len(),
                    HM_BINARY,
                );
                map = (api.hmput)(
                    map,
                    BINARY_ELEMENT_SIZE,
                    second.as_mut_ptr().cast(),
                    second.len(),
                    HM_BINARY,
                );
                let table = &mut *(*map_header(map, BINARY_ELEMENT_SIZE))
                    .hash_table
                    .cast::<HashIndex>();
                let mut corrupted = false;
                for bucket_number in 0..table.slot_count / BUCKET_LENGTH {
                    let bucket = &mut *table.storage.add(bucket_number);
                    for slot in 0..BUCKET_LENGTH {
                        if bucket.index[slot] == 1 {
                            if case == "hmdel_moved_key_missing" {
                                bucket.hash[slot] = 0;
                                bucket.index[slot] = -1;
                            } else {
                                bucket.index[slot] = 0;
                            }
                            corrupted = true;
                        }
                    }
                }
                assert!(corrupted);
                let _ = (api.hmdel)(
                    map,
                    BINARY_ELEMENT_SIZE,
                    first.as_mut_ptr().cast(),
                    first.len(),
                    0,
                    HM_BINARY,
                );
            }
            "arrgrow_oversized" => {
                let _ = (api.arrgrow)(ptr::null_mut(), usize::MAX, 1, usize::MAX);
            }
            "hash_bytes_oversized" => {
                let mut byte = 1_u8;
                let _ = (api.hash_bytes)(&mut byte as *mut u8 as *mut c_void, usize::MAX, 0);
            }
            other => panic!("unknown child case {other}"),
        }
    }
}
