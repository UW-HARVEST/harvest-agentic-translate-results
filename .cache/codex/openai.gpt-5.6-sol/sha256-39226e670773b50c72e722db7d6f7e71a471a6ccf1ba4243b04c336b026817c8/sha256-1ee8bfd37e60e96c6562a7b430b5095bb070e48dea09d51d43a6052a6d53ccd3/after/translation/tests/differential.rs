use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

type Normalize = unsafe extern "C" fn(*mut f32, *const f32, c_int);

struct Api {
    _library: Library,
    normalize: Normalize,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let normalize = unsafe {
            *library
                .get::<Normalize>(b"normalize\0")
                .unwrap_or_else(|error| {
                    panic!("failed to resolve normalize in {}: {error}", path.display())
                })
        };
        Self {
            _library: library,
            normalize,
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-PdYL9r.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libnormalize_lib.so")
}

fn c_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| Api::load(&c_library_path()))
}

fn rust_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| Api::load(&rust_library_path()))
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn len(&mut self) -> usize {
        2 + (self.next_u32() as usize % 63)
    }

    fn finite_nonzero(&mut self) -> f32 {
        let magnitude = 1 + self.next_u32() % 1_000_000;
        let sign = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        sign * magnitude as f32 / 257.0
    }

    fn initial_dest(&mut self, len: usize) -> Vec<f32> {
        (0..len).map(|_| f32::from_bits(self.next_u32())).collect()
    }

    fn nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x007f_ffff) | 1;
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }

    fn subnormal(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let mantissa = (self.next_u32() & 0x007f_ffff) | 1;
        f32::from_bits(sign | mantissa)
    }
}

fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

fn assert_bits_eq(c: &[f32], rust: &[f32], context: &str) {
    assert_eq!(bits(c), bits(rust), "{context}");
}

fn compare_alias(source: &[f32]) {
    let mut c_buffer = source.to_vec();
    let mut rust_buffer = source.to_vec();
    let size = source.len() as c_int;
    unsafe {
        (c_api().normalize)(c_buffer.as_mut_ptr(), c_buffer.as_ptr(), size);
        (rust_api().normalize)(rust_buffer.as_mut_ptr(), rust_buffer.as_ptr(), size);
    }
    assert_bits_eq(&c_buffer, &rust_buffer, "aliased output differs");
}

fn compare_distinct(source: &[f32], initial_dest: &[f32]) {
    assert_eq!(source.len(), initial_dest.len());
    let c_source = source.to_vec();
    let rust_source = source.to_vec();
    let mut c_dest = initial_dest.to_vec();
    let mut rust_dest = initial_dest.to_vec();
    let size = source.len() as c_int;
    unsafe {
        (c_api().normalize)(c_dest.as_mut_ptr(), c_source.as_ptr(), size);
        (rust_api().normalize)(rust_dest.as_mut_ptr(), rust_source.as_ptr(), size);
    }
    assert_bits_eq(&c_dest, &rust_dest, "distinct output differs");
    assert_bits_eq(&c_source, &rust_source, "distinct source differs");
}

fn repeat_alias(seed: u64, mut generate: impl FnMut(&mut Rng) -> Vec<f32>) {
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        compare_alias(&generate(&mut rng));
    }
}

fn repeat_distinct(seed: u64, mut generate: impl FnMut(&mut Rng) -> Vec<f32>) {
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let source = generate(&mut rng);
        let dest = rng.initial_dest(source.len());
        compare_distinct(&source, &dest);
    }
}

fn one_finite(rng: &mut Rng) -> Vec<f32> {
    vec![rng.finite_nonzero()]
}

fn one_zero(rng: &mut Rng) -> Vec<f32> {
    vec![f32::from_bits(rng.next_u32() & 0x8000_0000)]
}

fn one_subnormal(rng: &mut Rng) -> Vec<f32> {
    vec![rng.subnormal()]
}

fn one_nan(rng: &mut Rng) -> Vec<f32> {
    vec![rng.nan()]
}

fn one_infinity(rng: &mut Rng) -> Vec<f32> {
    vec![if rng.next_u32() & 1 == 0 {
        f32::INFINITY
    } else {
        f32::NEG_INFINITY
    }]
}

fn many_finite(rng: &mut Rng) -> Vec<f32> {
    let len = rng.len();
    (0..len).map(|_| rng.finite_nonzero()).collect()
}

fn many_zeros(rng: &mut Rng) -> Vec<f32> {
    let len = rng.len();
    (0..len)
        .map(|_| f32::from_bits(rng.next_u32() & 0x8000_0000))
        .collect()
}

fn many_subnormals(rng: &mut Rng) -> Vec<f32> {
    let len = rng.len();
    (0..len).map(|_| rng.subnormal()).collect()
}

fn many_overflowing_finite(rng: &mut Rng) -> Vec<f32> {
    let len = 4 + (rng.next_u32() as usize % 61);
    (0..len)
        .map(|_| {
            if rng.next_u32() & 1 == 0 {
                1.0e19_f32
            } else {
                -1.0e19_f32
            }
        })
        .collect()
}

fn many_with_nan(rng: &mut Rng) -> Vec<f32> {
    let mut values = many_finite(rng);
    let index = rng.next_u32() as usize % values.len();
    values[index] = rng.nan();
    values
}

fn many_with_infinity(rng: &mut Rng) -> Vec<f32> {
    let mut values = many_finite(rng);
    let index = rng.next_u32() as usize % values.len();
    values[index] = if rng.next_u32() & 1 == 0 {
        f32::INFINITY
    } else {
        f32::NEG_INFINITY
    };
    values
}

#[test]
fn config_c01_empty_alias() {
    let mut rng = Rng::new(0xc01);
    for _ in 0..128 {
        let len = 1 + rng.next_u32() as usize % 32;
        let original = rng.initial_dest(len);
        let mut c = original.clone();
        let mut rust = original.clone();
        unsafe {
            (c_api().normalize)(c.as_mut_ptr(), c.as_ptr(), 0);
            (rust_api().normalize)(rust.as_mut_ptr(), rust.as_ptr(), 0);
        }
        assert_bits_eq(&c, &rust, "empty aliased buffers differ");
        assert_bits_eq(&c, &original, "C changed an empty aliased buffer");
    }
}

#[test]
fn config_c02_empty_distinct() {
    let mut rng = Rng::new(0xc02);
    for _ in 0..128 {
        let len = 1 + rng.next_u32() as usize % 32;
        let source = rng.initial_dest(len);
        let original_dest = rng.initial_dest(len);
        let mut c_dest = original_dest.clone();
        let mut rust_dest = original_dest.clone();
        unsafe {
            (c_api().normalize)(c_dest.as_mut_ptr(), source.as_ptr(), 0);
            (rust_api().normalize)(rust_dest.as_mut_ptr(), source.as_ptr(), 0);
        }
        assert_bits_eq(&c_dest, &rust_dest, "empty distinct buffers differ");
        assert_bits_eq(&c_dest, &original_dest, "C changed an empty destination");
    }
}

#[test]
fn config_c03_one_finite_alias() {
    repeat_alias(0xc03, one_finite);
}

#[test]
fn config_c04_one_finite_distinct() {
    repeat_distinct(0xc04, one_finite);
}

#[test]
fn config_c05_one_zero_alias() {
    repeat_alias(0xc05, one_zero);
}

#[test]
fn config_c06_one_zero_distinct() {
    repeat_distinct(0xc06, one_zero);
}

#[test]
fn config_c07_one_subnormal_alias() {
    repeat_alias(0xc07, one_subnormal);
}

#[test]
fn config_c08_one_subnormal_distinct() {
    repeat_distinct(0xc08, one_subnormal);
}

#[test]
fn config_c09_one_nan_alias() {
    repeat_alias(0xc09, one_nan);
}

#[test]
fn config_c10_one_nan_distinct() {
    repeat_distinct(0xc10, one_nan);
}

#[test]
fn config_c11_one_infinity_alias() {
    repeat_alias(0xc11, one_infinity);
}

#[test]
fn config_c12_one_infinity_distinct() {
    repeat_distinct(0xc12, one_infinity);
}

#[test]
fn config_c13_many_finite_alias() {
    repeat_alias(0xc13, many_finite);
}

#[test]
fn config_c14_many_finite_distinct() {
    repeat_distinct(0xc14, many_finite);
}

#[test]
fn config_c15_many_zeros_alias() {
    repeat_alias(0xc15, many_zeros);
}

#[test]
fn config_c16_many_zeros_distinct() {
    repeat_distinct(0xc16, many_zeros);
}

#[test]
fn config_c17_many_subnormals_alias() {
    repeat_alias(0xc17, many_subnormals);
}

#[test]
fn config_c18_many_subnormals_distinct() {
    repeat_distinct(0xc18, many_subnormals);
}

#[test]
fn config_c19_many_overflowing_finite_alias() {
    repeat_alias(0xc19, many_overflowing_finite);
}

#[test]
fn config_c20_many_overflowing_finite_distinct() {
    repeat_distinct(0xc20, many_overflowing_finite);
}

#[test]
fn config_c21_many_with_nan_alias() {
    repeat_alias(0xc21, many_with_nan);
}

#[test]
fn config_c22_many_with_nan_distinct() {
    repeat_distinct(0xc22, many_with_nan);
}

#[test]
fn config_c23_many_with_infinity_alias() {
    repeat_alias(0xc23, many_with_infinity);
}

#[test]
fn config_c24_many_with_infinity_distinct() {
    repeat_distinct(0xc24, many_with_infinity);
}

fn compare_partial_overlap(seed: u64, dest_offset: usize, src_offset: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..128 {
        let size = rng.len();
        let initial: Vec<f32> = (0..size + 1).map(|_| rng.finite_nonzero()).collect();
        let mut c = initial.clone();
        let mut rust = initial.clone();
        unsafe {
            let c_base = c.as_mut_ptr();
            let rust_base = rust.as_mut_ptr();
            (c_api().normalize)(
                c_base.add(dest_offset),
                c_base.add(src_offset),
                size as c_int,
            );
            (rust_api().normalize)(
                rust_base.add(dest_offset),
                rust_base.add(src_offset),
                size as c_int,
            );
        }
        assert_bits_eq(&c, &rust, "partially overlapping buffers differ");
    }
}

#[test]
fn config_c25_partial_overlap_dest_below_src() {
    compare_partial_overlap(0xc25, 0, 1);
}

#[test]
fn config_c26_partial_overlap_dest_above_src() {
    compare_partial_overlap(0xc26, 1, 0);
}

#[test]
fn boundary_b01_both_null_zero_size() {
    unsafe {
        (c_api().normalize)(std::ptr::null_mut(), std::ptr::null(), 0);
        (rust_api().normalize)(std::ptr::null_mut(), std::ptr::null(), 0);
    }
}

#[test]
fn boundary_b02_null_dest_zero_size() {
    let source = [1.0_f32];
    unsafe {
        (c_api().normalize)(std::ptr::null_mut(), source.as_ptr(), 0);
        (rust_api().normalize)(std::ptr::null_mut(), source.as_ptr(), 0);
    }
}

#[test]
fn boundary_b03_null_src_zero_size() {
    let mut c_dest = [f32::from_bits(0xdead_beef)];
    let mut rust_dest = c_dest;
    unsafe {
        (c_api().normalize)(c_dest.as_mut_ptr(), std::ptr::null(), 0);
        (rust_api().normalize)(rust_dest.as_mut_ptr(), std::ptr::null(), 0);
    }
    assert_bits_eq(&c_dest, &rust_dest, "zero-size null-source result differs");
}

#[test]
fn boundary_b06_negative_size_alias() {
    let original = [
        f32::from_bits(0x8000_0000),
        f32::from_bits(0x7fc0_1234),
        42.0,
    ];
    let mut c = original;
    let mut rust = original;
    unsafe {
        (c_api().normalize)(c.as_mut_ptr(), c.as_ptr(), -1);
        (rust_api().normalize)(rust.as_mut_ptr(), rust.as_ptr(), -1);
    }
    assert_bits_eq(&c, &rust, "negative-size aliased result differs");
    assert_bits_eq(&c, &original, "C changed a negative-size aliased buffer");
}

fn run_crash_child(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("crash_probe_child")
        .arg("--nocapture")
        .env("NORMALIZE_CRASH_LIBRARY", library)
        .env("NORMALIZE_CRASH_CASE", case)
        .status()
        .unwrap_or_else(|error| panic!("failed to launch crash probe: {error}"))
}

#[cfg(unix)]
fn assert_matching_crash(case: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_crash_child(&c_library_path(), case);
    let rust_status = run_crash_child(&rust_library_path(), case);
    assert!(!c_status.success(), "C unexpectedly returned for {case}");
    assert!(
        !rust_status.success(),
        "Rust unexpectedly returned for {case}"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "termination signal differs for {case}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
fn boundary_b04_null_dest_positive_size() {
    assert_matching_crash("null_dest_positive");
}

#[test]
fn boundary_b05_null_src_positive_size() {
    assert_matching_crash("null_src_positive");
}

#[test]
fn boundary_b07_negative_size_distinct() {
    assert_matching_crash("negative_distinct");
}

#[test]
fn boundary_b08_oversized_length() {
    assert_matching_crash("oversized");
}

#[test]
fn crash_probe_child() {
    let Ok(library_path) = std::env::var("NORMALIZE_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("NORMALIZE_CRASH_CASE").expect("crash case");
    let api = Api::load(Path::new(&library_path));
    let mut dest = [2.0_f32];
    let source = [1.0_f32];
    unsafe {
        match case.as_str() {
            "null_dest_positive" => (api.normalize)(std::ptr::null_mut(), source.as_ptr(), 1),
            "null_src_positive" => (api.normalize)(dest.as_mut_ptr(), std::ptr::null(), 1),
            "negative_distinct" => (api.normalize)(dest.as_mut_ptr(), source.as_ptr(), -1),
            "oversized" => (api.normalize)(dest.as_mut_ptr(), source.as_ptr(), c_int::MAX),
            _ => panic!("unknown crash case: {case}"),
        }
    }
}
