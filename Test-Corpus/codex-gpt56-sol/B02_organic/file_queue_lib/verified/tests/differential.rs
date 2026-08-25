use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_long, c_uint, c_void};
use std::fs;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const MAIL_SET: c_int = 0x001;
const EXEC_SET: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;
const MAX_FQUEUE: usize = 256;
static CWD_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
struct CFile {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Stat {
    bytes: [u8; 144],
}

#[repr(C)]
struct FileQueue {
    last_change: i64,
    year: c_int,
    day: c_int,
    flags: c_int,
    mon: [c_char; 4],
    file_name: [c_char; MAX_FQUEUE + 1],
    fp: *mut CFile,
    f_status: Stat,
}

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

type OsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
type OsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type OsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type Merror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
type FreeAlertData = unsafe extern "C" fn(*mut AlertData);
type GetAlertData = unsafe extern "C" fn(c_int, *mut CFile) -> *mut AlertData;
type InitFileQueue = unsafe extern "C" fn(*mut FileQueue, *const Tm, c_int) -> c_int;
type ReadFileMon = unsafe extern "C" fn(*mut FileQueue, *const Tm, c_uint) -> *mut AlertData;
type Driver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;

struct Api {
    _library: Library,
    os_calloc: OsCalloc,
    os_realloc: OsRealloc,
    os_strdup: OsStrdup,
    merror: Merror,
    free_alert_data: FreeAlertData,
    get_alert_data: GetAlertData,
    init_file_queue: InitFileQueue,
    read_file_mon: ReadFileMon,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap_or_else(|error| {
            panic!("failed to load {}: {error}", path.display());
        });
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        Self {
            os_calloc: symbol!("os_calloc", OsCalloc),
            os_realloc: symbol!("os_realloc", OsRealloc),
            os_strdup: symbol!("os_strdup", OsStrdup),
            merror: symbol!("merror", Merror),
            free_alert_data: symbol!("FreeAlertData", FreeAlertData),
            get_alert_data: symbol!("GetAlertData", GetAlertData),
            init_file_queue: symbol!("Init_FileQueue", InitFileQueue),
            read_file_mon: symbol!("Read_FileMon", ReadFileMon),
            driver: symbol!("driver", Driver),
            _library: library,
        }
    }
}

unsafe extern "C" {
    fn tmpfile() -> *mut CFile;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut CFile;
    fn fdopen(fd: c_int, mode: *const c_char) -> *mut CFile;
    fn fmemopen(buffer: *mut c_void, size: usize, mode: *const c_char) -> *mut CFile;
    fn fwrite(data: *const c_void, size: usize, count: usize, fp: *mut CFile) -> usize;
    fn fflush(fp: *mut CFile) -> c_int;
    fn rewind(fp: *mut CFile);
    fn fclose(fp: *mut CFile) -> c_int;
    fn ftell(fp: *mut CFile) -> c_long;
    fn free(ptr: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn write(fd: c_int, data: *const c_void, count: usize) -> isize;
    fn setrlimit(resource: c_int, limit: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

#[derive(Debug, PartialEq, Eq)]
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

unsafe fn optional_bytes(value: *const c_char) -> Option<Vec<u8>> {
    (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
}

unsafe fn snapshot_alert(value: *mut AlertData) -> Option<AlertSnapshot> {
    if value.is_null() {
        return None;
    }
    let value = unsafe { &*value };
    Some(AlertSnapshot {
        rule: value.rule,
        level: value.level,
        alertid: unsafe { optional_bytes(value.alertid) },
        date: unsafe { optional_bytes(value.date) },
        location: unsafe { optional_bytes(value.location) },
        comment: unsafe { optional_bytes(value.comment) },
        group: unsafe { optional_bytes(value.group) },
        srcip: unsafe { optional_bytes(value.srcip) },
        srcport: value.srcport,
        dstip: unsafe { optional_bytes(value.dstip) },
        dstport: value.dstport,
        user: unsafe { optional_bytes(value.user) },
        filename: unsafe { optional_bytes(value.filename) },
    })
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn library_path(kind: &str) -> PathBuf {
    match kind {
        "c" => manifest_dir().join("c_src/build/libdriver.so"),
        "rust" => manifest_dir().join("target/debug/libdriver.so"),
        _ => panic!("unknown library kind {kind}"),
    }
}

unsafe fn stream(contents: &[u8]) -> *mut CFile {
    let fp = unsafe { tmpfile() };
    assert!(!fp.is_null());
    if !contents.is_empty() {
        assert_eq!(
            unsafe { fwrite(contents.as_ptr().cast(), 1, contents.len(), fp) },
            contents.len()
        );
    }
    assert_eq!(unsafe { fflush(fp) }, 0);
    unsafe { rewind(fp) };
    fp
}

unsafe fn parse(api: &Api, flags: c_int, contents: &[u8]) -> Option<AlertSnapshot> {
    let fp = unsafe { stream(contents) };
    let value = unsafe { (api.get_alert_data)(flags, fp) };
    let snapshot = unsafe { snapshot_alert(value) };
    if !value.is_null() {
        unsafe { (api.free_alert_data)(value) };
    }
    assert_eq!(unsafe { fclose(fp) }, 0);
    snapshot
}

fn compare_parse(c: &Api, rust: &Api, flags: c_int, contents: &[u8], label: &str) {
    let left = unsafe { parse(c, flags, contents) };
    let right = unsafe { parse(rust, flags, contents) };
    assert_eq!(
        left,
        right,
        "{label}; input={}",
        String::from_utf8_lossy(contents)
    );
}

fn complete_alert(id: &str, mode: &str, group: &str, location: &str, body: &str) -> Vec<u8> {
    format!("** Alert {id}: {mode} - {group}\n2026 Aug 25 01:02:03: {location}\n{body}")
        .into_bytes()
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, upper: u32) -> u32 {
        self.next() % upper
    }

    fn text(&mut self, min: usize, extra: usize) -> String {
        let len = min + self.range(extra as u32 + 1) as usize;
        (0..len)
            .map(|_| (b'a' + self.range(26) as u8) as char)
            .collect()
    }
}

fn assert_layouts() {
    assert_eq!(size_of::<Tm>(), 56);
    assert_eq!(size_of::<Stat>(), 144);
    assert_eq!(size_of::<FileQueue>(), 440);
    assert_eq!(size_of::<AlertData>(), 96);
}

unsafe fn allocation_matrix(api: &Api) -> Vec<Vec<u8>> {
    let mut observations = Vec::new();

    // C01: zero initialization for scalar and array allocations.
    for (count, width) in [(1, 1), (1, 37), (17, 9), (128, 4)] {
        let len = count * width;
        let value = unsafe { (api.os_calloc)(count, width) };
        assert!(!value.is_null());
        let bytes = unsafe { std::slice::from_raw_parts(value.cast::<u8>(), len) }.to_vec();
        observations.push(bytes);
        unsafe { free(value) };
    }

    // C02: NULL allocation followed by growth and shrinkage.
    let mut value = unsafe { (api.os_realloc)(ptr::null_mut(), 16) };
    assert!(!value.is_null());
    for index in 0..16 {
        unsafe { *value.cast::<u8>().add(index) = index as u8 };
    }
    value = unsafe { (api.os_realloc)(value, 64) };
    observations.push(unsafe { std::slice::from_raw_parts(value.cast::<u8>(), 16) }.to_vec());
    value = unsafe { (api.os_realloc)(value, 8) };
    observations.push(unsafe { std::slice::from_raw_parts(value.cast::<u8>(), 8) }.to_vec());
    unsafe { free(value) };

    // C03: empty and populated strings.
    for input in [b"\0".as_slice(), b"ffi bytes 0123456789\0".as_slice()] {
        let duplicate = unsafe { (api.os_strdup)(input.as_ptr().cast()) };
        assert!(!duplicate.is_null());
        observations.push(unsafe { CStr::from_ptr(duplicate) }.to_bytes().to_vec());
        unsafe { free(duplicate.cast()) };
    }

    // C05: sparse and fully populated ownership graphs.
    let sparse = unsafe { (api.os_calloc)(1, size_of::<AlertData>()).cast::<AlertData>() };
    unsafe { (api.free_alert_data)(sparse) };

    let full = unsafe { (api.os_calloc)(1, size_of::<AlertData>()).cast::<AlertData>() };
    let fields = [
        &mut unsafe { &mut *full }.alertid,
        &mut unsafe { &mut *full }.date,
        &mut unsafe { &mut *full }.location,
        &mut unsafe { &mut *full }.comment,
        &mut unsafe { &mut *full }.group,
        &mut unsafe { &mut *full }.srcip,
        &mut unsafe { &mut *full }.dstip,
        &mut unsafe { &mut *full }.user,
        &mut unsafe { &mut *full }.filename,
    ];
    for (index, field) in fields.into_iter().enumerate() {
        let text = CString::new(format!("owned-{index}")).unwrap();
        *field = unsafe { (api.os_strdup)(text.as_ptr()) };
    }
    unsafe { (api.free_alert_data)(full) };
    observations
}

unsafe fn parse_sequence(
    api: &Api,
    flags: c_int,
    contents: &[u8],
    count: usize,
) -> Vec<Option<AlertSnapshot>> {
    let fp = unsafe { stream(contents) };
    let mut output = Vec::new();
    for _ in 0..count {
        let value = unsafe { (api.get_alert_data)(flags, fp) };
        output.push(unsafe { snapshot_alert(value) });
        if !value.is_null() {
            unsafe { (api.free_alert_data)(value) };
        }
    }
    assert_eq!(unsafe { fclose(fp) }, 0);
    output
}

#[test]
fn valid_allocation_and_parser_matrix() {
    assert_layouts();
    let c = unsafe { Api::load(&library_path("c")) };
    let rust = unsafe { Api::load(&library_path("rust")) };

    assert_eq!(
        unsafe { allocation_matrix(&c) },
        unsafe { allocation_matrix(&rust) },
        "C01-C03/C05 allocation observations"
    );

    // C06: empty input and arbitrary prelude.
    for input in [b"".as_slice(), b"noise\nmore noise\n".as_slice()] {
        compare_parse(&c, &rust, 0, input, "C06");
        assert!(unsafe { parse(&c, 0, input) }.is_none());
    }

    // C07-C09: parser flag filtering and ignored bits.
    let mail = complete_alert(
        "100",
        "mail active-response",
        "authentication",
        "/var/log/auth.log",
        "Rule: 10 (level 5) -> 'accepted'\n",
    );
    let non_mail = complete_alert(
        "101",
        "active-response",
        "authentication",
        "/var/log/auth.log",
        "Rule: 11 (level 6) -> 'ordinary'\n",
    );
    for flags in [
        0,
        EXEC_SET | READ_FAILED | 0x4000_0000,
        c_int::MIN,
        c_int::MAX,
    ] {
        compare_parse(&c, &rust, flags, &mail, "C07/C09 mail");
        compare_parse(&c, &rust, flags, &non_mail, "C07/C09 non-mail");
    }
    compare_parse(&c, &rust, MAIL_SET, &mail, "C08");

    // C10-C12: group forms and date/location splitting.
    for header in [
        "** Alert a: mail\n",
        "** Alert b: mail - \n",
        "** Alert c: mail -       spaced-group\n",
    ] {
        let input =
            format!("{header}2026 Aug 25 12:34:56: host:with:colons\nRule: 1 (level 2) -> 'x'\n");
        compare_parse(&c, &rust, 0, input.as_bytes(), "C10/C12");
    }
    for group in ["ordinary", "prefix-syscheck-suffix"] {
        let input = complete_alert(
            "group",
            "mail",
            group,
            "agent->location",
            "Rule: 2 (level 3) -> 'grouped'\n",
        );
        compare_parse(&c, &rust, 0, &input, "C11");
    }

    // C13-C16: randomized numeric/text values, all optional fields, repeats,
    // and unrecognized state-2 log lines.
    let mut random = Lcg(0x8d26_4f31_a50c_7719);
    for case in 0..96 {
        let id = random.text(1, 15);
        let group = random.text(1, 20);
        let location = format!("/var/log/{}", random.text(1, 24));
        let comment = random.text(0, 80);
        let srcip = format!(
            "{}.{}.{}.{}",
            random.range(256),
            random.range(256),
            random.range(256),
            random.range(256)
        );
        let dstip = format!("2001:db8::{:x}", random.next());
        let srcport = random.next() as i32;
        let dstport = random.next() as i32;
        let user = random.text(1, 30);
        let rule = random.next() & 0x7fff_ffff;
        let level = random.range(100_000);
        let body = format!(
            "unrecognized line {case}\n\
             Rule: {rule} (level {level}) -> '{comment}'\n\
             Src IP: old\nSrc IP: {srcip}\n\
             Src Port: {srcport}\n\
             Dst IP: old\nDst IP: {dstip}\n\
             Dst Port: {dstport}\n\
             User: old\nUser: {user}\n\
             Rule: {rule} (level {level}) -> '{comment}'\n"
        );
        let input = complete_alert(&id, "mail", &group, &location, &body);
        compare_parse(&c, &rust, 0, &input, "C13-C16");
    }

    // C14 also covers every optional field absent.
    let minimal = complete_alert("minimal", "mail", "g", "loc", "\n");
    compare_parse(&c, &rust, 0, &minimal, "C14 absent");

    // C17-C18: syscheck's one-line filename extraction state.
    let matching = complete_alert(
        "sys-1",
        "mail",
        "syscheck",
        "agent",
        "Integrity checksum changed for: '/etc/passwd'\n",
    );
    compare_parse(&c, &rust, 0, &matching, "C17");
    let delayed = complete_alert(
        "sys-2",
        "mail",
        "syscheck",
        "agent",
        "not an integrity line\nIntegrity checksum changed for: '/etc/shadow'\n",
    );
    compare_parse(&c, &rust, 0, &delayed, "C18");

    // C19-C20: EOF completion and rewind-driven multi-alert iteration.
    let mut multiple = complete_alert(
        "first",
        "mail",
        "one",
        "loc-one",
        "Rule: 1 (level 1) -> 'first'\n",
    );
    multiple.extend(complete_alert(
        "second",
        "mail",
        "two",
        "loc-two",
        "Rule: 2 (level 2) -> 'second'",
    ));
    assert_eq!(
        unsafe { parse_sequence(&c, 0, &multiple, 3) },
        unsafe { parse_sequence(&rust, 0, &multiple, 3) },
        "C19-C20"
    );

    // C21: exact fgets boundary neighborhoods while in parser state 2.
    for width in [1022usize, 1023, 1024, 2046, 2047] {
        let body = format!(
            "{}\nRule: 77 (level 8) -> 'after-long-line'\n",
            "x".repeat(width)
        );
        let input = complete_alert("long", "mail", "group", "location", &body);
        compare_parse(&c, &rust, 0, &input, "C21");
    }
}

#[derive(Debug, PartialEq, Eq)]
struct QueueSnapshot {
    result: c_int,
    position: Option<c_long>,
    bytes_without_fp: Vec<u8>,
}

unsafe fn queue_snapshot(queue: &FileQueue, result: c_int) -> QueueSnapshot {
    let mut bytes =
        unsafe { std::slice::from_raw_parts((queue as *const FileQueue).cast::<u8>(), 440) }
            .to_vec();
    bytes[288..296].fill(0);
    QueueSnapshot {
        result,
        position: (!queue.fp.is_null()).then(|| unsafe { ftell(queue.fp) }),
        bytes_without_fp: bytes,
    }
}

fn test_time(month: c_int) -> Tm {
    Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 25,
        tm_mon: month,
        tm_year: 126,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: ptr::null(),
    }
}

unsafe fn init_named_file(
    api: &Api,
    flags: c_int,
    time: &Tm,
    supplied_path: Option<&Path>,
) -> QueueSnapshot {
    let mut queue: FileQueue = unsafe { zeroed() };
    if let Some(path) = supplied_path {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        queue.fp = unsafe { fopen(path.as_ptr(), c"r".as_ptr()) };
        assert!(!queue.fp.is_null());
    }
    let result = unsafe { (api.init_file_queue)(&mut queue, time, flags) };
    let snapshot = unsafe { queue_snapshot(&queue, result) };
    if !queue.fp.is_null() {
        assert_eq!(unsafe { fclose(queue.fp) }, 0);
    }
    snapshot
}

unsafe fn read_case(
    api: &Api,
    init_flags: c_int,
    timeout: c_uint,
    time: &Tm,
) -> (c_int, Option<AlertSnapshot>) {
    let mut queue: FileQueue = unsafe { zeroed() };
    let initialized = unsafe { (api.init_file_queue)(&mut queue, time, init_flags) };
    let value = unsafe { (api.read_file_mon)(&mut queue, time, timeout) };
    let snapshot = unsafe { snapshot_alert(value) };
    if !value.is_null() {
        unsafe { (api.free_alert_data)(value) };
    }
    if !queue.fp.is_null() {
        assert_eq!(unsafe { fclose(queue.fp) }, 0);
    }
    (initialized, snapshot)
}

unsafe fn driver_case(
    api: &Api,
    day: c_int,
    month: c_int,
    year: c_int,
    timeout: c_uint,
    flags: c_int,
) -> Option<AlertSnapshot> {
    let value = unsafe { (api.driver)(day, month, year, timeout, flags) };
    let snapshot = unsafe { snapshot_alert(value) };
    if !value.is_null() {
        unsafe { (api.free_alert_data)(value) };
    }
    snapshot
}

struct CurrentDirGuard(PathBuf);

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("restore current directory");
    }
}

fn in_temporary_directory<T>(action: impl FnOnce(&Path) -> T) -> T {
    let _lock = CWD_LOCK.lock().unwrap();
    let original = std::env::current_dir().unwrap();
    let path = std::env::temp_dir().join(format!("driver-differential-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).unwrap();
    std::env::set_current_dir(&path).unwrap();
    let guard = CurrentDirGuard(original);
    let result = action(&path);
    drop(guard);
    fs::remove_dir_all(path).unwrap();
    result
}

#[test]
fn valid_queue_and_driver_matrix() {
    let c = unsafe { Api::load(&library_path("c")) };
    let rust = unsafe { Api::load(&library_path("rust")) };

    in_temporary_directory(|directory| {
        let named = directory.join("supplied.log");
        fs::write(&named, b"0123456789abcdef").unwrap();
        fs::write("alerts.log", b"0123456789abcdef").unwrap();
        let time = test_time(7);

        // C22-C25: fixed versus supplied streams crossed with READ_ALL.
        for (flags, supplied) in [
            (0, None),
            (READ_ALL, None),
            (FP_SET, Some(named.as_path())),
            (FP_SET | READ_ALL, Some(named.as_path())),
        ] {
            assert_eq!(
                unsafe { init_named_file(&c, flags, &time, supplied) },
                unsafe { init_named_file(&rust, flags, &time, supplied) },
                "C22-C25 flags={flags:#x}"
            );
        }

        // C26: all valid month indices and ignored flag values.
        for month in 0..12 {
            let time = Tm {
                tm_mday: month + 1,
                tm_year: -1900 + month * 17,
                ..test_time(month)
            };
            let flags = READ_ALL | EXEC_SET | READ_FAILED | 0x2000_0000;
            assert_eq!(
                unsafe { init_named_file(&c, flags, &time, None) },
                unsafe { init_named_file(&rust, flags, &time, None) },
                "C26 month={month}"
            );
        }

        let alert = complete_alert(
            "queue",
            "mail",
            "queue-group",
            "queue-location",
            "Rule: 42 (level 9) -> 'queue-comment'\n",
        );
        fs::write("alerts.log", &alert).unwrap();

        // C27/C29: immediate result is independent of timeout.
        for timeout in [0, 1, 7, c_uint::MAX] {
            assert_eq!(
                unsafe { read_case(&c, READ_ALL, timeout, &time) },
                unsafe { read_case(&rust, READ_ALL, timeout, &time) },
                "C27/C29 timeout={timeout}"
            );
        }

        // C28: existing EOF, successful reopen, no retry iterations.
        assert_eq!(
            unsafe { read_case(&c, 0, 0, &time) },
            unsafe { read_case(&rust, 0, 0, &time) },
            "C28"
        );

        // C30-C33: randomized end-to-end driver calls and flag interactions.
        let mut random = Lcg(0x4b1d_7792_012f_caa5);
        for case in 0..48 {
            let id = random.text(1, 12);
            let comment = random.text(0, 40);
            let mode = if case % 2 == 0 {
                "mail"
            } else {
                "active-response"
            };
            let alert = complete_alert(
                &id,
                mode,
                "driver-group",
                "driver-location",
                &format!(
                    "Rule: {} (level {}) -> '{}'\n",
                    random.range(100_000),
                    random.range(100),
                    comment
                ),
            );
            fs::write("alerts.log", alert).unwrap();
            for flags in [
                READ_ALL,
                READ_ALL | MAIL_SET,
                READ_ALL | EXEC_SET | READ_FAILED | 0x1000_0000,
                0,
            ] {
                assert_eq!(
                    unsafe { driver_case(&c, 25, 7, 126, 0, flags) },
                    unsafe { driver_case(&rust, 25, 7, 126, 0, flags) },
                    "C30-C33 case={case} flags={flags:#x}"
                );
            }
        }
    });
}

unsafe fn pipe_stream(contents: &[u8]) -> *mut CFile {
    let mut fds = [-1, -1];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    assert_eq!(
        unsafe { write(fds[1], contents.as_ptr().cast(), contents.len()) },
        contents.len() as isize
    );
    assert_eq!(unsafe { close(fds[1]) }, 0);
    let fp = unsafe { fdopen(fds[0], c"r".as_ptr()) };
    assert!(!fp.is_null());
    fp
}

unsafe fn parse_pipe_rewind_failure(api: &Api, contents: &[u8]) -> Option<AlertSnapshot> {
    let fp = unsafe { pipe_stream(contents) };
    let value = unsafe { (api.get_alert_data)(0, fp) };
    let snapshot = unsafe { snapshot_alert(value) };
    if !value.is_null() {
        unsafe { (api.free_alert_data)(value) };
    }
    assert_eq!(unsafe { fclose(fp) }, 0);
    snapshot
}

#[test]
fn parser_error_matrix() {
    let c = unsafe { Api::load(&library_path("c")) };
    let rust = unsafe { Api::load(&library_path("rust")) };
    let valid = String::from_utf8(complete_alert(
        "valid",
        "mail",
        "group",
        "location",
        "Rule: 1 (level 1) -> 'valid'\n",
    ))
    .unwrap();

    // E05: a non-seekable stream reaches the next alert header.
    let two = format!("{valid}{valid}");
    let c_result = unsafe { parse_pipe_rewind_failure(&c, two.as_bytes()) };
    let rust_result = unsafe { parse_pipe_rewind_failure(&rust, two.as_bytes()) };
    assert_eq!(c_result, None);
    assert_eq!(c_result, rust_result, "E05");

    // E06-E08: malformed/filtered candidates are skipped, not fatal.
    for (flags, prefix, id) in [
        (0, "** Alert no-colon\n", "E06"),
        (0, "** Alert bad:no-space\n", "E07"),
        (
            MAIL_SET,
            "** Alert rejected: active-response - group\n2026 Aug 25 00:00:00: loc\n",
            "E08",
        ),
    ] {
        let input = format!("{prefix}{valid}");
        let left = unsafe { parse(&c, flags, input.as_bytes()) };
        let right = unsafe { parse(&rust, flags, input.as_bytes()) };
        assert!(left.is_some(), "{id} should continue to the valid alert");
        assert_eq!(left, right, "{id}");
    }

    // E09-E14: each parser error branch returns the exact NULL sentinel.
    let failures = [
        (
            "E09",
            "** Alert x: mail - g\n12:34:no-following-space\n".to_owned(),
        ),
        (
            "E10",
            "** Alert x: mail - g\nno-colon-or-location\n".to_owned(),
        ),
        (
            "E11",
            "** Alert x: mail - g\n2026 Aug 25 00:00:00: loc\nRule: 1 only\n".to_owned(),
        ),
        (
            "E12",
            "** Alert x: mail - g\n2026 Aug 25 00:00:00: loc\nRule: 1 level 2 noquote\n".to_owned(),
        ),
        (
            "E13",
            "** Alert x: mail - g\n2026 Aug 25 00:00:00: loc\nRule: 1 level 2 'unclosed\n"
                .to_owned(),
        ),
        ("E14", "** Alert x: mail - g\n".to_owned()),
    ];
    for (id, input) in failures {
        let left = unsafe { parse(&c, 0, input.as_bytes()) };
        let right = unsafe { parse(&rust, 0, input.as_bytes()) };
        assert_eq!(left, None, "{id} C sentinel");
        assert_eq!(left, right, "{id} differential");
    }
}

unsafe fn init_pipe_seek_failure(api: &Api) -> QueueSnapshot {
    let mut queue: FileQueue = unsafe { zeroed() };
    queue.fp = unsafe { pipe_stream(b"") };
    let result = unsafe { (api.init_file_queue)(&mut queue, &test_time(0), FP_SET) };
    unsafe { queue_snapshot(&queue, result) }
}

unsafe fn init_fstat_failure(api: &Api) -> QueueSnapshot {
    let mut queue: FileQueue = unsafe { zeroed() };
    let mut backing = b"memory stream\0".to_vec();
    queue.fp = unsafe { fmemopen(backing.as_mut_ptr().cast(), backing.len(), c"r".as_ptr()) };
    assert!(!queue.fp.is_null());
    let result = unsafe { (api.init_file_queue)(&mut queue, &test_time(0), FP_SET | READ_ALL) };
    unsafe { queue_snapshot(&queue, result) }
}

unsafe fn read_missing_initial_queue(api: &Api) -> Option<AlertSnapshot> {
    let mut queue: FileQueue = unsafe { zeroed() };
    let value = unsafe { (api.read_file_mon)(&mut queue, &test_time(0), 0) };
    unsafe { snapshot_alert(value) }
}

unsafe fn read_failed_parse_then_reopen(api: &Api) -> Option<AlertSnapshot> {
    let mut queue: FileQueue = unsafe { zeroed() };
    queue.fp = unsafe { stream(b"** Alert incomplete: mail - group\n") };
    queue.flags = FP_SET | READ_ALL;
    let value = unsafe { (api.read_file_mon)(&mut queue, &test_time(0), 0) };
    let result = unsafe { snapshot_alert(value) };
    if !queue.fp.is_null() {
        unsafe { fclose(queue.fp) };
    }
    result
}

#[test]
fn queue_error_matrix() {
    let c = unsafe { Api::load(&library_path("c")) };
    let rust = unsafe { Api::load(&library_path("rust")) };

    in_temporary_directory(|_| {
        // E15: missing fixed queue is deliberately reported as successful init.
        let time = test_time(0);
        let left = unsafe { init_named_file(&c, 0, &time, None) };
        let right = unsafe { init_named_file(&rust, 0, &time, None) };
        assert_eq!(left.result, 0);
        assert_eq!(left, right, "E15");

        // E16: FP_SET with no supplied FILE also returns successful init.
        let mut c_queue: FileQueue = unsafe { zeroed() };
        let mut rust_queue: FileQueue = unsafe { zeroed() };
        let c_result = unsafe { (c.init_file_queue)(&mut c_queue, &time, FP_SET) };
        let rust_result = unsafe { (rust.init_file_queue)(&mut rust_queue, &time, FP_SET) };
        assert_eq!(c_result, 0);
        assert_eq!(
            unsafe { queue_snapshot(&c_queue, c_result) },
            unsafe { queue_snapshot(&rust_queue, rust_result) },
            "E16"
        );

        // E17: pipes cannot satisfy seek-to-end.
        let left = unsafe { init_pipe_seek_failure(&c) };
        let right = unsafe { init_pipe_seek_failure(&rust) };
        assert_eq!(left.result, -1);
        assert_eq!(left, right, "E17");

        // E18: fmemopen streams have fileno == -1, so fstat fails.
        let left = unsafe { init_fstat_failure(&c) };
        let right = unsafe { init_fstat_failure(&rust) };
        assert_eq!(left.result, -1);
        assert_eq!(left, right, "E18");

        // E19: each side performs the required single five-second sleep.
        let left = unsafe { read_missing_initial_queue(&c) };
        let right = unsafe { read_missing_initial_queue(&rust) };
        assert_eq!(left, None);
        assert_eq!(left, right, "E19");

        // E20: malformed supplied stream, then fixed "<stdin>" reopen failure.
        let left = unsafe { read_failed_parse_then_reopen(&c) };
        let right = unsafe { read_failed_parse_then_reopen(&rust) };
        assert_eq!(left, None);
        assert_eq!(left, right, "E20");

        // E21: successful reopen followed by one unsuccessful retry/sleep.
        fs::write("alerts.log", b"").unwrap();
        let left = unsafe { read_case(&c, READ_ALL, 1, &time) };
        let right = unsafe { read_case(&rust, READ_ALL, 1, &time) };
        assert_eq!(left.1, None);
        assert_eq!(left, right, "E21");

        // E22: opening a directory succeeds but seek-to-end fails in driver.
        fs::remove_file("alerts.log").unwrap();
        fs::create_dir("alerts.log").unwrap();
        let left = unsafe { driver_case(&c, 1, 0, 126, 0, 0) };
        let right = unsafe { driver_case(&rust, 1, 0, 126, 0, 0) };
        assert_eq!(left, None);
        assert_eq!(left, right, "E22");
    });
}

fn child_output(kind: &str, case: &str) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_child")
        .arg("--nocapture")
        .env("DRIVER_CHILD_KIND", kind)
        .env("DRIVER_CHILD_CASE", case)
        .output()
        .unwrap()
}

fn assert_same_child_status(case: &str) {
    let c = child_output("c", case);
    let rust = child_output("rust", case);
    assert_eq!(
        c.status.code(),
        rust.status.code(),
        "{case} exit code; C stderr={}, Rust stderr={}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c.status.signal(),
            rust.status.signal(),
            "{case} signal; C stderr={}, Rust stderr={}",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr)
        );
    }
}

#[test]
fn fatal_and_boundary_matrix() {
    // E01-E04 terminate identically.
    for (case, message) in [
        ("calloc_fail", "Memory allocation failed in os_calloc"),
        ("realloc_fail", "Memory allocation failed in os_realloc"),
        ("strdup_null", "NULL string passed to os_strdup"),
        ("strdup_oom", "Memory allocation failed in os_strdup"),
    ] {
        let c = child_output("c", case);
        let rust = child_output("rust", case);
        assert_eq!(c.status.code(), Some(1), "{case} C status");
        assert_eq!(rust.status.code(), Some(1), "{case} Rust status");
        assert!(String::from_utf8_lossy(&c.stderr).contains(message));
        assert!(String::from_utf8_lossy(&rust.stderr).contains(message));
    }

    // This libc returns a unique allocation for the zero-size request.
    let c = child_output("c", "realloc_zero");
    let rust = child_output("rust", "realloc_zero");
    assert!(c.status.success());
    assert!(rust.status.success());

    // C04: compare merror's exact emitted bytes (apart from test harness text).
    let c = child_output("c", "merror");
    let rust = child_output("rust", "merror");
    assert!(c.status.success());
    assert!(rust.status.success());
    assert_eq!(c.stderr, rust.stderr, "C04 merror bytes");
    assert_eq!(c.stderr, b"failure file.bin -37 details\n");
    let c = child_output("c", "merror_null_fields");
    let rust = child_output("rust", "merror_null_fields");
    assert_eq!(c.status.code(), rust.status.code());
    assert_eq!(c.stderr, rust.stderr);

    // Generic null-pointer boundaries have undefined C return values, so the
    // observable process termination is compared.
    for case in [
        "null_get",
        "null_free",
        "null_init_queue",
        "null_init_time",
        "null_read_queue",
        "null_read_time",
        "merror_null_template",
        "month_minus_one",
        "month_twelve",
    ] {
        assert_same_child_status(case);
    }
}

#[test]
fn ffi_child() {
    let Ok(kind) = std::env::var("DRIVER_CHILD_KIND") else {
        return;
    };
    let case = std::env::var("DRIVER_CHILD_CASE").unwrap();
    let api = unsafe { Api::load(&library_path(&kind)) };
    match case.as_str() {
        "calloc_fail" => {
            unsafe { (api.os_calloc)(usize::MAX, usize::MAX) };
        }
        "realloc_fail" => {
            unsafe { (api.os_realloc)(ptr::null_mut(), usize::MAX) };
        }
        "realloc_zero" => {
            unsafe { (api.os_realloc)(ptr::null_mut(), 0) };
        }
        "strdup_null" => {
            unsafe { (api.os_strdup)(ptr::null()) };
        }
        "strdup_oom" => {
            let mut input = vec![b'x'; 64 * 1024 * 1024];
            input.push(0);
            let pages = fs::read_to_string("/proc/self/statm")
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let limit = RLimit {
                current: pages * 4096,
                maximum: pages * 4096,
            };
            assert_eq!(unsafe { setrlimit(9, &limit) }, 0);
            unsafe { (api.os_strdup)(input.as_ptr().cast()) };
        }
        "merror" => unsafe {
            (api.merror)(
                c"failure %s %d %s".as_ptr(),
                c"file.bin".as_ptr(),
                -37,
                c"details".as_ptr(),
            );
        },
        "merror_null_fields" => unsafe {
            (api.merror)(c"%s %d %s".as_ptr(), ptr::null(), 0, ptr::null());
        },
        "merror_null_template" => unsafe {
            (api.merror)(ptr::null(), c"file".as_ptr(), 0, c"details".as_ptr());
        },
        "null_get" => unsafe {
            (api.get_alert_data)(0, ptr::null_mut());
        },
        "null_free" => unsafe {
            (api.free_alert_data)(ptr::null_mut());
        },
        "null_init_queue" => unsafe {
            (api.init_file_queue)(ptr::null_mut(), &test_time(0), 0);
        },
        "null_init_time" => unsafe {
            let mut queue: FileQueue = zeroed();
            (api.init_file_queue)(&mut queue, ptr::null(), FP_SET | READ_ALL);
        },
        "null_read_queue" => unsafe {
            (api.read_file_mon)(ptr::null_mut(), &test_time(0), 0);
        },
        "null_read_time" => unsafe {
            let mut queue: FileQueue = zeroed();
            queue.fp = stream(b"");
            queue.flags = FP_SET | READ_ALL;
            (api.read_file_mon)(&mut queue, ptr::null(), 0);
        },
        "month_twelve" => unsafe {
            let mut queue: FileQueue = zeroed();
            (api.init_file_queue)(&mut queue, &test_time(12), FP_SET | READ_ALL);
        },
        "month_minus_one" => unsafe {
            let mut queue: FileQueue = zeroed();
            (api.init_file_queue)(&mut queue, &test_time(-1), FP_SET | READ_ALL);
        },
        _ => panic!("unknown child case {case}"),
    }
}
