use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type SynthPair = unsafe extern "C" fn(*mut i16, c_int, *const f32);

const Z_LEN: usize = 14 * 64 + 3;
const RANDOM_CASES_PER_CONFIG: usize = 256;

#[derive(Clone, Copy, Debug)]
enum ScalePath {
    PositiveSaturation,
    NegativeSaturation,
    InteriorNonnegative,
    InteriorNegative,
    NanFallthrough,
}

impl ScalePath {
    const ALL: [Self; 5] = [
        Self::PositiveSaturation,
        Self::NegativeSaturation,
        Self::InteriorNonnegative,
        Self::InteriorNegative,
        Self::NanFallthrough,
    ];
}

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

    fn unit_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1_u32 << 24) as f32
    }

    fn range_f32(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit_f32()
    }

    fn finite_noise(&mut self) -> f32 {
        self.range_f32(-0.01, 0.01)
    }

    fn nan(&mut self) -> f32 {
        let sign = ((self.next_u64() as u32) & 1) << 31;
        let payload = ((self.next_u64() as u32) & 0x003f_ffff) | 1;
        f32::from_bits(sign | 0x7fc0_0000 | payload)
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-QXNeUh.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libsynth_pair_lib.so")
}

unsafe fn call(library: &Library, pcm: *mut i16, nch: c_int, z: *const f32) {
    let synth_pair: Symbol<SynthPair> =
        unsafe { library.get(b"synth_pair\0") }.expect("load synth_pair");
    unsafe { synth_pair(pcm, nch, z) };
}

fn open_libraries() -> (Library, Library) {
    let c = unsafe { Library::new(c_library_path()) }.expect("load C shared library");
    let rust = unsafe { Library::new(rust_library_path()) }.expect("load Rust shared library");
    (c, rust)
}

fn target_for(path: ScalePath, rng: &mut Rng) -> f32 {
    match path {
        ScalePath::PositiveSaturation => rng.range_f32(33_000.0, 1_000_000.0),
        ScalePath::NegativeSaturation => rng.range_f32(-1_000_000.0, -33_000.0),
        ScalePath::InteriorNonnegative => rng.range_f32(0.0, 32_000.0),
        ScalePath::InteriorNegative => rng.range_f32(-32_000.0, -1.0),
        ScalePath::NanFallthrough => rng.nan(),
    }
}

fn first_accumulator(z: &[f32]) -> f32 {
    let mut a = (z[14 * 64] - z[0]) * 29.0;
    a += (z[64] + z[13 * 64]) * 213.0;
    a += (z[12 * 64] - z[2 * 64]) * 459.0;
    a += (z[3 * 64] + z[11 * 64]) * 2037.0;
    a += (z[10 * 64] - z[4 * 64]) * 5153.0;
    a += (z[5 * 64] + z[9 * 64]) * 6574.0;
    a += (z[8 * 64] - z[6 * 64]) * 37489.0;
    a += z[7 * 64] * 75038.0;
    a
}

fn second_accumulator(z: &[f32]) -> f32 {
    let z = &z[2..];
    let mut a = z[14 * 64] * 104.0;
    a += z[12 * 64] * 1567.0;
    a += z[10 * 64] * 9727.0;
    a += z[8 * 64] * 64019.0;
    a += z[6 * 64] * -9975.0;
    a += z[4 * 64] * -45.0;
    a += z[2 * 64] * 146.0;
    a += z[0] * -5.0;
    a
}

fn matches_path(sample: f32, path: ScalePath) -> bool {
    match path {
        ScalePath::PositiveSaturation => sample >= 32766.5,
        ScalePath::NegativeSaturation => sample <= -32767.5,
        ScalePath::InteriorNonnegative => sample.is_finite() && sample >= 0.0 && sample < 32766.5,
        ScalePath::InteriorNegative => sample.is_finite() && sample > -32767.5 && sample <= -1.0,
        ScalePath::NanFallthrough => sample.is_nan(),
    }
}

fn make_input(first: ScalePath, second: ScalePath, rng: &mut Rng) -> Vec<f32> {
    let mut z = vec![0.0; Z_LEN];

    let first_indices = [
        0,
        64,
        2 * 64,
        3 * 64,
        4 * 64,
        5 * 64,
        6 * 64,
        8 * 64,
        9 * 64,
        10 * 64,
        11 * 64,
        12 * 64,
        13 * 64,
        14 * 64,
    ];
    for index in first_indices {
        z[index] = rng.finite_noise();
    }

    let first_target = target_for(first, rng);
    if first_target.is_nan() {
        z[7 * 64] = first_target;
    } else {
        let before_final = first_accumulator(&z);
        z[7 * 64] = (first_target - before_final) / 75038.0;
    }

    let second_indices = [
        2 + 2 * 64,
        2 + 4 * 64,
        2 + 6 * 64,
        2 + 8 * 64,
        2 + 10 * 64,
        2 + 12 * 64,
        2 + 14 * 64,
    ];
    for index in second_indices {
        z[index] = rng.finite_noise();
    }

    let second_target = target_for(second, rng);
    if second_target.is_nan() {
        z[2] = second_target;
    } else {
        z[2] = 0.0;
        let before_final = second_accumulator(&z);
        z[2] = (second_target - before_final) / -5.0;
    }

    for value in &mut z {
        if *value == 0.0 && (rng.next_u64() & 7) == 0 {
            *value = rng.finite_noise();
        }
    }

    z
}

fn compare_once(
    c: &Library,
    rust: &Library,
    nch: c_int,
    first: ScalePath,
    second: ScalePath,
    rng: &mut Rng,
) {
    let z = make_input(first, second, rng);
    assert!(
        matches_path(first_accumulator(&z), first),
        "generated input missed first path {first:?}: {}",
        first_accumulator(&z)
    );
    assert!(
        matches_path(second_accumulator(&z), second),
        "generated input missed second path {second:?}: {}",
        second_accumulator(&z)
    );

    let len = (16 * nch as usize) + 17;
    let mut initial = Vec::with_capacity(len);
    for _ in 0..len {
        initial.push(rng.next_u64() as i16);
    }
    let mut c_pcm = initial.clone();
    let mut rust_pcm = initial;

    unsafe {
        call(c, c_pcm.as_mut_ptr(), nch, z.as_ptr());
        call(rust, rust_pcm.as_mut_ptr(), nch, z.as_ptr());
    }

    assert_eq!(
        c_pcm,
        rust_pcm,
        "nch={nch}, first={first:?}, second={second:?}, \
         first_accumulator={}, second_accumulator={}",
        first_accumulator(&z),
        second_accumulator(&z)
    );
}

#[test]
fn all_configuration_rows_match_randomized() {
    let (c, rust) = open_libraries();
    let mut row = 0;
    for nch in [1, 2] {
        for first in ScalePath::ALL {
            for second in ScalePath::ALL {
                row += 1;
                let mut rng = Rng::new(0xd1ff_e000_0000_0000 ^ row);
                for _ in 0..RANDOM_CASES_PER_CONFIG {
                    compare_once(&c, &rust, nch, first, second, &mut rng);
                }
            }
        }
    }
    assert_eq!(row, 50);
}

#[test]
fn threshold_and_rounding_boundaries_match() {
    let (c, rust) = open_libraries();
    let values = [
        f32::NEG_INFINITY,
        -32768.0,
        -32767.501953125,
        -32767.5,
        -32767.498046875,
        -1.5000001,
        -1.5,
        -1.4999999,
        -0.50000006,
        -0.5,
        -0.49999997,
        -0.0,
        0.0,
        0.49999997,
        0.5,
        0.50000006,
        32766.498046875,
        32766.5,
        32766.501953125,
        32767.0,
        f32::INFINITY,
    ];

    for (index, first_target) in values.into_iter().enumerate() {
        for (second_index, second_target) in values.into_iter().enumerate() {
            let mut z = vec![0.0; Z_LEN];
            z[7 * 64] = first_target / 75038.0;
            z[2] = second_target / -5.0;
            let mut c_pcm = vec![0x1234; 64];
            let mut rust_pcm = c_pcm.clone();
            unsafe {
                call(&c, c_pcm.as_mut_ptr(), 2, z.as_ptr());
                call(&rust, rust_pcm.as_mut_ptr(), 2, z.as_ptr());
            }
            assert_eq!(
                c_pcm, rust_pcm,
                "boundary mismatch at first[{index}]={first_target:?}, \
                 second[{second_index}]={second_target:?}"
            );
        }
    }
}

fn compare_channel_boundary(nch: c_int, pcm_len: usize, pcm_origin: usize) {
    let (c, rust) = open_libraries();
    let mut rng = Rng::new(0xb0a0_da7a ^ nch as u64);
    for _ in 0..512 {
        let z = make_input(
            ScalePath::InteriorNegative,
            ScalePath::InteriorNonnegative,
            &mut rng,
        );
        let mut initial = Vec::with_capacity(pcm_len);
        for _ in 0..pcm_len {
            initial.push(rng.next_u64() as i16);
        }
        let mut c_pcm = initial.clone();
        let mut rust_pcm = initial;
        unsafe {
            call(&c, c_pcm.as_mut_ptr().add(pcm_origin), nch, z.as_ptr());
            call(
                &rust,
                rust_pcm.as_mut_ptr().add(pcm_origin),
                nch,
                z.as_ptr(),
            );
        }
        assert_eq!(c_pcm, rust_pcm, "channel boundary mismatch for nch={nch}");
    }
}

#[test]
fn generic_channel_boundaries_match() {
    compare_channel_boundary(0, 64, 0);
    compare_channel_boundary(-1, 64, 16);
    compare_channel_boundary(4096, 65_537, 0);
}

fn run_crash_probe(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("crash_probe")
        .arg("--nocapture")
        .env("DIFF_CRASH_LIBRARY", library)
        .env("DIFF_CRASH_CASE", case)
        .status()
        .expect("run crash probe")
}

#[cfg(unix)]
#[test]
fn null_pointer_faults_match() {
    use std::os::unix::process::ExitStatusExt;

    for case in ["null_pcm", "null_z"] {
        let c = run_crash_probe("c", case);
        let rust = run_crash_probe("rust", case);
        assert_eq!(c.signal(), Some(11), "C {case} status: {c:?}");
        assert_eq!(rust.signal(), c.signal(), "Rust {case} status: {rust:?}");
    }
}

#[test]
fn crash_probe() {
    let Ok(which) = std::env::var("DIFF_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CRASH_CASE").expect("crash case");
    let path = match which.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        _ => panic!("unknown crash library"),
    };
    let library = unsafe { Library::new(path) }.expect("load crash-probe library");
    let mut pcm = vec![0_i16; 64];
    let z = vec![0.0_f32; Z_LEN];

    unsafe {
        match case.as_str() {
            "null_pcm" => call(&library, std::ptr::null_mut(), 1, z.as_ptr()),
            "null_z" => call(&library, pcm.as_mut_ptr(), 1, std::ptr::null()),
            _ => panic!("unknown crash case"),
        }
    }
    panic!("crash probe unexpectedly returned");
}
