use libc::{FILE, c_char, c_int, c_uint, c_void, stat, time_t, tm};
use libloading::Library;
use std::ffi::{CStr, CString, OsStr};
use std::fs;
use std::io;
use std::mem::{self, MaybeUninit};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAIL_SET: c_int = 0x001;
const EXEC_SET: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;
const MAX_FQUEUE: usize = 256;
static TEST_LOCK: Mutex<()> = Mutex::new(());

type FreeAlertData = unsafe extern "C" fn(*mut AlertData);
type GetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut AlertData;
type InitFileQueue = unsafe extern "C" fn(*mut FileQueue, *const tm, c_int) -> c_int;
type ReadFileMon = unsafe extern "C" fn(*mut FileQueue, *const tm, c_uint) -> *mut AlertData;
type Driver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;
type Merror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
type OsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type OsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type OsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

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

#[derive(Debug, Eq, PartialEq)]
struct AlertSnapshot {
    rule: c_uint,
    level: c_uint,
    alertid: Option<Vec<u8>>,
    date: Option<Vec<u8>>,
    location: Option<Vec<u8>>,
    comment: Option<Vec<u8>>,
    group: Option<Vec<u8>>,
    srcip: Option<Vec<u8>>,
    srcport: c_int,
    dstip: Option<Vec<u8>>,
    dstport: c_int,
    user: Option<Vec<u8>>,
    filename: Option<Vec<u8>>,
}

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            lib: unsafe { Library::new(path) }.unwrap(),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.lib.get::<T>(name) }.unwrap()
    }

    unsafe fn snapshot_and_free(&self, value: *mut AlertData) -> Option<AlertSnapshot> {
        if value.is_null() {
            return None;
        }
        let alert = unsafe { &*value };
        let snapshot = AlertSnapshot {
            rule: alert.rule,
            level: alert.level,
            alertid: unsafe { c_bytes(alert.alertid) },
            date: unsafe { c_bytes(alert.date) },
            location: unsafe { c_bytes(alert.location) },
            comment: unsafe { c_bytes(alert.comment) },
            group: unsafe { c_bytes(alert.group) },
            srcip: unsafe { c_bytes(alert.srcip) },
            srcport: alert.srcport,
            dstip: unsafe { c_bytes(alert.dstip) },
            dstport: alert.dstport,
            user: unsafe { c_bytes(alert.user) },
            filename: unsafe { c_bytes(alert.filename) },
        };
        let free: FreeAlertData = unsafe { self.symbol(b"FreeAlertData\0") };
        unsafe { free(value) };
        Some(snapshot)
    }
}

struct TempDir {
    old_cwd: PathBuf,
    path: PathBuf,
}

impl TempDir {
    fn enter(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let old_cwd = std::env::current_dir().unwrap();
        let path = std::env::temp_dir().join(format!(
            "driver-diff-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            label
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        std::env::set_current_dir(&path).unwrap();
        Self { old_cwd, path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.old_cwd).unwrap();
        fs::remove_dir_all(&self.path).unwrap();
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, upper: u32) -> u32 {
        (self.next() % u64::from(upper)) as u32
    }

    fn ascii(&mut self, min: usize, extra: usize) -> String {
        let len = min + self.range((extra + 1) as u32) as usize;
        (0..len)
            .map(|_| (b'a' + self.range(26) as u8) as char)
            .collect()
    }
}

fn c_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .unwrap()
}

fn rust_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .unwrap()
}

unsafe fn c_bytes(value: *const c_char) -> Option<Vec<u8>> {
    (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
}

unsafe fn stream(data: &[u8]) -> *mut FILE {
    let fp = unsafe { libc::tmpfile() };
    assert!(!fp.is_null());
    if !data.is_empty() {
        assert_eq!(
            unsafe { libc::fwrite(data.as_ptr().cast(), 1, data.len(), fp) },
            data.len()
        );
    }
    unsafe { libc::rewind(fp) };
    fp
}

unsafe fn parse(api: &Api, data: &[u8], flag: c_int) -> Option<AlertSnapshot> {
    let fp = unsafe { stream(data) };
    let get: GetAlertData = unsafe { api.symbol(b"GetAlertData\0") };
    let out = unsafe { api.snapshot_and_free(get(flag, fp)) };
    unsafe { libc::fclose(fp) };
    out
}

unsafe fn parse_twice(
    api: &Api,
    data: &[u8],
    flag: c_int,
) -> (Option<AlertSnapshot>, Option<AlertSnapshot>) {
    let fp = unsafe { stream(data) };
    let get: GetAlertData = unsafe { api.symbol(b"GetAlertData\0") };
    let first = unsafe { api.snapshot_and_free(get(flag, fp)) };
    let second = unsafe { api.snapshot_and_free(get(flag, fp)) };
    unsafe { libc::fclose(fp) };
    (first, second)
}

fn compare_parse(c: &Api, rust: &Api, data: &[u8], flag: c_int) {
    assert_eq!(
        unsafe { parse(c, data, flag) },
        unsafe { parse(rust, data, flag) },
        "parser mismatch for flag {flag:#x} and input {:?}",
        String::from_utf8_lossy(data)
    );
}

fn base_alert(id: &str, mode: &str, group: &str, date: &str, location: &str) -> String {
    format!("** Alert {id}: {mode} - {group}\n{date}:00 {location}\n")
}

unsafe fn queue_bytes(queue: &FileQueue) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(
            (queue as *const FileQueue).cast::<u8>(),
            mem::size_of::<FileQueue>(),
        )
        .to_vec()
    }
}

unsafe fn init_queue(
    api: &Api,
    initial_fp: *mut FILE,
    time: &tm,
    flags: c_int,
) -> (c_int, FileQueue) {
    let mut queue: FileQueue = unsafe { mem::zeroed() };
    queue.fp = initial_fp;
    let init: InitFileQueue = unsafe { api.symbol(b"Init_FileQueue\0") };
    let result = unsafe { init(&mut queue, time, flags) };
    (result, queue)
}

unsafe fn close_queue(queue: &mut FileQueue) {
    if !queue.fp.is_null() {
        unsafe { libc::fclose(queue.fp) };
        queue.fp = ptr::null_mut();
    }
}

fn capture_stderr(call: impl FnOnce()) -> Vec<u8> {
    unsafe {
        let mut fds = [0; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let saved = libc::dup(libc::STDERR_FILENO);
        assert!(saved >= 0);
        assert_eq!(libc::dup2(fds[1], libc::STDERR_FILENO), libc::STDERR_FILENO);
        libc::close(fds[1]);
        call();
        libc::fflush(ptr::null_mut());
        assert_eq!(libc::dup2(saved, libc::STDERR_FILENO), libc::STDERR_FILENO);
        libc::close(saved);

        let mut bytes = Vec::new();
        let mut buf = [0_u8; 512];
        loop {
            let count = libc::read(fds[0], buf.as_mut_ptr().cast(), buf.len());
            if count <= 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..count as usize]);
        }
        libc::close(fds[0]);
        bytes
    }
}

fn compare_allocators(c: &Api, rust: &Api) {
    let mut rng = Rng::new(0x7f4a_7c15_d1ff_a55a);
    for _ in 0..64 {
        let count = 1 + rng.range(64) as usize;
        let size = 1 + rng.range(16) as usize;
        for api in [c, rust] {
            unsafe {
                let calloc: OsCalloc = api.symbol(b"os_calloc\0");
                let ptr = calloc(count, size);
                assert!(!ptr.is_null());
                assert!(
                    std::slice::from_raw_parts(ptr.cast::<u8>(), count * size)
                        .iter()
                        .all(|byte| *byte == 0)
                );
                libc::free(ptr);
            }
        }
    }

    for (count, size) in [(0, 7), (7, 0), (0, 0)] {
        for api in [c, rust] {
            unsafe {
                let calloc: OsCalloc = api.symbol(b"os_calloc\0");
                let ptr = calloc(count, size);
                assert!(!ptr.is_null());
                libc::free(ptr);
            }
        }
    }

    for api in [c, rust] {
        unsafe {
            let realloc: OsRealloc = api.symbol(b"os_realloc\0");
            let mut ptr = realloc(ptr::null_mut(), 64);
            assert!(!ptr.is_null());
            for index in 0..64 {
                *ptr.cast::<u8>().add(index) = index as u8;
            }
            for size in [32, 32, 256] {
                ptr = realloc(ptr, size);
                assert!(!ptr.is_null());
                let preserved = size.min(32);
                assert_eq!(
                    std::slice::from_raw_parts(ptr.cast::<u8>(), preserved),
                    &(0..preserved as u8).collect::<Vec<_>>()
                );
            }
            libc::free(ptr);
        }
    }

    for _ in 0..64 {
        let text = if rng.range(4) == 0 {
            String::new()
        } else {
            rng.ascii(1, 128)
        };
        let text = CString::new(text).unwrap();
        for api in [c, rust] {
            unsafe {
                let strdup: OsStrdup = api.symbol(b"os_strdup\0");
                let out = strdup(text.as_ptr());
                assert_eq!(CStr::from_ptr(out).to_bytes(), text.as_bytes());
                libc::free(out.cast());
            }
        }
    }
}

fn compare_merror(c: &Api, rust: &Api) {
    let cases = [
        ("error %s (%d): %s", "file.log", 17, "bad input"),
        (
            "error %s (%d): %s",
            &"f".repeat(220),
            -2,
            &"message".repeat(20),
        ),
    ];
    for (template, file, code, message) in cases {
        let template = CString::new(template).unwrap();
        let file = CString::new(file).unwrap();
        let message = CString::new(message).unwrap();
        let invoke = |api: &Api| {
            capture_stderr(|| unsafe {
                let merror: Merror = api.symbol(b"merror\0");
                merror(template.as_ptr(), file.as_ptr(), code, message.as_ptr());
            })
        };
        assert_eq!(invoke(c), invoke(rust));
    }
}

fn compare_free(c: &Api, rust: &Api) {
    for api in [c, rust] {
        unsafe {
            let free: FreeAlertData = api.symbol(b"FreeAlertData\0");
            let empty = libc::calloc(1, mem::size_of::<AlertData>()).cast::<AlertData>();
            free(empty);

            let full = libc::calloc(1, mem::size_of::<AlertData>()).cast::<AlertData>();
            let values = [
                "",
                "date",
                "location",
                "comment",
                "group",
                "192.0.2.1",
                "198.51.100.2",
                "user",
                "/tmp/file",
            ];
            let fields = [
                &mut (*full).alertid,
                &mut (*full).date,
                &mut (*full).location,
                &mut (*full).comment,
                &mut (*full).group,
                &mut (*full).srcip,
                &mut (*full).dstip,
                &mut (*full).user,
                &mut (*full).filename,
            ];
            for (field, value) in fields.into_iter().zip(values) {
                *field = libc::strdup(CString::new(value).unwrap().as_ptr());
            }
            free(full);
        }
    }
}

fn compare_parser(c: &Api, rust: &Api) {
    for data in [
        b"".as_slice(),
        b"preamble\nnoise\n",
        b"** Alert malformed\nnoise\n",
        b"** Alert id:no-space\nnoise\n",
    ] {
        compare_parse(c, rust, data, 0);
    }

    let two = concat!(
        "** Alert 1: mail - group-one\n",
        "2026 Aug 27 05:00:00 host-a\n",
        "Rule: 10 (level 4) 'first'\n",
        "** Alert 2: exec - group-two\n",
        "2026 Aug 27 06:00:00 host-b\n",
        "Rule: 11 (level 5) 'second'\n"
    );
    assert_eq!(unsafe { parse_twice(c, two.as_bytes(), 0) }, unsafe {
        parse_twice(rust, two.as_bytes(), 0)
    });

    let mail = base_alert("3", "mail", "group", "2026 Aug 27 07:00", "host");
    let exec = base_alert("4", "exec", "group", "2026 Aug 27 08:00", "host");
    compare_parse(c, rust, mail.as_bytes(), 0);
    compare_parse(c, rust, exec.as_bytes(), 0);
    compare_parse(c, rust, mail.as_bytes(), MAIL_SET);
    compare_parse(c, rust, exec.as_bytes(), MAIL_SET);

    for marker in [
        "** Alert 5: mail\n2026 Aug 27 09:00:00 host\n",
        "** Alert 5: mail - ordinary\n2026 Aug 27 09:00:00 host\n",
        "** Alert 5: mail -     spaced\n2026 Aug 27 09:00:00 host\n",
    ] {
        compare_parse(c, rust, marker.as_bytes(), 0);
    }

    for body in [
        "Rule: 0 (level 0) 'zero'\n",
        "Rule: 42 (level 15) 'positive comment'\n",
        "Rule: -3 (level -2) 'negative'\n",
    ] {
        let data = format!("{mail}{body}");
        compare_parse(c, rust, data.as_bytes(), 0);
    }

    let metadata = format!(
        "{mail}\
         Src IP: 192.0.2.1\n\
         Src Port: 12345\n\
         Dst IP: 198.51.100.9\n\
         Dst Port: 443\n\
         User: first\n\
         Src IP: 203.0.113.8\n\
         User: second\n"
    );
    compare_parse(c, rust, metadata.as_bytes(), 0);

    let replaced_group = concat!(
        "** Alert 40: mail - first-group\n",
        "** Alert 41: mail - second-group\n",
        "2026 Aug 27 10:30:00 host\n"
    );
    compare_parse(c, rust, replaced_group.as_bytes(), 0);

    for body in [
        "Integrity checksum changed for: '/var/log/auth.log'\n",
        "ordinary log line\nIntegrity checksum changed for: '/ignored'\n",
        "Rule: 7 (level 3) 'rule'\nIntegrity checksum changed for: '/etc/passwd'\n",
    ] {
        let data = format!(
            "{}{body}",
            base_alert("6", "mail", "audit,syscheck,", "2026 Aug 27 10:00", "host")
        );
        compare_parse(c, rust, data.as_bytes(), 0);
    }

    for length in [1022, 1023, 1024, 1025, 2048] {
        let data = format!("{mail}{}\n", "x".repeat(length));
        compare_parse(c, rust, data.as_bytes(), 0);
    }

    for flags in [EXEC_SET, READ_FAILED, 0x4000_0000, -1] {
        compare_parse(c, rust, mail.as_bytes(), flags);
    }

    let mut rng = Rng::new(0xd6e8_feb8_6659_fd93);
    for _ in 0..128 {
        let id = (1 + rng.range(100_000)).to_string();
        let group = rng.ascii(1, 40);
        let location = rng.ascii(1, 80);
        let comment = rng.ascii(0, 120);
        let src = format!(
            "{}Rule: {} (level {}) '{}'\nSrc Port: {}\nDst Port: {}\nUser: {}\n",
            base_alert(
                &id,
                if rng.range(2) == 0 { "mail" } else { "exec" },
                &group,
                "2026 Aug 27 11:22",
                &location,
            ),
            rng.range(100_000),
            rng.range(20),
            comment,
            rng.range(65_536),
            rng.range(65_536),
            rng.ascii(0, 32)
        );
        compare_parse(c, rust, src.as_bytes(), 0);
    }
}

fn sample_tm(month: c_int) -> tm {
    let mut value: tm = unsafe { mem::zeroed() };
    value.tm_mday = 1 + month;
    value.tm_mon = month;
    value.tm_year = 126;
    value
}

fn compare_init(c: &Api, rust: &Api) {
    let _temp = TempDir::enter("init");
    for month in 0..12 {
        let time = sample_tm(month);
        for flags in [0, READ_ALL, EXEC_SET | READ_FAILED, READ_ALL | 0x4000_0000] {
            let (c_result, mut cq) = unsafe { init_queue(c, ptr::null_mut(), &time, flags) };
            let (r_result, mut rq) = unsafe { init_queue(rust, ptr::null_mut(), &time, flags) };
            assert_eq!(c_result, r_result);
            assert_eq!(unsafe { queue_bytes(&cq) }, unsafe { queue_bytes(&rq) });
            unsafe {
                close_queue(&mut cq);
                close_queue(&mut rq);
            }
        }
    }

    fs::write("alerts.log", b"prefix-data").unwrap();
    for flags in [0, READ_ALL] {
        let time = sample_tm(7);
        let (c_result, mut cq) = unsafe { init_queue(c, ptr::null_mut(), &time, flags) };
        let (r_result, mut rq) = unsafe { init_queue(rust, ptr::null_mut(), &time, flags) };
        assert_eq!(c_result, r_result);
        assert_eq!(unsafe { libc::ftell(cq.fp) }, unsafe { libc::ftell(rq.fp) });
        let cfp = cq.fp;
        let rfp = rq.fp;
        cq.fp = ptr::null_mut();
        rq.fp = ptr::null_mut();
        assert_eq!(unsafe { queue_bytes(&cq) }, unsafe { queue_bytes(&rq) });
        unsafe {
            libc::fclose(cfp);
            libc::fclose(rfp);
        }
    }

    fs::write("supplied.log", b"abcdef").unwrap();
    let supplied = c"supplied.log";
    let read_mode = c"r";
    for flags in [FP_SET, FP_SET | READ_ALL, FP_SET | EXEC_SET | 0x2000_0000] {
        let time = sample_tm(3);
        let cfp = unsafe { libc::fopen(supplied.as_ptr(), read_mode.as_ptr()) };
        let rfp = unsafe { libc::fopen(supplied.as_ptr(), read_mode.as_ptr()) };
        assert!(!cfp.is_null() && !rfp.is_null());
        let (c_result, mut cq) = unsafe { init_queue(c, cfp, &time, flags) };
        let (r_result, mut rq) = unsafe { init_queue(rust, rfp, &time, flags) };
        assert_eq!(c_result, r_result);
        assert_eq!(unsafe { libc::ftell(cq.fp) }, unsafe { libc::ftell(rq.fp) });
        cq.fp = ptr::null_mut();
        rq.fp = ptr::null_mut();
        assert_eq!(unsafe { queue_bytes(&cq) }, unsafe { queue_bytes(&rq) });
        unsafe {
            libc::fclose(cfp);
            libc::fclose(rfp);
        }
    }
}

fn compare_read_file_mon(c: &Api, rust: &Api) {
    let _temp = TempDir::enter("read");
    let data = base_alert("20", "mail", "group", "2026 Aug 27 12:00", "host");
    let time = sample_tm(7);
    for api in [c, rust] {
        unsafe {
            let fp = stream(data.as_bytes());
            let (_, mut queue) = init_queue(api, fp, &time, FP_SET | READ_ALL);
            let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
            let result = api.snapshot_and_free(read(&mut queue, &time, 0));
            assert!(result.is_some());
            close_queue(&mut queue);
        }
    }
    let c_direct = unsafe {
        let fp = stream(data.as_bytes());
        let (_, mut queue) = init_queue(c, fp, &time, FP_SET | READ_ALL);
        let read: ReadFileMon = c.symbol(b"Read_FileMon\0");
        let out = c.snapshot_and_free(read(&mut queue, &time, 0));
        close_queue(&mut queue);
        out
    };
    let r_direct = unsafe {
        let fp = stream(data.as_bytes());
        let (_, mut queue) = init_queue(rust, fp, &time, FP_SET | READ_ALL);
        let read: ReadFileMon = rust.symbol(b"Read_FileMon\0");
        let out = rust.snapshot_and_free(read(&mut queue, &time, 0));
        close_queue(&mut queue);
        out
    };
    assert_eq!(c_direct, r_direct);

    fs::write("alerts.log", data.as_bytes()).unwrap();
    for api in [c, rust] {
        unsafe {
            let fp = stream(b"");
            let (_, mut queue) = init_queue(api, fp, &time, FP_SET | READ_ALL);
            let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
            assert!(read(&mut queue, &time, 0).is_null());
            close_queue(&mut queue);
        }
    }

    for api in [c, rust] {
        unsafe {
            let mut queue: FileQueue = mem::zeroed();
            let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
            assert!(read(&mut queue, &time, 0).is_null());
            close_queue(&mut queue);
        }
    }
}

fn run_driver(api: &Api, flags: c_int) -> Option<AlertSnapshot> {
    unsafe {
        let driver: Driver = api.symbol(b"driver\0");
        api.snapshot_and_free(driver(27, 7, 126, 0, flags))
    }
}

fn compare_driver(c: &Api, rust: &Api) {
    let _temp = TempDir::enter("driver");
    let mail = format!(
        "{}Rule: 501 (level 9) 'driver comment'\nSrc IP: 192.0.2.7\n",
        base_alert("30", "mail", "driver", "2026 Aug 27 13:00", "host")
    );
    let exec = base_alert("31", "exec", "driver", "2026 Aug 27 14:00", "host");

    for (contents, flags) in [
        (mail.as_bytes(), READ_ALL),
        (mail.as_bytes(), READ_ALL | MAIL_SET),
        (exec.as_bytes(), READ_ALL | MAIL_SET),
        (mail.as_bytes(), 0),
        (mail.as_bytes(), FP_SET),
        (mail.as_bytes(), FP_SET | READ_ALL),
        (
            mail.as_bytes(),
            READ_ALL | EXEC_SET | READ_FAILED | 0x4000_0000,
        ),
    ] {
        fs::write("alerts.log", contents).unwrap();
        assert_eq!(
            run_driver(c, flags),
            run_driver(rust, flags),
            "flags={flags:#x}"
        );
    }
}

#[test]
fn valid_configuration_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    let c = unsafe { Api::open(&c_path()) };
    let rust = unsafe { Api::open(&rust_path()) };
    compare_allocators(&c, &rust);
    compare_merror(&c, &rust);
    compare_free(&c, &rust);
    compare_parser(&c, &rust);
    compare_init(&c, &rust);
    compare_read_file_mon(&c, &rust);
    compare_driver(&c, &rust);
}

fn child_output(library: &Path, case: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("child_error_case")
        .arg("--nocapture")
        .env("DRIVER_DIFF_CHILD_LIB", library)
        .env("DRIVER_DIFF_CHILD_CASE", case)
        .output()
        .unwrap()
}

fn assert_same_child(case: &str) {
    let c = child_output(&c_path(), case);
    let rust = child_output(&rust_path(), case);
    assert_eq!(
        (c.status.code(), c.status.signal()),
        (rust.status.code(), rust.status.signal()),
        "termination mismatch for {case}\nC stderr: {}\nRust stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(c.stderr, rust.stderr, "stderr mismatch for {case}");
}

#[test]
fn error_surface_matches() {
    if std::env::var_os("DRIVER_DIFF_CHILD_CASE").is_some() {
        return;
    }
    let _guard = TEST_LOCK.lock().unwrap();
    for case in [
        "calloc_oom",
        "realloc_oom",
        "strdup_null",
        "strdup_oom",
        "get_second_marker_unseekable",
        "get_date_colon_no_space",
        "get_date_no_colon",
        "get_rule_missing_spaces",
        "get_rule_missing_open_quote",
        "get_rule_missing_close_quote",
        "get_incomplete_eof",
        "init_seek_failure",
        "init_fstat_failure",
        "read_missing_initial",
        "read_missing_refresh",
        "read_timeout_zero",
        "driver_init_failure",
        "free_null",
        "get_null_fp",
        "init_null_queue",
        "init_null_time",
        "read_null_queue",
        "read_null_time",
        "merror_null_template",
    ] {
        assert_same_child(case);
    }
}

unsafe fn pipe_stream(data: &[u8]) -> *mut FILE {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    if !data.is_empty() {
        assert_eq!(
            unsafe { libc::write(fds[1], data.as_ptr().cast(), data.len()) },
            data.len() as isize
        );
    }
    unsafe { libc::close(fds[1]) };
    let mode = c"r";
    let fp = unsafe { libc::fdopen(fds[0], mode.as_ptr()) };
    assert!(!fp.is_null());
    fp
}

fn set_address_space_to_current() {
    let statm = fs::read_to_string("/proc/self/statm").unwrap();
    let pages: u64 = statm.split_whitespace().next().unwrap().parse().unwrap();
    let bytes = pages * unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 };
    let limit = libc::rlimit {
        rlim_cur: bytes,
        rlim_max: bytes,
    };
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) }, 0);
}

fn child_case(api: &Api, case: &str) {
    unsafe {
        match case {
            "calloc_oom" => {
                let call: OsCalloc = api.symbol(b"os_calloc\0");
                call(usize::MAX, 2);
            }
            "realloc_oom" => {
                let call: OsRealloc = api.symbol(b"os_realloc\0");
                call(ptr::null_mut(), usize::MAX);
            }
            "strdup_null" => {
                let call: OsStrdup = api.symbol(b"os_strdup\0");
                call(ptr::null());
            }
            "strdup_oom" => {
                let source = vec![b'x'; 32 * 1024 * 1024];
                let source = CString::new(source).unwrap();
                set_address_space_to_current();
                let call: OsStrdup = api.symbol(b"os_strdup\0");
                call(source.as_ptr());
            }
            "get_second_marker_unseekable" => {
                let fp = pipe_stream(
                    concat!(
                        "** Alert 1: mail - group\n",
                        "2026 Aug 27 01:00:00 host\n",
                        "** Alert 2: mail - group\n"
                    )
                    .as_bytes(),
                );
                let get: GetAlertData = api.symbol(b"GetAlertData\0");
                assert!(get(0, fp).is_null());
                libc::fclose(fp);
            }
            "get_date_colon_no_space" => {
                assert!(parse(api, b"** Alert 1: mail - g\nbad:date\n", 0).is_none());
            }
            "get_date_no_colon" => {
                assert!(parse(api, b"** Alert 1: mail - g\nbad date\n", 0).is_none());
            }
            "get_rule_missing_spaces" => {
                assert!(
                    parse(api, b"** Alert 1: mail - g\n2026:00 host\nRule: 1 bad\n", 0,).is_none()
                );
            }
            "get_rule_missing_open_quote" => {
                assert!(
                    parse(
                        api,
                        b"** Alert 1: mail - g\n2026:00 host\nRule: 1 (level 2) noquote\n",
                        0,
                    )
                    .is_none()
                );
            }
            "get_rule_missing_close_quote" => {
                assert!(
                    parse(
                        api,
                        b"** Alert 1: mail - g\n2026:00 host\nRule: 1 (level 2) 'open\n",
                        0,
                    )
                    .is_none()
                );
            }
            "get_incomplete_eof" => {
                assert!(parse(api, b"preamble\n** Alert 1: mail - g\n", 0).is_none());
            }
            "init_seek_failure" => {
                let fp = pipe_stream(b"data");
                let time = sample_tm(0);
                let (result, queue) = init_queue(api, fp, &time, FP_SET);
                assert_eq!(result, -1);
                assert!(queue.fp.is_null());
            }
            "init_fstat_failure" => {
                let fp = stream(b"data");
                libc::close(libc::fileno(fp));
                let time = sample_tm(0);
                let (result, queue) = init_queue(api, fp, &time, FP_SET | READ_ALL);
                assert_eq!(result, -1);
                assert!(queue.fp.is_null());
            }
            "read_missing_initial" => {
                let _temp = TempDir::enter("child-read-initial");
                let mut queue: FileQueue = mem::zeroed();
                let time = sample_tm(0);
                let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
                assert!(read(&mut queue, &time, 0).is_null());
            }
            "read_missing_refresh" => {
                let _temp = TempDir::enter("child-read-refresh");
                let fp = stream(b"");
                let time = sample_tm(0);
                let (_, mut queue) = init_queue(api, fp, &time, FP_SET | READ_ALL);
                let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
                assert!(read(&mut queue, &time, 0).is_null());
            }
            "read_timeout_zero" => {
                let _temp = TempDir::enter("child-timeout");
                fs::write("alerts.log", b"").unwrap();
                let time = sample_tm(0);
                let (result, mut queue) = init_queue(api, ptr::null_mut(), &time, READ_ALL);
                assert_eq!(result, 0);
                let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
                assert!(read(&mut queue, &time, 0).is_null());
                close_queue(&mut queue);
            }
            "driver_init_failure" => {
                let _temp = TempDir::enter("child-driver");
                fs::create_dir("alerts.log").unwrap();
                let driver: Driver = api.symbol(b"driver\0");
                assert!(driver(1, 0, 126, 0, 0).is_null());
            }
            "free_null" => {
                let free: FreeAlertData = api.symbol(b"FreeAlertData\0");
                free(ptr::null_mut());
            }
            "get_null_fp" => {
                let get: GetAlertData = api.symbol(b"GetAlertData\0");
                get(0, ptr::null_mut());
            }
            "init_null_queue" => {
                let init: InitFileQueue = api.symbol(b"Init_FileQueue\0");
                init(ptr::null_mut(), &sample_tm(0), 0);
            }
            "init_null_time" => {
                let mut queue: FileQueue = mem::zeroed();
                let init: InitFileQueue = api.symbol(b"Init_FileQueue\0");
                init(&mut queue, ptr::null(), 0);
            }
            "read_null_queue" => {
                let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
                read(ptr::null_mut(), &sample_tm(0), 0);
            }
            "read_null_time" => {
                let fp = stream(b"");
                let mut queue: FileQueue = mem::zeroed();
                queue.fp = fp;
                queue.flags = FP_SET | READ_ALL;
                let read: ReadFileMon = api.symbol(b"Read_FileMon\0");
                read(&mut queue, ptr::null(), 0);
            }
            "merror_null_template" => {
                let merror: Merror = api.symbol(b"merror\0");
                merror(ptr::null(), c"file".as_ptr(), 1, c"message".as_ptr());
            }
            _ => panic!("unknown child case {case}"),
        }
    }
}

#[test]
fn child_error_case() {
    let Some(case) = std::env::var_os("DRIVER_DIFF_CHILD_CASE") else {
        return;
    };
    let library = std::env::var_os("DRIVER_DIFF_CHILD_LIB").unwrap();
    let api = unsafe { Api::open(Path::new(&library)) };
    child_case(&api, OsStr::from_bytes(case.as_bytes()).to_str().unwrap());
}

#[test]
fn ffi_layout_matches_reference_headers() {
    assert_eq!(mem::size_of::<AlertData>(), 96);
    assert_eq!(mem::size_of::<FileQueue>(), 440);
    assert_eq!(mem::offset_of!(FileQueue, fp), 288);
    assert_eq!(mem::offset_of!(FileQueue, f_status), 296);
    let _: MaybeUninit<FileQueue> = MaybeUninit::uninit();
    let _: io::Result<()> = Ok(());
}
