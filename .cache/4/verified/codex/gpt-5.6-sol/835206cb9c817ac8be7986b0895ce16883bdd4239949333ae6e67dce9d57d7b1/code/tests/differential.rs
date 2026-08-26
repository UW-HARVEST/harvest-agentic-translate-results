use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

type SynthPair = unsafe extern "C" fn(*mut i16, c_int, *const f32);

const Z_LEN: usize = 899;
const PCM_LEN: usize = 64;
const ITERATIONS_PER_CONFIG: usize = 256;

static RUST_LIBRARY_BUILT: OnceLock<()> = OnceLock::new();

const FIRST_TERMS: &[(usize, f32)] = &[
    (14 * 64, 29.0),
    (0, -29.0),
    (64, 213.0),
    (13 * 64, 213.0),
    (12 * 64, 459.0),
    (2 * 64, -459.0),
    (3 * 64, 2037.0),
    (11 * 64, 2037.0),
    (10 * 64, 5153.0),
    (4 * 64, -5153.0),
    (5 * 64, 6574.0),
    (9 * 64, 6574.0),
    (8 * 64, 37489.0),
    (6 * 64, -37489.0),
    (7 * 64, 75038.0),
];

const SECOND_TERMS: &[(usize, f32)] = &[
    (2 + 14 * 64, 104.0),
    (2 + 12 * 64, 1567.0),
    (2 + 10 * 64, 9727.0),
    (2 + 8 * 64, 64019.0),
    (2 + 6 * 64, -9975.0),
    (2 + 4 * 64, -45.0),
    (2 + 2 * 64, 146.0),
    (2, -5.0),
];

#[derive(Clone, Copy, Debug)]
enum Region {
    High,
    Low,
    Negative,
    Nonnegative,
}

impl Region {
    const ALL: [Self; 4] = [Self::High, Self::Low, Self::Negative, Self::Nonnegative];

    fn target(self, random: &mut XorShift32) -> f32 {
        let fraction = random.next_u32() as f32 / u32::MAX as f32;
        match self {
            Self::High => 40_000.0 + fraction * 1_000_000.0,
            Self::Low => -40_000.0 - fraction * 1_000_000.0,
            Self::Negative => -1.0 - fraction * 30_000.0,
            Self::Nonnegative => fraction * 30_000.0,
        }
    }

    fn assert_output(self, output: i16) {
        match self {
            Self::High => assert_eq!(output, i16::MAX),
            Self::Low => assert_eq!(output, i16::MIN),
            Self::Negative => assert!(output < 0 && output > i16::MIN),
            Self::Nonnegative => assert!((0..i16::MAX).contains(&output)),
        }
    }
}

struct XorShift32(u32);

impl XorShift32 {
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn index(&mut self, len: usize) -> usize {
        self.next_u32() as usize % len
    }
}

struct LoadedPair {
    c: Library,
    rust: Library,
}

impl LoadedPair {
    unsafe fn load() -> Self {
        ensure_rust_library_built();
        Self {
            c: unsafe { Library::new(c_library_path()) }
                .expect("failed to load the C shared object"),
            rust: unsafe { Library::new(rust_library_path()) }
                .expect("failed to load the Rust shared object"),
        }
    }

    unsafe fn functions(&self) -> (Symbol<'_, SynthPair>, Symbol<'_, SynthPair>) {
        (
            unsafe { self.c.get(b"synth_pair\0") }.expect("C synth_pair export is missing"),
            unsafe { self.rust.get(b"synth_pair\0") }.expect("Rust synth_pair export is missing"),
        )
    }
}

fn ensure_rust_library_built() {
    RUST_LIBRARY_BUILT.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let status = Command::new("timeout")
            .arg("600")
            .arg(cargo)
            .args(["build", "--no-default-features"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("failed to execute cargo build for the Rust cdylib");
        assert!(status.success(), "Rust cdylib build failed: {status}");
    });
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("test executable path is unavailable");
    executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is not under target/<profile>/deps")
        .join("libsynth_pair_lib.so")
}

fn inputs_for_regions(
    first_region: Region,
    second_region: Region,
    random: &mut XorShift32,
) -> Vec<f32> {
    let mut z = vec![0.0; Z_LEN];
    let (first_index, first_coefficient) = FIRST_TERMS[random.index(FIRST_TERMS.len())];
    let (second_index, second_coefficient) = SECOND_TERMS[random.index(SECOND_TERMS.len())];
    z[first_index] = first_region.target(random) / first_coefficient;
    z[second_index] = second_region.target(random) / second_coefficient;
    z
}

unsafe fn compare_call(
    c_function: &SynthPair,
    rust_function: &SynthPair,
    z: &[f32],
    nch: c_int,
) -> Vec<i16> {
    let mut c_pcm = vec![0xa5a5_u16 as i16; PCM_LEN];
    let mut rust_pcm = c_pcm.clone();
    unsafe {
        c_function(c_pcm.as_mut_ptr(), nch, z.as_ptr());
        rust_function(rust_pcm.as_mut_ptr(), nch, z.as_ptr());
    }
    assert_eq!(c_pcm, rust_pcm, "whole output buffer differs for nch={nch}");
    c_pcm
}

#[test]
fn every_configuration_row_matches_randomized_inputs() {
    let libraries = unsafe { LoadedPair::load() };
    let (c_function, rust_function) = unsafe { libraries.functions() };
    let mut random = XorShift32(0x243f_6a88);
    let mut rows = 0;

    for nch in [1, 2] {
        for first_region in Region::ALL {
            for second_region in Region::ALL {
                rows += 1;
                for _ in 0..ITERATIONS_PER_CONFIG {
                    let z = inputs_for_regions(first_region, second_region, &mut random);
                    let pcm = unsafe { compare_call(&c_function, &rust_function, &z, nch) };
                    first_region.assert_output(pcm[0]);
                    second_region.assert_output(pcm[16 * nch as usize]);
                }
            }
        }
    }

    assert_eq!(rows, 32);
}

#[test]
fn fully_randomized_inputs_match() {
    let libraries = unsafe { LoadedPair::load() };
    let (c_function, rust_function) = unsafe { libraries.functions() };
    let mut random = XorShift32(0x1319_8a2e);
    let mut z = vec![0.0; Z_LEN];

    for iteration in 0..20_000 {
        for value in &mut z {
            let centered = (random.next_u32() >> 8) as i32 - 8_388_608;
            *value = centered as f32 / (8_388_608.0 * 1024.0);
        }
        unsafe {
            compare_call(&c_function, &rust_function, &z, 1 + iteration % 3);
        }
    }
}

#[test]
fn arbitrary_float_bit_patterns_match() {
    let libraries = unsafe { LoadedPair::load() };
    let (c_function, rust_function) = unsafe { libraries.functions() };
    let mut random = XorShift32(0xa409_3822);
    let mut z = vec![0.0; Z_LEN];

    for iteration in 0..2_000 {
        for value in &mut z {
            *value = f32::from_bits(random.next_u32());
        }
        unsafe {
            compare_call(&c_function, &rust_function, &z, 1 + iteration % 3);
        }
    }
}

#[test]
fn saturation_boundaries_and_special_values_match() {
    let libraries = unsafe { LoadedPair::load() };
    let (c_function, rust_function) = unsafe { libraries.functions() };

    for (value, expected) in [
        (32766.5_f32, Some(i16::MAX)),
        (f32::from_bits(32766.5_f32.to_bits() - 1), None),
        (f32::from_bits(32766.5_f32.to_bits() + 1), None),
        (-32767.5_f32, Some(i16::MIN)),
        (f32::from_bits((-32767.5_f32).to_bits() - 1), None),
        (f32::from_bits((-32767.5_f32).to_bits() + 1), None),
        (f32::NAN, None),
        (f32::INFINITY, Some(i16::MAX)),
        (f32::NEG_INFINITY, Some(i16::MIN)),
    ] {
        let mut z = vec![0.0; Z_LEN];
        z[7 * 64] = value / 75038.0;
        z[2 + 8 * 64] = value / 64019.0;
        for nch in [1, 2] {
            let pcm = unsafe { compare_call(&c_function, &rust_function, &z, nch) };
            if let Some(expected) = expected {
                assert_eq!(pcm[0], expected);
                assert_eq!(pcm[16 * nch as usize], expected);
            }
        }
    }
}

#[test]
fn stride_boundaries_match() {
    let libraries = unsafe { LoadedPair::load() };
    let (c_function, rust_function) = unsafe { libraries.functions() };
    let z = vec![0.0; Z_LEN];

    for nch in [0, 3, 268_435_456] {
        unsafe {
            compare_call(&c_function, &rust_function, &z, nch);
        }
    }

    let mut c_pcm = vec![0xa5a5_u16 as i16; PCM_LEN];
    let mut rust_pcm = c_pcm.clone();
    unsafe {
        c_function(c_pcm.as_mut_ptr().add(32), -1, z.as_ptr());
        rust_function(rust_pcm.as_mut_ptr().add(32), -1, z.as_ptr());
    }
    assert_eq!(c_pcm, rust_pcm, "negative channel stride differs");
}

fn run_null_child(library: &str, pointer: &str) -> ExitStatus {
    ensure_rust_library_built();
    Command::new(std::env::current_exe().expect("test executable path is unavailable"))
        .arg("null_boundary_child")
        .arg("--exact")
        .env("DIFFERENTIAL_NULL_LIBRARY", library)
        .env("DIFFERENTIAL_NULL_POINTER", pointer)
        .status()
        .expect("failed to execute null-boundary child")
}

#[test]
fn null_pointer_failures_match() {
    for pointer in ["pcm", "z"] {
        let c_status = run_null_child("c", pointer);
        let rust_status = run_null_child("rust", pointer);
        assert!(
            !c_status.success(),
            "C unexpectedly accepted null {pointer}"
        );
        assert_eq!(
            c_status, rust_status,
            "C and Rust terminate differently for null {pointer}"
        );
    }
}

#[test]
fn null_boundary_child() {
    let Ok(library_kind) = std::env::var("DIFFERENTIAL_NULL_LIBRARY") else {
        return;
    };
    let pointer =
        std::env::var("DIFFERENTIAL_NULL_POINTER").expect("null pointer selection is missing");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown library selection {other}"),
    };
    let library = unsafe { Library::new(path) }.expect("failed to load selected library");
    let function: Symbol<'_, SynthPair> =
        unsafe { library.get(b"synth_pair\0") }.expect("synth_pair export is missing");
    let mut pcm = vec![0_i16; PCM_LEN];
    let z = vec![0.0_f32; Z_LEN];

    unsafe {
        match pointer.as_str() {
            "pcm" => function(std::ptr::null_mut(), 1, z.as_ptr()),
            "z" => function(pcm.as_mut_ptr(), 1, std::ptr::null()),
            other => panic!("unknown pointer selection {other}"),
        }
    }
    panic!("selected library unexpectedly returned after a null {pointer}");
}
