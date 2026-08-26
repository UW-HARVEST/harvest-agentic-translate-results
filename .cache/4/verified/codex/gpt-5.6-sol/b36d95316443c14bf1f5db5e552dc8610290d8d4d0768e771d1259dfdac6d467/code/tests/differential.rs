use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[repr(C)]
struct TflacBitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

type BitwriterAdd = unsafe extern "C" fn(*mut TflacBitwriter, u32, u64) -> c_int;

struct Api {
    _library: Library,
    bitwriter_add: BitwriterAdd,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let bitwriter_add = unsafe {
            *library
                .get::<BitwriterAdd>(b"bitwriter_add\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load bitwriter_add from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            bitwriter_add,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Case {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: usize,
    add_bits: u32,
    add_val: u64,
}

#[repr(C, align(8))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct StateBytes([u8; 32]);

impl StateBytes {
    fn from_case(case: Case) -> Self {
        let mut bytes = [0_u8; 32];
        bytes[0..8].copy_from_slice(&case.val.to_ne_bytes());
        bytes[8..12].copy_from_slice(&case.bits.to_ne_bytes());
        bytes[12..16].copy_from_slice(&case.pos.to_ne_bytes());
        bytes[16..20].copy_from_slice(&case.len.to_ne_bytes());
        bytes[20..24].copy_from_slice(&case.tot.to_ne_bytes());
        bytes[24..32].copy_from_slice(&case.buffer.to_ne_bytes());
        Self(bytes)
    }

    fn as_writer_mut(&mut self) -> *mut TflacBitwriter {
        self.0.as_mut_ptr().cast()
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }

    fn compare(&self, label: &str, iteration: usize, case: Case) {
        let mut c_state = StateBytes::from_case(case);
        let mut rust_state = c_state.clone();

        let c_result =
            unsafe { (self.c.bitwriter_add)(c_state.as_writer_mut(), case.add_bits, case.add_val) };
        let rust_result = unsafe {
            (self.rust.bitwriter_add)(rust_state.as_writer_mut(), case.add_bits, case.add_val)
        };

        assert_eq!(
            rust_result, c_result,
            "{label} iteration {iteration}: return mismatch for {case:?}"
        );
        assert_eq!(
            rust_state, c_state,
            "{label} iteration {iteration}: state mismatch for {case:?}"
        );
    }
}

#[derive(Clone, Copy)]
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

    fn inclusive_u32(&mut self, min: u32, max: u32) -> u32 {
        let width = u64::from(max) - u64::from(min) + 1;
        min + (self.next_u64() % width) as u32
    }

    fn case(&mut self, bits: u32, add_bits: u32, buffer: usize) -> Case {
        Case {
            val: self.next_u64(),
            bits,
            pos: self.next_u32(),
            len: self.next_u32(),
            tot: self.next_u32(),
            buffer,
            add_bits,
            add_val: self.next_u64(),
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable path");
    let deps = executable.parent().expect("target profile deps directory");
    let profile = deps.parent().expect("target profile directory");
    let candidates = [
        profile.join("libbitwriter_add_lib.so"),
        deps.join("libbitwriter_add_lib.so"),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("libbitwriter_add_lib.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found; checked profile {}, deps {}, and release",
                profile.display(),
                deps.display()
            )
        })
}

fn buffer_pointer(rng: &mut Rng, backing: &mut u8) -> usize {
    if rng.next_u64() & 1 == 0 {
        0
    } else {
        std::ptr::from_mut(backing) as usize
    }
}

#[test]
fn config_c1_empty_writer_below_word_boundary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x3f84_d5b5_b547_0917);
    let mut backing = 0_u8;

    for iteration in 0..4096 {
        let add_bits = rng.inclusive_u32(1, 63);
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let mut case = rng.case(0, add_bits, pointer);
        apply_scalar_extremes(iteration, &mut case);
        libraries.compare("C1", iteration, case);
    }
}

#[test]
fn config_c2_partial_writer_below_word_boundary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
    let mut backing = 0_u8;

    for iteration in 0..4096 {
        let bits = rng.inclusive_u32(1, 62);
        let add_bits = rng.inclusive_u32(1, 63 - bits);
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let mut case = rng.case(bits, add_bits, pointer);
        apply_scalar_extremes(iteration, &mut case);
        libraries.compare("C2", iteration, case);
    }
}

#[test]
fn config_c3_exact_word_boundary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xd1b5_4a32_d192_ed03);
    let mut backing = 0_u8;

    for iteration in 0..4096 {
        let bits = rng.inclusive_u32(0, 63);
        let add_bits = 64 - bits;
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let mut case = rng.case(bits, add_bits, pointer);
        apply_scalar_extremes(iteration, &mut case);
        libraries.compare("C3", iteration, case);
    }
}

#[test]
fn config_c4_above_word_boundary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x94d0_49bb_1331_11eb);
    let mut backing = 0_u8;

    for iteration in 0..4096 {
        let bits = rng.inclusive_u32(1, 63);
        let add_bits = rng.inclusive_u32(65 - bits, 64);
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let mut case = rng.case(bits, add_bits, pointer);
        apply_scalar_extremes(iteration, &mut case);
        libraries.compare("C4", iteration, case);
    }
}

fn apply_scalar_extremes(iteration: usize, case: &mut Case) {
    match iteration {
        0 => {
            case.val = 0;
            case.tot = 0;
            case.add_val = 0;
        }
        1 => {
            case.val = u64::MAX;
            case.tot = u32::MAX;
            case.add_val = u64::MAX;
        }
        _ => {}
    }
}

#[test]
fn generic_g2_zero_bit_count() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xbf58_476d_1ce4_e5b9);
    let mut backing = 0_u8;

    for iteration in 0..1024 {
        let bits = rng.inclusive_u32(0, 63);
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let case = rng.case(bits, 0, pointer);
        libraries.compare("G2", iteration, case);
    }
}

#[test]
fn generic_g3_oversized_bit_count() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x4f1b_bcdd_3a2c_7d45);
    let mut backing = 0_u8;

    for (iteration, add_bits) in [65, u32::MAX].into_iter().cycle().take(2048).enumerate() {
        let bits = rng.inclusive_u32(0, 63);
        let pointer = buffer_pointer(&mut rng, &mut backing);
        let case = rng.case(bits, add_bits, pointer);
        libraries.compare("G3", iteration, case);
    }
}

#[test]
fn null_pointer_child() {
    let Some(which) = std::env::var_os("BITWRITER_NULL_CHILD") else {
        return;
    };
    let path = if which == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::load(&path) };

    unsafe {
        (api.bitwriter_add)(std::ptr::null_mut(), 1, 0);
    }
}

fn run_null_child(which: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("BITWRITER_NULL_CHILD", which)
        .status()
        .unwrap_or_else(|error| panic!("failed to run {which} null-pointer child: {error}"))
}

#[cfg(unix)]
#[test]
fn generic_g1_null_pointer_termination_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_null_child("c");
    let rust_status = run_null_child("rust");

    assert!(!c_status.success(), "C unexpectedly accepted a null writer");
    assert!(
        !rust_status.success(),
        "Rust unexpectedly accepted a null writer"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "null writer terminated differently: C={c_status:?}, Rust={rust_status:?}"
    );
}
