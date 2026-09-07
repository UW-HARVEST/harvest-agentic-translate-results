use libloading::Library;
use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type DriverFn = unsafe extern "C" fn(*const c_char);

static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn sysconf(name: c_int) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const SC_PAGESIZE: c_int = 30;

struct Apis {
    _c: Library,
    _rust: Library,
    c_foo: FooFn,
    rust_foo: FooFn,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

impl Apis {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = env::var_os("DRIVER_C_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("../c_src/build/libdriver.so"));
        let rust_path = env::var_os("DRIVER_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target/release/libdriver.so"));

        assert!(
            c_path.is_file(),
            "C shared object missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared object missing: {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path).expect("load C shared object");
            let rust = Library::new(&rust_path).expect("load Rust shared object");
            let c_foo = *c.get::<FooFn>(b"foo\0").expect("load C foo");
            let rust_foo = *rust.get::<FooFn>(b"foo\0").expect("load Rust foo");
            let c_driver = *c.get::<DriverFn>(b"driver\0").expect("load C driver");
            let rust_driver = *rust.get::<DriverFn>(b"driver\0").expect("load Rust driver");
            Self {
                _c: c,
                _rust: rust,
                c_foo,
                rust_foo,
                c_driver,
                rust_driver,
            }
        }
    }
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
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn range(&mut self, low: usize, high_exclusive: usize) -> usize {
        low + self.next_u32() as usize % (high_exclusive - low)
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.range(1, 256)) as u8
    }

    fn byte_except(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let byte = self.nonzero_byte();
            if !excluded.contains(&byte) {
                return byte;
            }
        }
    }
}

fn lock_tests() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn call_foo_pair(apis: &Apis, bytes: &[u8], target: u8) {
    let input = CString::new(bytes).expect("generated input contains no NUL");
    let target = target as i8 as c_char;
    let c = unsafe { (apis.c_foo)(input.as_ptr(), target) };
    let rust = unsafe { (apis.rust_foo)(input.as_ptr(), target) };
    assert_eq!(
        rust, c,
        "foo diverged for target {target:?} and bytes {bytes:?}"
    );
}

fn capture_driver(function: DriverFn, input: &CString) -> (c_int, Vec<u8>) {
    let mut fds = [-1; 2];
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "fflush before fork failed");
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(fds[0]);
            if dup2(fds[1], 1) < 0 {
                _exit(120);
            }
            close(fds[1]);
            function(input.as_ptr());
            fflush(ptr::null_mut());
            _exit(0);
        }

        close(fds[1]);
        let mut output = Vec::new();
        File::from_raw_fd(fds[0])
            .read_to_end(&mut output)
            .expect("read child stdout");
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        (status, output)
    }
}

fn call_driver_pair(apis: &Apis, bytes: &[u8]) {
    let input = CString::new(bytes).expect("generated input contains no NUL");
    let c = capture_driver(apis.c_driver, &input);
    let rust = capture_driver(apis.rust_driver, &input);
    assert_eq!(rust, c, "driver diverged for bytes {bytes:?}");
}

fn child_status_foo(function: FooFn, input: *const c_char, target: c_char) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            function(input, target);
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        status
    }
}

fn child_status_driver(function: DriverFn, input: *const c_char) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            function(input);
            fflush(ptr::null_mut());
            _exit(0);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        status
    }
}

#[test]
fn config_01_foo_empty_string() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0101_a11c_e001);
    for _ in 0..512 {
        call_foo_pair(&apis, &[], rng.nonzero_byte());
    }
}

#[test]
fn config_02_foo_target_absent() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0202_a11c_e002);
    for _ in 0..512 {
        let target = rng.nonzero_byte();
        let length = rng.range(1, 257);
        let bytes = (0..length)
            .map(|_| rng.byte_except(&[target]))
            .collect::<Vec<_>>();
        call_foo_pair(&apis, &bytes, target);
    }
}

#[test]
fn config_03_foo_exactly_one_match() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0303_a11c_e003);
    for iteration in 0..512 {
        let target = rng.nonzero_byte();
        let length = rng.range(1, 257);
        let mut bytes = (0..length)
            .map(|_| rng.byte_except(&[target]))
            .collect::<Vec<_>>();
        let position = match iteration % 3 {
            0 => 0,
            1 => length / 2,
            _ => length - 1,
        };
        bytes[position] = target;
        call_foo_pair(&apis, &bytes, target);
    }
}

#[test]
fn config_04_foo_multiple_separated_matches() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0404_a11c_e004);
    for _ in 0..512 {
        let target = rng.nonzero_byte();
        let matches = rng.range(2, 33);
        let mut bytes = Vec::with_capacity(matches * 2 + 2);
        for _ in 0..matches {
            bytes.push(target);
            bytes.push(rng.byte_except(&[target]));
        }
        if rng.next_u32() & 1 != 0 {
            bytes.insert(0, rng.byte_except(&[target]));
        }
        call_foo_pair(&apis, &bytes, target);
    }
}

#[test]
fn config_05_foo_multiple_consecutive_matches() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0505_a11c_e005);
    for _ in 0..512 {
        let target = rng.nonzero_byte();
        let run = rng.range(2, 65);
        let prefix = rng.range(0, 65);
        let suffix = rng.range(0, 65);
        let mut bytes = (0..prefix)
            .map(|_| rng.byte_except(&[target]))
            .collect::<Vec<_>>();
        bytes.extend(std::iter::repeat_n(target, run));
        bytes.extend((0..suffix).map(|_| rng.byte_except(&[target])));
        call_foo_pair(&apis, &bytes, target);
    }
}

#[test]
fn config_06_foo_non_ascii_signed_char_target() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0606_a11c_e006);
    for _ in 0..512 {
        let target = rng.range(0x80, 0x100) as u8;
        let length = rng.range(1, 257);
        let mut bytes = (0..length)
            .map(|_| rng.byte_except(&[target]))
            .collect::<Vec<_>>();
        for _ in 0..rng.range(0, 17) {
            let index = rng.range(0, length);
            bytes[index] = target;
        }
        call_foo_pair(&apis, &bytes, target);
    }
}

#[test]
fn config_07_driver_neither_fixed_target() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0707_a11c_e007);
    call_driver_pair(&apis, &[]);
    for _ in 0..64 {
        let length = rng.range(1, 257);
        let bytes = (0..length)
            .map(|_| rng.byte_except(&[b'A', b'x']))
            .collect::<Vec<_>>();
        call_driver_pair(&apis, &bytes);
    }
}

#[test]
fn config_08_driver_a_only() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0808_a11c_e008);
    for _ in 0..64 {
        let length = rng.range(1, 257);
        let mut bytes = (0..length)
            .map(|_| rng.byte_except(&[b'A', b'x']))
            .collect::<Vec<_>>();
        for _ in 0..rng.range(1, length + 1) {
            let index = rng.range(0, length);
            bytes[index] = b'A';
        }
        call_driver_pair(&apis, &bytes);
    }
}

#[test]
fn config_09_driver_x_only() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x0909_a11c_e009);
    for _ in 0..64 {
        let length = rng.range(1, 257);
        let mut bytes = (0..length)
            .map(|_| rng.byte_except(&[b'A', b'x']))
            .collect::<Vec<_>>();
        for _ in 0..rng.range(1, length + 1) {
            let index = rng.range(0, length);
            bytes[index] = b'x';
        }
        call_driver_pair(&apis, &bytes);
    }
}

#[test]
fn config_10_driver_both_fixed_targets() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let mut rng = Lcg::new(0x1010_a11c_e010);
    for _ in 0..64 {
        let length = rng.range(2, 257);
        let mut bytes = (0..length)
            .map(|_| rng.byte_except(&[b'A', b'x']))
            .collect::<Vec<_>>();
        let a_index = rng.range(0, length);
        let mut x_index = rng.range(0, length);
        if x_index == a_index {
            x_index = (x_index + 1) % length;
        }
        bytes[a_index] = b'A';
        bytes[x_index] = b'x';
        for _ in 0..rng.range(0, length + 1) {
            let index = rng.range(0, length);
            bytes[index] = if rng.next_u32() & 1 == 0 { b'A' } else { b'x' };
        }
        if !bytes.contains(&b'A') {
            bytes[a_index] = b'A';
        }
        if !bytes.contains(&b'x') {
            bytes[x_index] = b'x';
        }
        call_driver_pair(&apis, &bytes);
    }
}

#[test]
fn error_01_foo_null_input_process_status() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let target = b'A' as c_char;
    let c = child_status_foo(apis.c_foo, ptr::null(), target);
    let rust = child_status_foo(apis.rust_foo, ptr::null(), target);
    assert_ne!(c, 0, "C unexpectedly accepted a null input");
    assert_eq!(rust, c, "foo null-input termination status diverged");
}

#[test]
fn error_02_driver_null_input_process_status() {
    let _guard = lock_tests();
    let apis = Apis::load();
    let c = child_status_driver(apis.c_driver, ptr::null());
    let rust = child_status_driver(apis.rust_driver, ptr::null());
    assert_ne!(c, 0, "C unexpectedly accepted a null input");
    assert_eq!(rust, c, "driver null-input termination status diverged");
}

#[test]
fn error_03_foo_nul_target_guard_page_status() {
    let _guard = lock_tests();
    let apis = Apis::load();
    unsafe {
        let page_size = sysconf(SC_PAGESIZE);
        assert!(page_size > 0, "sysconf(_SC_PAGESIZE) failed");
        let page_size = page_size as usize;
        let mapping = mmap(
            ptr::null_mut(),
            page_size * 2,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(mapping as isize, -1, "mmap failed");
        assert_eq!(
            mprotect(mapping.add(page_size), page_size, PROT_NONE),
            0,
            "mprotect failed"
        );

        let input = mapping.cast::<u8>().add(page_size - 16);
        ptr::write_bytes(input, b'Q', 15);
        input.add(15).write(0);

        let c = child_status_foo(apis.c_foo, input.cast(), 0);
        let rust = child_status_foo(apis.rust_foo, input.cast(), 0);
        assert_ne!(c, 0, "C unexpectedly returned for a guard-page NUL target");
        assert_eq!(rust, c, "foo NUL-target termination status diverged");
        assert_eq!(munmap(mapping, page_size * 2), 0, "munmap failed");
    }
}
