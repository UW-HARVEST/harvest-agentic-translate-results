use libloading::Library;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type RgbToHsv = unsafe extern "C" fn(*mut f32, *const f32);

struct Harness {
    _c_library: Library,
    _rust_library: Library,
    c: RgbToHsv,
    rust: RgbToHsv,
}

impl Harness {
    fn load() -> Self {
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
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c = *c_library
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("load C rgb_to_hsv");
            let rust = *rust_library
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("load Rust rgb_to_hsv");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn compare_disjoint(&self, input: [f32; 3], case: &str) {
        let poison = [
            f32::from_bits(0x7fc1_2345),
            f32::from_bits(0xffc2_3456),
            f32::from_bits(0x7fa3_4567),
        ];
        let mut c_output = poison;
        let mut rust_output = poison;
        unsafe {
            (self.c)(c_output.as_mut_ptr(), input.as_ptr());
            (self.rust)(rust_output.as_mut_ptr(), input.as_ptr());
        }
        assert_float_slices_eq(&c_output, &rust_output, case, &input);
    }
}

#[derive(Clone, Copy)]
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
        (value >> 16) as u32
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1_u32 << 24) as f32
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn gap(&mut self) -> f32 {
        self.between(0.125, 40.0)
    }

    fn finite_bits(&mut self) -> f32 {
        loop {
            let value = f32::from_bits(self.next_u32());
            if value.is_finite() {
                return value;
            }
        }
    }

    fn quiet_nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = self.next_u32() & 0x003f_ffff;
        f32::from_bits(sign | 0x7fc0_0000 | payload)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-1dcjw8.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/librgb_to_hsv_lib.so")
}

fn assert_float_slices_eq(c: &[f32], rust: &[f32], case: &str, input: &[f32]) {
    let c_bits: Vec<u32> = c.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<u32> = rust.iter().map(|value| value.to_bits()).collect();
    assert_eq!(
        c_bits,
        rust_bits,
        "{case}: input bits={:08x?}, C={:08x?}, Rust={:08x?}",
        input
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        c_bits,
        rust_bits
    );
}

fn rotate(values: [f32; 3], amount: usize) -> [f32; 3] {
    match amount % 3 {
        0 => values,
        1 => [values[2], values[0], values[1]],
        _ => [values[1], values[2], values[0]],
    }
}

#[test]
fn config_01_all_positive_zero() {
    let harness = Harness::load();
    for iteration in 0..512 {
        harness.compare_disjoint([0.0, 0.0, 0.0], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_02_signed_zero_permutations() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0202_0202_0202_0202);
    for iteration in 0..512 {
        let input = [
            f32::from_bits((rng.next_u32() & 1) << 31),
            f32::from_bits((rng.next_u32() & 1) << 31),
            f32::from_bits((rng.next_u32() & 1) << 31),
        ];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_03_equal_finite_nonzero() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for iteration in 0..512 {
        let mut value = rng.between(-10_000.0, 10_000.0);
        if value == 0.0 {
            value = 1.0;
        }
        harness.compare_disjoint([value; 3], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_04_zero_maximum_with_nonzero_delta() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    for iteration in 0..512 {
        let input = rotate(
            [0.0, -rng.between(0.125, 100.0), -rng.between(0.125, 100.0)],
            (rng.next_u32() % 3) as usize,
        );
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_05_red_maximum_without_wrap() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for iteration in 0..512 {
        let g = rng.between(-100.0, 100.0);
        let b = if iteration % 4 == 0 { g } else { g - rng.gap() };
        let r = g + rng.gap();
        harness.compare_disjoint([r, g, b], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_06_red_maximum_with_negative_hue_wrap() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for iteration in 0..512 {
        let b = rng.between(-100.0, 100.0);
        let g = b - rng.gap();
        let r = b + rng.gap();
        harness.compare_disjoint([r, g, b], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_07_red_green_tied_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0707_0707_0707_0707);
    for iteration in 0..512 {
        let low = rng.between(-100.0, 100.0);
        let high = low + rng.gap();
        harness.compare_disjoint([high, high, low], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_08_red_blue_tied_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0808_0808_0808_0808);
    for iteration in 0..512 {
        let low = rng.between(-100.0, 100.0);
        let high = low + rng.gap();
        harness.compare_disjoint([high, low, high], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_09_unique_green_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x0909_0909_0909_0909);
    for iteration in 0..512 {
        let high = rng.between(-60.0, 100.0);
        let r = high - rng.gap();
        let b = high - rng.gap();
        harness.compare_disjoint([r, high, b], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_10_green_blue_tied_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1010_1010_1010_1010);
    for iteration in 0..512 {
        let low = rng.between(-100.0, 100.0);
        let high = low + rng.gap();
        harness.compare_disjoint([low, high, high], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_11_unique_blue_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1111_1111_1111_1111);
    for iteration in 0..512 {
        let high = rng.between(-60.0, 100.0);
        let r = high - rng.gap();
        let g = high - rng.gap();
        harness.compare_disjoint([r, g, high], &format!("iteration {iteration}"));
    }
}

#[test]
fn config_12_unique_negative_maximum() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1212_1212_1212_1212);
    for iteration in 0..512 {
        let high = -rng.between(0.125, 50.0);
        let middle = high - rng.gap();
        let low = middle - rng.gap();
        let input = rotate([high, middle, low], iteration);
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_13_infinities() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1313_1313_1313_1313);
    for iteration in 0..512 {
        let x = rng.between(-100.0, 100.0);
        let y = rng.between(-100.0, 100.0);
        let input = match iteration % 8 {
            0 => [f32::INFINITY, x, y],
            1 => [x, f32::INFINITY, y],
            2 => [x, y, f32::INFINITY],
            3 => [f32::NEG_INFINITY, x, y],
            4 => [x, f32::NEG_INFINITY, y],
            5 => [x, y, f32::NEG_INFINITY],
            6 => [f32::INFINITY, f32::NEG_INFINITY, x],
            _ => [f32::NEG_INFINITY, f32::INFINITY, y],
        };
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_14_nan_in_red() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1414_1414_1414_1414);
    for iteration in 0..512 {
        let input = [
            rng.quiet_nan(),
            rng.between(-100.0, 100.0),
            rng.between(-100.0, 100.0),
        ];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_15_nan_in_green() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1515_1515_1515_1515);
    for iteration in 0..512 {
        let input = [
            rng.between(-100.0, 100.0),
            rng.quiet_nan(),
            rng.between(-100.0, 100.0),
        ];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_16_nan_in_blue() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1616_1616_1616_1616);
    for iteration in 0..512 {
        let input = [
            rng.between(-100.0, 100.0),
            rng.between(-100.0, 100.0),
            rng.quiet_nan(),
        ];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_17_finite_ieee_boundaries() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1717_1717_1717_1717);
    let values = [
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x007f_ffff),
        f32::MIN_POSITIVE,
        1.0,
        f32::MAX,
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x807f_ffff),
        -f32::MIN_POSITIVE,
        -1.0,
        -f32::MAX,
    ];
    for iteration in 0..512 {
        let input = [
            values[rng.next_u32() as usize % values.len()],
            values[rng.next_u32() as usize % values.len()],
            values[rng.next_u32() as usize % values.len()],
        ];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_18_arbitrary_finite_disjoint() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1818_1818_1818_1818);
    for iteration in 0..4096 {
        let input = [rng.finite_bits(), rng.finite_bits(), rng.finite_bits()];
        harness.compare_disjoint(input, &format!("iteration {iteration}"));
    }
}

#[test]
fn config_19_exact_alias() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x1919_1919_1919_1919);
    for iteration in 0..512 {
        let input = [rng.finite_bits(), rng.finite_bits(), rng.finite_bits()];
        let mut c_buffer = input;
        let mut rust_buffer = input;
        unsafe {
            (harness.c)(c_buffer.as_mut_ptr(), c_buffer.as_ptr());
            (harness.rust)(rust_buffer.as_mut_ptr(), rust_buffer.as_ptr());
        }
        assert_float_slices_eq(
            &c_buffer,
            &rust_buffer,
            &format!("iteration {iteration}"),
            &input,
        );
    }
}

#[test]
fn config_20_destination_overlaps_source_forward() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x2020_2020_2020_2020);
    for iteration in 0..512 {
        let input = [
            rng.finite_bits(),
            rng.finite_bits(),
            rng.finite_bits(),
            rng.finite_bits(),
        ];
        let mut c_buffer = input;
        let mut rust_buffer = input;
        unsafe {
            (harness.c)(c_buffer.as_mut_ptr().add(1), c_buffer.as_ptr());
            (harness.rust)(rust_buffer.as_mut_ptr().add(1), rust_buffer.as_ptr());
        }
        assert_float_slices_eq(
            &c_buffer,
            &rust_buffer,
            &format!("iteration {iteration}"),
            &input,
        );
    }
}

#[test]
fn config_21_source_overlaps_destination_forward() {
    let harness = Harness::load();
    let mut rng = Rng::new(0x2121_2121_2121_2121);
    for iteration in 0..512 {
        let input = [
            rng.finite_bits(),
            rng.finite_bits(),
            rng.finite_bits(),
            rng.finite_bits(),
        ];
        let mut c_buffer = input;
        let mut rust_buffer = input;
        unsafe {
            (harness.c)(c_buffer.as_mut_ptr(), c_buffer.as_ptr().add(1));
            (harness.rust)(rust_buffer.as_mut_ptr(), rust_buffer.as_ptr().add(1));
        }
        assert_float_slices_eq(
            &c_buffer,
            &rust_buffer,
            &format!("iteration {iteration}"),
            &input,
        );
    }
}

#[test]
fn crash_probe_entry() {
    let Ok(probe) = env::var("RGB_TO_HSV_CRASH_PROBE") else {
        return;
    };
    let harness = Harness::load();
    let mut destination = [0.0_f32; 3];
    let source = [1.0_f32, 0.25, 0.5];
    let function = match probe.as_str() {
        value if value.starts_with("c:") => harness.c,
        value if value.starts_with("rust:") => harness.rust,
        _ => panic!("unknown probe library: {probe}"),
    };
    unsafe {
        match probe.split_once(':').map(|(_, pointer)| pointer) {
            Some("src") => function(destination.as_mut_ptr(), std::ptr::null()),
            Some("dest") => function(std::ptr::null_mut(), source.as_ptr()),
            _ => panic!("unknown probe pointer: {probe}"),
        }
    }
}

fn run_crash_probe(probe: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("crash_probe_entry")
        .arg("--nocapture")
        .env("RGB_TO_HSV_CRASH_PROBE", probe)
        .status()
        .unwrap_or_else(|error| panic!("run crash probe {probe}: {error}"))
}

#[cfg(unix)]
fn compare_crash_behavior(pointer: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_crash_probe(&format!("c:{pointer}"));
    let rust_status = run_crash_probe(&format!("rust:{pointer}"));
    assert!(
        !c_status.success(),
        "C unexpectedly accepted null {pointer}"
    );
    assert!(
        !rust_status.success(),
        "Rust unexpectedly accepted null {pointer}"
    );
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "different terminating signals for null {pointer}: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        c_status.signal().is_some(),
        "C null {pointer} did not terminate by signal: {c_status:?}"
    );
}

#[test]
#[cfg(unix)]
fn error_01_null_source() {
    compare_crash_behavior("src");
}

#[test]
#[cfg(unix)]
fn error_02_null_destination() {
    compare_crash_behavior("dest");
}

fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only", "--extern-only"])
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("run nm on {}: {error}", path.display()));
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("nm output is UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn symbol_surface_has_no_missing_rust_exports() {
    let c_symbols = defined_dynamic_symbols(&c_library_path());
    let rust_symbols = defined_dynamic_symbols(&rust_library_path());
    let missing: Vec<_> = c_symbols.difference(&rust_symbols).cloned().collect();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");
}
