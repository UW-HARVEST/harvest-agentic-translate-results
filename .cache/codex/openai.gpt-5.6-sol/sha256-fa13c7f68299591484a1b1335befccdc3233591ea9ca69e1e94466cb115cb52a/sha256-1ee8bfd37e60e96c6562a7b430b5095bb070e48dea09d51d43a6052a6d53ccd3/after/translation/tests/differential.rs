use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Dataentry = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct DifferentialLibraries {
    c: Library,
    rust: Library,
}

impl DifferentialLibraries {
    fn load() -> Self {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_dir.join("../c_src/build/libharvest-work-IWODxW.so");
        let rust_path = rust_library_path(crate_dir);

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

        // SAFETY: Both paths are build artifacts controlled by this test.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: Both paths are build artifacts controlled by this test.
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        Self { c, rust }
    }

    fn compare(&self, row: &str, args: [c_int; 4]) -> c_int {
        // SAFETY: Phase A established that both libraries export `dataentry`
        // with the C header's four-int signature.
        let (c_result, rust_result) = unsafe {
            let c_fn: Symbol<'_, Dataentry> = self
                .c
                .get(b"dataentry\0")
                .expect("C library does not export dataentry");
            let rust_fn: Symbol<'_, Dataentry> = self
                .rust
                .get(b"dataentry\0")
                .expect("Rust library does not export dataentry");

            (
                c_fn(args[0], args[1], args[2], args[3]),
                rust_fn(args[0], args[1], args[2], args[3]),
            )
        };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "{row} diverged for arguments {args:?}: C={c_result}, Rust={rust_result}"
        );
        c_result
    }
}

fn rust_library_path(crate_dir: &Path) -> PathBuf {
    crate_dir.join("target/release/libdataentry_lib.so")
}

struct FixedRng(u64);

impl FixedRng {
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

    fn range(&mut self, low: c_int, high_exclusive: c_int) -> c_int {
        assert!(low < high_exclusive);
        low + (self.next_u32() % (high_exclusive - low) as u32) as c_int
    }

    fn any_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }

    fn nonzero_i32(&mut self) -> c_int {
        loop {
            let value = self.any_i32();
            if value != 0 {
                return value;
            }
        }
    }
}

#[test]
fn all_configuration_rows_match_across_randomized_inputs() {
    const CASES: usize = 256;

    let libraries = DifferentialLibraries::load();
    let mut rng = FixedRng::new(0x434f_4e46_4947_5331);

    // CONFIGS row 1: fallback count and a present target.
    for _ in 0..CASES {
        let count_selector = rng.range(-64, 1);
        let target = rng.range(0, 5);
        libraries.compare("CONFIGS row 1", [1, count_selector, target, rng.any_i32()]);
    }

    // CONFIGS row 2: the one-element input shape.
    for _ in 0..CASES {
        libraries.compare("CONFIGS row 2", [1, 1, 0, rng.any_i32()]);
    }

    // CONFIGS row 3: first, interior, and last targets in many-element inputs.
    for case in 0..CASES {
        let count = rng.range(2, 65);
        let target = match case % 3 {
            0 => 0,
            1 => count / 2,
            _ => count - 1,
        };
        libraries.compare("CONFIGS row 3", [1, count, target, rng.any_i32()]);
    }

    // CONFIGS row 4: missing targets with fallback count.
    for case in 0..CASES {
        let target = if case % 2 == 0 {
            rng.range(-64, 0)
        } else {
            rng.range(5, 69)
        };
        libraries.compare(
            "CONFIGS row 4",
            [1, rng.range(-64, 1), target, rng.any_i32()],
        );
    }

    // CONFIGS row 5: missing targets with explicit positive counts.
    for case in 0..CASES {
        let count = rng.range(1, 65);
        let target = if case % 2 == 0 {
            rng.range(-64, 0)
        } else {
            count + rng.range(0, 64)
        };
        libraries.compare("CONFIGS row 5", [1, count, target, rng.any_i32()]);
    }
    libraries.compare("CONFIGS row 5", [1, 1, c_int::MIN, c_int::MIN]);
    libraries.compare("CONFIGS row 5", [1, 1, c_int::MAX, c_int::MAX]);

    // CONFIGS rows 6-11: fallback/one/many counts crossed with zero/nonzero
    // multipliers, including the compiled C implementation's overflow edges.
    for _ in 0..CASES {
        libraries.compare("CONFIGS row 6", [2, rng.range(-64, 1), 0, rng.any_i32()]);
        libraries.compare(
            "CONFIGS row 7",
            [2, rng.range(-64, 1), rng.nonzero_i32(), rng.any_i32()],
        );
        libraries.compare("CONFIGS row 8", [2, 1, 0, rng.any_i32()]);
        libraries.compare("CONFIGS row 9", [2, 1, rng.nonzero_i32(), rng.any_i32()]);

        let many_count = rng.range(2, 65);
        libraries.compare("CONFIGS row 10", [2, many_count, 0, rng.any_i32()]);
        libraries.compare(
            "CONFIGS row 11",
            [2, many_count, rng.nonzero_i32(), rng.any_i32()],
        );
    }

    // CONFIGS row 12: every valid lookup-table coordinate.
    for case in 0..CASES {
        let row = (case % 4) as c_int;
        let col = ((case / 4) % 3) as c_int;
        libraries.compare("CONFIGS row 12", [3, row, col, rng.any_i32()]);
    }

    // CONFIGS rows 13-16: each side of each lookup bound.
    for _ in 0..CASES {
        libraries.compare(
            "CONFIGS row 13",
            [3, rng.range(-64, 0), rng.any_i32(), rng.any_i32()],
        );
        libraries.compare(
            "CONFIGS row 14",
            [3, rng.range(4, 68), rng.any_i32(), rng.any_i32()],
        );
        libraries.compare(
            "CONFIGS row 15",
            [3, rng.range(0, 4), rng.range(-64, 0), rng.any_i32()],
        );
        libraries.compare(
            "CONFIGS row 16",
            [3, rng.range(0, 4), rng.range(3, 67), rng.any_i32()],
        );
    }

    // CONFIGS row 17: all values outside the three explicit mode cases.
    for _ in 0..CASES {
        let mode = loop {
            let candidate = rng.any_i32();
            if ![1, 2, 3].contains(&candidate) {
                break candidate;
            }
        };
        libraries.compare(
            "CONFIGS row 17",
            [mode, rng.any_i32(), rng.any_i32(), rng.any_i32()],
        );
    }
}

#[test]
fn all_public_error_rows_match_the_exact_c_sentinel() {
    const CASES: usize = 256;

    let libraries = DifferentialLibraries::load();
    let mut rng = FixedRng::new(0x4552_524f_5253_3031);

    let exact_boundaries = [
        ("ERRORS row 1", [1, 0, -1, 0], -2),
        ("ERRORS row 2", [1, 0, 5, 0], -2),
        ("ERRORS row 3", [1, 1, -1, 0], -2),
        ("ERRORS row 4", [1, 1, 1, 0], -2),
        ("ERRORS row 5", [3, -1, 0, 0], 0),
        ("ERRORS row 6", [3, 4, 0, 0], 0),
        ("ERRORS row 7", [3, 0, -1, 0], 0),
        ("ERRORS row 8", [3, 0, 3, 0], 0),
    ];
    for (row, args, expected) in exact_boundaries {
        assert_eq!(libraries.compare(row, args), expected);
    }

    for _ in 0..CASES {
        let result = libraries.compare(
            "ERRORS row 1",
            [1, rng.range(-64, 1), rng.range(-64, 0), rng.any_i32()],
        );
        assert_eq!(result, -2);

        let result = libraries.compare(
            "ERRORS row 2",
            [1, rng.range(-64, 1), rng.range(5, 69), rng.any_i32()],
        );
        assert_eq!(result, -2);

        let count = rng.range(1, 65);
        let result =
            libraries.compare("ERRORS row 3", [1, count, rng.range(-64, 0), rng.any_i32()]);
        assert_eq!(result, -2);

        let result = libraries.compare(
            "ERRORS row 4",
            [1, count, count + rng.range(0, 64), rng.any_i32()],
        );
        assert_eq!(result, -2);

        let result = libraries.compare(
            "ERRORS row 5",
            [3, rng.range(-64, 0), rng.any_i32(), rng.any_i32()],
        );
        assert_eq!(result, 0);

        let result = libraries.compare(
            "ERRORS row 6",
            [3, rng.range(4, 68), rng.any_i32(), rng.any_i32()],
        );
        assert_eq!(result, 0);

        let result = libraries.compare(
            "ERRORS row 7",
            [3, rng.range(0, 4), rng.range(-64, 0), rng.any_i32()],
        );
        assert_eq!(result, 0);

        let result = libraries.compare(
            "ERRORS row 8",
            [3, rng.range(0, 4), rng.range(3, 67), rng.any_i32()],
        );
        assert_eq!(result, 0);
    }
}
