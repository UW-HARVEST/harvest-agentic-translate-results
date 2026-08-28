use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type SynthPair = unsafe extern "C" fn(*mut i16, i32, *const f32);

const Z_LEN: usize = 899;
const PCM_LEN: usize = 513;
const PCM_BASE: usize = PCM_LEN / 2;
const CASES_PER_ROW: usize = 128;
const NCH_VALUES: [i32; 7] = [-8, -3, -1, 0, 1, 2, 8];

#[derive(Clone, Copy, Debug)]
enum ScaleClass {
    LowSaturation,
    NegativeConverted,
    NonnegativeConverted,
    HighSaturation,
}

impl ScaleClass {
    const ALL: [Self; 4] = [
        Self::LowSaturation,
        Self::NegativeConverted,
        Self::NonnegativeConverted,
        Self::HighSaturation,
    ];
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn unit_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        bits as f32 / 0x00ff_ffff_u32 as f32
    }

    fn range_f32(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit_f32()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-jskHrr.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libsynth_pair_lib.so")
}

unsafe fn load_synth_pair(library: &Library) -> Symbol<'_, SynthPair> {
    unsafe { library.get(b"synth_pair\0") }.expect("load synth_pair")
}

fn desired_sample(class: ScaleClass, rng: &mut Rng) -> f32 {
    match class {
        ScaleClass::LowSaturation => rng.range_f32(-200_000.0, -40_000.0),
        ScaleClass::NegativeConverted => rng.range_f32(-32_000.0, -2.0),
        ScaleClass::NonnegativeConverted => rng.range_f32(-1.0, 32_000.0),
        ScaleClass::HighSaturation => rng.range_f32(40_000.0, 200_000.0),
    }
}

fn first_partial(z: &[f32]) -> f32 {
    let mut a = (z[14 * 64] - z[0]) * 29.0;
    a += (z[64] + z[13 * 64]) * 213.0;
    a += (z[12 * 64] - z[2 * 64]) * 459.0;
    a += (z[3 * 64] + z[11 * 64]) * 2037.0;
    a += (z[10 * 64] - z[4 * 64]) * 5153.0;
    a += (z[5 * 64] + z[9 * 64]) * 6574.0;
    a += (z[8 * 64] - z[6 * 64]) * 37489.0;
    a
}

fn second_partial(z: &[f32]) -> f32 {
    let z = &z[2..];
    let mut a = z[14 * 64] * 104.0;
    a += z[12 * 64] * 1567.0;
    a += z[10 * 64] * 9727.0;
    a += z[6 * 64] * -9975.0;
    a += z[4 * 64] * -45.0;
    a += z[2 * 64] * 146.0;
    a += z[0] * -5.0;
    a
}

fn computed_samples(z: &[f32]) -> [f32; 2] {
    let first = first_partial(z) + z[7 * 64] * 75038.0;
    let z = &z[2..];
    let mut second = z[8 * 64] * 64019.0;
    second += z[14 * 64] * 104.0;
    second += z[12 * 64] * 1567.0;
    second += z[10 * 64] * 9727.0;
    second += z[6 * 64] * -9975.0;
    second += z[4 * 64] * -45.0;
    second += z[2 * 64] * 146.0;
    second += z[0] * -5.0;
    [first, second]
}

fn assert_scale_class(sample: f32, class: ScaleClass) {
    let converted = (sample + 0.5) as i16;
    let matches = match class {
        ScaleClass::LowSaturation => sample <= -32767.5,
        ScaleClass::NegativeConverted => sample > -32767.5 && sample < 32766.5 && converted < 0,
        ScaleClass::NonnegativeConverted => sample > -32767.5 && sample < 32766.5 && converted >= 0,
        ScaleClass::HighSaturation => sample >= 32766.5,
    };
    assert!(matches, "sample {sample} does not exercise {class:?}");
}

fn input_for(first_class: ScaleClass, second_class: ScaleClass, rng: &mut Rng) -> Vec<f32> {
    let mut z = (0..Z_LEN)
        .map(|_| rng.range_f32(-0.25, 0.25))
        .collect::<Vec<_>>();

    z[7 * 64] = 0.0;
    let first_target = desired_sample(first_class, rng);
    z[7 * 64] = (first_target - first_partial(&z)) / 75038.0;

    z[2 + 8 * 64] = 0.0;
    let second_target = desired_sample(second_class, rng);
    z[2 + 8 * 64] = (second_target - second_partial(&z)) / 64019.0;

    let [first, second] = computed_samples(&z);
    assert_scale_class(first, first_class);
    assert_scale_class(second, second_class);
    z
}

fn as_bytes(values: &[i16]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

unsafe fn compare_call(
    c_fn: &Symbol<'_, SynthPair>,
    rust_fn: &Symbol<'_, SynthPair>,
    z: &[f32],
    nch: i32,
    context: &str,
) {
    let mut c_pcm = vec![0x5a5a_i16; PCM_LEN];
    let mut rust_pcm = c_pcm.clone();
    let c_ptr = unsafe { c_pcm.as_mut_ptr().add(PCM_BASE) };
    let rust_ptr = unsafe { rust_pcm.as_mut_ptr().add(PCM_BASE) };

    unsafe {
        c_fn(c_ptr, nch, z.as_ptr());
        rust_fn(rust_ptr, nch, z.as_ptr());
    }

    assert_eq!(
        as_bytes(&c_pcm),
        as_bytes(&rust_pcm),
        "{context}, nch={nch}"
    );
}

#[test]
fn all_configuration_rows_match() {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn = unsafe { load_synth_pair(&c_library) };
    let rust_fn = unsafe { load_synth_pair(&rust_library) };
    let mut rng = Rng::new(0x5eed_d1ff_e2e7_2026);

    for (first_index, first_class) in ScaleClass::ALL.into_iter().enumerate() {
        for (second_index, second_class) in ScaleClass::ALL.into_iter().enumerate() {
            let row = first_index * ScaleClass::ALL.len() + second_index + 1;
            for case in 0..CASES_PER_ROW {
                let z = input_for(first_class, second_class, &mut rng);
                for nch in NCH_VALUES {
                    let context = format!(
                        "CONFIGS.md row {row}, case {case}, \
                         first={first_class:?}, second={second_class:?}"
                    );
                    unsafe { compare_call(&c_fn, &rust_fn, &z, nch, &context) };
                }
            }
        }
    }
}

#[test]
fn conversion_boundaries_match() {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");
    let c_fn = unsafe { load_synth_pair(&c_library) };
    let rust_fn = unsafe { load_synth_pair(&rust_library) };
    let thresholds = [-32767.5_f32, -1.5, -0.5, 0.0, 32766.5];

    for threshold in thresholds {
        let first_center = threshold / 75038.0;
        let second_center = threshold / 64019.0;
        for delta in -32..=32 {
            let mut z = vec![0.0; Z_LEN];
            z[7 * 64] = f32::from_bits(first_center.to_bits().wrapping_add_signed(delta));
            z[2 + 8 * 64] = f32::from_bits(second_center.to_bits().wrapping_add_signed(delta));
            for nch in NCH_VALUES {
                let context = format!("threshold={threshold}, bit_delta={delta}");
                unsafe { compare_call(&c_fn, &rust_fn, &z, nch, &context) };
            }
        }
    }
}

#[test]
fn null_pointer_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    for pointer in ["pcm", "z"] {
        let mut signals = Vec::new();
        for implementation in ["c", "rust"] {
            let status = Command::new(std::env::current_exe().expect("current test executable"))
                .args(["--exact", "null_pointer_probe", "--nocapture"])
                .env("SYNTH_PAIR_NULL_PROBE", pointer)
                .env("SYNTH_PAIR_IMPLEMENTATION", implementation)
                .status()
                .expect("run null-pointer probe");
            signals.push(status.signal());
        }

        assert!(
            signals[0].is_some(),
            "C unexpectedly survived null {pointer}"
        );
        assert_eq!(
            signals[0], signals[1],
            "null {pointer} process signal differs"
        );
    }
}

#[test]
fn null_pointer_probe() {
    let Ok(pointer) = std::env::var("SYNTH_PAIR_NULL_PROBE") else {
        return;
    };
    let implementation = std::env::var("SYNTH_PAIR_IMPLEMENTATION").expect("probe implementation");
    let path = match implementation.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown implementation {other}"),
    };
    let library = unsafe { Library::new(path) }.expect("load probe library");
    let synth_pair = unsafe { load_synth_pair(&library) };
    let mut pcm = vec![0_i16; 17];
    let z = vec![0.0_f32; Z_LEN];

    match pointer.as_str() {
        "pcm" => unsafe { synth_pair(std::ptr::null_mut(), 1, z.as_ptr()) },
        "z" => unsafe { synth_pair(pcm.as_mut_ptr(), 1, std::ptr::null()) },
        other => panic!("unknown pointer {other}"),
    }
}
