use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const CASES_PER_ROW: usize = 2_048;
const NULL_CHILD_ENV: &str = "TFLAC_NULL_CHILD_LIBRARY";

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TflacBitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

type BitwriterAdd = unsafe extern "C" fn(*mut TflacBitwriter, u32, u64) -> c_int;

struct LoadedApi {
    _library: Library,
    bitwriter_add: BitwriterAdd,
}

impl LoadedApi {
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

    unsafe fn add(&self, state: *mut TflacBitwriter, bits: u32, val: u64) -> c_int {
        unsafe { (self.bitwriter_add)(state, bits, val) }
    }
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

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn range_inclusive(&mut self, start: u32, end: u32) -> u32 {
        start + self.next_u32() % (end - start + 1)
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-jeC2iM.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libbitwriter_add_lib.so")
}

fn random_state(rng: &mut Rng, bits: u32) -> TflacBitwriter {
    TflacBitwriter {
        val: rng.next_u64(),
        bits,
        pos: rng.next_u32(),
        len: rng.next_u32(),
        tot: rng.next_u32(),
        buffer: if rng.next_u32() & 1 == 0 {
            std::ptr::null_mut()
        } else {
            std::ptr::dangling_mut()
        },
    }
}

fn state_bytes(state: &TflacBitwriter) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref(state).cast::<u8>(),
            std::mem::size_of::<TflacBitwriter>(),
        )
    }
}

fn compare_case(
    c_api: &LoadedApi,
    rust_api: &LoadedApi,
    initial: TflacBitwriter,
    bits: u32,
    val: u64,
    context: &str,
) {
    let mut c_state = initial;
    let mut rust_state = initial;
    let c_result = unsafe { c_api.add(&mut c_state, bits, val) };
    let rust_result = unsafe { rust_api.add(&mut rust_state, bits, val) };

    assert_eq!(rust_result, c_result, "{context}: return value");
    assert_eq!(
        state_bytes(&rust_state),
        state_bytes(&c_state),
        "{context}: output state\nC: {c_state:?}\nRust: {rust_state:?}"
    );
}

fn run_randomized_row(seed: u64, mut case: impl FnMut(&mut Rng) -> (TflacBitwriter, u32, u64)) {
    assert_eq!(std::mem::size_of::<TflacBitwriter>(), 32);
    let c_api = unsafe { LoadedApi::load(&c_library_path()) };
    let rust_api = unsafe { LoadedApi::load(&rust_library_path()) };
    let mut rng = Rng::new(seed);

    for index in 0..CASES_PER_ROW {
        let (initial, bits, val) = case(&mut rng);
        compare_case(
            &c_api,
            &rust_api,
            initial,
            bits,
            val,
            &format!("seed={seed:#x}, case={index}, bits={bits}"),
        );
    }
}

#[test]
fn config_1_zero_width() {
    run_randomized_row(0x243f_6a88_85a3_08d3, |rng| {
        let initial_bits = rng.range_inclusive(0, 63);
        (random_state(rng, initial_bits), 0, rng.next_u64())
    });
}

#[test]
fn config_2_sum_below_word_width() {
    run_randomized_row(0x1319_8a2e_0370_7344, |rng| {
        let initial_bits = rng.range_inclusive(0, 62);
        let bits = rng.range_inclusive(1, 63 - initial_bits);
        (random_state(rng, initial_bits), bits, rng.next_u64())
    });
}

#[test]
fn config_3_sum_at_word_width() {
    run_randomized_row(0xa409_3822_299f_31d0, |rng| {
        let initial_bits = rng.range_inclusive(0, 62);
        let bits = 64 - initial_bits;
        (random_state(rng, initial_bits), bits, rng.next_u64())
    });
}

#[test]
fn config_4_sum_above_word_width() {
    run_randomized_row(0x082e_fa98_ec4e_6c89, |rng| {
        let initial_bits = rng.range_inclusive(1, 62);
        let bits = rng.range_inclusive(65 - initial_bits, 64);
        (random_state(rng, initial_bits), bits, rng.next_u64())
    });
}

#[test]
fn config_5_initial_state_at_word_boundary() {
    run_randomized_row(0x4528_21e6_38d0_1377, |rng| {
        let bits = rng.range_inclusive(1, 64);
        (random_state(rng, 63), bits, rng.next_u64())
    });
}

#[test]
fn generic_oversized_widths() {
    const WIDTHS: [u32; 8] = [65, 66, 127, 128, 255, 256, u32::MAX - 1, u32::MAX];

    run_randomized_row(0xbe54_66cf_34e9_0c6c, |rng| {
        let bits = WIDTHS[rng.next_u32() as usize % WIDTHS.len()];
        let initial_bits = rng.range_inclusive(0, 63);
        (random_state(rng, initial_bits), bits, rng.next_u64())
    });
}

#[test]
fn generic_out_of_range_state_widths() {
    const STATE_WIDTHS: [u32; 7] = [64, 65, 127, 128, 255, 256, u32::MAX];

    run_randomized_row(0xc0ac_29b7_c97c_50dd, |rng| {
        let initial_bits = STATE_WIDTHS[rng.next_u32() as usize % STATE_WIDTHS.len()];
        let bits = rng.next_u32();
        (random_state(rng, initial_bits), bits, rng.next_u64())
    });
}

fn run_null_child(path: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("null_bw_boundary_matches")
        .arg("--nocapture")
        .env(NULL_CHILD_ENV, path)
        .status()
        .unwrap_or_else(|error| panic!("failed to run null child for {}: {error}", path.display()))
}

#[test]
fn null_bw_boundary_matches() {
    if let Some(path) = std::env::var_os(NULL_CHILD_ENV) {
        let api = unsafe { LoadedApi::load(Path::new(&path)) };
        let _ = unsafe { api.add(std::ptr::null_mut(), 1, 1) };
        panic!("null bitwriter pointer unexpectedly returned");
    }

    let c_status = run_null_child(&c_library_path());
    let rust_status = run_null_child(&rust_library_path());
    assert!(!c_status.success(), "C accepted a null bitwriter pointer");
    assert_eq!(
        rust_status, c_status,
        "null bitwriter process termination differs"
    );
}
