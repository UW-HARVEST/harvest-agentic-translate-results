use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};

type LdexpQ2 = unsafe extern "C" fn(c_float, c_int) -> c_float;

const RANDOM_CASES: usize = 2_048;
const EDGE_FLOAT_BITS: &[u32] = &[
    0x0000_0000,
    0x8000_0000,
    0x0000_0001,
    0x8000_0001,
    0x007f_ffff,
    0x807f_ffff,
    0x0080_0000,
    0x8080_0000,
    0x3f80_0000,
    0xbf80_0000,
    0x7f7f_ffff,
    0xff7f_ffff,
    0x7f80_0000,
    0xff80_0000,
    0x7f80_0001,
    0xff80_0001,
    0x7fc0_0000,
    0xffc0_0000,
    0x7fff_ffff,
    0xffff_ffff,
];

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn below(&mut self, upper: u32) -> u32 {
        self.next_u32() % upper
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("test executable path");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    let library_name = format!(
        "{}ldexp_q2_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let profile_library = profile_dir.join(&library_name);
    if profile_library.is_file() {
        return profile_library;
    }

    let release_library = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release")
        .join(library_name);
    if release_library.is_file() {
        return release_library;
    }

    panic!(
        "missing Rust cdylib; build it before testing (checked {} and {})",
        profile_library.display(),
        release_library.display()
    );
}

fn assert_same(row: &str, case: usize, y_bits: u32, exp_q2: i32, c_result: f32, rust_result: f32) {
    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "{row} case {case}: y=0x{y_bits:08x}, exp_q2={exp_q2}, \
         C=0x{:08x}, Rust=0x{:08x}",
        c_result.to_bits(),
        rust_result.to_bits()
    );
}

fn compare_row(row: &str, seed: u64, mut exponent: impl FnMut(&mut Rng, usize) -> i32) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
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
        let c_ldexp: Symbol<'_, LdexpQ2> = c_library.get(b"ldexp_q2\0").expect("load C ldexp_q2");
        let rust_ldexp: Symbol<'_, LdexpQ2> =
            rust_library.get(b"ldexp_q2\0").expect("load Rust ldexp_q2");
        let mut rng = Rng::new(seed);

        for case in 0..RANDOM_CASES {
            let y_bits = EDGE_FLOAT_BITS
                .get(case)
                .copied()
                .unwrap_or_else(|| rng.next_u32());
            let exp_q2 = exponent(&mut rng, case);
            let y = f32::from_bits(y_bits);
            let c_result = c_ldexp(y, exp_q2);
            let rust_result = rust_ldexp(y, exp_q2);
            assert_same(row, case, y_bits, exp_q2, c_result, rust_result);
        }
    }
}

fn negative_with_index(rng: &mut Rng, index: i32) -> i32 {
    let negative = (rng.next_u32() | 0x8000_0000) as i32;
    (negative & !3) | index
}

fn positive_below_120_with_index(rng: &mut Rng, index: i32) -> i32 {
    let first = if index == 0 { 4 } else { index };
    first + 4 * rng.below(((119 - first) / 4 + 1) as u32) as i32
}

fn repeated_with_terminal_index(rng: &mut Rng, index: i32) -> i32 {
    let full_chunks = 1 + rng.below(64) as i32;
    120 * full_chunks + positive_below_120_with_index(rng, index)
}

#[test]
fn c1_negative_table_index_0() {
    compare_row("C1", 0xc100_0000_0000_0001, |rng, case| {
        if case == 0 {
            i32::MIN
        } else {
            negative_with_index(rng, 0)
        }
    });
}

#[test]
fn c2_negative_table_index_1() {
    compare_row("C2", 0xc200_0000_0000_0001, |rng, _| {
        negative_with_index(rng, 1)
    });
}

#[test]
fn c3_negative_table_index_2() {
    compare_row("C3", 0xc300_0000_0000_0001, |rng, _| {
        negative_with_index(rng, 2)
    });
}

#[test]
fn c4_negative_table_index_3() {
    compare_row("C4", 0xc400_0000_0000_0001, |rng, _| {
        negative_with_index(rng, 3)
    });
}

#[test]
fn c5_zero_exponent() {
    compare_row("C5", 0xc500_0000_0000_0001, |_, _| 0);
}

#[test]
fn c6_single_chunk_table_index_0() {
    compare_row("C6", 0xc600_0000_0000_0001, |rng, case| match case {
        0 => 4,
        1 => 116,
        _ => positive_below_120_with_index(rng, 0),
    });
}

#[test]
fn c7_single_chunk_table_index_1() {
    compare_row("C7", 0xc700_0000_0000_0001, |rng, case| match case {
        0 => 1,
        1 => 117,
        _ => positive_below_120_with_index(rng, 1),
    });
}

#[test]
fn c8_single_chunk_table_index_2() {
    compare_row("C8", 0xc800_0000_0000_0001, |rng, case| match case {
        0 => 2,
        1 => 118,
        _ => positive_below_120_with_index(rng, 2),
    });
}

#[test]
fn c9_single_chunk_table_index_3() {
    compare_row("C9", 0xc900_0000_0000_0001, |rng, case| match case {
        0 => 3,
        1 => 119,
        _ => positive_below_120_with_index(rng, 3),
    });
}

#[test]
fn c10_exact_chunk_cap() {
    compare_row("C10", 0xca00_0000_0000_0001, |_, _| 120);
}

#[test]
fn c11_repeated_terminal_remainder_0() {
    compare_row("C11", 0xcb00_0000_0000_0001, |rng, case| {
        if case == 0 {
            240
        } else {
            120 * (2 + rng.below(64) as i32)
        }
    });
}

#[test]
fn c12_repeated_terminal_table_index_1() {
    compare_row("C12", 0xcc00_0000_0000_0001, |rng, case| {
        if case == 0 {
            121
        } else {
            repeated_with_terminal_index(rng, 1)
        }
    });
}

#[test]
fn c13_repeated_terminal_table_index_2() {
    compare_row("C13", 0xcd00_0000_0000_0001, |rng, case| {
        if case == 0 {
            122
        } else {
            repeated_with_terminal_index(rng, 2)
        }
    });
}

#[test]
fn c14_repeated_terminal_table_index_3() {
    compare_row("C14", 0xce00_0000_0000_0001, |rng, case| {
        if case == 0 {
            123
        } else {
            repeated_with_terminal_index(rng, 3)
        }
    });
}

#[test]
fn c15_repeated_terminal_table_index_0() {
    compare_row("C15", 0xcf00_0000_0000_0001, |rng, case| {
        if case == 0 {
            124
        } else {
            repeated_with_terminal_index(rng, 0)
        }
    });
}

#[test]
fn c16_large_positive_and_int_max() {
    compare_row("C16", 0xd000_0000_0000_0001, |rng, _| {
        7_680 + rng.below(192_321) as i32
    });

    unsafe {
        let c_library = Library::new(c_library_path()).expect("load C shared library");
        let rust_library = Library::new(rust_library_path()).expect("load Rust shared library");
        let c_ldexp: Symbol<'_, LdexpQ2> = c_library.get(b"ldexp_q2\0").expect("load C ldexp_q2");
        let rust_ldexp: Symbol<'_, LdexpQ2> =
            rust_library.get(b"ldexp_q2\0").expect("load Rust ldexp_q2");
        let y_bits = 0x3f80_0000;
        let c_result = c_ldexp(f32::from_bits(y_bits), i32::MAX);
        let rust_result = rust_ldexp(f32::from_bits(y_bits), i32::MAX);
        assert_same("C16", RANDOM_CASES, y_bits, i32::MAX, c_result, rust_result);
    }
}
