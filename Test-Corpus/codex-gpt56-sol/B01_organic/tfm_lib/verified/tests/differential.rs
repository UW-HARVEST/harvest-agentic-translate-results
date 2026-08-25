use libloading::Library;
use std::ffi::{c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Tfm = unsafe extern "C" fn(*mut f32, *const f32, c_int);

const C_SO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/c_src/build/libtranslated_rust.so"
);
const CASES: usize = 256;

#[derive(Clone, Copy)]
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

    fn bounded_finite(&mut self) -> f32 {
        let numerator = (self.next_u32() % 2_000_001) as i32 - 1_000_000;
        numerator as f32 / 997.0
    }

    fn positive_delta(&mut self) -> f32 {
        (self.next_u32() % 100_000 + 1) as f32 / 991.0
    }

    fn quiet_nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x003f_ffff).max(1);
        f32::from_bits(sign | 0x7fc0_0000 | payload)
    }
}

fn rust_so() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let profile_directory = executable
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory");
    let profile_library = profile_directory.join("libtfm_lib.so");
    if profile_library.is_file() {
        return profile_library;
    }

    let release_library = profile_directory
        .parent()
        .expect("Cargo target directory")
        .join("release/libtfm_lib.so");
    assert!(
        release_library.is_file(),
        "Rust cdylib not found; build it before running integration tests: {}",
        release_library.display()
    );
    release_library
}

fn load_tfm(path: &Path) -> (Library, Tfm) {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let function = unsafe {
        *library
            .get::<Tfm>(b"tfm\0")
            .unwrap_or_else(|error| panic!("failed to load tfm from {}: {error}", path.display()))
    };
    (library, function)
}

fn with_tfms<R>(run: impl FnOnce(Tfm, Tfm) -> R) -> R {
    let (_c_library, c_tfm) = load_tfm(Path::new(C_SO));
    let rust_path = rust_so();
    let (_rust_library, rust_tfm) = load_tfm(&rust_path);
    run(c_tfm, rust_tfm)
}

fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn assert_disjoint(c_tfm: Tfm, rust_tfm: Tfm, source: &[f32], label: &str) {
    assert_eq!(source.len() % 3, 0, "{label}: malformed source");
    let count = source.len() / 3;
    let mut c_destination = vec![f32::from_bits(0x7fa5_a5a5); count * 2 + 4];
    let mut rust_destination = c_destination.clone();

    unsafe {
        c_tfm(
            c_destination.as_mut_ptr().add(2),
            source.as_ptr(),
            count as c_int,
        );
        rust_tfm(
            rust_destination.as_mut_ptr().add(2),
            source.as_ptr(),
            count as c_int,
        );
    }

    assert_eq!(
        bits(&c_destination),
        bits(&rust_destination),
        "{label}: source bits={:08x?}",
        bits(source)
    );
}

fn assert_overlap(
    c_tfm: Tfm,
    rust_tfm: Tfm,
    initial: &[f32],
    destination_offset: usize,
    source_offset: usize,
    count: usize,
    label: &str,
) {
    let required = (destination_offset + count * 2).max(source_offset + count * 3);
    assert!(
        initial.len() >= required,
        "{label}: backing buffer too short"
    );
    let mut c_buffer = initial.to_vec();
    let mut rust_buffer = initial.to_vec();

    unsafe {
        c_tfm(
            c_buffer.as_mut_ptr().add(destination_offset),
            c_buffer.as_ptr().add(source_offset),
            count as c_int,
        );
        rust_tfm(
            rust_buffer.as_mut_ptr().add(destination_offset),
            rust_buffer.as_ptr().add(source_offset),
            count as c_int,
        );
    }

    assert_eq!(
        bits(&c_buffer),
        bits(&rust_buffer),
        "{label}: initial bits={:08x?}",
        bits(initial)
    );
}

fn local_sqd(source: [f32; 3]) -> f32 {
    let (dy2, dx2) = if source[0] < source[1] {
        (source[1], source[0])
    } else {
        (source[0], source[1])
    };
    (dy2 * dy2) - (2.0 * dx2 * dy2) + (dx2 * dx2) + (4.0 * source[2] * source[2])
}

fn randomized_negative_sqd_cases(base: [u32; 3], first_branch: bool, seed: u64) -> Vec<[f32; 3]> {
    let mut rng = Rng::new(seed);
    let mut cases = Vec::new();
    for _ in 0..10_000 {
        let source = base.map(|bits| {
            let delta = (rng.next_u32() % 8193) as i32 - 4096;
            f32::from_bits(bits.wrapping_add_signed(delta))
        });
        if (source[0] < source[1]) == first_branch && local_sqd(source) < 0.0 {
            cases.push(source);
            if cases.len() == 128 {
                break;
            }
        }
    }
    assert_eq!(cases.len(), 128, "insufficient rounded-negative sqd cases");
    cases
}

fn huge_finite(rng: &mut Rng) -> f32 {
    let sign = rng.next_u32() & 0x8000_0000;
    let mantissa = rng.next_u32() & 0x007f_ffff;
    f32::from_bits(sign | 0x7e80_0000 | mantissa)
}

#[test]
fn config_01_single_finite_first_branch() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x01_5446_4d);
        for case in 0..CASES {
            let first = rng.bounded_finite();
            let source = [first, first + rng.positive_delta(), rng.bounded_finite()];
            assert!(source[0] < source[1] && local_sqd(source) >= 0.0);
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 1 case {case}"));
        }
    });
}

#[test]
fn config_02_single_finite_second_branch() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x02_5446_4d);
        for case in 0..CASES {
            let second = rng.bounded_finite();
            let source = [second + rng.positive_delta(), second, rng.bounded_finite()];
            assert!(source[0] > source[1] && local_sqd(source) >= 0.0);
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 2 case {case}"));
        }
    });
}

#[test]
fn config_03_single_equal_selects_second_branch() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x03_5446_4d);
        for case in 0..CASES {
            let equal = rng.bounded_finite();
            let source = [equal, equal, rng.bounded_finite()];
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 3 case {case}"));
        }
    });
}

#[test]
fn config_04_first_branch_negative_sqd_clamp() {
    with_tfms(|c_tfm, rust_tfm| {
        let cases = randomized_negative_sqd_cases(
            [0x9a59_b08e, 0x99f6_a322, 0x0e1f_de55],
            true,
            0x04_5446_4d,
        );
        for (case, source) in cases.iter().enumerate() {
            assert_disjoint(c_tfm, rust_tfm, source, &format!("config 4 case {case}"));
        }
    });
}

#[test]
fn config_05_second_branch_negative_sqd_clamp() {
    with_tfms(|c_tfm, rust_tfm| {
        let cases = randomized_negative_sqd_cases(
            [0x9996_2da8, 0x99df_d4b9, 0x9754_b3fd],
            false,
            0x05_5446_4d,
        );
        for (case, source) in cases.iter().enumerate() {
            assert_disjoint(c_tfm, rust_tfm, source, &format!("config 5 case {case}"));
        }
    });
}

#[test]
fn config_06_nan_in_first_comparison_operand() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x06_5446_4d);
        for case in 0..CASES {
            let source = [rng.quiet_nan(), rng.bounded_finite(), rng.bounded_finite()];
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 6 case {case}"));
        }
    });
}

#[test]
fn config_07_nan_in_second_comparison_operand() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x07_5446_4d);
        for case in 0..CASES {
            let source = [rng.bounded_finite(), rng.quiet_nan(), rng.bounded_finite()];
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 7 case {case}"));
        }
    });
}

#[test]
fn config_08_nan_in_dxy_bypasses_clamp() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x08_5446_4d);
        for case in 0..CASES {
            let source = [rng.bounded_finite(), rng.bounded_finite(), rng.quiet_nan()];
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 8 case {case}"));
        }
    });
}

#[test]
fn config_09_signed_zero_and_subnormal_values() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x09_5446_4d);
        for case in 0..CASES {
            let mut value = || {
                let sign = rng.next_u32() & 0x8000_0000;
                let magnitude = if rng.next_u32() & 3 == 0 {
                    0
                } else {
                    (rng.next_u32() & 0x007f_ffff).max(1)
                };
                f32::from_bits(sign | magnitude)
            };
            let source = [value(), value(), value()];
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 9 case {case}"));
        }
    });
}

#[test]
fn config_10_infinite_and_overflowing_intermediates() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x10_5446_4d);
        for case in 0..CASES {
            let infinity = f32::from_bits((rng.next_u32() & 0x8000_0000) | 0x7f80_0000);
            let source = match case % 4 {
                0 => [infinity, huge_finite(&mut rng), huge_finite(&mut rng)],
                1 => [huge_finite(&mut rng), infinity, huge_finite(&mut rng)],
                2 => [huge_finite(&mut rng), huge_finite(&mut rng), infinity],
                _ => [
                    huge_finite(&mut rng),
                    huge_finite(&mut rng),
                    huge_finite(&mut rng),
                ],
            };
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 10 case {case}"));
        }
    });
}

#[test]
fn config_11_many_items_all_first_branch() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x11_5446_4d);
        for case in 0..128 {
            let count = 2 + (rng.next_u32() as usize % 31);
            let mut source = Vec::with_capacity(count * 3);
            for _ in 0..count {
                let first = rng.bounded_finite();
                source.extend([first, first + rng.positive_delta(), rng.bounded_finite()]);
            }
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 11 case {case}"));
        }
    });
}

#[test]
fn config_12_many_items_all_second_branch() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x12_5446_4d);
        for case in 0..128 {
            let count = 2 + (rng.next_u32() as usize % 31);
            let mut source = Vec::with_capacity(count * 3);
            for _ in 0..count {
                let second = rng.bounded_finite();
                source.extend([second + rng.positive_delta(), second, rng.bounded_finite()]);
            }
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 12 case {case}"));
        }
    });
}

#[test]
fn config_13_many_items_mixed_classes() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x13_5446_4d);
        for case in 0..128 {
            let count = 8 + (rng.next_u32() as usize % 25);
            let mut source = Vec::with_capacity(count * 3);
            for item in 0..count {
                let triple = match item % 8 {
                    0 => {
                        let first = rng.bounded_finite();
                        [first, first + rng.positive_delta(), rng.bounded_finite()]
                    }
                    1 => {
                        let second = rng.bounded_finite();
                        [second + rng.positive_delta(), second, rng.bounded_finite()]
                    }
                    2 => [rng.quiet_nan(), rng.bounded_finite(), rng.bounded_finite()],
                    3 => [rng.bounded_finite(), rng.quiet_nan(), rng.bounded_finite()],
                    4 => [rng.bounded_finite(), rng.bounded_finite(), rng.quiet_nan()],
                    5 => [f32::INFINITY, f32::NEG_INFINITY, rng.bounded_finite()],
                    6 => [
                        f32::from_bits((rng.next_u32() & 0x807f_ffff).max(1)),
                        0.0,
                        -0.0,
                    ],
                    _ => [
                        f32::from_bits(rng.next_u32()),
                        f32::from_bits(rng.next_u32()),
                        f32::from_bits(rng.next_u32()),
                    ],
                };
                source.extend(triple);
            }
            assert_disjoint(c_tfm, rust_tfm, &source, &format!("config 13 case {case}"));
        }
    });
}

fn random_backing(rng: &mut Rng, length: usize) -> Vec<f32> {
    (0..length)
        .map(|_| f32::from_bits(rng.next_u32()))
        .collect()
}

#[test]
fn config_14_exact_alias() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x14_5446_4d);
        for case in 0..128 {
            let count = 2 + (rng.next_u32() as usize % 31);
            let initial = random_backing(&mut rng, count * 3 + 4);
            assert_overlap(
                c_tfm,
                rust_tfm,
                &initial,
                0,
                0,
                count,
                &format!("config 14 case {case}"),
            );
        }
    });
}

#[test]
fn config_15_forward_overlap_mutates_future_source() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x15_5446_4d);
        for case in 0..128 {
            let count = 2 + (rng.next_u32() as usize % 31);
            let initial = random_backing(&mut rng, (2 + count * 2).max(count * 3) + 4);
            assert_overlap(
                c_tfm,
                rust_tfm,
                &initial,
                2,
                0,
                count,
                &format!("config 15 case {case}"),
            );
        }
    });
}

#[test]
fn config_16_backward_overlap() {
    with_tfms(|c_tfm, rust_tfm| {
        let mut rng = Rng::new(0x16_5446_4d);
        for case in 0..128 {
            let count = 2 + (rng.next_u32() as usize % 31);
            let initial = random_backing(&mut rng, 1 + count * 3 + 4);
            assert_overlap(
                c_tfm,
                rust_tfm,
                &initial,
                0,
                1,
                count,
                &format!("config 16 case {case}"),
            );
        }
    });
}

#[test]
fn error_g1_null_pointers_zero_count() {
    with_tfms(|c_tfm, rust_tfm| unsafe {
        c_tfm(std::ptr::null_mut(), std::ptr::null(), 0);
        rust_tfm(std::ptr::null_mut(), std::ptr::null(), 0);
    });
}

#[test]
fn error_g2_null_pointers_int_min_count() {
    with_tfms(|c_tfm, rust_tfm| unsafe {
        c_tfm(std::ptr::null_mut(), std::ptr::null(), c_int::MIN);
        rust_tfm(std::ptr::null_mut(), std::ptr::null(), c_int::MIN);
    });
}

fn assert_unchanged_for_nonpositive_count(count: c_int, label: &str) {
    with_tfms(|c_tfm, rust_tfm| {
        let source = [1.0_f32, 2.0, 3.0];
        let initial = [f32::from_bits(0x7fc1_2345), f32::from_bits(0xffc5_4321)];
        let mut c_destination = initial;
        let mut rust_destination = initial;
        unsafe {
            c_tfm(c_destination.as_mut_ptr(), source.as_ptr(), count);
            rust_tfm(rust_destination.as_mut_ptr(), source.as_ptr(), count);
        }
        assert_eq!(
            bits(&c_destination),
            bits(&initial),
            "{label}: C changed output"
        );
        assert_eq!(
            bits(&rust_destination),
            bits(&initial),
            "{label}: Rust changed output"
        );
    });
}

#[test]
fn error_g3_negative_count_leaves_destination_unchanged() {
    assert_unchanged_for_nonpositive_count(-1, "G3");
}

#[test]
fn error_g4_zero_count_leaves_destination_unchanged() {
    assert_unchanged_for_nonpositive_count(0, "G4");
}

#[cfg(unix)]
fn run_crash_probe(library: &Path, probe: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "crash_probe_child", "--nocapture"])
        .env("TFM_CRASH_LIBRARY", library)
        .env("TFM_CRASH_PROBE", probe)
        .status()
        .unwrap_or_else(|error| panic!("failed to run {probe} child: {error}"))
}

#[cfg(unix)]
fn assert_matching_sigsegv(probe: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_crash_probe(Path::new(C_SO), probe);
    let rust_status = run_crash_probe(&rust_so(), probe);
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "{probe}: C={c_status:?}, Rust={rust_status:?}"
    );
    assert_eq!(c_status.signal(), Some(11), "{probe}: expected SIGSEGV");
}

#[test]
#[cfg(unix)]
fn error_g5_null_destination_positive_count() {
    assert_matching_sigsegv("null_destination");
}

#[test]
#[cfg(unix)]
fn error_g6_null_source_positive_count() {
    assert_matching_sigsegv("null_source");
}

#[test]
#[cfg(unix)]
fn error_g7_oversized_count_reaches_guard_page() {
    assert_matching_sigsegv("guard_page");
}

#[test]
fn crash_probe_child() {
    let Ok(probe) = std::env::var("TFM_CRASH_PROBE") else {
        return;
    };
    let library_path = std::env::var_os("TFM_CRASH_LIBRARY").expect("crash library path");
    let (_library, tfm) = load_tfm(Path::new(&library_path));

    match probe.as_str() {
        "null_destination" => {
            let source = [1.0_f32, 2.0, 3.0];
            unsafe { tfm(std::ptr::null_mut(), source.as_ptr(), 1) };
        }
        "null_source" => {
            let mut destination = [0.0_f32; 2];
            unsafe { tfm(destination.as_mut_ptr(), std::ptr::null(), 1) };
        }
        "guard_page" => unsafe { run_guard_page_probe(tfm) },
        other => panic!("unknown crash probe {other}"),
    }

    panic!("{probe} unexpectedly returned");
}

#[cfg(unix)]
unsafe fn run_guard_page_probe(tfm: Tfm) {
    const PROT_NONE: c_int = 0;
    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;
    const MAP_PRIVATE: c_int = 2;
    const MAP_ANONYMOUS: c_int = 0x20;

    unsafe extern "C" {
        fn getpagesize() -> c_int;
        fn mmap(
            address: *mut c_void,
            length: usize,
            protection: c_int,
            flags: c_int,
            descriptor: c_int,
            offset: c_long,
        ) -> *mut c_void;
        fn mprotect(address: *mut c_void, length: usize, protection: c_int) -> c_int;
    }

    let page_size = unsafe { getpagesize() } as usize;
    let mapping = unsafe {
        mmap(
            std::ptr::null_mut(),
            page_size * 2,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(mapping as usize, usize::MAX, "mmap failed");
    assert_eq!(
        unsafe {
            mprotect(
                (mapping as *mut u8).add(page_size).cast(),
                page_size,
                PROT_NONE,
            )
        },
        0,
        "mprotect failed"
    );

    let source = unsafe {
        (mapping as *mut u8)
            .add(page_size - 3 * size_of::<f32>())
            .cast::<f32>()
    };
    unsafe {
        source.write(1.0);
        source.add(1).write(2.0);
        source.add(2).write(3.0);
    }
    let mut destination = [0.0_f32; 4];
    unsafe { tfm(destination.as_mut_ptr(), source, 2) };
}
