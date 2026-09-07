use libc::{FILE, c_char, c_int, c_uint, c_void, size_t, stat, time_t, tm};
use libloading::Library;
use std::ffi::{CStr, CString};
use std::fs;
use std::io::Write;
use std::mem;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const MAX_FQUEUE: usize = 256;
const CRALERT_MAIL_SET: c_int = 0x001;
const CRALERT_EXEC_SET: c_int = 0x002;
const CRALERT_READ_ALL: c_int = 0x004;
const CRALERT_READ_FAILED: c_int = 0x008;
const CRALERT_FP_SET: c_int = 0x010;

#[repr(C)]
struct AlertData {
    rule: c_uint,
    level: c_uint,
    alertid: *mut c_char,
    date: *mut c_char,
    location: *mut c_char,
    comment: *mut c_char,
    group: *mut c_char,
    srcip: *mut c_char,
    srcport: c_int,
    dstip: *mut c_char,
    dstport: c_int,
    user: *mut c_char,
    filename: *mut c_char,
}

#[repr(C)]
struct FileQueue {
    last_change: time_t,
    year: c_int,
    day: c_int,
    flags: c_int,
    mon: [c_char; 4],
    file_name: [c_char; MAX_FQUEUE + 1],
    fp: *mut FILE,
    f_status: stat,
}

type OsCalloc = unsafe extern "C" fn(size_t, size_t) -> *mut c_void;
type OsRealloc = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;
type OsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FreeAlertData = unsafe extern "C" fn(*mut AlertData);
type GetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut AlertData;
type Merror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
type InitFileQueue = unsafe extern "C" fn(*mut FileQueue, *const tm, c_int) -> c_int;
type ReadFileMon = unsafe extern "C" fn(*mut FileQueue, *const tm, c_uint) -> *mut AlertData;
type Driver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;

struct Api {
    _lib: Library,
    os_calloc: OsCalloc,
    os_realloc: OsRealloc,
    os_strdup: OsStrdup,
    free_alert_data: FreeAlertData,
    get_alert_data: GetAlertData,
    merror: Merror,
    init_file_queue: InitFileQueue,
    read_file_mon: ReadFileMon,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }.unwrap();
        let os_calloc = unsafe { *lib.get::<OsCalloc>(b"os_calloc\0").unwrap() };
        let os_realloc = unsafe { *lib.get::<OsRealloc>(b"os_realloc\0").unwrap() };
        let os_strdup = unsafe { *lib.get::<OsStrdup>(b"os_strdup\0").unwrap() };
        let free_alert_data = unsafe { *lib.get::<FreeAlertData>(b"FreeAlertData\0").unwrap() };
        let get_alert_data = unsafe { *lib.get::<GetAlertData>(b"GetAlertData\0").unwrap() };
        let merror = unsafe { *lib.get::<Merror>(b"merror\0").unwrap() };
        let init_file_queue = unsafe { *lib.get::<InitFileQueue>(b"Init_FileQueue\0").unwrap() };
        let read_file_mon = unsafe { *lib.get::<ReadFileMon>(b"Read_FileMon\0").unwrap() };
        let driver = unsafe { *lib.get::<Driver>(b"driver\0").unwrap() };
        Self {
            _lib: lib,
            os_calloc,
            os_realloc,
            os_strdup,
            free_alert_data,
            get_alert_data,
            merror,
            init_file_queue,
            read_file_mon,
            driver,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlertSnapshot {
    rule: u32,
    level: u32,
    alertid: Option<Vec<u8>>,
    date: Option<Vec<u8>>,
    location: Option<Vec<u8>>,
    comment: Option<Vec<u8>>,
    group: Option<Vec<u8>>,
    srcip: Option<Vec<u8>>,
    srcport: i32,
    dstip: Option<Vec<u8>>,
    dstport: i32,
    user: Option<Vec<u8>>,
    filename: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueueSnapshot {
    result: i32,
    last_change: i64,
    year: i32,
    day: i32,
    flags: i32,
    mon: Vec<u8>,
    file_name: Vec<u8>,
    fp_is_null: bool,
    position: Option<i64>,
}

unsafe fn optional_c_bytes(value: *const c_char) -> Option<Vec<u8>> {
    if value.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
    }
}

unsafe fn snapshot_alert(value: *const AlertData) -> AlertSnapshot {
    let value = unsafe { &*value };
    AlertSnapshot {
        rule: value.rule,
        level: value.level,
        alertid: unsafe { optional_c_bytes(value.alertid) },
        date: unsafe { optional_c_bytes(value.date) },
        location: unsafe { optional_c_bytes(value.location) },
        comment: unsafe { optional_c_bytes(value.comment) },
        group: unsafe { optional_c_bytes(value.group) },
        srcip: unsafe { optional_c_bytes(value.srcip) },
        srcport: value.srcport,
        dstip: unsafe { optional_c_bytes(value.dstip) },
        dstport: value.dstport,
        user: unsafe { optional_c_bytes(value.user) },
        filename: unsafe { optional_c_bytes(value.filename) },
    }
}

unsafe fn parse_one(api: &Api, input: &[u8], flags: c_int) -> Option<AlertSnapshot> {
    let fp = unsafe { file_from_bytes(input) };
    let value = unsafe { (api.get_alert_data)(flags, fp) };
    let result = if value.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(value) };
        unsafe { (api.free_alert_data)(value) };
        Some(snapshot)
    };
    unsafe { libc::fclose(fp) };
    result
}

unsafe fn parse_many(
    api: &Api,
    input: &[u8],
    flags: c_int,
    count: usize,
) -> Vec<Option<AlertSnapshot>> {
    let fp = unsafe { file_from_bytes(input) };
    let mut out = Vec::new();
    for _ in 0..count {
        let value = unsafe { (api.get_alert_data)(flags, fp) };
        if value.is_null() {
            out.push(None);
        } else {
            out.push(Some(unsafe { snapshot_alert(value) }));
            unsafe { (api.free_alert_data)(value) };
        }
    }
    unsafe { libc::fclose(fp) };
    out
}

unsafe fn file_from_bytes(input: &[u8]) -> *mut FILE {
    let fp = unsafe { libc::tmpfile() };
    assert!(!fp.is_null());
    if !input.is_empty() {
        let written = unsafe { libc::fwrite(input.as_ptr().cast(), 1, input.len(), fp) };
        assert_eq!(written, input.len());
    }
    assert_eq!(unsafe { libc::fseek(fp, 0, libc::SEEK_SET) }, 0);
    fp
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        manifest.join("../c_src/build/libdriver.so"),
        manifest.join("target/release/libdriver.so"),
    )
}

unsafe fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.exists(), "missing {}", c_path.display());
    assert!(rust_path.exists(), "missing {}", rust_path.display());
    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
}

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        static NEXT: OnceLock<Mutex<u64>> = OnceLock::new();
        let mut next = NEXT.get_or_init(|| Mutex::new(0)).lock().unwrap();
        *next += 1;
        let path = std::env::temp_dir().join(format!(
            "driver-differential-{}-{}-{}",
            std::process::id(),
            label,
            *next
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct CwdGuard(PathBuf);

impl CwdGuard {
    fn enter(path: &Path) -> Self {
        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self(old)
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).unwrap();
    }
}

fn make_tm(day: i32, month: i32, year: i32) -> tm {
    let mut value: tm = unsafe { mem::zeroed() };
    value.tm_mday = day;
    value.tm_mon = month;
    value.tm_year = year;
    value
}

fn c_array_bytes<const N: usize>(value: &[c_char; N]) -> Vec<u8> {
    value
        .iter()
        .map(|v| *v as u8)
        .take_while(|v| *v != 0)
        .collect()
}

unsafe fn queue_snapshot(result: i32, queue: &FileQueue) -> QueueSnapshot {
    let position = if queue.fp.is_null() {
        None
    } else {
        Some(unsafe { libc::ftell(queue.fp) } as i64)
    };
    QueueSnapshot {
        result,
        last_change: queue.last_change as i64,
        year: queue.year,
        day: queue.day,
        flags: queue.flags,
        mon: c_array_bytes(&queue.mon),
        file_name: c_array_bytes(&queue.file_name),
        fp_is_null: queue.fp.is_null(),
        position,
    }
}

fn basic_alert(id: &str, channel: &str, group: Option<&str>, body: &str) -> Vec<u8> {
    let mut value = format!("** Alert {id}: {channel}");
    if let Some(group) = group {
        value.push_str(" - ");
        value.push_str(group);
    }
    value.push('\n');
    value.push_str("2026 Sep 05 01:02:03 location\n");
    value.push_str(body);
    value.into_bytes()
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }

    fn ascii(&mut self, min: usize, extra: usize) -> String {
        let len = min + self.range((extra + 1) as u32) as usize;
        (0..len)
            .map(|_| (b'a' + self.range(26) as u8) as char)
            .collect()
    }
}

fn assert_parse_equal(c: &Api, rust: &Api, input: &[u8], flags: c_int) {
    let c_value = unsafe { parse_one(c, input, flags) };
    let rust_value = unsafe { parse_one(rust, input, flags) };
    assert_eq!(
        c_value,
        rust_value,
        "input={}",
        String::from_utf8_lossy(input)
    );
}

fn spawn_child(lib_kind: &str, case: &str) -> Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("ffi_child_dispatch")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DRIVER_CHILD_LIB", lib_kind)
        .env("DRIVER_CHILD_CASE", case);
    if case == "read_post_reopen_null" {
        command.env("LD_PRELOAD", fseek_interposer());
    }
    command.output().unwrap()
}

fn fseek_interposer() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output_dir = manifest.join("target/test-support");
        fs::create_dir_all(&output_dir).unwrap();
        let output = output_dir.join("libfseek_delay.so");
        let status = Command::new("cc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-O2")
            .arg("-o")
            .arg(&output)
            .arg(manifest.join("tests/support/fseek_delay.c"))
            .arg("-ldl")
            .status()
            .unwrap();
        assert!(status.success());
        output
    })
}

fn normalized_child_stdout(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !line.starts_with(b"test result:") {
            out.extend_from_slice(line);
        }
    }
    out
}

fn assert_child_equal(case: &str) -> (Output, Output) {
    let c = spawn_child("c", case);
    let rust = spawn_child("rust", case);
    assert_eq!(
        c.status.code(),
        rust.status.code(),
        "status mismatch for {case}: C={:?}, Rust={:?}",
        c.status,
        rust.status
    );
    assert_eq!(
        c.status.signal(),
        rust.status.signal(),
        "signal mismatch for {case}: C={:?}, Rust={:?}",
        c.status,
        rust.status
    );
    assert_eq!(
        normalized_child_stdout(&c.stdout),
        normalized_child_stdout(&rust.stdout),
        "stdout mismatch for {case}"
    );
    assert_eq!(c.stderr, rust.stderr, "stderr mismatch for {case}");
    (c, rust)
}

fn selected_child_api() -> Option<(Api, String)> {
    let case = std::env::var("DRIVER_CHILD_CASE").ok()?;
    let kind = std::env::var("DRIVER_CHILD_LIB").unwrap();
    let (c_path, rust_path) = library_paths();
    let path = if kind == "c" { c_path } else { rust_path };
    Some((unsafe { Api::load(&path) }, case))
}

#[test]
fn ffi_child_dispatch() {
    let Some((api, case)) = selected_child_api() else {
        return;
    };
    match case.as_str() {
        "merror_short" => unsafe {
            (api.merror)(
                c"file=%s errno=%d message=%s".as_ptr(),
                c"alerts.log".as_ptr(),
                17,
                c"short".as_ptr(),
            );
        },
        "merror_long" => {
            let file = CString::new("f".repeat(300)).unwrap();
            let message = CString::new("m".repeat(300)).unwrap();
            unsafe {
                (api.merror)(c"%s/%d/%s".as_ptr(), file.as_ptr(), 999, message.as_ptr());
            }
        }
        "calloc_fail" => unsafe {
            let _ = (api.os_calloc)(usize::MAX, 2);
        },
        "realloc_fail" => unsafe {
            let _ = (api.os_realloc)(ptr::null_mut(), usize::MAX);
        },
        "strdup_null" => unsafe {
            let _ = (api.os_strdup)(ptr::null());
        },
        "strdup_oom" => {
            let mut input = vec![b'x'; 256 * 1024 * 1024 + 1];
            *input.last_mut().unwrap() = 0;
            let statm = fs::read_to_string("/proc/self/statm").unwrap();
            let pages: u64 = statm.split_whitespace().next().unwrap().parse().unwrap();
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
            let limit = pages * page_size;
            let rlimit = libc::rlimit {
                rlim_cur: limit,
                rlim_max: limit,
            };
            assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_AS, &rlimit) }, 0);
            let result = unsafe { (api.os_strdup)(input.as_ptr().cast()) };
            unsafe { libc::free(result.cast()) };
            std::process::exit(99);
        }
        "null_free" => unsafe {
            (api.free_alert_data)(ptr::null_mut());
        },
        "null_get" => unsafe {
            let _ = (api.get_alert_data)(0, ptr::null_mut());
        },
        "null_init_fileq" => {
            let time = make_tm(1, 0, 126);
            unsafe {
                let _ = (api.init_file_queue)(ptr::null_mut(), &time, 0);
            }
        }
        "null_init_tm" => {
            let mut queue: FileQueue = unsafe { mem::zeroed() };
            unsafe {
                let _ = (api.init_file_queue)(
                    &mut queue,
                    ptr::null(),
                    CRALERT_FP_SET | CRALERT_READ_ALL,
                );
            }
        }
        "null_read_fileq" => {
            let time = make_tm(1, 0, 126);
            unsafe {
                let _ = (api.read_file_mon)(ptr::null_mut(), &time, 0);
            }
        }
        "null_read_tm" => {
            let mut queue: FileQueue = unsafe { mem::zeroed() };
            queue.fp = unsafe { libc::tmpfile() };
            queue.flags = CRALERT_FP_SET | CRALERT_READ_ALL;
            unsafe {
                let _ = (api.read_file_mon)(&mut queue, ptr::null(), 0);
            }
        }
        "read_missing" => {
            let mut queue: FileQueue = unsafe { mem::zeroed() };
            let time = make_tm(1, 0, 126);
            unsafe {
                libc::strncpy(
                    queue.file_name.as_mut_ptr(),
                    c"definitely-missing-alerts.log".as_ptr(),
                    MAX_FQUEUE,
                );
                let value = (api.read_file_mon)(&mut queue, &time, 0);
                println!("{}", value.is_null());
            }
        }
        "read_reopen_missing" => {
            let mut queue: FileQueue = unsafe { mem::zeroed() };
            queue.fp = unsafe { libc::tmpfile() };
            queue.flags = 0;
            let time = make_tm(1, 0, 126);
            unsafe {
                let value = (api.read_file_mon)(&mut queue, &time, 0);
                println!("{}", value.is_null());
            }
        }
        "read_post_reopen_null" => {
            let mut queue: FileQueue = unsafe { mem::zeroed() };
            unsafe {
                libc::strncpy(
                    queue.file_name.as_mut_ptr(),
                    c"alerts.log".as_ptr(),
                    MAX_FQUEUE,
                );
            }
            let queue_address = (&mut queue as *mut FileQueue) as usize;
            let original_fp = Arc::new(AtomicUsize::new(0));
            let thread_fp = Arc::clone(&original_fp);
            let watcher = thread::spawn(move || {
                let queue = queue_address as *mut FileQueue;
                loop {
                    let fp = unsafe { ptr::read_volatile(&(*queue).fp) };
                    if !fp.is_null() {
                        thread_fp.store(fp as usize, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(10));
                        unsafe {
                            ptr::write_volatile(&mut (*queue).fp, ptr::null_mut());
                        }
                        break;
                    }
                    std::hint::spin_loop();
                }
            });
            let time = make_tm(1, 0, 126);
            let value = unsafe { (api.read_file_mon)(&mut queue, &time, 0) };
            watcher.join().unwrap();
            let fp = original_fp.load(Ordering::SeqCst) as *mut FILE;
            if !fp.is_null() {
                unsafe { libc::fclose(fp) };
            }
            println!("{}", value.is_null() && queue.fp.is_null());
        }
        "driver_init_fail" => unsafe {
            let value = (api.driver)(1, 0, 126, 0, 0);
            println!("{}", value.is_null());
        },
        "month_negative" => unsafe {
            let value = (api.driver)(1, -1, 126, 0, CRALERT_READ_ALL);
            if !value.is_null() {
                (api.free_alert_data)(value);
            }
        },
        "month_twelve" => unsafe {
            let value = (api.driver)(1, 12, 126, 0, CRALERT_READ_ALL);
            if !value.is_null() {
                (api.free_alert_data)(value);
            }
        },
        other => panic!("unknown child case {other}"),
    }
}

unsafe fn fopen_path(path: &Path, mode: &CStr) -> *mut FILE {
    let bytes = path.as_os_str().as_encoded_bytes();
    let path = CString::new(bytes).unwrap();
    unsafe { libc::fopen(path.as_ptr(), mode.as_ptr()) }
}

unsafe fn call_init(
    api: &Api,
    time: &tm,
    flags: c_int,
    supplied_file: Option<&Path>,
) -> QueueSnapshot {
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    if let Some(path) = supplied_file {
        queue.fp = unsafe { fopen_path(path, c"r") };
        assert!(!queue.fp.is_null());
    }
    let result = unsafe { (api.init_file_queue)(&mut queue, time, flags) };
    let snapshot = unsafe { queue_snapshot(result, &queue) };
    if !queue.fp.is_null() {
        unsafe { libc::fclose(queue.fp) };
    }
    snapshot
}

unsafe fn call_read_with_initialized_stream(
    api: &Api,
    path: &Path,
    time: &tm,
    flags: c_int,
    timeout: u32,
) -> Option<AlertSnapshot> {
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = unsafe { fopen_path(path, c"r") };
    assert!(!queue.fp.is_null());
    let init = unsafe { (api.init_file_queue)(&mut queue, time, flags) };
    assert_eq!(init, 0);
    let value = unsafe { (api.read_file_mon)(&mut queue, time, timeout) };
    let snapshot = if value.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(value) };
        unsafe { (api.free_alert_data)(value) };
        Some(snapshot)
    };
    if !queue.fp.is_null() {
        unsafe { libc::fclose(queue.fp) };
    }
    snapshot
}

unsafe fn call_read_after_append_retry(
    api: &Api,
    alerts_path: &Path,
    time: &tm,
    input: &[u8],
) -> Option<AlertSnapshot> {
    fs::write(alerts_path, b"").unwrap();
    let producer_path = alerts_path.to_path_buf();
    let producer_input = input.to_vec();
    let producer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(producer_path)
            .unwrap();
        file.write_all(&producer_input).unwrap();
        file.flush().unwrap();
    });

    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = unsafe { libc::tmpfile() };
    assert!(!queue.fp.is_null());
    queue.flags = CRALERT_READ_ALL;
    let value = unsafe { (api.read_file_mon)(&mut queue, time, 2) };
    producer.join().unwrap();
    let snapshot = if value.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(value) };
        unsafe { (api.free_alert_data)(value) };
        Some(snapshot)
    };
    if !queue.fp.is_null() {
        unsafe { libc::fclose(queue.fp) };
    }
    snapshot
}

#[test]
fn valid_allocator_and_parser_surface() {
    let _guard = test_lock().lock().unwrap();
    let (c, rust) = unsafe { apis() };

    for &(count, size) in &[(1usize, 1usize), (4, 17), (33, 8), (0, 8), (8, 0)] {
        let c_ptr = unsafe { (c.os_calloc)(count, size) };
        let rust_ptr = unsafe { (rust.os_calloc)(count, size) };
        assert_eq!(c_ptr.is_null(), rust_ptr.is_null());
        if !c_ptr.is_null() && count.saturating_mul(size) != 0 {
            let len = count * size;
            let c_bytes = unsafe { std::slice::from_raw_parts(c_ptr.cast::<u8>(), len) };
            let rust_bytes = unsafe { std::slice::from_raw_parts(rust_ptr.cast::<u8>(), len) };
            assert_eq!(c_bytes, rust_bytes);
            assert!(c_bytes.iter().all(|value| *value == 0));
        }
        unsafe {
            libc::free(c_ptr);
            libc::free(rust_ptr);
        }
    }

    for api in [&c, &rust] {
        let mut ptr = unsafe { (api.os_realloc)(ptr::null_mut(), 32) };
        assert!(!ptr.is_null());
        for index in 0..32 {
            unsafe { *ptr.cast::<u8>().add(index) = index as u8 };
        }
        ptr = unsafe { (api.os_realloc)(ptr, 128) };
        assert!(!ptr.is_null());
        for index in 0..32 {
            assert_eq!(unsafe { *ptr.cast::<u8>().add(index) }, index as u8);
        }
        ptr = unsafe { (api.os_realloc)(ptr, 12) };
        assert!(!ptr.is_null());
        for index in 0..12 {
            assert_eq!(unsafe { *ptr.cast::<u8>().add(index) }, index as u8);
        }
        unsafe { libc::free(ptr) };

        for text in [
            b"".as_slice(),
            b"abc".as_slice(),
            b"spaces and symbols !@#".as_slice(),
        ] {
            let input = CString::new(text).unwrap();
            let output = unsafe { (api.os_strdup)(input.as_ptr()) };
            assert_eq!(unsafe { CStr::from_ptr(output) }.to_bytes(), text);
            unsafe { libc::free(output.cast()) };
        }

        let empty = unsafe { (api.os_calloc)(1, mem::size_of::<AlertData>()).cast::<AlertData>() };
        unsafe { (api.free_alert_data)(empty) };

        let full = unsafe { (api.os_calloc)(1, mem::size_of::<AlertData>()).cast::<AlertData>() };
        for field in unsafe {
            [
                &mut (*full).alertid,
                &mut (*full).date,
                &mut (*full).location,
                &mut (*full).comment,
                &mut (*full).group,
                &mut (*full).srcip,
                &mut (*full).dstip,
                &mut (*full).user,
                &mut (*full).filename,
            ]
        } {
            *field = unsafe { (api.os_strdup)(c"allocated".as_ptr()) };
        }
        unsafe { (api.free_alert_data)(full) };
    }

    let skipped = [
        b"noise before any alert\n".as_slice(),
        b"** Alert header without colon\n".as_slice(),
        b"** Alert bad:no-space\n".as_slice(),
        &basic_alert("ok", "log", Some("group"), ""),
    ]
    .concat();
    assert_parse_equal(&c, &rust, &skipped, 0);

    let mail_filtered = [
        basic_alert("skip", "log", Some("other"), "Rule: 1 (level 2) 'skip'\n"),
        basic_alert("keep", "mail", Some("chosen"), "Rule: 3 (level 4) 'keep'\n"),
    ]
    .concat();
    assert_parse_equal(&c, &rust, &mail_filtered, CRALERT_MAIL_SET);

    for group in [None, Some("     plain-group")] {
        let input = basic_alert("group", "mail", group, "");
        assert_parse_equal(&c, &rust, &input, 0);
    }

    let syscheck = basic_alert(
        "sys",
        "mail",
        Some("syscheck"),
        "Integrity checksum changed for: '/tmp/random-file'\n",
    );
    assert_parse_equal(&c, &rust, &syscheck, 0);

    let syscheck_cleared = basic_alert(
        "sys-clear",
        "mail",
        Some("prefix-syscheck-suffix"),
        "first ordinary line\nIntegrity checksum changed for: '/tmp/not-captured'\n",
    );
    assert_parse_equal(&c, &rust, &syscheck_cleared, 0);

    let minimal = basic_alert("minimal", "mail", Some("plain"), "");
    assert_parse_equal(&c, &rust, &minimal, 0);

    let mut rng = Lcg::new(0x5eed_1234_9876_abcd);
    for index in 0..128u32 {
        let rule_text = match index % 4 {
            0 => rng.range(100_000).to_string(),
            1 => format!("-{}", rng.range(100_000)),
            2 => format!("+{}", rng.range(100_000)),
            _ => format!("{}junk", rng.range(100_000)),
        };
        let level_text = match index % 3 {
            0 => rng.range(32).to_string(),
            1 => format!("-{}", rng.range(32)),
            _ => format!("{}tail", rng.range(32)),
        };
        let comment = rng.ascii(0, 64);
        let body = format!("Rule: {rule_text} (level {level_text}) '{comment}'\n");
        let input = basic_alert(&format!("r{index}"), "mail", Some("random"), &body);
        assert_parse_equal(&c, &rust, &input, 0);
    }

    for index in 0..96u32 {
        let src = if index % 2 == 0 {
            rng.ascii(0, 30)
        } else {
            format!(
                "{}.{}.{}.{}",
                rng.range(256),
                rng.range(256),
                rng.range(256),
                rng.range(256)
            )
        };
        let dst = rng.ascii(0, 40);
        let user = rng.ascii(0, 30);
        let src_port = match index % 3 {
            0 => rng.range(65_536).to_string(),
            1 => format!("-{}", rng.range(1000)),
            _ => format!("{}suffix", rng.range(1000)),
        };
        let dst_port = format!("+{}", rng.range(100_000));
        let body = format!(
            "Src IP: {src}\nSrc Port: {src_port}\nDst IP: {dst}\nDst Port: {dst_port}\nUser: {user}\n"
        );
        let input = basic_alert(&format!("o{index}"), "mail", Some("optional"), &body);
        assert_parse_equal(&c, &rust, &input, 0);
    }

    let repeated = [
        b"** Alert repeated: mail - first\n".as_slice(),
        b"** Alert repeated: mail - second\n".as_slice(),
        b"2026 Sep 05 01:02:03 location\n".as_slice(),
        b"Rule: 1 (level 2) 'first'\nRule: 3 (level 4) 'second'\n".as_slice(),
        b"Src IP: first\nSrc IP: second\nSrc Port: 1\nSrc Port: 2\n".as_slice(),
        b"Dst IP: first\nDst IP: second\nDst Port: 3\nDst Port: 4\n".as_slice(),
        b"User: first\nUser: second\n".as_slice(),
    ]
    .concat();
    assert_parse_equal(&c, &rust, &repeated, 0);

    for count in [0usize, 1, 100, 101, 140] {
        let body = (0..count)
            .map(|index| format!("ordinary log line {index}\n"))
            .collect::<String>();
        let input = basic_alert("logs", "mail", Some("plain"), &body);
        assert_parse_equal(&c, &rust, &input, 0);
    }

    for length in [1021usize, 1022, 1023, 1024, 1025, 2048] {
        let body = format!("{}\n", "x".repeat(length));
        let input = basic_alert("long", "mail", Some("plain"), &body);
        assert_parse_equal(&c, &rust, &input, 0);
    }

    let two_alerts = [
        basic_alert("first", "mail", Some("one"), "Rule: 1 (level 2) 'first'\n"),
        basic_alert(
            "second",
            "mail",
            Some("two"),
            "Rule: 3 (level 4) 'second'\n",
        ),
    ]
    .concat();
    let c_values = unsafe { parse_many(&c, &two_alerts, 0, 3) };
    let rust_values = unsafe { parse_many(&rust, &two_alerts, 0, 3) };
    assert_eq!(c_values, rust_values);

    let all_flags = CRALERT_MAIL_SET
        | CRALERT_EXEC_SET
        | CRALERT_READ_ALL
        | CRALERT_READ_FAILED
        | CRALERT_FP_SET
        | 0x4000_0000;
    assert_parse_equal(&c, &rust, &mail_filtered, all_flags);
}

#[test]
fn valid_queue_and_driver_surface() {
    let _guard = test_lock().lock().unwrap();
    let (c, rust) = unsafe { apis() };
    let temp = TempDir::new("queue");
    let _cwd = CwdGuard::enter(&temp.path);
    let stream_path = temp.path.join("stream.log");
    let input = basic_alert(
        "queue",
        "mail",
        Some("queue-group"),
        "Rule: 17 (level 9) 'queue-comment'\nSrc IP: 127.0.0.1\n",
    );
    fs::write(&stream_path, &input).unwrap();

    for month in 0..12 {
        let time = make_tm(1 + month, month, 100 + month);

        let c_queue = unsafe { call_init(&c, &time, CRALERT_FP_SET, Some(&stream_path)) };
        let rust_queue = unsafe { call_init(&rust, &time, CRALERT_FP_SET, Some(&stream_path)) };
        assert_eq!(c_queue, rust_queue);
        assert_eq!(c_queue.mon.len(), 3);
        assert_eq!(c_queue.file_name, b"<stdin>");
        assert_eq!(c_queue.position, Some(input.len() as i64));

        let c_queue = unsafe {
            call_init(
                &c,
                &time,
                CRALERT_FP_SET | CRALERT_READ_ALL,
                Some(&stream_path),
            )
        };
        let rust_queue = unsafe {
            call_init(
                &rust,
                &time,
                CRALERT_FP_SET | CRALERT_READ_ALL,
                Some(&stream_path),
            )
        };
        assert_eq!(c_queue, rust_queue);
        assert_eq!(c_queue.position, Some(0));
    }

    let all_unused = CRALERT_EXEC_SET | CRALERT_READ_FAILED | 0x4000_0000;
    let time = make_tm(5, 8, 126);
    let c_queue = unsafe {
        call_init(
            &c,
            &time,
            CRALERT_FP_SET | CRALERT_READ_ALL | all_unused,
            Some(&stream_path),
        )
    };
    let rust_queue = unsafe {
        call_init(
            &rust,
            &time,
            CRALERT_FP_SET | CRALERT_READ_ALL | all_unused,
            Some(&stream_path),
        )
    };
    assert_eq!(c_queue, rust_queue);
    assert_eq!(
        c_queue.flags,
        CRALERT_FP_SET | CRALERT_READ_ALL | all_unused
    );

    let alerts_path = temp.path.join("alerts.log");
    let _ = fs::remove_file(&alerts_path);
    let c_missing = unsafe { call_init(&c, &time, 0, None) };
    let rust_missing = unsafe { call_init(&rust, &time, 0, None) };
    assert_eq!(c_missing, rust_missing);
    assert_eq!(c_missing.result, 0);
    assert!(c_missing.fp_is_null);

    fs::write(&alerts_path, &input).unwrap();
    for flags in [
        0,
        CRALERT_READ_ALL,
        all_unused,
        CRALERT_READ_ALL | all_unused,
    ] {
        let c_queue = unsafe { call_init(&c, &time, flags, None) };
        let rust_queue = unsafe { call_init(&rust, &time, flags, None) };
        assert_eq!(c_queue, rust_queue);
        if flags & CRALERT_READ_ALL != 0 {
            assert_eq!(c_queue.position, Some(0));
        } else {
            assert_eq!(c_queue.position, Some(input.len() as i64));
        }
    }

    for timeout in [0, u32::MAX] {
        let c_value = unsafe {
            call_read_with_initialized_stream(
                &c,
                &stream_path,
                &time,
                CRALERT_FP_SET | CRALERT_READ_ALL,
                timeout,
            )
        };
        let rust_value = unsafe {
            call_read_with_initialized_stream(
                &rust,
                &stream_path,
                &time,
                CRALERT_FP_SET | CRALERT_READ_ALL,
                timeout,
            )
        };
        assert_eq!(c_value, rust_value);
        assert!(c_value.is_some());
    }

    let filtered = [
        basic_alert("skip", "log", Some("other"), "Rule: 1 (level 1) 'skip'\n"),
        basic_alert("keep", "mail", Some("mail"), "Rule: 2 (level 2) 'keep'\n"),
    ]
    .concat();
    fs::write(&stream_path, &filtered).unwrap();
    let read_flags = CRALERT_FP_SET | CRALERT_READ_ALL | CRALERT_MAIL_SET;
    let c_value =
        unsafe { call_read_with_initialized_stream(&c, &stream_path, &time, read_flags, 0) };
    let rust_value =
        unsafe { call_read_with_initialized_stream(&rust, &stream_path, &time, read_flags, 0) };
    assert_eq!(c_value, rust_value);

    fs::write(&alerts_path, &input).unwrap();
    let c_driver = unsafe { (c.driver)(5, 8, 126, 0, CRALERT_READ_ALL) };
    let c_value = if c_driver.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(c_driver) };
        unsafe { (c.free_alert_data)(c_driver) };
        Some(snapshot)
    };
    let rust_driver = unsafe { (rust.driver)(5, 8, 126, 0, CRALERT_READ_ALL) };
    let rust_value = if rust_driver.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(rust_driver) };
        unsafe { (rust.free_alert_data)(rust_driver) };
        Some(snapshot)
    };
    assert_eq!(c_value, rust_value);

    fs::write(&alerts_path, &filtered).unwrap();
    let driver_flags = CRALERT_READ_ALL | CRALERT_MAIL_SET | all_unused;
    for month in 0..12 {
        let c_driver = unsafe { (c.driver)(month + 1, month, 100 + month, u32::MAX, driver_flags) };
        let c_value = if c_driver.is_null() {
            None
        } else {
            let snapshot = unsafe { snapshot_alert(c_driver) };
            unsafe { (c.free_alert_data)(c_driver) };
            Some(snapshot)
        };
        let rust_driver =
            unsafe { (rust.driver)(month + 1, month, 100 + month, u32::MAX, driver_flags) };
        let rust_value = if rust_driver.is_null() {
            None
        } else {
            let snapshot = unsafe { snapshot_alert(rust_driver) };
            unsafe { (rust.free_alert_data)(rust_driver) };
            Some(snapshot)
        };
        assert_eq!(c_value, rust_value);
    }

    let retry_input = basic_alert(
        "retry",
        "mail",
        Some("retry"),
        "Rule: 21 (level 7) 'appended during sleep'\n",
    );
    let c_value = unsafe { call_read_after_append_retry(&c, &alerts_path, &time, &retry_input) };
    let rust_value =
        unsafe { call_read_after_append_retry(&rust, &alerts_path, &time, &retry_input) };
    assert_eq!(c_value, rust_value);
    assert!(c_value.is_some());
}

unsafe fn get_from_pipe(api: &Api, input: &[u8], flags: c_int) -> Option<AlertSnapshot> {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let mut written = 0;
    while written < input.len() {
        let count = unsafe {
            libc::write(
                fds[1],
                input[written..].as_ptr().cast(),
                input.len() - written,
            )
        };
        assert!(count > 0);
        written += count as usize;
    }
    unsafe { libc::close(fds[1]) };
    let fp = unsafe { libc::fdopen(fds[0], c"r".as_ptr()) };
    assert!(!fp.is_null());
    let value = unsafe { (api.get_alert_data)(flags, fp) };
    let snapshot = if value.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(value) };
        unsafe { (api.free_alert_data)(value) };
        Some(snapshot)
    };
    unsafe { libc::fclose(fp) };
    snapshot
}

unsafe fn init_with_unseekable_pipe(api: &Api, time: &tm) -> QueueSnapshot {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    unsafe { libc::close(fds[1]) };
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = unsafe { libc::fdopen(fds[0], c"r".as_ptr()) };
    assert!(!queue.fp.is_null());
    let result = unsafe { (api.init_file_queue)(&mut queue, time, CRALERT_FP_SET) };
    unsafe { queue_snapshot(result, &queue) }
}

unsafe fn init_with_fmemopen(api: &Api, time: &tm) -> QueueSnapshot {
    let mut bytes = b"input".to_vec();
    let fp = unsafe { libc::fmemopen(bytes.as_mut_ptr().cast(), bytes.len(), c"r".as_ptr()) };
    assert!(!fp.is_null());
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = fp;
    let result =
        unsafe { (api.init_file_queue)(&mut queue, time, CRALERT_FP_SET | CRALERT_READ_ALL) };
    unsafe { queue_snapshot(result, &queue) }
}

unsafe fn read_timeout_zero_after_reopen(api: &Api, time: &tm) -> Option<AlertSnapshot> {
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = unsafe { libc::tmpfile() };
    assert!(!queue.fp.is_null());
    queue.flags = 0;
    let value = unsafe { (api.read_file_mon)(&mut queue, time, 0) };
    let snapshot = if value.is_null() {
        None
    } else {
        let snapshot = unsafe { snapshot_alert(value) };
        unsafe { (api.free_alert_data)(value) };
        Some(snapshot)
    };
    if !queue.fp.is_null() {
        unsafe { libc::fclose(queue.fp) };
    }
    snapshot
}

#[test]
fn error_surface() {
    let _guard = test_lock().lock().unwrap();
    let (c, rust) = unsafe { apis() };

    for case in [
        "merror_short",
        "merror_long",
        "calloc_fail",
        "realloc_fail",
        "strdup_null",
        "strdup_oom",
    ] {
        let (c_output, _) = assert_child_equal(case);
        if case.ends_with("_fail") || case == "strdup_null" || case == "strdup_oom" {
            assert_eq!(c_output.status.code(), Some(1), "{case}");
        } else {
            assert!(c_output.status.success(), "{case}");
        }
    }

    let second_header = [
        basic_alert("one", "mail", Some("one"), "Rule: 1 (level 1) 'one'\n"),
        basic_alert("two", "mail", Some("two"), "Rule: 2 (level 2) 'two'\n"),
    ]
    .concat();
    let c_value = unsafe { get_from_pipe(&c, &second_header, 0) };
    let rust_value = unsafe { get_from_pipe(&rust, &second_header, 0) };
    assert_eq!(c_value, rust_value);
    assert!(c_value.is_none());

    let malformed = [
        b"** Alert date-space: mail - g\ndate:without-space\n".to_vec(),
        b"** Alert date-colon: mail - g\ndate without colon\n".to_vec(),
        basic_alert("rule-space-1", "mail", Some("g"), "Rule: 123\n"),
        basic_alert("rule-space-2", "mail", Some("g"), "Rule: 123 one\n"),
        basic_alert(
            "rule-open",
            "mail",
            Some("g"),
            "Rule: 1 (level 2) no-quote\n",
        ),
        basic_alert(
            "rule-close",
            "mail",
            Some("g"),
            "Rule: 1 (level 2) 'no-close\n",
        ),
        Vec::new(),
        b"pre-alert only\n".to_vec(),
        b"** Alert header: mail - group\n".to_vec(),
    ];
    for input in malformed {
        let c_value = unsafe { parse_one(&c, &input, 0) };
        let rust_value = unsafe { parse_one(&rust, &input, 0) };
        assert_eq!(
            c_value,
            rust_value,
            "input={}",
            String::from_utf8_lossy(&input)
        );
        assert!(
            c_value.is_none(),
            "expected rejection for input={}",
            String::from_utf8_lossy(&input)
        );
    }

    let time = make_tm(5, 8, 126);
    let mut c_queue: FileQueue = unsafe { mem::zeroed() };
    let mut rust_queue: FileQueue = unsafe { mem::zeroed() };
    let c_result = unsafe { (c.init_file_queue)(&mut c_queue, &time, CRALERT_FP_SET) };
    let rust_result = unsafe { (rust.init_file_queue)(&mut rust_queue, &time, CRALERT_FP_SET) };
    assert_eq!(c_result, rust_result);
    assert_eq!(c_result, 0);
    assert!(c_queue.fp.is_null() && rust_queue.fp.is_null());

    let c_seek = unsafe { init_with_unseekable_pipe(&c, &time) };
    let rust_seek = unsafe { init_with_unseekable_pipe(&rust, &time) };
    assert_eq!(c_seek, rust_seek);
    assert_eq!(c_seek.result, -1);
    assert!(c_seek.fp_is_null);

    let c_stat = unsafe { init_with_fmemopen(&c, &time) };
    let rust_stat = unsafe { init_with_fmemopen(&rust, &time) };
    assert_eq!(c_stat, rust_stat);
    assert_eq!(c_stat.result, -1);
    assert!(c_stat.fp_is_null);

    let temp = TempDir::new("errors");
    let _cwd = CwdGuard::enter(&temp.path);

    for case in ["read_missing", "read_reopen_missing"] {
        let (c_output, _) = assert_child_equal(case);
        assert!(c_output.status.success());
        assert!(String::from_utf8_lossy(&c_output.stdout).contains("true"));
    }

    fs::write("alerts.log", b"").unwrap();
    let (c_output, _) = assert_child_equal("read_post_reopen_null");
    assert!(c_output.status.success());
    assert!(String::from_utf8_lossy(&c_output.stdout).contains("true"));

    let c_value = unsafe { read_timeout_zero_after_reopen(&c, &time) };
    let rust_value = unsafe { read_timeout_zero_after_reopen(&rust, &time) };
    assert_eq!(c_value, rust_value);
    assert!(c_value.is_none());

    fs::remove_file("alerts.log").unwrap();
    fs::create_dir("alerts.log").unwrap();
    let (c_output, _) = assert_child_equal("driver_init_fail");
    assert!(c_output.status.success());
    assert!(String::from_utf8_lossy(&c_output.stdout).contains("true"));

    fs::remove_dir("alerts.log").unwrap();
    fs::write("alerts.log", basic_alert("bounds", "mail", Some("g"), "")).unwrap();

    for case in [
        "null_free",
        "null_get",
        "null_init_fileq",
        "null_init_tm",
        "null_read_fileq",
        "null_read_tm",
        "month_negative",
        "month_twelve",
    ] {
        let _ = assert_child_equal(case);
    }
}
