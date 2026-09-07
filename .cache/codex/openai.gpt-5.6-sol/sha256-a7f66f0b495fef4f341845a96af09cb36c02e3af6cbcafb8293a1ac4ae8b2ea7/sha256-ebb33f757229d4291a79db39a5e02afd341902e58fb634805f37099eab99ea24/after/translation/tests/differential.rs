use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};

type Driver = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir.join("../c_src/build/libdriver.so");
    let rust_library = crate_dir.join("target/release/libdriver.so");
    assert!(
        c_library.is_file(),
        "C shared library is missing: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "Rust shared library is missing: {}",
        rust_library.display()
    );
    (c_library, rust_library)
}

fn run_library(path: &Path, cases: &[(c_int, c_int)]) -> Vec<u8> {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { pipe(fds.as_mut_ptr()) },
        0,
        "pipe failed for {}",
        path.display()
    );

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed for {}", path.display());

    if pid == 0 {
        unsafe {
            close(fds[0]);
            if dup2(fds[1], 1) == -1 {
                _exit(120);
            }
            close(fds[1]);

            let library = match Library::new(path) {
                Ok(library) => library,
                Err(_) => _exit(121),
            };
            let driver: Symbol<'_, Driver> = match library.get(b"driver\0") {
                Ok(driver) => driver,
                Err(_) => _exit(122),
            };

            for &(x, y) in cases {
                driver(x, y);
            }

            if fflush(std::ptr::null_mut()) != 0 {
                _exit(123);
            }
            drop(driver);
            drop(library);
            _exit(0);
        }
    }

    unsafe {
        close(fds[1]);
    }
    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("failed to read captured stdout");
    drop(reader);

    let mut status = -1;
    assert_eq!(unsafe { waitpid(pid, &mut status, 0) }, pid);
    assert_eq!(
        status,
        0,
        "child calling {} failed with wait status {status}",
        path.display()
    );
    output
}

fn assert_equivalent(row: usize, cases: &[(c_int, c_int)]) {
    let (c_library, rust_library) = library_paths();
    let c_output = run_library(&c_library, cases);
    let rust_output = run_library(&rust_library, cases);
    assert_eq!(
        rust_output,
        c_output,
        "CONFIGS.md row {row} diverged for {} cases; first cases: {:?}",
        cases.len(),
        &cases[..cases.len().min(8)]
    );
}

#[derive(Clone, Copy)]
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

    fn bounded(&mut self, upper_exclusive: u32) -> c_int {
        (self.next_u32() % upper_exclusive) as c_int
    }
}

#[test]
fn config_01_empty_outer_loop() {
    let mut rng = Lcg::new(0x0101_5eed);
    let mut cases = vec![
        (0, 0),
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, 0),
        (0, c_int::MIN),
    ];
    cases.extend((0..128).map(|_| (-rng.bounded(65), -rng.bounded(65))));
    assert_equivalent(1, &cases);
}

#[test]
fn config_02_one_x_with_zero_y() {
    assert_equivalent(2, &vec![(1, 0); 64]);
}

#[test]
fn config_03_many_x_with_zero_y() {
    let mut rng = Lcg::new(0x0303_5eed);
    let mut cases = vec![(2, 0), (3, 0), (64, 0)];
    cases.extend((0..128).map(|_| (2 + rng.bounded(63), 0)));
    assert_equivalent(3, &cases);
}

#[test]
fn config_04_one_y_with_nonpositive_x() {
    let mut rng = Lcg::new(0x0404_5eed);
    let mut cases = vec![(0, 1), (-1, 1), (c_int::MIN, 1)];
    cases.extend((0..128).map(|_| (-rng.bounded(65), 1)));
    assert_equivalent(4, &cases);
}

#[test]
fn config_05_many_y_with_nonpositive_x() {
    let mut rng = Lcg::new(0x0505_5eed);
    let mut cases = vec![(0, 2), (-1, 2), (c_int::MIN, 64)];
    cases.extend((0..128).map(|_| (-rng.bounded(65), 2 + rng.bounded(63))));
    assert_equivalent(5, &cases);
}

#[test]
fn config_06_exact_special_jump() {
    assert_equivalent(6, &vec![(1, 4); 64]);
}

#[test]
fn config_07_one_x_normal_inner_path() {
    let mut rng = Lcg::new(0x0707_5eed);
    let mut cases = vec![(1, 1), (1, 2), (1, 3), (1, 5), (1, 64)];
    cases.extend((0..128).map(|_| {
        let raw = 1 + rng.bounded(63);
        (1, if raw == 4 { 5 } else { raw })
    }));
    assert_equivalent(7, &cases);
}

#[test]
fn config_08_two_or_three_x_immediate_inner_path() {
    let mut rng = Lcg::new(0x0808_5eed);
    let mut cases = vec![(2, 1), (3, 1), (2, 64), (3, 64)];
    cases.extend((0..128).map(|_| (2 + rng.bounded(2), 1 + rng.bounded(64))));
    assert_equivalent(8, &cases);
}

#[test]
fn config_09_four_x_one_y() {
    assert_equivalent(9, &vec![(4, 1); 64]);
}

#[test]
fn config_10_four_x_many_y() {
    let mut rng = Lcg::new(0x1010_5eed);
    let mut cases = vec![(4, 2), (4, 3), (4, 4), (4, 64)];
    cases.extend((0..128).map(|_| (4, 2 + rng.bounded(63))));
    assert_equivalent(10, &cases);
}

#[test]
fn config_11_y_exhausts_before_low_x() {
    let mut rng = Lcg::new(0x1111_5eed);
    let mut cases = vec![(5, 1), (5, 2), (6, 3), (64, 61)];
    cases.extend((0..128).map(|_| {
        let x = 5 + rng.bounded(60);
        let y = 1 + rng.bounded((x - 3) as u32);
        (x, y)
    }));
    assert_equivalent(11, &cases);
}

#[test]
fn config_12_low_x_reached_while_y_remains() {
    let mut rng = Lcg::new(0x1212_5eed);
    let mut cases = vec![(5, 3), (5, 4), (6, 4), (64, 62), (64, 96)];
    cases.extend((0..128).map(|_| {
        let x = 5 + rng.bounded(60);
        let y = x - 2 + rng.bounded(33);
        (x, y)
    }));
    assert_equivalent(12, &cases);
}

#[test]
fn generic_ffi_scalar_boundaries_match() {
    let cases = [
        (c_int::MIN, c_int::MIN),
        (c_int::MIN, -1),
        (-1, c_int::MIN),
        (-1, -1),
        (0, c_int::MIN),
        (c_int::MIN, 0),
        (0, 0),
        (c_int::MIN, 1),
        (-1, 1),
        (0, 1),
        (1, 0),
    ];
    assert_equivalent(1, &cases);
}
