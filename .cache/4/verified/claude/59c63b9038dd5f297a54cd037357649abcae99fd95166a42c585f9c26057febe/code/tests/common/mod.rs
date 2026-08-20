//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported `update_frame_header` symbol. No Rust function
//! is ever called directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what is under test.

#![allow(dead_code)]

use std::path::PathBuf;

use libloading::{Library, Symbol};

/// `struct tflac` from `c_src/include/lib.h`.
///
/// Layout verified against the C compiler on this platform:
/// ```text
/// sizeof=24 align=4
/// samplerate=0 channels=4 bitdepth=8 channel_mode=12 frame_header=16 cur_blocksize=20
/// ```
/// The 3 padding bytes after `channel_mode` are made explicit so the test can
/// initialise them deterministically and prove neither implementation touches
/// them.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tflac {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub pad: [u8; 3],
    pub frame_header: u32,
    pub cur_blocksize: u32,
}

impl Default for Tflac {
    fn default() -> Self {
        Self {
            samplerate: 0,
            channels: 0,
            bitdepth: 0,
            channel_mode: 0,
            pad: [0; 3],
            frame_header: 0,
            cur_blocksize: 0,
        }
    }
}

impl std::fmt::Debug for Tflac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tflac {{ samplerate: {} (0x{:08X}), channels: {} (0x{:08X}), \
             bitdepth: {} (0x{:08X}), channel_mode: {} (0x{:02X}), pad: {:02X?}, \
             frame_header: 0x{:08X}, cur_blocksize: {} (0x{:08X}) }}",
            self.samplerate,
            self.samplerate,
            self.channels,
            self.channels,
            self.bitdepth,
            self.bitdepth,
            self.channel_mode,
            self.channel_mode,
            self.pad,
            self.frame_header,
            self.cur_blocksize,
            self.cur_blocksize
        )
    }
}

impl Tflac {
    pub fn new(samplerate: u32, channels: u32, bitdepth: u32, channel_mode: u8, cur_blocksize: u32) -> Self {
        Self {
            samplerate,
            channels,
            bitdepth,
            channel_mode,
            pad: [0; 3],
            frame_header: 0,
            cur_blocksize,
        }
    }

    /// All 24 bytes of the struct, padding included.
    pub fn as_bytes(&self) -> [u8; 24] {
        let mut out = [0u8; 24];
        // SAFETY: `Tflac` is `#[repr(C)]`, 24 bytes, and has no invalid bit
        // patterns (every field is an integer / integer array).
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Tflac as *const u8,
                out.as_mut_ptr(),
                std::mem::size_of::<Tflac>(),
            );
        }
        out
    }
}

pub type UpdateFrameHeaderFn = unsafe extern "C" fn(*mut Tflac);

/// Directory holding this crate's build artifacts (`target/debug` or
/// `target/release`), derived from the running test executable's path
/// (`target/<profile>/deps/<test>-<hash>`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared object. Overridable with `HARVEST_C_SO` so the same
/// suite can be run against C objects built at different optimisation levels
/// (`c_src/CMakeLists.txt` pins no `CMAKE_BUILD_TYPE`, so a consumer may build
/// it with or without optimisation).
pub fn c_so_path() -> PathBuf {
    match std::env::var_os("HARVEST_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src/build/libtranslated_rust.so"),
    }
}

pub fn rust_so_path() -> PathBuf {
    target_profile_dir().join("libupdate_frame_header_lib.so")
}

/// Holds both `dlopen`ed libraries plus the resolved symbols.
pub struct Diff {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: UpdateFrameHeaderFn,
    pub rust: UpdateFrameHeaderFn,
    pub mismatches: usize,
    pub cases: usize,
    first_failure: Option<String>,
}

impl Diff {
    pub fn load() -> Self {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.is_file(),
            "C shared object not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared object not found at {}. Build it with `cargo build`.",
            rust_path.display()
        );

        // SAFETY: both objects are plain C-ABI libraries with no init side
        // effects beyond the usual CRT/std startup.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

        let c = unsafe {
            let s: Symbol<UpdateFrameHeaderFn> = c_lib
                .get(b"update_frame_header\0")
                .expect("C .so is missing symbol `update_frame_header`");
            *s
        };
        let rust = unsafe {
            let s: Symbol<UpdateFrameHeaderFn> = rust_lib
                .get(b"update_frame_header\0")
                .expect("Rust .so is missing symbol `update_frame_header`");
            *s
        };

        Self {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            mismatches: 0,
            cases: 0,
            first_failure: None,
        }
    }

    /// Runs one differential case: calls the C `.so` and the Rust `.so` on
    /// identical copies of `input` and compares **all 24 struct bytes**.
    pub fn check(&mut self, label: &str, input: Tflac) {
        self.cases += 1;

        let mut c_state = input;
        let mut rust_state = input;
        unsafe {
            (self.c)(&mut c_state as *mut Tflac);
            (self.rust)(&mut rust_state as *mut Tflac);
        }

        let c_bytes = c_state.as_bytes();
        let rust_bytes = rust_state.as_bytes();
        if c_bytes != rust_bytes {
            self.mismatches += 1;
            if self.first_failure.is_none() {
                self.first_failure = Some(format!(
                    "[{label}]\n  input : {input:?}\n  C     : {c_state:?}\n  Rust  : {rust_state:?}\n  \
                     C bytes   : {c_bytes:02X?}\n  Rust bytes: {rust_bytes:02X?}\n  \
                     frame_header C=0x{:08X} Rust=0x{:08X} (xor 0x{:08X})",
                    c_state.frame_header,
                    rust_state.frame_header,
                    c_state.frame_header ^ rust_state.frame_header
                ));
            }
        }
    }

    /// Same as [`Self::check`], but returns the C result so a test can make
    /// additional assertions about the ground-truth value.
    pub fn check_and_get(&mut self, label: &str, input: Tflac) -> Tflac {
        self.check(label, input);
        let mut c_state = input;
        unsafe { (self.c)(&mut c_state as *mut Tflac) };
        c_state
    }

    pub fn finish(&self, what: &str) {
        if let Some(f) = &self.first_failure {
            panic!(
                "{what}: {} / {} differential cases MISMATCHED between the C .so and the Rust .so.\n\
                 First failure:\n{f}",
                self.mismatches, self.cases
            );
        }
        assert!(self.cases > 0, "{what}: no cases were executed");
        eprintln!("{what}: {} cases, 0 mismatches", self.cases);
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible, no dependencies.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    /// A struct with all four input fields uniformly random and deterministic
    /// non-zero padding / prior `frame_header` so that "must not be touched"
    /// and "must be overwritten, not OR-ed" are both under test.
    pub fn tflac(&mut self) -> Tflac {
        Tflac {
            samplerate: self.next_u32(),
            channels: self.next_u32(),
            bitdepth: self.next_u32(),
            channel_mode: self.next_u8(),
            pad: [self.next_u8(), self.next_u8(), self.next_u8()],
            frame_header: self.next_u32(),
            cur_blocksize: self.next_u32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Class representatives, mechanically transcribed from c_src/src/lib.c.
// ---------------------------------------------------------------------------

/// One representative per `cur_blocksize` class B1..B15 (see CONFIGS.md).
pub const BS_CLASS_REPS: [u32; 15] = [
    192, 576, 1152, 2304, 4608, // B1..B5
    256, 512, 1024, 2048, 4096, 8192, 16384, 32768, // B6..B13
    0,   // B14: default & <= 256
    257, // B15: default & > 256
];

/// One representative per `samplerate` class S1..S17 (see CONFIGS.md).
pub const SR_CLASS_REPS: [u32; 17] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000, // S1..S11
    1000,      // S12: %1000==0, /1000 < 256
    256000,    // S13: %1000==0, /1000 >= 256      -> no bits
    22051,     // S14: %1000!=0, < 65536
    88200,     // S15: %1000!=0, >=65536, %10==0, /10 < 65536
    655360,    // S16: %1000!=0, >=65536, %10==0, /10 >= 65536 -> no bits
    65537,     // S17: %1000!=0, >=65536, %10!=0   -> no bits
];

/// One representative per channel class C1..C8: `(channel_mode, channels)`.
pub const CH_CLASS_REPS: [(u8, u32); 8] = [
    (0, 1),          // C1
    (0, 2),          // C2
    (0, 8),          // C3
    (0, 0),          // C4: unsigned underflow
    (0, 0xFFFFFFFF), // C5: overflow past the 4-bit field
    (1, 2),          // C6: LEFT_SIDE
    (2, 2),          // C7: SIDE_RIGHT
    (3, 2),          // C8: MID_SIDE
];

/// One representative per `bitdepth` class D1..D7.
pub const BD_CLASS_REPS: [u32; 7] = [8, 12, 16, 20, 24, 32, 0];

/// The 13 `cur_blocksize` values with an explicit `case` label.
pub const BS_LITERALS: [u32; 13] =
    [192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768];

/// The 11 `samplerate` values with an explicit `case` label (note: the C source
/// really does list `882000`, not the FLAC-spec `88200`).
pub const SR_LITERALS: [u32; 11] =
    [882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000];

/// The 6 `bitdepth` values with an explicit `case` label.
pub const BD_LITERALS: [u32; 6] = [8, 12, 16, 20, 24, 32];
