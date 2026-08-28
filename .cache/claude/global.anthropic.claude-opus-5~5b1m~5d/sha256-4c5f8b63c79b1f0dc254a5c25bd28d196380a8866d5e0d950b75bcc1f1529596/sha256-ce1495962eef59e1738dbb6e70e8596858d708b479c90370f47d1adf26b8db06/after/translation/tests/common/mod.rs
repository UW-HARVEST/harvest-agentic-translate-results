//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called across the FFI boundary — the Rust side is never called directly,
//! so the `#[no_mangle]` / `extern "C"` export wrappers are under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/* ------------------------------------------------------------------ */
/* Loading                                                             */
/* ------------------------------------------------------------------ */

/// The three exported entry points, as raw C function pointers.
#[derive(Clone, Copy)]
pub struct Api {
    pub pack_u64le: unsafe extern "C" fn(*mut u8, u64),
    pub addsample: unsafe extern "C" fn(*mut u8, u32, u64),
    pub update_md5: unsafe extern "C" fn(*mut u8, *const i32) -> u32,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/lib*.so` (the name is derived from the parent
/// directory name by `c_src/CMakeLists.txt`, so it is discovered by globbing).
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C library first", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no lib*.so in {}", build.display());
    found.remove(0)
}

/// The Rust `cdylib`.
///
/// `cargo test` builds the *test harnesses* but not the `cdylib` artifact, so
/// the `.so` must have been produced by `cargo build [--release]` first (see
/// `run_all.sh`). The lookup prefers the profile the test binary itself was
/// built with, and refuses to use a `.so` that is older than `src/lib.rs`,
/// which would silently test stale code.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    const LIB: &str = "libupdate_md5_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    let exe_s = exe.to_string_lossy().to_string();

    let mut candidates: Vec<PathBuf> = Vec::new();
    // target/<profile>/deps/<test-bin>  ->  target/<profile>/<lib>
    let mut dir = exe.parent().map(|p| p.to_path_buf());
    while let Some(d) = dir {
        candidates.push(d.join(LIB));
        dir = d.parent().map(|p| p.to_path_buf());
        if candidates.len() >= 4 {
            break;
        }
    }
    let target = manifest_dir().join("target");
    if exe_s.contains("/release/") {
        candidates.push(target.join("release").join(LIB));
        candidates.push(target.join("debug").join(LIB));
    } else {
        candidates.push(target.join("debug").join(LIB));
        candidates.push(target.join("release").join(LIB));
    }

    let found = candidates.iter().find(|p| p.exists()).unwrap_or_else(|| {
        panic!(
            "{LIB} not found (looked in {:?}).\n\
             `cargo test` does not build cdylib artifacts — run `cargo build` \
             (and/or `cargo build --release`) first, or use ./run_all.sh",
            candidates
        )
    });

    // Freshness guard: never verify a stale artifact.
    let src = manifest_dir().join("src/lib.rs");
    if let (Ok(a), Ok(b)) = (
        std::fs::metadata(found).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        assert!(
            a >= b,
            "{} is older than {} — rebuild with `cargo build` (see run_all.sh)",
            found.display(),
            src.display()
        );
    }
    found.clone()
}

fn load(path: &Path) -> Api {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let pack_u64le = *lib
            .get::<unsafe extern "C" fn(*mut u8, u64)>(b"tflac_pack_u64le\0")
            .expect("tflac_pack_u64le");
        let addsample = *lib
            .get::<unsafe extern "C" fn(*mut u8, u32, u64)>(b"tflac_md5_addsample\0")
            .expect("tflac_md5_addsample");
        let update_md5 = *lib
            .get::<unsafe extern "C" fn(*mut u8, *const i32) -> u32>(b"update_md5\0")
            .expect("update_md5");
        // Keep the library mapped for the lifetime of the process; the raw
        // function pointers above outlive the `Library` handle otherwise.
        std::mem::forget(lib);
        Api {
            pack_u64le,
            addsample,
            update_md5,
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

/// Load both shared objects (once per process).
pub fn both() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: load(&c_lib_path()),
        rust: load(&rust_lib_path()),
    })
}

/* ------------------------------------------------------------------ */
/* Deterministic RNG (splitmix64)                                      */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A value biased towards interesting shapes (extremes, small, sparse).
    pub fn interesting_u64(&mut self) -> u64 {
        let v = self.next_u64();
        match self.below(8) {
            0 => 0,
            1 => u64::MAX,
            2 => 1u64 << (self.below(64)),
            3 => !(1u64 << (self.below(64))),
            4 => v & 0xFF,
            5 => v | 0xFF00_0000_0000_0000,
            _ => v,
        }
    }
    /// A sample value biased towards low-byte extremes and sign changes.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(10) {
            0 => 0,
            1 => -1,
            2 => i32::MIN,
            3 => i32::MAX,
            4 => (self.next_u32() & 0xFFFF_FF00) as i32, // low byte 0x00
            5 => (self.next_u32() | 0x0000_00FF) as i32, // low byte 0xFF
            6 => -(self.below(256) as i32),
            _ => self.next_i32(),
        }
    }
}

/* ------------------------------------------------------------------ */
/* Arenas                                                              */
/* ------------------------------------------------------------------ */

/// Field offsets, verified against the C compiler (`offsetof`).
pub const OFF_POS: usize = 0;
pub const OFF_TOTAL: usize = 8;
pub const OFF_BUFFER: usize = 16;
pub const BUFFER_LEN: usize = 64 + 8;
pub const SIZEOF_MD5: usize = 88;
pub const OFF_CUR_BLOCKSIZE: usize = 88;
pub const OFF_CHANNELS: usize = 92;
pub const SIZEOF_TFLAC: usize = 96;

/// Size of the 8-aligned arena a struct is placed in. The C deliberately reads
/// `buffer[64 + i]` for `i` up to 62 (out of the 72-byte buffer) when `pos` is
/// not sanitised; a generous arena makes those reads land in defined memory
/// that is identical for both libraries, so the comparison stays deterministic.
pub const ARENA_BYTES: usize = 512;

/// 8-byte-aligned scratch memory.
pub struct Arena {
    words: Vec<u64>,
}

impl Arena {
    /// Fresh arena, filled with a deterministic, position-dependent pattern so
    /// that *any* stray write or mis-indexed read is visible in the diff.
    pub fn new(seed: u64) -> Arena {
        let mut a = Arena {
            words: vec![0u64; ARENA_BYTES / 8],
        };
        let mut rng = Rng::new(seed ^ 0xA5A5_1234_DEAD_BEEF);
        for w in a.words.iter_mut() {
            *w = rng.next_u64();
        }
        a
    }
    pub fn zeroed() -> Arena {
        Arena {
            words: vec![0u64; ARENA_BYTES / 8],
        }
    }
    pub fn as_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.words.len() * 8)
        }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, self.words.len() * 8)
        }
    }
    pub fn clone_arena(&self) -> Arena {
        Arena {
            words: self.words.clone(),
        }
    }

    /* --- typed field accessors (struct starts at offset 0) --- */
    pub fn set_pos(&mut self, v: u32) {
        self.bytes_mut()[OFF_POS..OFF_POS + 4].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn pos(&self) -> u32 {
        u32::from_ne_bytes(self.bytes()[OFF_POS..OFF_POS + 4].try_into().unwrap())
    }
    pub fn set_total(&mut self, v: u64) {
        self.bytes_mut()[OFF_TOTAL..OFF_TOTAL + 8].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn total(&self) -> u64 {
        u64::from_ne_bytes(self.bytes()[OFF_TOTAL..OFF_TOTAL + 8].try_into().unwrap())
    }
    pub fn set_buffer(&mut self, data: &[u8; BUFFER_LEN]) {
        self.bytes_mut()[OFF_BUFFER..OFF_BUFFER + BUFFER_LEN].copy_from_slice(data);
    }
    pub fn buffer(&self) -> &[u8] {
        &self.bytes()[OFF_BUFFER..OFF_BUFFER + BUFFER_LEN]
    }
    /* --- raw, offset-based accessors (for a struct placed at `off`) --- */
    pub fn set_u32_at(&mut self, off: usize, v: u32) {
        self.bytes_mut()[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn set_u64_at(&mut self, off: usize, v: u64) {
        self.bytes_mut()[off..off + 8].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn set_bytes_at(&mut self, off: usize, data: &[u8]) {
        self.bytes_mut()[off..off + data.len()].copy_from_slice(data);
    }

    pub fn set_cur_blocksize(&mut self, v: u32) {
        self.bytes_mut()[OFF_CUR_BLOCKSIZE..OFF_CUR_BLOCKSIZE + 4].copy_from_slice(&v.to_ne_bytes());
    }
    pub fn set_channels(&mut self, v: u32) {
        self.bytes_mut()[OFF_CHANNELS..OFF_CHANNELS + 4].copy_from_slice(&v.to_ne_bytes());
    }
}

/* ------------------------------------------------------------------ */
/* Differential drivers                                                */
/* ------------------------------------------------------------------ */

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Report the first differing byte of two arenas, with context.
fn diff_report(ctx: &str, c: &Arena, r: &Arena) -> String {
    let cb = c.bytes();
    let rb = r.bytes();
    let idx = (0..cb.len()).find(|&i| cb[i] != rb[i]).unwrap_or(0);
    let lo = idx.saturating_sub(8);
    let hi = (idx + 24).min(cb.len());
    format!(
        "{ctx}\n  first diff at byte {idx} (field: {})\n  C   [{lo}..{hi}] = {}\n  RS  [{lo}..{hi}] = {}\n  C: pos={} total={}\n  R: pos={} total={}",
        field_name(idx),
        hex(&cb[lo..hi]),
        hex(&rb[lo..hi]),
        c.pos(),
        c.total(),
        r.pos(),
        r.total()
    )
}

fn field_name(off: usize) -> &'static str {
    match off {
        0..=3 => "md5.pos",
        4..=7 => "md5.<padding>",
        8..=15 => "md5.total",
        16..=79 => "md5.buffer[0..64]",
        80..=87 => "md5.buffer[64..72]",
        88..=91 => "cur_blocksize",
        92..=95 => "channels",
        _ => "arena guard (outside struct)",
    }
}

/// Run `tflac_pack_u64le` on both libraries over identical arenas and compare
/// the complete memory image.
pub fn diff_pack(ctx: &str, arena: &Arena, offset: usize, n: u64) {
    let api = both();
    let mut ca = arena.clone_arena();
    let mut ra = arena.clone_arena();
    unsafe {
        (api.c.pack_u64le)(ca.as_ptr().add(offset), n);
        (api.rust.pack_u64le)(ra.as_ptr().add(offset), n);
    }
    assert_eq!(
        ca.bytes(),
        ra.bytes(),
        "{}",
        diff_report(
            &format!("pack_u64le mismatch [{ctx}] offset={offset} n=0x{n:016x}"),
            &ca,
            &ra
        )
    );
}

/// Run `tflac_md5_addsample` on both libraries over identical arenas.
pub fn diff_addsample(ctx: &str, arena: &Arena, bits: u32, val: u64) {
    let api = both();
    let mut ca = arena.clone_arena();
    let mut ra = arena.clone_arena();
    unsafe {
        (api.c.addsample)(ca.as_ptr(), bits, val);
        (api.rust.addsample)(ra.as_ptr(), bits, val);
    }
    assert_eq!(
        ca.bytes(),
        ra.bytes(),
        "{}",
        diff_report(
            &format!(
                "addsample mismatch [{ctx}] entry pos={} total={} bits={bits} val=0x{val:016x}",
                arena.pos(),
                arena.total()
            ),
            &ca,
            &ra
        )
    );
}

/// Run `update_md5` on both libraries over identical arenas + identical
/// samples; compare the return value *and* the memory image.
pub fn diff_update(ctx: &str, arena: &Arena, samples: &[i32]) {
    assert!(
        samples.len() >= 136,
        "update_md5 reads samples[0..136]; give it at least 136 elements"
    );
    let api = both();
    let mut ca = arena.clone_arena();
    let mut ra = arena.clone_arena();
    let (cret, rret) = unsafe {
        (
            (api.c.update_md5)(ca.as_ptr(), samples.as_ptr()),
            (api.rust.update_md5)(ra.as_ptr(), samples.as_ptr()),
        )
    };
    assert_eq!(
        cret, rret,
        "update_md5 return mismatch [{ctx}]: C={cret} (0x{cret:08x}) RS={rret} (0x{rret:08x})"
    );
    assert_eq!(
        ca.bytes(),
        ra.bytes(),
        "{}",
        diff_report(
            &format!(
                "update_md5 memory mismatch [{ctx}] entry pos={} total={}",
                arena.pos(),
                arena.total()
            ),
            &ca,
            &ra
        )
    );
}

/// `tflac_md5_addsample` with the struct placed at an arbitrary byte offset
/// inside the arena (used to exercise *misaligned* struct pointers, which the
/// C accepts on this target).
pub fn diff_addsample_off(ctx: &str, arena: &Arena, struct_off: usize, bits: u32, val: u64) {
    let api = both();
    let mut ca = arena.clone_arena();
    let mut ra = arena.clone_arena();
    unsafe {
        (api.c.addsample)(ca.as_ptr().add(struct_off), bits, val);
        (api.rust.addsample)(ra.as_ptr().add(struct_off), bits, val);
    }
    assert_eq!(
        ca.bytes(),
        ra.bytes(),
        "{}",
        diff_report(
            &format!("addsample(off={struct_off}) mismatch [{ctx}] bits={bits} val=0x{val:016x}"),
            &ca,
            &ra
        )
    );
}

/// `update_md5` with the struct at an arbitrary byte offset and the sample
/// array optionally misaligned by `sample_byte_off` bytes.
pub fn diff_update_off(
    ctx: &str,
    arena: &Arena,
    struct_off: usize,
    sample_bytes: &[u8],
    sample_byte_off: usize,
) {
    assert!(
        sample_bytes.len() >= sample_byte_off + 136 * 4,
        "need at least 136 samples past the offset"
    );
    let api = both();
    let mut ca = arena.clone_arena();
    let mut ra = arena.clone_arena();
    let (cret, rret) = unsafe {
        let sp = sample_bytes.as_ptr().add(sample_byte_off) as *const i32;
        (
            (api.c.update_md5)(ca.as_ptr().add(struct_off), sp),
            (api.rust.update_md5)(ra.as_ptr().add(struct_off), sp),
        )
    };
    assert_eq!(
        cret, rret,
        "update_md5(off={struct_off},soff={sample_byte_off}) return mismatch [{ctx}]: \
         C=0x{cret:08x} RS=0x{rret:08x}"
    );
    assert_eq!(
        ca.bytes(),
        ra.bytes(),
        "{}",
        diff_report(
            &format!("update_md5(off={struct_off},soff={sample_byte_off}) memory mismatch [{ctx}]"),
            &ca,
            &ra
        )
    );
}

/// Build a `tflac` arena with the given state.
pub fn tflac_arena(
    seed: u64,
    pos: u32,
    total: u64,
    cur_blocksize: u32,
    channels: u32,
    buffer: Option<&[u8; BUFFER_LEN]>,
) -> Arena {
    let mut a = Arena::new(seed);
    a.set_pos(pos);
    a.set_total(total);
    a.set_cur_blocksize(cur_blocksize);
    a.set_channels(channels);
    if let Some(b) = buffer {
        a.set_buffer(b);
    }
    a
}

/// A deterministic 136+-element sample block.
pub fn random_samples(rng: &mut Rng, len: usize) -> Vec<i32> {
    (0..len).map(|_| rng.interesting_i32()).collect()
}
