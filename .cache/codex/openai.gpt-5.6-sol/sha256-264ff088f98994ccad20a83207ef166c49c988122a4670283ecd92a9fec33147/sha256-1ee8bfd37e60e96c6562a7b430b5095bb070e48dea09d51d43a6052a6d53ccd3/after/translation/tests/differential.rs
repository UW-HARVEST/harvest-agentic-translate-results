use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

type Jumpnode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

const CASES_PER_CONFIG: usize = 128;
const DECIMAL_BOUNDARIES: [i32; 19] = [
    1,
    9,
    10,
    99,
    100,
    999,
    1_000,
    9_999,
    10_000,
    99_999,
    100_000,
    999_999,
    1_000_000,
    9_999_999,
    10_000_000,
    99_999_999,
    100_000_000,
    999_999_999,
    1_000_000_000,
];

#[derive(Clone, Copy, Debug)]
enum IntShape {
    Negative,
    Zero,
    Positive,
}

#[derive(Clone, Copy, Debug)]
enum FlagShape {
    Zero,
    LowOnly,
    HighOnly,
    PositiveMixed,
    NegativeMixed,
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"))
}

fn rust_library_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir().join(profile).join("libjumpnode_lib.so")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-oGS0vJ.so")
}

fn with_apis(test: impl FnOnce(Jumpnode, Jumpnode)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(
        c_path.is_file(),
        "C shared library missing: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library missing: {}",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        let rust_library = Library::new(&rust_path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
        let c_jumpnode: Symbol<Jumpnode> = c_library
            .get(b"jumpnode\0")
            .expect("C library does not export jumpnode");
        let rust_jumpnode: Symbol<Jumpnode> = rust_library
            .get(b"jumpnode\0")
            .expect("Rust library does not export jumpnode");
        test(*c_jumpnode, *rust_jumpnode);
    }
}

fn shaped_int(shape: IntShape, iteration: usize, rng: &mut Rng) -> i32 {
    match shape {
        IntShape::Negative => {
            if iteration == 0 {
                i32::MIN
            } else if iteration <= DECIMAL_BOUNDARIES.len() {
                -DECIMAL_BOUNDARIES[iteration - 1]
            } else {
                rng.next_i32() | i32::MIN
            }
        }
        IntShape::Zero => 0,
        IntShape::Positive => {
            if iteration == 0 {
                i32::MAX
            } else if iteration <= DECIMAL_BOUNDARIES.len() {
                DECIMAL_BOUNDARIES[iteration - 1]
            } else {
                (rng.next_i32() & i32::MAX).max(1)
            }
        }
    }
}

fn shaped_flags(shape: FlagShape, iteration: usize, rng: &mut Rng) -> i32 {
    let low = ((rng.next_u32() % 127) + 1) as i32;
    match shape {
        FlagShape::Zero => 0,
        FlagShape::LowOnly => {
            if iteration == 0 {
                127
            } else {
                low
            }
        }
        FlagShape::HighOnly => match iteration {
            0 => 128,
            1 => i32::MIN,
            _ => {
                let value = rng.next_i32() & !127;
                if value == 0 { 128 } else { value }
            }
        },
        FlagShape::PositiveMixed => match iteration {
            0 => i32::MAX,
            _ => (rng.next_i32() & i32::MAX & !127) | 128 | low,
        },
        FlagShape::NegativeMixed => match iteration {
            0 => -1,
            _ => (rng.next_i32() | i32::MIN) | low,
        },
    }
}

fn assert_same(
    row: usize,
    case: usize,
    c_jumpnode: Jumpnode,
    rust_jumpnode: Jumpnode,
    args: [i32; 4],
) -> i32 {
    let c_result = unsafe { c_jumpnode(args[0], args[1], args[2], args[3]) };
    let rust_result = unsafe { rust_jumpnode(args[0], args[1], args[2], args[3]) };
    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "row {row}, randomized case {case}, args {args:?}: C={c_result}, Rust={rust_result}"
    );
    c_result
}

#[test]
fn all_configuration_rows_match_across_randomized_inputs() {
    let int_shapes = [IntShape::Negative, IntShape::Zero, IntShape::Positive];
    let flag_shapes = [
        FlagShape::Zero,
        FlagShape::LowOnly,
        FlagShape::HighOnly,
        FlagShape::PositiveMixed,
        FlagShape::NegativeMixed,
    ];

    with_apis(|c_jumpnode, rust_jumpnode| {
        let mut row = 0;
        for node_shape in int_shapes {
            for depth_shape in int_shapes {
                for flag_shape in flag_shapes {
                    row += 1;
                    let mut rng = Rng::new(0x4a55_4d50_0000_0000 | row as u64);
                    for case in 0..CASES_PER_CONFIG {
                        let node_id = shaped_int(node_shape, case, &mut rng);
                        let depth = shaped_int(depth_shape, case, &mut rng);
                        let flags = shaped_flags(flag_shape, case, &mut rng);
                        assert_same(
                            row,
                            case,
                            c_jumpnode,
                            rust_jumpnode,
                            [0o3, node_id, depth, flags],
                        );
                    }
                }
            }
        }
        assert_eq!(row, 45, "CONFIGS.md row count changed");
    });
}

#[test]
fn every_error_surface_row_matches_exactly() {
    let error_rows = [(1, 0o1, 0o22), (2, 0o2, 0o42), (3, 0o4, 0o102)];

    with_apis(|c_jumpnode, rust_jumpnode| {
        for (row, mode, expected) in error_rows {
            let mut rng = Rng::new(0x4552_524f_5200_0000 | row as u64);
            for case in 0..CASES_PER_CONFIG {
                let args = [mode, rng.next_i32(), rng.next_i32(), rng.next_i32()];
                let actual = assert_same(row, case, c_jumpnode, rust_jumpnode, args);
                assert_eq!(actual, expected, "ERRORS.md row {row}, args {args:?}");
            }
        }

        let boundary_modes = [0, 5, -1, i32::MIN, i32::MAX];
        for (case, mode) in boundary_modes.into_iter().enumerate() {
            let args = [mode, i32::MIN, i32::MAX, -1];
            let actual = assert_same(4, case, c_jumpnode, rust_jumpnode, args);
            assert_eq!(actual, 0o202, "ERRORS.md row 4, args {args:?}");
        }

        let mut rng = Rng::new(0x4552_524f_5200_0004);
        for case in boundary_modes.len()..CASES_PER_CONFIG {
            let mut mode = rng.next_i32();
            while (1..=4).contains(&mode) {
                mode = rng.next_i32();
            }
            let args = [mode, rng.next_i32(), rng.next_i32(), rng.next_i32()];
            let actual = assert_same(4, case, c_jumpnode, rust_jumpnode, args);
            assert_eq!(actual, 0o202, "ERRORS.md row 4, args {args:?}");
        }
    });
}

#[test]
fn generic_integer_boundaries_match() {
    let boundary_args = [
        [0, 0, 0, 0],
        [5, 0, 0, 0],
        [-1, -1, -1, -1],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [0o3, i32::MIN, i32::MIN, i32::MIN],
        [0o3, i32::MAX, i32::MAX, i32::MAX],
        [0o3, 0, 0, 128],
        [0o3, 0, 0, 129],
    ];

    with_apis(|c_jumpnode, rust_jumpnode| {
        for (case, args) in boundary_args.into_iter().enumerate() {
            assert_same(0, case, c_jumpnode, rust_jumpnode, args);
        }
    });
}
