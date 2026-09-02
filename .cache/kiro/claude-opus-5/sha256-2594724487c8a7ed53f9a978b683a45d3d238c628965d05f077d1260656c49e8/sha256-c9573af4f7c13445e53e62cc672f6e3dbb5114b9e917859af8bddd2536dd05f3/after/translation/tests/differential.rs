//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Neither library is ever called directly as a Rust function — both are
//! `dlopen`ed and driven through their exported `hdr_bitrate` symbol, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Row IDs (`C01..C30`) refer to `CONFIGS.md`; (`E1..E7`) refer to `ERRORS.md`.

use std::ffi::{c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::os::unix::Library as UnixLibrary;
use libloading::Library;

// ---------------------------------------------------------------------------
// Library discovery + loading
// ---------------------------------------------------------------------------

type HdrBitrate = unsafe extern "C" fn(*const u8) -> c_uint;

struct Both {
    c: HdrBitrate,
    rust: HdrBitrate,
    c_path: PathBuf,
    rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The single `lib*.so` produced by `c_src/CMakeLists.txt`. Its name is derived
/// from the parent directory name by the CMake script, so it is discovered by
/// scanning rather than hard-coded.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let entries = std::fs::read_dir(&build_dir).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    });
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no lib*.so found in {}", build_dir.display()),
        _ => found.remove(0),
    }
}

/// The Rust `cdylib`. Prefers `$HDR_RUST_SO`, then the target dir of the running
/// test binary (`target/<profile>/`), then `release`, then `debug`.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_RUST_SO") {
        return PathBuf::from(p);
    }
    let name = "libhdr_bitrate_lib.so";
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/deps/<test-bin>  ->  target/<profile>/
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join(name));
        }
    }
    candidates.push(manifest_dir().join("target/release").join(name));
    candidates.push(manifest_dir().join("target/debug").join(name));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib {name} not found. Tried: {candidates:?}\nRun `cargo build --release` first."
    );
}

/// Newest mtime among the crate's Rust sources.
fn newest_source_mtime() -> std::time::SystemTime {
    fn walk(dir: &Path, newest: &mut std::time::SystemTime) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                walk(&p, newest);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if m > *newest {
                        *newest = m;
                    }
                }
            }
        }
    }
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    walk(&manifest_dir().join("src"), &mut newest);
    for f in ["Cargo.toml"] {
        if let Ok(m) = std::fs::metadata(manifest_dir().join(f)).and_then(|m| m.modified()) {
            if m > newest {
                newest = m;
            }
        }
    }
    newest
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, because
/// no test target links against it. Without this guard the whole suite would
/// happily diff a stale `.so` and report success for code that was never
/// compiled. Always run `cargo build` before `cargo test` (see `run_all.sh`).
fn assert_rust_so_is_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    let src_mtime = newest_source_mtime();
    assert!(
        so_mtime >= src_mtime,
        "STALE ARTIFACT: {} is older than the crate sources.\n\
         `cargo test` does not rebuild a cdylib -- run `cargo build --release` (or ./run_all.sh) first,\n\
         otherwise these differential tests validate a .so that does not match src/.",
        so.display()
    );
}

fn both() -> &'static Both {
    static BOTH: OnceLock<Both> = OnceLock::new();
    BOTH.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        assert_rust_so_is_fresh(&rust_path);
        // Leaked so the resolved function pointers stay valid for the whole
        // process lifetime.
        let c_lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()))
        }));
        let rust_lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()))
        }));
        let c: HdrBitrate = *unsafe {
            c_lib
                .get::<HdrBitrate>(b"hdr_bitrate\0")
                .expect("C .so exports hdr_bitrate")
        };
        let rust: HdrBitrate = *unsafe {
            rust_lib
                .get::<HdrBitrate>(b"hdr_bitrate\0")
                .expect("Rust .so exports hdr_bitrate")
        };
        Both {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

/// Call both exports on the same buffer and assert byte-identical results.
#[track_caller]
fn diff(row: &str, buf: &[u8]) -> c_uint {
    let b = both();
    assert!(buf.len() >= 3, "{row}: header needs >= 3 bytes");
    let cv = unsafe { (b.c)(buf.as_ptr()) };
    let rv = unsafe { (b.rust)(buf.as_ptr()) };
    assert_eq!(
        cv, rv,
        "{row}: divergence for header {:02x?} (len {}) -- C={cv} Rust={rv}\n  C:    {}\n  Rust: {}",
        &buf[..buf.len().min(8)],
        buf.len(),
        b.c_path.display(),
        b.rust_path.display()
    );
    cv
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64, fixed seed for reproducibility)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

const SEED: u64 = 0x5DEE_CE66_D;

// ---------------------------------------------------------------------------
// Header construction
// ---------------------------------------------------------------------------

/// Build a header with the three fields the C reads set explicitly and every
/// bit the C ignores filled with random noise (axis D of `CONFIGS.md`).
///
/// * `i`     — version bit, `h[1] & 0x8`
/// * `layer` — raw 2-bit layer field, `(h[1] >> 1) & 3`
/// * `k`     — bitrate nibble, `h[2] >> 4`
fn header(rng: &mut Rng, i: u8, layer: u8, k: u8, len: usize) -> Vec<u8> {
    assert!(i < 2 && layer < 4 && k < 16);
    let mut buf = vec![0u8; len];
    for b in buf.iter_mut() {
        *b = rng.u8();
    }
    buf[1] = (rng.u8() & 0xF0) | (i << 3) | (layer << 1) | (rng.u8() & 0x01);
    buf[2] = (k << 4) | (rng.u8() & 0x0F);
    buf
}

/// Exercise one `CONFIGS.md` row: every `k` in `ks`, `iters` randomized inputs
/// each (random ignored bits, random trailing bytes, random buffer length).
#[track_caller]
fn row(name: &str, i: u8, layer: u8, ks: &[u8], iters: usize) {
    let mut rng = Rng::new(SEED ^ (u64::from(i) << 40) ^ (u64::from(layer) << 32));
    for &k in ks {
        for _ in 0..iters {
            let len = 3 + rng.below(30) as usize;
            let buf = header(&mut rng, i, layer, k, len);
            diff(name, &buf);
        }
    }
}

const ITERS: usize = 128;
const ALL_MID: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows
// ---------------------------------------------------------------------------

#[test]
fn c01_i0_layer0_k0() {
    row("C01", 0, 0, &[0], ITERS);
}

#[test]
fn c02_i0_layer0_k1_14() {
    row("C02", 0, 0, ALL_MID, ITERS);
}

#[test]
fn c03_i0_layer0_k15() {
    row("C03", 0, 0, &[15], ITERS);
}

#[test]
fn c04_i0_layer1_k0() {
    row("C04", 0, 1, &[0], ITERS);
}

#[test]
fn c05_i0_layer1_k1_14() {
    row("C05", 0, 1, ALL_MID, ITERS);
}

#[test]
fn c06_i0_layer1_k15() {
    row("C06", 0, 1, &[15], ITERS);
}

#[test]
fn c07_i0_layer2_k0() {
    row("C07", 0, 2, &[0], ITERS);
}

#[test]
fn c08_i0_layer2_k1_14() {
    row("C08", 0, 2, ALL_MID, ITERS);
}

#[test]
fn c09_i0_layer2_k15() {
    row("C09", 0, 2, &[15], ITERS);
}

#[test]
fn c10_i0_layer3_k0() {
    row("C10", 0, 3, &[0], ITERS);
}

#[test]
fn c11_i0_layer3_k1_14() {
    row("C11", 0, 3, ALL_MID, ITERS);
}

#[test]
fn c12_i0_layer3_k15() {
    row("C12", 0, 3, &[15], ITERS);
}

#[test]
fn c13_i1_layer0_k0() {
    row("C13", 1, 0, &[0], ITERS);
}

#[test]
fn c14_i1_layer0_k1_14() {
    row("C14", 1, 0, ALL_MID, ITERS);
}

#[test]
fn c15_i1_layer0_k15() {
    row("C15", 1, 0, &[15], ITERS);
}

#[test]
fn c16_i1_layer1_k0() {
    row("C16", 1, 1, &[0], ITERS);
}

#[test]
fn c17_i1_layer1_k1_14() {
    row("C17", 1, 1, ALL_MID, ITERS);
}

#[test]
fn c18_i1_layer1_k15() {
    row("C18", 1, 1, &[15], ITERS);
}

#[test]
fn c19_i1_layer2_k0() {
    row("C19", 1, 2, &[0], ITERS);
}

#[test]
fn c20_i1_layer2_k1_14() {
    row("C20", 1, 2, ALL_MID, ITERS);
}

#[test]
fn c21_i1_layer2_k15() {
    row("C21", 1, 2, &[15], ITERS);
}

#[test]
fn c22_i1_layer3_k0() {
    row("C22", 1, 3, &[0], ITERS);
}

#[test]
fn c23_i1_layer3_k1_14() {
    row("C23", 1, 3, ALL_MID, ITERS);
}

#[test]
fn c24_i1_layer3_k15_past_table_end() {
    row("C24", 1, 3, &[15], ITERS);
}

/// C25 — the bits the C never reads must not change the result, in C or Rust.
#[test]
fn c25_ignored_bits_are_invariant() {
    let mut rng = Rng::new(SEED ^ 0x25);
    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                // Reference value with all ignored bits zero.
                let mut base = vec![0u8; 4];
                base[1] = (i << 3) | (layer << 1);
                base[2] = k << 4;
                let expected = diff("C25/base", &base);

                for _ in 0..64 {
                    let mut buf = vec![0u8; 4];
                    buf[0] = rng.u8();
                    buf[3] = rng.u8();
                    buf[1] = (rng.u8() & 0xF0) | (i << 3) | (layer << 1) | (rng.u8() & 0x01);
                    buf[2] = (k << 4) | (rng.u8() & 0x0F);
                    let got = diff("C25", &buf);
                    assert_eq!(
                        expected, got,
                        "C25: ignored bits changed the result for i={i} layer={layer} k={k}, \
                         header {:02x?}",
                        buf
                    );
                }
            }
        }
    }
}

/// C26 — trailing bytes and buffer length are irrelevant.
#[test]
fn c26_trailing_bytes_and_length_invariant() {
    let mut rng = Rng::new(SEED ^ 0x26);
    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                let mut base = vec![0u8; 3];
                base[1] = (i << 3) | (layer << 1);
                base[2] = k << 4;
                let expected = diff("C26/base", &base);

                for len in 3..=64usize {
                    let mut buf = vec![0u8; len];
                    for b in buf.iter_mut() {
                        *b = rng.u8();
                    }
                    buf[1] = (i << 3) | (layer << 1);
                    buf[2] = k << 4;
                    let got = diff("C26", &buf);
                    assert_eq!(expected, got, "C26: length {len} changed the result");
                }
            }
        }
    }
}

/// C27 — pointer offset / alignment inside a larger buffer is irrelevant.
#[test]
fn c27_pointer_offset_and_alignment() {
    let b = both();
    let mut rng = Rng::new(SEED ^ 0x27);
    let mut backing = vec![0u8; 64];
    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                let h1 = (rng.u8() & 0xF0) | (i << 3) | (layer << 1) | (rng.u8() & 0x01);
                let h2 = (k << 4) | (rng.u8() & 0x0F);
                let mut expected: Option<c_uint> = None;
                for off in 0..16usize {
                    for x in backing.iter_mut() {
                        *x = rng.u8();
                    }
                    backing[off + 1] = h1;
                    backing[off + 2] = h2;
                    let p = unsafe { backing.as_ptr().add(off) };
                    let cv = unsafe { (b.c)(p) };
                    let rv = unsafe { (b.rust)(p) };
                    assert_eq!(
                        cv, rv,
                        "C27: divergence at offset {off} for h1={h1:#04x} h2={h2:#04x}"
                    );
                    match expected {
                        None => expected = Some(cv),
                        Some(e) => assert_eq!(e, cv, "C27: offset {off} changed the result"),
                    }
                }
            }
        }
    }
}

/// C29 — statelessness: interleaving inputs never changes a result.
#[test]
fn c29_stateless_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0x29);
    let mut table = Vec::new();
    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                let mut buf = vec![0u8; 4];
                buf[1] = (i << 3) | (layer << 1);
                buf[2] = k << 4;
                let v = diff("C29/seed", &buf);
                table.push((buf, v));
            }
        }
    }
    for _ in 0..4000 {
        let idx = rng.below(table.len() as u64) as usize;
        let (buf, expected) = &table[idx];
        let got = diff("C29", buf);
        assert_eq!(*expected, got, "C29: result changed on repeat call");
    }
}

/// C30 / E7 — exhaustive over the entire input space the C branches on:
/// all 256 x 256 `(h[1], h[2])` pairs, with randomized ignored bytes.
#[test]
fn c30_e7_exhaustive_all_header_bytes() {
    let mut rng = Rng::new(SEED ^ 0x30);
    let mut buf = vec![0u8; 8];
    let mut mismatches = 0usize;
    let b = both();
    for h1 in 0..=255u8 {
        for h2 in 0..=255u8 {
            for x in buf.iter_mut() {
                *x = rng.u8();
            }
            buf[1] = h1;
            buf[2] = h2;
            let cv = unsafe { (b.c)(buf.as_ptr()) };
            let rv = unsafe { (b.rust)(buf.as_ptr()) };
            if cv != rv {
                if mismatches < 20 {
                    eprintln!("C30: h1={h1:#04x} h2={h2:#04x} C={cv} Rust={rv}");
                }
                mismatches += 1;
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "C30/E7: {mismatches} of 65536 (h[1],h[2]) combinations diverged"
    );
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows
// ---------------------------------------------------------------------------

/// E2 — reserved layer field `0b00` with version bit clear: middle index is
/// `-1`, reads the 15 bytes *before* the table. Exhaustive over `k`.
#[test]
fn e2_reserved_layer_negative_index_low_version() {
    row("E2", 0, 0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15], ITERS);
}

/// E3 — reserved layer field `0b00` with version bit set: flat offsets 30..45,
/// which alias `halfrate[0][2][*]` instead of erroring.
#[test]
fn e3_reserved_layer_negative_index_high_version() {
    row("E3", 1, 0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15], ITERS);
}

/// E4 — "bad" bitrate nibble 15 (one past the declared 15-byte row) for every
/// `(i, layer)` combination.
#[test]
fn e4_bad_bitrate_nibble_15() {
    for i in 0..2u8 {
        for layer in 0..4u8 {
            row("E4", i, layer, &[15], ITERS);
        }
    }
}

/// E5 — maximal index combination `i=1, layer=0b11, k=15`: flat offset 90,
/// one byte past the end of the entire table.
#[test]
fn e5_max_index_past_end_of_table() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..ITERS {
        let len = 3 + rng.below(20) as usize;
        let buf = header(&mut rng, 1, 3, 15, len);
        assert_eq!(buf[1] & 0x08, 0x08);
        assert_eq!((buf[1] >> 1) & 3, 3);
        assert_eq!(buf[2] >> 4, 15);
        diff("E5", &buf);
    }
}

/// E7 (enum-domain part) — the version / layer / bitrate fields are unvalidated
/// bit-fields, so every out-of-range "variant" (reserved layer `0b00`, `free`
/// bitrate `0b0000`, `bad` bitrate `0b1111`) is a real input. Also passes the
/// full `int` domain of each field explicitly, one step past every documented
/// range.
#[test]
fn e7_out_of_range_enum_variants() {
    // Every layer field value including the reserved 0b00, and every bitrate
    // nibble including `free` (0) and `bad` (15), for both version bits.
    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                let mut buf = vec![0u8; 3];
                buf[1] = (i << 3) | (layer << 1);
                buf[2] = k << 4;
                diff("E7", &buf);
                // and with all ignored bits set, to catch sign/width slips
                let mut buf2 = vec![0xFFu8; 3];
                buf2[1] = 0xF0 | (i << 3) | (layer << 1) | 0x01;
                buf2[2] = (k << 4) | 0x0F;
                diff("E7/sat", &buf2);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E6 / C28 — over-read detection using an unmapped guard page.
// libc is reached through the already-loaded process image, so no extra
// dependency is needed.
// ---------------------------------------------------------------------------

type MmapFn = unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, i64) -> *mut c_void;
type MprotectFn = unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int;
type MunmapFn = unsafe extern "C" fn(*mut c_void, usize) -> c_int;
type GetPageSizeFn = unsafe extern "C" fn() -> c_int;

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;

/// E6 + C28 — place the 3-byte header so that `h[2]` is the last readable byte
/// before an unmapped page. Any read of `h[3]` (or beyond) by either library
/// faults and kills this test process, which is a loud failure. Conversely a
/// successful run proves both read only `h[1]` and `h[2]`.
#[test]
fn e6_c28_reads_exactly_three_bytes() {
    let this = UnixLibrary::this();
    let mmap: MmapFn = *unsafe { this.get::<MmapFn>(b"mmap\0").expect("libc mmap") };
    let mprotect: MprotectFn =
        *unsafe { this.get::<MprotectFn>(b"mprotect\0").expect("libc mprotect") };
    let munmap: MunmapFn = *unsafe { this.get::<MunmapFn>(b"munmap\0").expect("libc munmap") };
    let getpagesize: GetPageSizeFn =
        *unsafe { this.get::<GetPageSizeFn>(b"getpagesize\0").expect("libc getpagesize") };

    let page = unsafe { getpagesize() } as usize;
    assert!(page >= 8 && page.is_power_of_two(), "odd page size {page}");
    let total = page * 2;

    let base = unsafe {
        mmap(
            std::ptr::null_mut(),
            total,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(
        base as isize != -1 && !base.is_null(),
        "mmap of {total} bytes failed"
    );

    // Make the second page unreadable: a guard page immediately after h[2].
    let guard = unsafe { (base as *mut u8).add(page) as *mut c_void };
    assert_eq!(
        unsafe { mprotect(guard, page, PROT_NONE) },
        0,
        "mprotect guard page failed"
    );

    let b = both();
    let mut rng = Rng::new(SEED ^ 0x06);
    // Header occupies the final 3 bytes of the readable page.
    let hdr = unsafe { (base as *mut u8).add(page - 3) };

    for i in 0..2u8 {
        for layer in 0..4u8 {
            for k in 0..16u8 {
                unsafe {
                    *hdr = rng.u8();
                    *hdr.add(1) = (rng.u8() & 0xF0) | (i << 3) | (layer << 1) | (rng.u8() & 0x01);
                    *hdr.add(2) = (k << 4) | (rng.u8() & 0x0F);
                }
                let cv = unsafe { (b.c)(hdr) };
                let rv = unsafe { (b.rust)(hdr) };
                assert_eq!(
                    cv, rv,
                    "E6/C28: divergence at guard-page boundary for i={i} layer={layer} k={k}"
                );
            }
        }
    }

    unsafe {
        munmap(base, total);
    }
}

// ---------------------------------------------------------------------------
// E1 — null pointer: both must fault the same way. Done in child processes,
// since the fault is fatal.
// ---------------------------------------------------------------------------

const CHILD_ENV: &str = "HDR_NULL_CALL";
const CHILD_TEST: &str = "zzz_null_pointer_child";

/// Child half of `e1_null_pointer_same_fault`. A no-op unless `$HDR_NULL_CALL`
/// selects a library, so it is inert during a normal test run.
#[test]
fn zzz_null_pointer_child() {
    let which = match std::env::var(CHILD_ENV) {
        Ok(v) => v,
        Err(_) => return,
    };
    let b = both();
    let f = match which.as_str() {
        "c" => b.c,
        "rust" => b.rust,
        other => panic!("bad {CHILD_ENV}={other}"),
    };
    eprintln!("child: calling {which} hdr_bitrate(NULL)");
    let v = unsafe { f(std::ptr::null()) };
    // Not expected to be reached; report it so a silent success is visible.
    eprintln!("child: {which} returned {v} without faulting");
    std::process::exit(77);
}

fn run_null_child(which: &str) -> std::process::Output {
    let b = both();
    std::process::Command::new(std::env::current_exe().expect("current_exe"))
        .args([CHILD_TEST, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, which)
        .env("HDR_C_SO", &b.c_path)
        .env("HDR_RUST_SO", &b.rust_path)
        .output()
        .expect("spawn child")
}

#[test]
fn e1_null_pointer_same_fault() {
    use std::os::unix::process::ExitStatusExt;

    let c_out = run_null_child("c");
    let r_out = run_null_child("rust");

    let describe = |o: &std::process::Output| {
        format!(
            "signal={:?} code={:?}",
            o.status.signal(),
            o.status.code()
        )
    };

    assert_eq!(
        c_out.status.signal(),
        r_out.status.signal(),
        "E1: hdr_bitrate(NULL) terminated differently.\n  C:    {}\n  Rust: {}\n  C stderr: {}\n  Rust stderr: {}",
        describe(&c_out),
        describe(&r_out),
        String::from_utf8_lossy(&c_out.stderr),
        String::from_utf8_lossy(&r_out.stderr),
    );
    assert_eq!(
        c_out.status.code(),
        r_out.status.code(),
        "E1: exit codes differ.\n  C: {}\n  Rust: {}",
        describe(&c_out),
        describe(&r_out),
    );
    // The C dereferences h[1] unconditionally, so a fault is expected; assert it
    // really happened rather than accepting two silent successes.
    assert_eq!(
        c_out.status.signal(),
        Some(11),
        "E1: expected SIGSEGV from the C library, got {}",
        describe(&c_out)
    );
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity, asserted from inside the test suite.
// ---------------------------------------------------------------------------

fn dynamic_defined_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn d_symbol_parity_c_so_vs_rust_so() {
    let b = both();
    let c_syms = dynamic_defined_symbols(&b.c_path);
    let rust_syms = dynamic_defined_symbols(&b.rust_path);
    assert!(
        c_syms.contains(&"hdr_bitrate".to_string()),
        "C .so must export hdr_bitrate; got {c_syms:?}"
    );
    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n  C:    {:?}\n  Rust: {:?}",
        missing.len(),
        c_syms,
        rust_syms
    );
}

#[test]
fn d_rust_so_has_no_unresolved_non_libc_symbols() {
    let b = both();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&b.rust_path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm --undefined-only failed");

    // Mechanical classification instead of a hand-written allowlist: an
    // undefined symbol is "libc / runtime provided" iff the dynamic loader can
    // actually resolve it in this process's global scope. Anything left over is
    // a genuinely unresolved reference from the translation.
    let this = UnixLibrary::this();
    let mut unresolved = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(raw) = line.split_whitespace().next() else {
            continue;
        };
        // Strip the ELF symbol-version suffix, e.g. `read@GLIBC_2.2.5`.
        let name = raw.split('@').next().unwrap_or(raw);
        // Weak runtime hooks are legitimately absent (e.g. __gmon_start__).
        let is_weak = line.split_whitespace().nth(1) == Some("w")
            || line.split_whitespace().nth(1) == Some("v");
        if is_weak {
            continue;
        }
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        if unsafe { this.get::<*const c_void>(&sym) }.is_err() {
            unresolved.push(raw.to_string());
        }
    }
    assert!(
        unresolved.is_empty(),
        "Rust .so has {} undefined symbol(s) that the loader cannot resolve: {unresolved:?}",
        unresolved.len()
    );
}
