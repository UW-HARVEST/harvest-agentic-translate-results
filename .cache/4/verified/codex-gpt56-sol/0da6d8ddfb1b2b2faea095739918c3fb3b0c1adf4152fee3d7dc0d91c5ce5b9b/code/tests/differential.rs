use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

const RANDOM_CASES: usize = 4_096;

struct Libraries {
    _c: Library,
    _rust: Library,
    c_hsv_to_rgb: HsvToRgb,
    rust_hsv_to_rgb: HsvToRgb,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);

        assert!(
            c_path.is_file(),
            "C shared library is missing at {}; build c_src first",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing at {}",
            rust_path.display()
        );

        unsafe {
            let c = Library::new(&c_path).expect("load C shared library");
            let rust = Library::new(&rust_path).expect("load Rust shared library");
            let c_symbol: Symbol<HsvToRgb> = c.get(b"hsv_to_rgb\0").expect("resolve C hsv_to_rgb");
            let rust_symbol: Symbol<HsvToRgb> =
                rust.get(b"hsv_to_rgb\0").expect("resolve Rust hsv_to_rgb");
            let c_hsv_to_rgb = *c_symbol;
            let rust_hsv_to_rgb = *rust_symbol;

            Self {
                _c: c,
                _rust: rust,
                c_hsv_to_rgb,
                rust_hsv_to_rgb,
            }
        }
    }

    fn assert_match(&self, input: [f32; 3], layout: Layout) {
        let c_output = unsafe { invoke(self.c_hsv_to_rgb, input, layout) };
        let rust_output = unsafe { invoke(self.rust_hsv_to_rgb, input, layout) };

        assert_eq!(
            output_bytes(c_output),
            output_bytes(rust_output),
            "input={input:?}, layout={layout:?}, C={c_output:?}, Rust={rust_output:?}"
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Layout {
    Separate,
    SameBuffer,
    DestinationBeforeSource,
    DestinationAfterSource,
}

unsafe fn invoke(function: HsvToRgb, input: [f32; 3], layout: Layout) -> [f32; 3] {
    match layout {
        Layout::Separate => {
            let mut output = [f32::from_bits(0x7fc0_1234); 3];
            unsafe { function(output.as_mut_ptr(), input.as_ptr()) };
            output
        }
        Layout::SameBuffer => {
            let mut buffer = input;
            let pointer = buffer.as_mut_ptr();
            unsafe { function(pointer, pointer.cast_const()) };
            buffer
        }
        Layout::DestinationBeforeSource => {
            let mut buffer = [f32::from_bits(0x7fc0_1234), input[0], input[1], input[2]];
            unsafe { function(buffer.as_mut_ptr(), buffer.as_ptr().add(1)) };
            [buffer[0], buffer[1], buffer[2]]
        }
        Layout::DestinationAfterSource => {
            let mut buffer = [input[0], input[1], input[2], f32::from_bits(0x7fc0_1234)];
            unsafe { function(buffer.as_mut_ptr().add(1), buffer.as_ptr()) };
            [buffer[1], buffer[2], buffer[3]]
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    root.join("target").join(profile).join(format!(
        "{}hsv_to_rgb_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

fn output_bytes(output: [f32; 3]) -> [u8; 12] {
    let mut bytes = [0_u8; 12];
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(output) {
        chunk.copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
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

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0_f32 / 16_777_216.0_f32)
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn nonzero_saturation(&mut self) -> f32 {
        let value = self.range(-2.0, 2.0);
        if value == 0.0 {
            f32::MIN_POSITIVE
        } else {
            value
        }
    }
}

fn exercise(seed: u64, input: impl Fn(&mut Rng, usize) -> [f32; 3]) {
    let libraries = Libraries::load();
    let layouts = [
        Layout::Separate,
        Layout::SameBuffer,
        Layout::DestinationBeforeSource,
        Layout::DestinationAfterSource,
    ];
    let mut rng = Rng::new(seed);

    for index in 0..RANDOM_CASES {
        libraries.assert_match(input(&mut rng, index), layouts[index % layouts.len()]);
    }
}

#[test]
fn saturation_zero_branch_matches() {
    exercise(0x1b87_3593_aa52_0d31, |rng, index| {
        let saturation = if index % 2 == 0 { 0.0 } else { -0.0 };
        [rng.range(-720.0, 720.0), saturation, rng.range(-10.0, 10.0)]
    });
}

fn exercise_sector(sector: i32, seed: u64) {
    exercise(seed, |rng, index| {
        let lower = sector as f32 * 60.0;
        let upper = lower + 60.0;
        let hue = match index % 257 {
            0 => lower,
            1 => f32::from_bits(upper.to_bits() - 1),
            _ => lower + rng.range(0.0, 60.0),
        };
        [hue, rng.nonzero_saturation(), rng.range(-10.0, 10.0)]
    });
}

#[test]
fn switch_case_zero_matches() {
    exercise_sector(0, 0xd362_6fb1_5a67_3e11);
}

#[test]
fn switch_case_one_matches() {
    exercise_sector(1, 0x1c7b_18ea_d591_4d2f);
}

#[test]
fn switch_case_two_matches() {
    exercise_sector(2, 0xe80a_923a_471c_9b09);
}

#[test]
fn switch_case_three_matches() {
    exercise_sector(3, 0x3380_12da_2610_70bb);
}

#[test]
fn switch_case_four_matches() {
    exercise_sector(4, 0x6d90_47bb_2b0d_c5eb);
}

#[test]
fn switch_default_matches() {
    exercise(0x947c_23f6_ab07_a493, |rng, index| {
        let sector = match index % 4 {
            0 => 5,
            1 => -1,
            2 => 6,
            _ => 12,
        };
        let hue = sector as f32 * 60.0 + rng.range(0.0, 60.0);
        [hue, rng.nonzero_saturation(), rng.range(-10.0, 10.0)]
    });
}

const CHILD_LIBRARY: &str = "HSV_TO_RGB_CHILD_LIBRARY";
const CHILD_POINTER: &str = "HSV_TO_RGB_CHILD_POINTER";

#[test]
fn null_pointer_child() {
    let Ok(library_name) = std::env::var(CHILD_LIBRARY) else {
        return;
    };
    let pointer_name = std::env::var(CHILD_POINTER).expect("child pointer selection");
    let libraries = Libraries::load();
    let function = match library_name.as_str() {
        "c" => libraries.c_hsv_to_rgb,
        "rust" => libraries.rust_hsv_to_rgb,
        _ => panic!("unknown child library {library_name}"),
    };

    unsafe {
        match pointer_name.as_str() {
            "source" => {
                let mut output = [0.0_f32; 3];
                function(output.as_mut_ptr(), std::ptr::null());
            }
            "destination" => {
                let input = [0.0_f32, 0.0_f32, 1.0_f32];
                function(std::ptr::null_mut(), input.as_ptr());
            }
            _ => panic!("unknown child pointer {pointer_name}"),
        }
    }

    panic!("invalid pointer unexpectedly returned");
}

fn null_pointer_status(library: &str, pointer: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env(CHILD_LIBRARY, library)
        .env(CHILD_POINTER, pointer)
        .status()
        .expect("run null-pointer child")
}

#[cfg(unix)]
#[test]
fn null_pointer_process_behavior_matches() {
    use std::os::unix::process::ExitStatusExt;

    for pointer in ["source", "destination"] {
        let c_status = null_pointer_status("c", pointer);
        let rust_status = null_pointer_status("rust", pointer);
        assert!(!c_status.success(), "C accepted null {pointer} pointer");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "null {pointer}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}
