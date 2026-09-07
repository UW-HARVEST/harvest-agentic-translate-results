use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::SystemTime;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Bitwriter {
    val: u64,
    bits: u32,
    pos: u32,
    len: u32,
    tot: u32,
    buffer: *mut u8,
}

type BitwriterAdd = unsafe extern "C" fn(*mut Bitwriter, u32, u64) -> c_int;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        let c = unsafe { Library::new(&c_path) }.unwrap_or_else(|error| {
            panic!("failed to load C library {}: {error}", c_path.display())
        });
        let rust = unsafe { Library::new(&rust_path) }.unwrap_or_else(|error| {
            panic!(
                "failed to load Rust library {}: {error}",
                rust_path.display()
            )
        });
        Self { c, rust }
    }

    fn compare(&self, label: &str, initial: Bitwriter, bits: u32, val: u64) {
        let mut c_state = initial;
        let mut rust_state = initial;

        let c_result = unsafe {
            let function: Symbol<'_, BitwriterAdd> =
                self.c.get(b"bitwriter_add\0").expect("C symbol missing");
            function(&mut c_state, bits, val)
        };
        let rust_result = unsafe {
            let function: Symbol<'_, BitwriterAdd> = self
                .rust
                .get(b"bitwriter_add\0")
                .expect("Rust symbol missing");
            function(&mut rust_state, bits, val)
        };

        assert_eq!(
            c_result, rust_result,
            "{label}: return mismatch for initial={initial:?}, bits={bits}, val={val:#018x}"
        );
        assert_eq!(
            state_bytes(&c_state),
            state_bytes(&rust_state),
            "{label}: state mismatch for initial={initial:?}, bits={bits}, val={val:#018x}; \
             C={c_state:?}, Rust={rust_state:?}"
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
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn range_inclusive(&mut self, min: u32, max: u32) -> u32 {
        assert!(min <= max);
        min + (self.next_u32() % (max - min + 1))
    }

    fn state(&mut self, bits: u32, tot: u32) -> Bitwriter {
        Bitwriter {
            val: self.next_u64(),
            bits,
            pos: self.next_u32(),
            len: self.next_u32(),
            tot,
            buffer: self.next_u64() as usize as *mut u8,
        }
    }
}

fn state_bytes(state: &Bitwriter) -> [u8; size_of::<Bitwriter>()] {
    let mut bytes = [0_u8; size_of::<Bitwriter>()];
    unsafe {
        std::ptr::copy_nonoverlapping(
            std::ptr::from_ref(state).cast::<u8>(),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    bytes
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sole_shared_library(directory: &Path, prefix: &str) -> PathBuf {
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.is_file() && name.starts_with(prefix) && name.ends_with(".so") {
                matches.push(path);
            }
        }
    }
    assert_eq!(
        matches.len(),
        1,
        "expected one {prefix}*.so in {}, found {matches:?}",
        directory.display()
    );
    matches.pop().unwrap()
}

fn c_library_path() -> PathBuf {
    sole_shared_library(&crate_root().join("../c_src/build"), "lib")
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("BITWRITER_RUST_SO") {
        return PathBuf::from(path);
    }

    let target = crate_root().join("target");
    let mut candidates = Vec::new();
    for directory in [
        target.join("debug"),
        target.join("debug/deps"),
        target.join("release"),
        target.join("release/deps"),
    ] {
        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                if path.is_file()
                    && name.starts_with("libbitwriter_add_lib")
                    && name.ends_with(".so")
                {
                    let modified = entry
                        .metadata()
                        .and_then(|metadata| metadata.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    candidates.push((modified, path));
                }
            }
        }
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    candidates
        .pop()
        .map(|(_, path)| path)
        .expect("Rust cdylib not found; build the crate before running this test")
}

#[test]
fn config_1_no_loop_nonwrapping_tot() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e001_0000_0001);
    for case in 0..512 {
        let initial_bits = rng.range_inclusive(0, 62);
        let input_bits = rng.range_inclusive(1, 63 - initial_bits);
        let tot = rng.range_inclusive(0, u32::MAX - input_bits);
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 1 case {case}"),
            state,
            input_bits,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_2_no_loop_wrapping_tot() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e002_0000_0002);
    for case in 0..512 {
        let initial_bits = rng.range_inclusive(0, 62);
        let input_bits = rng.range_inclusive(1, 63 - initial_bits);
        let distance_from_max = rng.range_inclusive(0, input_bits - 1);
        let state = rng.state(initial_bits, u32::MAX - distance_from_max);
        libraries.compare(
            &format!("CONFIGS row 2 case {case}"),
            state,
            input_bits,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_3_loop_available_branch_and_cap() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e003_0000_0003);
    for case in 0..512 {
        let initial_bits = rng.range_inclusive(1, 63);
        let boundary = 64 - initial_bits;
        let input_bits = if case % 2 == 0 {
            boundary
        } else {
            rng.range_inclusive(boundary, 63)
        };
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 3 case {case}"),
            state,
            input_bits,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_4_loop_input_branch_and_cap() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e004_0000_0004);
    for case in 0..512 {
        let initial_bits = rng.range_inclusive(64, 4096);
        let input_bits = rng.range_inclusive(1, 63);
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 4 case {case}"),
            state,
            input_bits,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_5_native_width_64() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e005_0000_0005);
    for case in 0..512 {
        let initial_bits = if case % 2 == 0 {
            rng.range_inclusive(0, 63)
        } else {
            rng.next_u32()
        };
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 5 case {case}"),
            state,
            64,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_6_zero_width_boundary() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e006_0000_0006);
    for case in 0..512 {
        let initial_bits = if case % 2 == 0 {
            rng.range_inclusive(0, 63)
        } else {
            rng.range_inclusive(64, 4096)
        };
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 6 case {case}"),
            state,
            0,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_7_one_past_width_65() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e007_0000_0007);
    for case in 0..512 {
        let initial_bits = if case % 2 == 0 {
            rng.range_inclusive(0, 4096)
        } else {
            u32::MAX - 64 + rng.range_inclusive(0, 63)
        };
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 7 case {case}"),
            state,
            65,
            rng.next_u64(),
        );
    }
}

#[test]
fn config_8_oversized_widths_and_wrapping_sums() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xa11c_e008_0000_0008);
    let widths = [
        66,
        127,
        128,
        129,
        255,
        256,
        257,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX - 1,
        u32::MAX,
    ];
    for case in 0..1024 {
        let input_bits = widths[case % widths.len()];
        let initial_bits = if case % 2 == 0 {
            0_u32
                .wrapping_sub(input_bits)
                .wrapping_add(rng.range_inclusive(0, 63))
        } else {
            64_u32
                .wrapping_sub(input_bits)
                .wrapping_add(rng.range_inclusive(0, 4096))
        };
        let tot = rng.next_u32();
        let state = rng.state(initial_bits, tot);
        libraries.compare(
            &format!("CONFIGS row 8 case {case}"),
            state,
            input_bits,
            rng.next_u64(),
        );
    }
}

#[test]
fn generic_boundary_return_values_are_exact() {
    let libraries = Libraries::load();
    let base = Bitwriter {
        val: 0x0123_4567_89ab_cdef,
        bits: 0,
        pos: 0x1122_3344,
        len: u32::MAX,
        tot: u32::MAX - 10,
        buffer: 0x1234_usize as *mut u8,
    };
    for (bits, val) in [
        (0, 0),
        (64, u64::MAX),
        (65, 1),
        (u32::MAX, 0x8000_0000_0000_0001),
    ] {
        libraries.compare("generic ABI boundary", base, bits, val);
    }
}

fn run_null_child(library: &Path) -> ExitStatus {
    let output = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--ignored", "--nocapture"])
        .env("NULL_POINTER_LIBRARY", library)
        .output()
        .expect("failed to run null-pointer child");
    output.status
}

#[test]
fn generic_null_pointer_termination_matches() {
    let c_status = run_null_child(&c_library_path());
    let rust_status = run_null_child(&rust_library_path());
    assert!(!c_status.success(), "C unexpectedly accepted null bw");
    assert!(!rust_status.success(), "Rust unexpectedly accepted null bw");
    assert_eq!(
        c_status.signal(),
        rust_status.signal(),
        "null-pointer termination differs: C={c_status:?}, Rust={rust_status:?}"
    );
    assert_eq!(
        c_status.code(),
        rust_status.code(),
        "null-pointer exit code differs: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[test]
#[ignore = "child process used by generic_null_pointer_termination_matches"]
fn null_pointer_child() {
    let path = std::env::var_os("NULL_POINTER_LIBRARY")
        .map(PathBuf::from)
        .expect("NULL_POINTER_LIBRARY must be set");
    let library = unsafe { Library::new(path) }.expect("load child library");
    let function: Symbol<'_, BitwriterAdd> =
        unsafe { library.get(b"bitwriter_add\0") }.expect("load bitwriter_add");
    unsafe {
        function(std::ptr::null_mut(), 1, 0);
    }
}
