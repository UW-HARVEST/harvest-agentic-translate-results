use libloading::Library;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

const RANDOM_CASES: usize = 512;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c: HslToRgb,
    rust: HslToRgb,
}

impl Apis {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-esKwV7.so");
        let rust_path = root.join("target/release/libhsl_to_rgb_lib.so");

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
                .get::<HslToRgb>(b"hsl_to_rgb\0")
                .expect("load C hsl_to_rgb");
            let rust = *rust_library
                .get::<HslToRgb>(b"hsl_to_rgb\0")
                .expect("load Rust hsl_to_rgb");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn compare(&self, src: [f32; 3]) {
        let initial = [
            f32::from_bits(0x7fa1_2345),
            f32::from_bits(0xffa2_3456),
            f32::from_bits(0x7fa3_4567),
        ];
        let mut c_dest = initial;
        let mut rust_dest = initial;

        unsafe {
            (self.c)(c_dest.as_mut_ptr(), src.as_ptr());
            (self.rust)(rust_dest.as_mut_ptr(), src.as_ptr());
        }

        assert_eq!(
            bits(c_dest),
            bits(rust_dest),
            "output mismatch for src bits {:08x?}",
            bits(src)
        );
    }

    fn compare_in_place(&self, src: [f32; 3]) {
        let mut c_buf = src;
        let mut rust_buf = src;

        unsafe {
            let c_ptr = c_buf.as_mut_ptr();
            let rust_ptr = rust_buf.as_mut_ptr();
            (self.c)(c_ptr, c_ptr.cast_const());
            (self.rust)(rust_ptr, rust_ptr.cast_const());
        }

        assert_eq!(
            bits(c_buf),
            bits(rust_buf),
            "in-place output mismatch for src bits {:08x?}",
            bits(src)
        );
    }
}

#[derive(Clone, Copy)]
struct FixedRng(u64);

impl FixedRng {
    fn new(stream: u64) -> Self {
        Self(0xd1b5_4a32_d192_ed03 ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x ^ (x >> 32)) as u32
    }

    fn any_float(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn finite(&mut self) -> f32 {
        loop {
            let value = self.any_float();
            if value.is_finite() {
                return value;
            }
        }
    }

    fn finite_nonzero(&mut self) -> f32 {
        loop {
            let value = self.finite();
            if value != 0.0 {
                return value;
            }
        }
    }

    fn hue_segment(&mut self, start: i32, width: u32) -> f32 {
        start as f32 + (self.next_u32() % (width * 10_000)) as f32 / 10_000.0
    }
}

fn bits(values: [f32; 3]) -> [u32; 3] {
    values.map(f32::to_bits)
}

fn next_down(value: f32) -> f32 {
    assert!(value.is_finite() && value > 0.0);
    f32::from_bits(value.to_bits() - 1)
}

fn next_up(value: f32) -> f32 {
    assert!(value.is_finite() && value >= 0.0);
    f32::from_bits(value.to_bits() + 1)
}

fn compare_hues(hues: &[f32], stream: u64) {
    let apis = Apis::load();
    let mut rng = FixedRng::new(stream);
    for &h in hues {
        for _ in 0..RANDOM_CASES {
            apis.compare([h, rng.finite_nonzero(), rng.finite()]);
        }
    }
}

fn compare_random_segment(start: i32, width: u32, boundaries: &[f32], stream: u64) {
    let apis = Apis::load();
    let mut rng = FixedRng::new(stream);
    for &h in boundaries {
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
    for _ in 0..RANDOM_CASES {
        apis.compare([
            rng.hue_segment(start, width),
            rng.finite_nonzero(),
            rng.finite(),
        ]);
    }
}

#[test]
fn config_01_s_zero_grayscale() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(1);
    let special_l = [
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0xffa1_2345),
    ];
    for &s in &[0.0, -0.0] {
        for &l in &special_l {
            apis.compare([rng.any_float(), s, l]);
        }
        for _ in 0..RANDOM_CASES {
            apis.compare([rng.any_float(), s, rng.any_float()]);
        }
    }
}

#[test]
fn config_02_negative_finite_hue() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(2);
    for &h in &[-f32::MIN_POSITIVE, -f32::from_bits(1), -1.0, -359.99997] {
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
    for _ in 0..RANDOM_CASES {
        let h = -((rng.next_u32() % 3_600_000) as f32 + 1.0) / 10_000.0;
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
}

#[test]
fn config_03_hue_0_to_60() {
    compare_random_segment(0, 60, &[0.0, next_up(0.0), next_down(60.0)], 3);
}

#[test]
fn config_04_hue_60_to_120() {
    compare_random_segment(60, 60, &[60.0, next_up(60.0), next_down(120.0)], 4);
}

#[test]
fn config_05_hue_120_to_180_fallback() {
    compare_random_segment(120, 60, &[120.0, next_up(120.0), next_down(180.0)], 5);
}

#[test]
fn config_06_hue_180_to_240() {
    compare_random_segment(180, 60, &[180.0, next_up(180.0), next_down(240.0)], 6);
}

#[test]
fn config_07_hue_240_to_300() {
    compare_random_segment(240, 60, &[240.0, next_up(240.0), next_down(300.0)], 7);
}

#[test]
fn config_08_hue_300_to_360() {
    compare_random_segment(300, 60, &[300.0, next_up(300.0), next_down(360.0)], 8);
}

#[test]
fn config_09_hue_at_least_360_fallback() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(9);
    for &h in &[360.0, next_up(360.0), f32::MAX] {
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
    for _ in 0..RANDOM_CASES {
        let h = 360.0 + (rng.next_u32() % 3_600_000) as f32 / 10_000.0;
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
}

#[test]
fn config_10_negative_infinity_hue() {
    compare_hues(&[f32::NEG_INFINITY], 10);
}

#[test]
fn config_11_positive_infinity_hue() {
    compare_hues(&[f32::INFINITY], 11);
}

#[test]
fn config_12_nan_hue_payloads() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(12);
    let fixed = [
        0x7fc0_0000,
        0x7fc0_0001,
        0x7fa0_0001,
        0xffc0_0000,
        0xffbf_ffff,
    ];
    for payload in fixed {
        apis.compare([f32::from_bits(payload), rng.finite_nonzero(), rng.finite()]);
    }
    for _ in 0..RANDOM_CASES {
        let sign = rng.next_u32() & 0x8000_0000;
        let payload = (rng.next_u32() & 0x007f_ffff).max(1);
        let h = f32::from_bits(sign | 0x7f80_0000 | payload);
        apis.compare([h, rng.finite_nonzero(), rng.finite()]);
    }
}

#[test]
fn config_13_finite_s_and_l_shapes() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(13);
    let hues = [
        -30.0, 0.0, 59.999, 60.0, 119.999, 120.0, 179.999, 180.0, 239.999, 240.0, 299.999, 300.0,
        359.999, 360.0,
    ];
    let finite_shapes = [
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        -f32::MAX,
        0.0,
        -0.0,
        1.0,
        -1.0,
    ];
    for &h in &hues {
        for &s in &finite_shapes {
            if s != 0.0 {
                for &l in &finite_shapes {
                    apis.compare([h, s, l]);
                }
            }
        }
    }
    for _ in 0..RANDOM_CASES {
        apis.compare([
            hues[(rng.next_u32() as usize) % hues.len()],
            rng.finite_nonzero(),
            rng.finite(),
        ]);
    }
}

#[test]
fn config_14_nonfinite_saturation() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(14);
    let saturation = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0x7fa1_2345),
        f32::from_bits(0xffc0_9876),
    ];
    let special_hues = [
        -30.0,
        0.0,
        60.0,
        120.0,
        180.0,
        240.0,
        300.0,
        360.0,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffa0_5678),
    ];
    let special_lightness = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_2468),
        f32::from_bits(0xffa0_1357),
    ];
    for &s in &saturation {
        for &h in &special_hues {
            for &l in &special_lightness {
                apis.compare([h, s, l]);
            }
        }
    }
    for _ in 0..RANDOM_CASES {
        let s = saturation[(rng.next_u32() as usize) % saturation.len()];
        apis.compare([rng.any_float(), s, rng.any_float()]);
    }
}

#[test]
fn config_15_nonfinite_lightness() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(15);
    let lightness = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0x7fa1_2345),
        f32::from_bits(0xffc0_9876),
    ];
    let special_hues = [
        -30.0,
        0.0,
        60.0,
        120.0,
        180.0,
        240.0,
        300.0,
        360.0,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffa0_5678),
    ];
    for &l in &lightness {
        for &h in &special_hues {
            for &s in &[
                f32::from_bits(1),
                -f32::from_bits(1),
                1.0,
                -1.0,
                f32::MAX,
                -f32::MAX,
            ] {
                apis.compare([h, s, l]);
            }
        }
    }
    for _ in 0..RANDOM_CASES {
        let l = lightness[(rng.next_u32() as usize) % lightness.len()];
        apis.compare([rng.any_float(), rng.finite_nonzero(), l]);
    }
}

#[test]
fn config_16_exact_in_place_alias() {
    let apis = Apis::load();
    let mut rng = FixedRng::new(16);
    let hues = [
        -30.0,
        0.0,
        60.0,
        120.0,
        180.0,
        240.0,
        300.0,
        360.0,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::from_bits(0x7fc0_1234),
    ];
    for &h in &hues {
        for _ in 0..RANDOM_CASES {
            apis.compare_in_place([h, rng.finite_nonzero(), rng.finite()]);
        }
    }
    for _ in 0..RANDOM_CASES {
        apis.compare_in_place([rng.any_float(), 0.0, rng.any_float()]);
    }
}

fn crash_status(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current integration-test executable"))
        .args(["--exact", "crash_probe", "--nocapture"])
        .env("HSL_CRASH_LIBRARY", library)
        .env("HSL_CRASH_CASE", case)
        .status()
        .expect("run crash probe")
}

#[test]
fn error_g1_null_src_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c = crash_status("c", "null_src");
    let rust = crash_status("rust", "null_src");
    assert_eq!(c.signal(), rust.signal(), "C={c:?}, Rust={rust:?}");
    assert_eq!(c.signal(), Some(11), "expected SIGSEGV, got {c:?}");
}

#[test]
fn error_g2_null_dest_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c = crash_status("c", "null_dest");
    let rust = crash_status("rust", "null_dest");
    assert_eq!(c.signal(), rust.signal(), "C={c:?}, Rust={rust:?}");
    assert_eq!(c.signal(), Some(11), "expected SIGSEGV, got {c:?}");
}

#[test]
fn crash_probe() {
    let Ok(library) = std::env::var("HSL_CRASH_LIBRARY") else {
        return;
    };
    let case = std::env::var("HSL_CRASH_CASE").expect("crash case");
    let apis = Apis::load();
    let function = match library.as_str() {
        "c" => apis.c,
        "rust" => apis.rust,
        other => panic!("unknown crash library {other}"),
    };

    unsafe {
        match case.as_str() {
            "null_src" => {
                let mut dest = [0.0_f32; 3];
                function(dest.as_mut_ptr(), std::ptr::null());
            }
            "null_dest" => {
                let src = [0.0_f32, 0.0, 0.5];
                function(std::ptr::null_mut(), src.as_ptr());
            }
            other => panic!("unknown crash case {other}"),
        }
    }

    panic!("crash probe unexpectedly returned");
}
