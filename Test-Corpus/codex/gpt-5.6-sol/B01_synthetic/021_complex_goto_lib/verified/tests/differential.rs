use libloading::Library;
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type Driver = unsafe extern "C" fn(c_int, c_int);

const SAMPLE_COUNT: usize = 32;
const OUTPUT_LIMIT: usize = 16 * 1024;
const STDOUT_FILENO: c_int = 1;
const SIGKILL: c_int = 9;
const _IONBF: c_int = 2;

unsafe extern "C" {
    static mut stdout: *mut c_void;

    fn pipe(pipefd: *mut c_int) -> c_int;
    fn fork() -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn kill(pid: c_int, signal: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buffer: *mut c_int, mode: c_int, size: usize) -> c_int;
    fn _exit(status: c_int) -> !;
}

struct Drivers {
    _c_library: Library,
    _rust_library: Library,
    c: Driver,
    rust: Driver,
}

impl Drivers {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let rust_path = rust_library_path(root);

        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c = *c_library
                .get::<Driver>(b"driver\0")
                .expect("resolve C driver");
            let rust = *rust_library
                .get::<Driver>(b"driver\0")
                .expect("resolve Rust driver");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    let target = option_env!("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    target.join("release/libdriver.so")
}

#[derive(Debug)]
struct Captured {
    bytes: Vec<u8>,
    completed: bool,
}

fn capture(driver: Driver, x: i32, y: i32) -> Captured {
    unsafe {
        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe failed");
        fflush(ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            close(fds[0]);
            if dup2(fds[1], STDOUT_FILENO) < 0 {
                _exit(120);
            }
            close(fds[1]);
            setvbuf(stdout, ptr::null_mut(), _IONBF, 0);
            driver(x, y);
            fflush(ptr::null_mut());
            _exit(0);
        }

        close(fds[1]);
        let mut bytes = Vec::with_capacity(OUTPUT_LIMIT);
        while bytes.len() < OUTPUT_LIMIT {
            let mut buffer = [0_u8; 1024];
            let wanted = buffer.len().min(OUTPUT_LIMIT - bytes.len());
            let count = read(fds[0], buffer.as_mut_ptr().cast(), wanted);
            assert!(count >= 0, "read failed");
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count as usize]);
        }
        close(fds[0]);

        let completed = bytes.len() < OUTPUT_LIMIT;
        if !completed {
            assert_eq!(kill(pid, SIGKILL), 0, "kill failed");
        }

        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        if completed {
            assert_eq!(status, 0, "child failed with wait status {status}");
        }

        Captured { bytes, completed }
    }
}

#[derive(Clone, Copy, Debug)]
enum XClass {
    NonPositive,
    One,
    Two,
    AtLeastThree,
}

#[derive(Clone, Copy, Debug)]
enum YClass {
    Negative,
    Zero,
    OneToThree,
    Four,
    AtLeastFive,
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn below(&mut self, limit: u32) -> i32 {
        (self.next() % limit) as i32
    }
}

fn x_value(class: XClass, sample: usize, rng: &mut Rng) -> i32 {
    match class {
        XClass::NonPositive => match sample {
            0 => i32::MIN,
            1 => 0,
            2 => -1,
            _ => -2 - rng.below(63),
        },
        XClass::One => 1,
        XClass::Two => 2,
        XClass::AtLeastThree => match sample {
            0 => i32::MAX,
            1 => 3,
            2 => 4,
            _ => 5 + rng.below(60),
        },
    }
}

fn y_value(class: YClass, x_class: XClass, sample: usize, rng: &mut Rng) -> i32 {
    match class {
        YClass::Negative => {
            if matches!(x_class, XClass::NonPositive) && sample == 0 {
                i32::MIN
            } else if sample == 1 {
                -1
            } else {
                -2 - rng.below(63)
            }
        }
        YClass::Zero => 0,
        YClass::OneToThree => 1 + rng.below(3),
        YClass::Four => 4,
        YClass::AtLeastFive => match sample {
            0 => i32::MAX,
            1 => 5,
            _ => 6 + rng.below(59),
        },
    }
}

fn compare(drivers: &Drivers, row: usize, sample: usize, x: i32, y: i32) -> bool {
    let c = capture(drivers.c, x, y);
    let rust = capture(drivers.rust, x, y);

    assert_eq!(
        c.bytes, rust.bytes,
        "output mismatch in CONFIGS.md row {row}, sample {sample}, x={x}, y={y}"
    );
    assert_eq!(
        c.completed, rust.completed,
        "completion mismatch in CONFIGS.md row {row}, sample {sample}, x={x}, y={y}"
    );
    c.completed
}

#[test]
fn all_configuration_rows_and_scalar_boundaries_match() {
    let drivers = Drivers::load();
    let x_classes = [
        XClass::NonPositive,
        XClass::One,
        XClass::Two,
        XClass::AtLeastThree,
    ];
    let y_classes = [
        YClass::Negative,
        YClass::Zero,
        YClass::OneToThree,
        YClass::Four,
        YClass::AtLeastFive,
    ];

    let mut row = 0;
    for x_class in x_classes {
        for y_class in y_classes {
            row += 1;
            let mut rng = Rng(0x5eed_d15c_a11f_0000_u64 ^ row as u64);
            for sample in 0..SAMPLE_COUNT {
                let x = x_value(x_class, sample, &mut rng);
                let y = y_value(y_class, x_class, sample, &mut rng);
                let completed = compare(&drivers, row, sample, x, y);

                let nonterminating =
                    !matches!(x_class, XClass::NonPositive) && matches!(y_class, YClass::Negative);
                if nonterminating {
                    assert!(
                        !completed,
                        "CONFIGS.md row {row} unexpectedly completed for x={x}, y={y}"
                    );
                } else if x != i32::MAX && y != i32::MAX {
                    assert!(
                        completed,
                        "CONFIGS.md row {row} exceeded the output limit for x={x}, y={y}"
                    );
                }
            }
        }
    }
}
