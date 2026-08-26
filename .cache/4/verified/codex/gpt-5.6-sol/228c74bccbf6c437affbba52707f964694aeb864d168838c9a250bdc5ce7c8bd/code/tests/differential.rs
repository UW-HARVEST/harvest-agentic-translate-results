use libloading::Library;
use std::env;
use std::ffi::c_int;
use std::path::PathBuf;
use std::process::Command;

type Dataentry = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_dataentry: Dataentry,
    rust_dataentry: Dataentry,
}

impl Libraries {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("c_src/build/libtranslated_rust.so");
        let target = env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest.join("target"));
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = target.join(profile).join("libdataentry_lib.so");

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
            let c = Library::new(&c_path).expect("load C shared library");
            let rust = Library::new(&rust_path).expect("load Rust shared library");
            let c_dataentry = *c
                .get::<Dataentry>(b"dataentry\0")
                .expect("load C dataentry");
            let rust_dataentry = *rust
                .get::<Dataentry>(b"dataentry\0")
                .expect("load Rust dataentry");

            Self {
                _c: c,
                _rust: rust,
                c_dataentry,
                rust_dataentry,
            }
        }
    }

    fn call(&self, args: [c_int; 4]) -> (c_int, c_int) {
        unsafe {
            (
                (self.c_dataentry)(args[0], args[1], args[2], args[3]),
                (self.rust_dataentry)(args[0], args[1], args[2], args[3]),
            )
        }
    }

    fn assert_same(&self, row: usize, args: [c_int; 4]) {
        let (c, rust) = self.call(args);
        assert_eq!(
            rust, c,
            "CONFIGS.md row {row} diverged for arguments {args:?}"
        );
    }

    fn assert_rejection(&self, row: usize, args: [c_int; 4], expected: c_int) {
        let (c, rust) = self.call(args);
        assert_eq!(
            c, expected,
            "C result changed for ERRORS.md row {row}, arguments {args:?}"
        );
        assert_eq!(
            rust, c,
            "ERRORS.md row {row} diverged for arguments {args:?}"
        );
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn range(&mut self, start: i32, end: i32) -> i32 {
        debug_assert!(start < end);
        start + (self.next_u32() % (end - start) as u32) as i32
    }

    fn nonzero_range(&mut self, start: i32, end: i32) -> i32 {
        loop {
            let value = self.range(start, end);
            if value != 0 {
                return value;
            }
        }
    }
}

#[test]
fn valid_configuration_surface_matches() {
    const CASES: usize = 256;

    let libraries = Libraries::load();
    let mut rng = Rng::new(0x6a09_e667_f3bc_c909);

    for case in 0..CASES {
        let default_count = if case % 2 == 0 {
            0
        } else {
            rng.range(-10_000, 0)
        };
        let valid_default_index = match case % 3 {
            0 => 0,
            1 => 2,
            _ => 4,
        };
        libraries.assert_same(1, [1, default_count, valid_default_index, rng.next_i32()]);
        libraries.assert_same(2, [1, default_count, rng.range(-1000, 0), rng.next_i32()]);
        libraries.assert_same(3, [1, default_count, rng.range(5, 1000), rng.next_i32()]);

        libraries.assert_same(4, [1, 1, 0, rng.next_i32()]);
        libraries.assert_same(5, [1, 1, rng.range(-1000, 0), rng.next_i32()]);
        libraries.assert_same(6, [1, 1, rng.range(1, 1000), rng.next_i32()]);

        let many_count = rng.range(2, 65);
        let valid_many_index = match case % 3 {
            0 => 0,
            1 => many_count / 2,
            _ => many_count - 1,
        };
        libraries.assert_same(7, [1, many_count, valid_many_index, rng.next_i32()]);
        libraries.assert_same(8, [1, many_count, rng.range(-1000, 0), rng.next_i32()]);
        libraries.assert_same(
            9,
            [
                1,
                many_count,
                many_count + rng.range(0, 1000),
                rng.next_i32(),
            ],
        );

        libraries.assert_same(10, [2, default_count, 0, rng.next_i32()]);
        libraries.assert_same(
            11,
            [
                2,
                default_count,
                rng.nonzero_range(-1000, 1001),
                rng.range(-1_000_000, 1_000_001),
            ],
        );
        libraries.assert_same(12, [2, 1, 0, rng.next_i32()]);
        libraries.assert_same(
            13,
            [
                2,
                1,
                rng.nonzero_range(-1000, 1001),
                rng.range(-1_000_000, 1_000_001),
            ],
        );
        libraries.assert_same(14, [2, many_count, 0, rng.next_i32()]);
        libraries.assert_same(
            15,
            [
                2,
                many_count,
                rng.nonzero_range(-1000, 1001),
                rng.range(-1_000_000, 1_000_001),
            ],
        );

        for row in 0..4 {
            for column in 0..3 {
                let config_row = 16 + row * 3 + column;
                libraries.assert_same(
                    config_row,
                    [
                        3,
                        row as i32,
                        column as i32,
                        rng.range(-1_000_000, 1_000_001),
                    ],
                );
            }
        }

        let default_modes = [i32::MIN, -1000, -1, 0, 4, 1000, i32::MAX, rng.next_i32()];
        let mut mode = default_modes[case % default_modes.len()];
        if (1..=3).contains(&mode) {
            mode = 4;
        }
        let param1 = match case % 8 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            _ => rng.next_i32(),
        };
        libraries.assert_same(28, [mode, param1, rng.next_i32(), rng.next_i32()]);
    }
}

#[test]
fn public_error_surface_matches() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    for _ in 0..256 {
        let count = rng.range(1, 65);
        libraries.assert_rejection(1, [1, count, rng.range(-1000, 0), rng.next_i32()], -2);
        libraries.assert_rejection(
            6,
            [1, count, count + rng.range(0, 1000), rng.next_i32()],
            -2,
        );

        libraries.assert_rejection(8, [3, rng.range(-1000, 0), 1, rng.next_i32()], 0);
        libraries.assert_rejection(9, [3, rng.range(4, 1000), 1, rng.next_i32()], 0);
        libraries.assert_rejection(10, [3, 1, rng.range(-1000, 0), rng.next_i32()], 0);
        libraries.assert_rejection(11, [3, 1, rng.range(3, 1000), rng.next_i32()], 0);
    }

    for args in [
        [3, -1, 0, 0],
        [3, 4, 0, 0],
        [3, 0, -1, 0],
        [3, 0, 3, 0],
        [3, i32::MIN, 0, 0],
        [3, i32::MAX, 0, 0],
        [3, 0, i32::MIN, 0],
        [3, 0, i32::MAX, 0],
    ] {
        let (c, rust) = libraries.call(args);
        assert_eq!(c, 0, "C boundary result changed for {args:?}");
        assert_eq!(rust, c, "boundary divergence for {args:?}");
    }
}

#[test]
fn generic_ffi_boundaries_match() {
    let libraries = Libraries::load();

    for args in [
        [0, 0, 0, 0],
        [1, 0, 0, 0],
        [2, 0, 0, 0],
        [i32::MIN, 7, i32::MIN, i32::MAX],
        [i32::MAX, -7, i32::MAX, i32::MIN],
    ] {
        let (c, rust) = libraries.call(args);
        assert_eq!(rust, c, "generic FFI boundary diverged for {args:?}");
    }
}

#[test]
fn private_error_guards_remain_publicly_unreachable() {
    let source = include_str!("../c_src/src/lib.c");
    assert!(source.contains("process_name(buffer, \"TestName\", NAME_LENGTH)"));
    assert!(source.contains("if (entries == NULL)"));
    assert!(source.contains("count = param1 > 0 ? param1 : 5"));
    assert!(source.contains("count = param1 > 0 ? param1 : 3"));

    let libraries = Libraries::load();
    for args in [[0, 11, 0, 0], [1, 0, 0, 0], [2, 0, 1, 0]] {
        let (c, rust) = libraries.call(args);
        assert_eq!(rust, c, "call-site invariant diverged for {args:?}");
    }
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct Rlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn setrlimit(resource: c_int, rlimit: *const Rlimit) -> c_int;
}

#[cfg(target_os = "linux")]
#[test]
fn allocation_failure_matches() {
    const CHILD_ENV: &str = "DATAENTRY_OOM_CHILD";
    const RLIMIT_AS: c_int = 9;

    if env::var_os(CHILD_ENV).is_none() {
        let status = Command::new(env::current_exe().expect("current test executable"))
            .arg("allocation_failure_matches")
            .arg("--exact")
            .arg("--nocapture")
            .env(CHILD_ENV, "1")
            .status()
            .expect("run isolated allocation-failure test");
        assert!(
            status.success(),
            "allocation-failure child failed: {status}"
        );
        return;
    }

    let libraries = Libraries::load();
    let one_gibibyte = 1024_u64 * 1024 * 1024;
    let limit = Rlimit {
        rlim_cur: one_gibibyte,
        rlim_max: one_gibibyte,
    };
    assert_eq!(
        unsafe { setrlimit(RLIMIT_AS, &limit) },
        0,
        "setrlimit failed"
    );

    libraries.assert_rejection(3, [1, i32::MAX, 0, 0], -1);
    libraries.assert_rejection(5, [1, i32::MAX, 0, 0], -1);
    libraries.assert_rejection(7, [2, i32::MAX, 1, 0], -1);
}
