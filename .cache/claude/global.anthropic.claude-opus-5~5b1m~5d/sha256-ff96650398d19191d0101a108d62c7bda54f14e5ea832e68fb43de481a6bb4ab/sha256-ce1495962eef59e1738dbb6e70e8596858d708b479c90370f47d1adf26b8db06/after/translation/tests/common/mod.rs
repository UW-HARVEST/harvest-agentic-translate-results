//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported `update_frame_header` symbol — the Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::Library;

/// `void update_frame_header(tflac *t)` — the record is passed as raw bytes so
/// that padding and any out-of-bounds writes are observable.
pub type UpdateFrameHeaderFn = unsafe extern "C" fn(*mut u8);

// ---------------------------------------------------------------------------
// struct tflac, byte-exact (verified with offsetof/sizeof on the C side)
// ---------------------------------------------------------------------------

pub const OFF_SAMPLERATE: usize = 0;
pub const OFF_CHANNELS: usize = 4;
pub const OFF_BITDEPTH: usize = 8;
pub const OFF_CHANNEL_MODE: usize = 12;
pub const OFF_FRAME_HEADER: usize = 16;
pub const OFF_CUR_BLOCKSIZE: usize = 20;
pub const TFLAC_SIZE: usize = 24;

/// Guard bytes placed before and after the record to catch stray writes.
pub const GUARD: usize = 16;
pub const BUF_LEN: usize = GUARD + TFLAC_SIZE + GUARD;

/// The inputs a caller sets before invoking `update_frame_header`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    /// Pre-call value of the output field; the C overwrites it unconditionally.
    pub frame_header: u32,
    pub cur_blocksize: u32,
    /// Pre-call value of the 3 padding bytes at offsets 13..16.
    pub padding: [u8; 3],
}

impl Default for Input {
    fn default() -> Self {
        Input {
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: 0,
            frame_header: 0,
            cur_blocksize: 4096,
            padding: [0, 0, 0],
        }
    }
}

impl Input {
    pub fn new(
        samplerate: u32,
        channels: u32,
        bitdepth: u32,
        channel_mode: u8,
        cur_blocksize: u32,
    ) -> Self {
        Input {
            samplerate,
            channels,
            bitdepth,
            channel_mode,
            frame_header: 0,
            cur_blocksize,
            padding: [0, 0, 0],
        }
    }
}

/// A 16-byte-aligned buffer: `[guard | tflac | guard]`.
#[repr(align(16))]
pub struct Buf(pub [u8; BUF_LEN]);

impl Buf {
    /// Lay the input out exactly as the C compiler would, surrounded by guards.
    pub fn new(input: &Input) -> Self {
        let mut b = Buf([0xA5u8; BUF_LEN]);
        let r = &mut b.0[GUARD..GUARD + TFLAC_SIZE];
        r[OFF_SAMPLERATE..OFF_SAMPLERATE + 4].copy_from_slice(&input.samplerate.to_ne_bytes());
        r[OFF_CHANNELS..OFF_CHANNELS + 4].copy_from_slice(&input.channels.to_ne_bytes());
        r[OFF_BITDEPTH..OFF_BITDEPTH + 4].copy_from_slice(&input.bitdepth.to_ne_bytes());
        r[OFF_CHANNEL_MODE] = input.channel_mode;
        r[OFF_CHANNEL_MODE + 1..OFF_CHANNEL_MODE + 4].copy_from_slice(&input.padding);
        r[OFF_FRAME_HEADER..OFF_FRAME_HEADER + 4].copy_from_slice(&input.frame_header.to_ne_bytes());
        r[OFF_CUR_BLOCKSIZE..OFF_CUR_BLOCKSIZE + 4]
            .copy_from_slice(&input.cur_blocksize.to_ne_bytes());
        b
    }

    /// Pointer to the `tflac` record (4-byte aligned; actually 16).
    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { self.0.as_mut_ptr().add(GUARD) }
    }

    /// The whole buffer, guards included.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn frame_header(&self) -> u32 {
        let mut w = [0u8; 4];
        w.copy_from_slice(&self.0[GUARD + OFF_FRAME_HEADER..GUARD + OFF_FRAME_HEADER + 4]);
        u32::from_ne_bytes(w)
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so`: `c_src/build/lib<parent-dir-name>.so`. The exact file name is
/// derived by CMake from the repository directory name, so it is discovered by
/// scanning rather than hard-coded. Override with `$C_LIB`.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && \
                 cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust cdylib, sitting next to the test executable's directory
/// (`target/<profile>/libupdate_frame_header_lib.so`). Override with `$RUST_LIB`.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test-bin>  ->  target/<profile>/
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    let name = "libupdate_frame_header_lib.so";
    for _ in 0..3 {
        let cand = dir.join(name);
        if cand.is_file() {
            return cand;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    panic!(
        "could not locate {name} near {}; run `cargo build` first or set $RUST_LIB",
        exe.display()
    );
}

/// The *release* cdylib — the artifact an external consumer actually links
/// against (`cargo build --release`). `None` if it has not been built.
pub fn rust_lib_release_path() -> Option<PathBuf> {
    // The .so may live in target/<profile>/ or target/<profile>/deps/, so find
    // the `target` dir by walking up past the profile directory.
    let p = rust_lib_path();
    let mut dir = p.parent()?;
    loop {
        let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == "debug" || name == "release" {
            let cand = dir.parent()?.join("release").join("libupdate_frame_header_lib.so");
            return if cand.is_file() { Some(cand) } else { None };
        }
        dir = dir.parent()?;
    }
}

/// True if the shared object was built with `debug_assertions`, which makes
/// rustc inject its own null-pointer / UB checks that `panic!` (and, inside an
/// `extern "C"` fn, `abort`) *before* the faulting memory access happens.
pub fn has_rustc_ub_checks(path: &Path) -> bool {
    let bytes = std::fs::read(path).unwrap_or_default();
    let needle = b"null pointer dereference occurred";
    bytes.windows(needle.len()).any(|w| w == needle)
}

/// Load `update_frame_header` from one specific shared object.
pub fn load_symbol(path: &Path) -> UpdateFrameHeaderFn {
    load(path)
}

fn load(path: &Path) -> UpdateFrameHeaderFn {
    unsafe {
        // Leaked on purpose: the function pointer must stay valid for the whole
        // test-process lifetime.
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        let sym = lib
            .get::<UpdateFrameHeaderFn>(b"update_frame_header\0")
            .unwrap_or_else(|e| panic!("dlsym update_frame_header in {}: {e}", path.display()));
        *sym
    }
}

/// Both implementations, loaded through `dlopen`/`dlsym`.
pub struct Diff {
    pub c: UpdateFrameHeaderFn,
    pub rust: UpdateFrameHeaderFn,
}

impl Diff {
    pub fn load() -> &'static Diff {
        use std::sync::OnceLock;
        static ONCE: OnceLock<Diff> = OnceLock::new();
        ONCE.get_or_init(|| Diff {
            c: load(&c_lib_path()),
            rust: load(&rust_lib_path()),
        })
    }

    /// Run the C implementation on a fresh record.
    pub fn run_c(&self, input: &Input) -> Buf {
        let mut b = Buf::new(input);
        unsafe { (self.c)(b.ptr()) };
        b
    }

    /// Run the Rust implementation on a fresh record.
    pub fn run_rust(&self, input: &Input) -> Buf {
        let mut b = Buf::new(input);
        unsafe { (self.rust)(b.ptr()) };
        b
    }

    /// Assert the two `.so`s produce byte-identical buffers (record + guards).
    /// Returns the resulting `frame_header`.
    #[track_caller]
    pub fn check(&self, input: &Input) -> u32 {
        let cb = self.run_c(input);
        let rb = self.run_rust(input);
        if cb.bytes() != rb.bytes() {
            panic!(
                "DIVERGENCE for {input:?}\n  C    frame_header = 0x{:08X}\n  Rust frame_header = \
                 0x{:08X}\n  C    bytes = {:02X?}\n  Rust bytes = {:02X?}",
                cb.frame_header(),
                rb.frame_header(),
                cb.bytes(),
                rb.bytes()
            );
        }
        // Guards must be untouched by both.
        let pristine = Buf::new(input);
        assert_eq!(
            &cb.bytes()[..GUARD],
            &pristine.bytes()[..GUARD],
            "C wrote before the record for {input:?}"
        );
        assert_eq!(
            &cb.bytes()[GUARD + TFLAC_SIZE..],
            &pristine.bytes()[GUARD + TFLAC_SIZE..],
            "C wrote past the record for {input:?}"
        );
        cb.frame_header()
    }

    /// `check` over an iterator of inputs, reporting how many were compared.
    #[track_caller]
    pub fn check_all<I: IntoIterator<Item = Input>>(&self, inputs: I) -> usize {
        let mut n = 0usize;
        for i in inputs {
            self.check(&i);
            n += 1;
        }
        assert!(n > 0, "empty input batch — the generator produced nothing");
        n
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D1CE_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn seeded() -> Self {
        Rng(SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        let span = (hi as u64 - lo as u64) + 1;
        lo + (self.next_u64() % span) as u32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Value pools taken from the branches in c_src/src/lib.c
// ---------------------------------------------------------------------------

/// The 13 exact `case` labels of the blocksize switch (`lib.c:13`).
pub const BLOCKSIZE_CASES: [u32; 13] = [
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];

/// The 11 exact `case` labels of the samplerate switch (`lib.c:58`).
pub const SAMPLERATE_CASES: [u32; 11] = [
    882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

/// The 6 exact `case` labels of the bitdepth switch (`lib.c:123`).
pub const BITDEPTH_CASES: [u32; 6] = [8, 12, 16, 20, 24, 32];

/// "Interesting" blocksizes: cases, their neighbours, boundaries, extremes.
pub fn blocksize_pool() -> Vec<u32> {
    let mut v = vec![0, 1, 2, 3, 255, 256, 257, 258, 65535, 65536, u32::MAX - 1, u32::MAX];
    for &c in BLOCKSIZE_CASES.iter() {
        v.push(c - 1);
        v.push(c);
        v.push(c + 1);
    }
    for p in 0..32u32 {
        v.push(1u32 << p);
    }
    v
}

/// "Interesting" samplerates: cases, their neighbours, every default sub-branch
/// boundary (255000/256000, 65535/65536, 655350/655360), extremes.
pub fn samplerate_pool() -> Vec<u32> {
    let mut v = vec![
        0,
        1,
        999,
        1000,
        1001,
        254999,
        255000,
        255001,
        255999,
        256000,
        256001,
        65534,
        65535,
        65536,
        65537,
        655340,
        655350,
        655351,
        655359,
        655360,
        655361,
        11025,
        88200,
        352800,
        384000,
        4294967000,
        4294967290,
        u32::MAX - 1,
        u32::MAX,
    ];
    for &c in SAMPLERATE_CASES.iter() {
        v.push(c - 1);
        v.push(c);
        v.push(c + 1);
    }
    v
}

/// "Interesting" channel counts: legal 1..8, the underflow, the nibble
/// overflow, and the `<< 4` truncation boundary.
pub fn channels_pool() -> Vec<u32> {
    let mut v = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 18, 255, 256, 257];
    v.extend_from_slice(&[
        0x0FFF_FFFE,
        0x0FFF_FFFF,
        0x1000_0000,
        0x1000_0001,
        0x2000_0000,
        0xFFFF_FFFE,
        u32::MAX,
    ]);
    for p in 0..32u32 {
        v.push(1u32 << p);
    }
    v
}

/// "Interesting" bitdepths: the 6 cases ± 1, 0, and extremes.
pub fn bitdepth_pool() -> Vec<u32> {
    let mut v = vec![0, 1, 2, 4, 6, 33, 64, 65, 255, 256, u32::MAX - 1, u32::MAX];
    for &c in BITDEPTH_CASES.iter() {
        v.push(c - 1);
        v.push(c);
        v.push(c + 1);
    }
    for p in 0..32u32 {
        v.push(1u32 << p);
    }
    v
}

/// All four value pools, built once (the generators allocate).
pub struct Pools {
    pub blocksize: Vec<u32>,
    pub samplerate: Vec<u32>,
    pub channels: Vec<u32>,
    pub bitdepth: Vec<u32>,
}

impl Pools {
    pub fn new() -> Self {
        Pools {
            blocksize: blocksize_pool(),
            samplerate: samplerate_pool(),
            channels: channels_pool(),
            bitdepth: bitdepth_pool(),
        }
    }
}

/// Randomize every field; callers then pin the axis under test. Each field is
/// drawn 50/50 from its "interesting value" pool or from the full domain, so
/// both value-dependent and out-of-range paths are hit.
pub fn random_input(rng: &mut Rng, p: &Pools) -> Input {
    Input {
        samplerate: if rng.bool() { rng.pick(&p.samplerate) } else { rng.next_u32() },
        channels: if rng.bool() { rng.pick(&p.channels) } else { rng.next_u32() },
        bitdepth: if rng.bool() { rng.pick(&p.bitdepth) } else { rng.next_u32() },
        channel_mode: rng.next_u8(),
        frame_header: if rng.bool() { rng.next_u32() } else { 0 },
        cur_blocksize: if rng.bool() { rng.pick(&p.blocksize) } else { rng.next_u32() },
        padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
    }
}

/// How many randomized inputs each `CONFIGS.md` row uses. Overridable with
/// `$DIFF_ITERS` to make local runs cheaper/heavier.
pub fn iters(default: usize) -> usize {
    std::env::var("DIFF_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
