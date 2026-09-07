//! Differential test harness: loads BOTH the C `libdriver.so` and the Rust
//! `libdriver.so` through `libloading` and compares them across the FFI
//! boundary. The Rust implementation is NEVER called directly — always through
//! its exported `#[no_mangle]` symbol, so the export wrappers are under test
//! too.
//!
//! Covers `CONFIGS.md` (valid paths) and `ERRORS.md` (rejection paths).

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

/// Directory holding the freshly-built Rust cdylib (`target/<profile>/`).
fn rust_artifact_dir() -> PathBuf {
    // current_exe() == target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("artifact dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    rust_artifact_dir().join("libdriver.so")
}

fn load(path: &PathBuf) -> Library {
    assert!(
        path.exists(),
        "shared library not found: {}\n\
         Build the C side with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with:\n  cd translation && cargo build --release",
        path.display()
    );
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
}

/// `cargo test` does **not** rebuild `crate-type = ["cdylib"]` artifacts — it
/// builds the lib as a test harness instead. Without this guard the whole suite
/// would silently validate a stale `.so` and pass even after the Rust source is
/// broken. Refuse to run unless both `.so`s are at least as new as their
/// sources.
fn assert_artifacts_fresh() {
    fn mtime(p: &PathBuf) -> std::time::SystemTime {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
            .modified()
            .expect("mtime")
    }

    let checks: [(PathBuf, Vec<PathBuf>, &str); 2] = [
        (
            rust_so_path(),
            vec![manifest_dir().join("src/lib.rs"), manifest_dir().join("Cargo.toml")],
            "cd translation && cargo build --release",
        ),
        (
            c_so_path(),
            vec![
                manifest_dir().parent().unwrap().join("c_src/src/lib.c"),
                manifest_dir().parent().unwrap().join("c_src/include/lib.h"),
            ],
            "cd c_src/build && cmake --build .",
        ),
    ];

    for (artifact, sources, how) in checks {
        let a = mtime(&artifact);
        for src in sources {
            let s = mtime(&src);
            assert!(
                a >= s,
                "STALE ARTIFACT: {} is older than {}.\n\
                 The test suite would validate an out-of-date library and pass \
                 vacuously.\nRebuild first:\n  {how}",
                artifact.display(),
                src.display()
            );
        }
    }
}

struct Libs {
    c: Library,
    rs: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        assert_artifacts_fresh();
        Libs {
            c: load(&c_so_path()),
            rs: load(&rust_so_path()),
        }
    })
}

fn c_tool_basename() -> Symbol<'static, ToolBasename> {
    unsafe { libs().c.get(b"tool_basename\0") }.expect("C .so must export tool_basename")
}

fn rs_tool_basename() -> Symbol<'static, ToolBasename> {
    unsafe { libs().rs.get(b"tool_basename\0") }.expect("Rust .so must export tool_basename")
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the zero state, which is a fixed point for xorshift64.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform-ish in `[0, n)`; `n == 0` yields 0.
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }

    /// Any byte in `0x01..=0xFF` (never the NUL terminator).
    fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}

// ---------------------------------------------------------------------------
// The differential check
// ---------------------------------------------------------------------------

/// Outcome of one call, in a form that is fully comparable across the two
/// implementations: where the returned pointer landed inside the caller's
/// buffer, the bytes it points at, and whether the buffer was mutated.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    /// Byte offset of the returned pointer from the start of the input buffer.
    offset: isize,
    /// The NUL-terminated bytes at the returned pointer (terminator excluded).
    bytes: Vec<u8>,
    /// The whole buffer after the call, to catch any input mutation.
    buffer_after: Vec<u8>,
}

/// Call one implementation on its own private copy of `buf`.
///
/// `buf` must contain at least one NUL byte (the string terminator); bytes past
/// it are trailing garbage that the callee must ignore.
fn run(f: &Symbol<'static, ToolBasename>, buf: &[u8]) -> Outcome {
    assert!(buf.contains(&0), "test buffer must be NUL-terminated");
    let mut owned: Vec<u8> = buf.to_vec();
    let base = owned.as_mut_ptr() as *mut c_char;

    let ret = unsafe { f(base) };
    assert!(!ret.is_null(), "tool_basename never returns NULL for valid input");

    let offset = unsafe { ret.offset_from(base) };
    assert!(
        offset >= 0 && (offset as usize) < owned.len(),
        "returned pointer {offset} escaped the input buffer (len {})",
        owned.len()
    );

    // Read the returned C string.
    let mut bytes = Vec::new();
    let mut p = ret;
    loop {
        let b = unsafe { *p } as u8;
        if b == 0 {
            break;
        }
        bytes.push(b);
        p = unsafe { p.add(1) };
    }

    Outcome {
        offset,
        bytes,
        buffer_after: owned,
    }
}

/// The core assertion: C and Rust must agree byte-for-byte.
#[track_caller]
fn assert_same(row: &str, buf: &[u8]) {
    let c = run(&c_tool_basename(), buf);
    let rs = run(&rs_tool_basename(), buf);

    assert_eq!(
        c.offset, rs.offset,
        "[{row}] returned pointer offset differs for input {}\n  C  offset = {}\n  Rust offset = {}",
        show(buf),
        c.offset,
        rs.offset
    );
    assert_eq!(
        c.bytes,
        rs.bytes,
        "[{row}] returned string differs for input {}\n  C    = {}\n  Rust = {}",
        show(buf),
        show(&c.bytes),
        show(&rs.bytes)
    );
    assert_eq!(
        c.buffer_after,
        rs.buffer_after,
        "[{row}] input buffer mutated differently for input {}",
        show(buf)
    );
    assert_eq!(
        c.buffer_after,
        buf.to_vec(),
        "[{row}] C mutated the caller's buffer for input {} (unexpected; Rust must match)",
        show(buf)
    );
}

/// Escaped, length-bounded rendering of a byte string for assertion messages.
fn show(b: &[u8]) -> String {
    let mut s = String::from("b\"");
    for (i, &c) in b.iter().enumerate() {
        if i == 96 {
            s.push_str("...");
            break;
        }
        match c {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7E => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s.push('"');
    s.push_str(&format!(" (len {})", b.len()));
    s
}

/// Build a NUL-terminated buffer from a payload with no interior NUL.
fn cstr(payload: &[u8]) -> Vec<u8> {
    assert!(!payload.contains(&0));
    let mut v = payload.to_vec();
    v.push(0);
    v
}

const SEP_F: u8 = b'/';
const SEP_B: u8 = b'\\';

/// Random payload of `len` bytes drawn from `alphabet`.
fn random_payload(rng: &mut Rng, len: usize, alphabet: &[u8]) -> Vec<u8> {
    (0..len).map(|_| alphabet[rng.below(alphabet.len())]).collect()
}

/// Bytes that are neither separator nor NUL.
const PLAIN: &[u8] = b"abcXYZ0129 .-_~+";

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

#[test]
fn configs_row_01_empty_string() {
    assert_same("configs#1", &cstr(b""));
}

#[test]
fn configs_row_02_single_non_separator_byte() {
    for b in 1u16..=255 {
        let b = b as u8;
        if b == SEP_F || b == SEP_B {
            continue;
        }
        assert_same("configs#2", &cstr(&[b]));
    }
}

#[test]
fn configs_row_03_single_forward_slash() {
    assert_same("configs#3", &cstr(b"/"));
}

#[test]
fn configs_row_04_single_backslash() {
    assert_same("configs#4", &cstr(b"\\"));
}

#[test]
fn configs_row_05_no_separators_random() {
    let mut rng = Rng::new(0x0005);
    for _ in 0..2000 {
        let len = rng.range(2, 64);
        let p = random_payload(&mut rng, len, PLAIN);
        assert_same("configs#5", &cstr(&p));
    }
}

/// Rows 6-8 / 11-13: exactly one separator, at a chosen position.
fn one_separator_at(rng: &mut Rng, sep: u8, row: &str, place: Place) {
    for _ in 0..1000 {
        let len = rng.range(2, 48);
        let mut p = random_payload(rng, len, PLAIN);
        let idx = match place {
            Place::First => 0,
            Place::Last => len - 1,
            Place::Interior => rng.range(1, len - 2).min(len - 2),
        };
        p[idx] = sep;
        assert_same(row, &cstr(&p));
    }
}

#[derive(Copy, Clone)]
enum Place {
    First,
    Interior,
    Last,
}

#[test]
fn configs_row_06_only_slash_interior() {
    one_separator_at(&mut Rng::new(0x0006), SEP_F, "configs#6", Place::Interior);
}

#[test]
fn configs_row_07_only_slash_first_byte() {
    one_separator_at(&mut Rng::new(0x0007), SEP_F, "configs#7", Place::First);
}

#[test]
fn configs_row_08_only_slash_last_byte_empty_component() {
    one_separator_at(&mut Rng::new(0x0008), SEP_F, "configs#8", Place::Last);
}

#[test]
fn configs_row_09_only_slash_many_random_positions() {
    let mut rng = Rng::new(0x0009);
    let alphabet: Vec<u8> = PLAIN.iter().copied().chain([SEP_F, SEP_F]).collect();
    for _ in 0..3000 {
        let len = rng.range(1, 80);
        let mut p = random_payload(&mut rng, len, &alphabet);
        // Guarantee at least one separator, and never a backslash.
        p[rng.below(len)] = SEP_F;
        assert_same("configs#9", &cstr(&p));
    }
}

#[test]
fn configs_row_10_all_forward_slashes() {
    for n in 1..=64 {
        assert_same("configs#10", &cstr(&vec![SEP_F; n]));
    }
}

#[test]
fn configs_row_11_only_backslash_interior() {
    one_separator_at(&mut Rng::new(0x0011), SEP_B, "configs#11", Place::Interior);
}

#[test]
fn configs_row_12_only_backslash_first_byte() {
    one_separator_at(&mut Rng::new(0x0012), SEP_B, "configs#12", Place::First);
}

#[test]
fn configs_row_13_only_backslash_last_byte_empty_component() {
    one_separator_at(&mut Rng::new(0x0013), SEP_B, "configs#13", Place::Last);
}

#[test]
fn configs_row_14_only_backslash_many_random_positions() {
    let mut rng = Rng::new(0x0014);
    let alphabet: Vec<u8> = PLAIN.iter().copied().chain([SEP_B, SEP_B]).collect();
    for _ in 0..3000 {
        let len = rng.range(1, 80);
        let mut p = random_payload(&mut rng, len, &alphabet);
        p[rng.below(len)] = SEP_B;
        assert_same("configs#14", &cstr(&p));
    }
}

#[test]
fn configs_row_15_all_backslashes() {
    for n in 1..=64 {
        assert_same("configs#15", &cstr(&vec![SEP_B; n]));
    }
}

#[test]
fn configs_row_16_both_last_slash_after_last_backslash() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..3000 {
        let len = rng.range(4, 64);
        let mut p = random_payload(&mut rng, len, PLAIN);
        // last '\' strictly before last '/'
        let i_b = rng.range(0, len - 2);
        let i_f = rng.range(i_b + 1, len - 1);
        p[i_b] = SEP_B;
        p[i_f] = SEP_F;
        // No other separators exist because PLAIN has none.
        assert_same("configs#16", &cstr(&p));
    }
}

#[test]
fn configs_row_17_both_last_backslash_after_last_slash() {
    let mut rng = Rng::new(0x0017);
    for _ in 0..3000 {
        let len = rng.range(4, 64);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let i_f = rng.range(0, len - 2);
        let i_b = rng.range(i_f + 1, len - 1);
        p[i_f] = SEP_F;
        p[i_b] = SEP_B;
        assert_same("configs#17", &cstr(&p));
    }
}

#[test]
fn configs_row_18_adjacent_slash_then_backslash() {
    let mut rng = Rng::new(0x0018);
    for _ in 0..1500 {
        let len = rng.range(2, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let i = rng.range(0, len - 2);
        p[i] = SEP_F;
        p[i + 1] = SEP_B;
        assert_same("configs#18", &cstr(&p));
    }
    assert_same("configs#18", &cstr(b"/\\"));
    assert_same("configs#18", &cstr(b"a/\\b"));
}

#[test]
fn configs_row_19_adjacent_backslash_then_slash() {
    let mut rng = Rng::new(0x0019);
    for _ in 0..1500 {
        let len = rng.range(2, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let i = rng.range(0, len - 2);
        p[i] = SEP_B;
        p[i + 1] = SEP_F;
        assert_same("configs#19", &cstr(&p));
    }
    assert_same("configs#19", &cstr(b"\\/"));
    assert_same("configs#19", &cstr(b"a\\/b"));
}

#[test]
fn configs_row_20_both_winner_is_last_byte() {
    let mut rng = Rng::new(0x0020);
    for _ in 0..2000 {
        let len = rng.range(3, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let other = rng.range(0, len - 2);
        // Winner at the very end; alternate which separator wins.
        let (win, lose) = if rng.next_u64() & 1 == 0 {
            (SEP_F, SEP_B)
        } else {
            (SEP_B, SEP_F)
        };
        p[other] = lose;
        p[len - 1] = win;
        assert_same("configs#20", &cstr(&p));
    }
}

#[test]
fn configs_row_21_both_winner_is_first_byte() {
    // The winner must be the LAST occurrence of its own kind, so put the loser
    // nowhere later; a 2-byte string with both separators covers this exactly.
    assert_same("configs#21", &cstr(b"\\/"));
    assert_same("configs#21", &cstr(b"/\\"));
    let mut rng = Rng::new(0x0021);
    for _ in 0..1000 {
        let len = rng.range(2, 32);
        let mut p = random_payload(&mut rng, len, PLAIN);
        p[0] = SEP_F;
        if len > 1 {
            // '\' earlier is impossible at index 0, so make them adjacent.
            p[1] = SEP_B;
        }
        assert_same("configs#21", &cstr(&p));
    }
}

#[test]
fn configs_row_22_both_many_random_interleaving() {
    let mut rng = Rng::new(0x0022);
    let alphabet: Vec<u8> = PLAIN.iter().copied().chain([SEP_F, SEP_B, SEP_F, SEP_B]).collect();
    for _ in 0..5000 {
        let len = rng.range(2, 64);
        let mut p = random_payload(&mut rng, len, &alphabet);
        // Force both kinds to appear at least once.
        p[rng.below(len)] = SEP_F;
        p[rng.below(len)] = SEP_B;
        assert_same("configs#22", &cstr(&p));
    }
}

#[test]
fn configs_row_23_high_bit_bytes_signed_char_sensitivity() {
    let mut rng = Rng::new(0x0023);
    // Alphabet of only high-bit bytes plus the two separators.
    let mut alphabet: Vec<u8> = (0x80u8..=0xFF).collect();
    alphabet.push(SEP_F);
    alphabet.push(SEP_B);
    for _ in 0..5000 {
        let len = rng.range(1, 64);
        let p = random_payload(&mut rng, len, &alphabet);
        assert_same("configs#23", &cstr(&p));
    }
    // Every single high-bit byte adjacent to each separator.
    for b in 0x80u8..=0xFF {
        assert_same("configs#23", &cstr(&[b]));
        assert_same("configs#23", &cstr(&[b, SEP_F, b]));
        assert_same("configs#23", &cstr(&[SEP_B, b]));
        assert_same("configs#23", &cstr(&[b, SEP_B]));
    }
}

#[test]
fn configs_row_24_fully_random_bytes() {
    let mut rng = Rng::new(0x0024);
    for _ in 0..8000 {
        let len = rng.range(0, 128);
        let p: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        assert_same("configs#24", &cstr(&p));
    }
}

#[test]
fn configs_row_25_trailing_bytes_after_terminator_ignored() {
    let mut rng = Rng::new(0x0025);
    for _ in 0..3000 {
        let len = rng.range(0, 48);
        let head: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let tail_len = rng.range(1, 32);
        let tail: Vec<u8> = (0..tail_len)
            .map(|_| {
                // Deliberately separator-heavy garbage past the NUL.
                if rng.next_u64() & 1 == 0 { SEP_F } else { SEP_B }
            })
            .collect();
        let mut buf = head;
        buf.push(0);
        buf.extend_from_slice(&tail);
        buf.push(0); // keep the allocation well-terminated overall
        assert_same("configs#25", &buf);
    }
}

#[test]
fn configs_row_26_large_64k_separators_in_first_half() {
    let mut rng = Rng::new(0x0026);
    const N: usize = 64 * 1024;
    for _ in 0..8 {
        let mut p = random_payload(&mut rng, N, PLAIN);
        for _ in 0..64 {
            let i = rng.below(N / 2);
            p[i] = if rng.next_u64() & 1 == 0 { SEP_F } else { SEP_B };
        }
        assert_same("configs#26", &cstr(&p));
    }
}

#[test]
fn configs_row_27_large_64k_separators_in_last_bytes() {
    let mut rng = Rng::new(0x0027);
    const N: usize = 64 * 1024;
    for _ in 0..8 {
        let mut p = random_payload(&mut rng, N, PLAIN);
        for _ in 0..4 {
            let i = N - 1 - rng.below(8);
            p[i] = if rng.next_u64() & 1 == 0 { SEP_F } else { SEP_B };
        }
        assert_same("configs#27", &cstr(&p));
    }
}

#[test]
fn configs_row_28_large_64k_all_slashes() {
    assert_same("configs#28", &cstr(&vec![SEP_F; 64 * 1024]));
    assert_same("configs#28", &cstr(&vec![SEP_B; 64 * 1024]));
}

#[test]
fn configs_row_29_composition_feed_result_back_in() {
    let mut rng = Rng::new(0x0029);
    let alphabet: Vec<u8> = PLAIN.iter().copied().chain([SEP_F, SEP_B]).collect();
    let c = c_tool_basename();
    let rs = rs_tool_basename();

    for _ in 0..3000 {
        let len = rng.range(1, 64);
        let p = random_payload(&mut rng, len, &alphabet);
        let buf = cstr(&p);

        // First pass through each library, then feed each library's own result
        // back into itself — the real-consumer pipeline.
        let mut c_buf = buf.clone();
        let mut rs_buf = buf.clone();
        let c1 = unsafe { c(c_buf.as_mut_ptr() as *mut c_char) };
        let r1 = unsafe { rs(rs_buf.as_mut_ptr() as *mut c_char) };
        let c2 = unsafe { c(c1) };
        let r2 = unsafe { rs(r1) };

        let c_off = unsafe { c2.offset_from(c_buf.as_mut_ptr() as *mut c_char) };
        let r_off = unsafe { r2.offset_from(rs_buf.as_mut_ptr() as *mut c_char) };
        assert_eq!(
            c_off,
            r_off,
            "[configs#29] second-pass offset differs for {}",
            show(&buf)
        );
        // Second application must be a no-op relative to the first.
        assert_eq!(
            unsafe { c2.offset_from(c1) },
            0,
            "[configs#29] C not idempotent for {}",
            show(&buf)
        );
        assert_eq!(
            unsafe { r2.offset_from(r1) },
            0,
            "[configs#29] Rust not idempotent for {}",
            show(&buf)
        );
    }
}

#[test]
fn configs_row_30_repeated_calls_no_hidden_state() {
    let mut rng = Rng::new(0x0030);
    let alphabet: Vec<u8> = PLAIN.iter().copied().chain([SEP_F, SEP_B]).collect();
    let c = c_tool_basename();
    let rs = rs_tool_basename();

    for _ in 0..500 {
        let len = rng.range(1, 40);
        let p = random_payload(&mut rng, len, &alphabet);
        let buf = cstr(&p);

        let mut c_buf = buf.clone();
        let mut rs_buf = buf.clone();
        let base_c = c_buf.as_mut_ptr() as *mut c_char;
        let base_rs = rs_buf.as_mut_ptr() as *mut c_char;

        let mut prev: Option<(isize, isize)> = None;
        for _ in 0..5 {
            let oc = unsafe { c(base_c).offset_from(base_c) };
            let or = unsafe { rs(base_rs).offset_from(base_rs) };
            assert_eq!(oc, or, "[configs#30] offset differs for {}", show(&buf));
            if let Some(p) = prev {
                assert_eq!(p, (oc, or), "[configs#30] result changed across calls");
            }
            prev = Some((oc, or));
        }
        // Neither side may mutate the caller's buffer.
        assert_eq!(c_buf, buf, "[configs#30] C mutated the buffer");
        assert_eq!(rs_buf, buf, "[configs#30] Rust mutated the buffer");
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// `ERRORS.md` row 1: `path == NULL`.
///
/// The C performs no NULL check, so `strrchr(NULL, '/')` faults. Neither side
/// can return, so the comparison is done out-of-process: the same test binary
/// is re-invoked for each library and the resulting termination signal is
/// compared.
#[test]
fn errors_row_01_null_pointer_same_fault() {
    use std::os::unix::process::ExitStatusExt;

    fn child(which: &str) -> std::process::ExitStatus {
        let exe = std::env::current_exe().expect("current_exe");
        std::process::Command::new(exe)
            .args(["--exact", "--nocapture", "harness_null_call"])
            .env("HARNESS_NULL", which)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn child")
    }

    let c = child("c");
    let r = child("rust");

    assert_eq!(
        c.signal(),
        r.signal(),
        "[errors#1] NULL input must fault identically: C signal {:?}, Rust signal {:?}",
        c.signal(),
        r.signal()
    );
    assert_eq!(
        c.code(),
        r.code(),
        "[errors#1] NULL input exit codes differ: C {:?}, Rust {:?}",
        c.code(),
        r.code()
    );
    // Both must actually die from a fault rather than returning something.
    assert_eq!(
        c.signal(),
        Some(libc_sigsegv()),
        "[errors#1] expected the C side to fault on NULL"
    );
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

/// Helper invoked only in the child process spawned by
/// `errors_row_01_null_pointer_same_fault`. It is a no-op in a normal run.
#[test]
fn harness_null_call() {
    let which = match std::env::var("HARNESS_NULL") {
        Ok(v) => v,
        Err(_) => return, // normal run: nothing to do
    };
    let f = match which.as_str() {
        "c" => c_tool_basename(),
        "rust" => rs_tool_basename(),
        other => panic!("bad HARNESS_NULL={other}"),
    };
    // Expected to fault; if it somehow returns, report that distinctly.
    let r = unsafe { f(std::ptr::null_mut()) };
    eprintln!("returned {r:?} instead of faulting");
    std::process::exit(42);
}

#[test]
fn errors_row_02_empty_string_returns_input_pointer() {
    let buf = cstr(b"");
    assert_same("errors#2", &buf);
    // Identity: offset must be 0 for both.
    assert_eq!(run(&c_tool_basename(), &buf).offset, 0);
    assert_eq!(run(&rs_tool_basename(), &buf).offset, 0);
}

#[test]
fn errors_row_03_trailing_separator_yields_empty_component() {
    for s in [
        &b"a/"[..],
        &b"a\\"[..],
        &b"abc/"[..],
        &b"abc\\"[..],
        &b"a/b/"[..],
        &b"a\\b\\"[..],
        &b"a/b\\"[..],
        &b"a\\b/"[..],
    ] {
        assert_same("errors#3", &cstr(s));
        let o = run(&c_tool_basename(), &cstr(s));
        assert!(o.bytes.is_empty(), "expected empty component for {}", show(s));
        assert_eq!(o.bytes, run(&rs_tool_basename(), &cstr(s)).bytes);
    }
}

#[test]
fn errors_row_04_all_separators() {
    for s in [
        &b"/"[..],
        &b"//"[..],
        &b"///"[..],
        &b"\\"[..],
        &b"\\\\"[..],
        &b"\\\\\\"[..],
        &b"/\\/\\"[..],
        &b"\\/\\/"[..],
        &b"//\\\\"[..],
        &b"\\\\//"[..],
    ] {
        assert_same("errors#4", &cstr(s));
        assert!(run(&c_tool_basename(), &cstr(s)).bytes.is_empty());
    }
}

#[test]
fn errors_row_05_no_separator_returns_identity_pointer() {
    let mut rng = Rng::new(0x0105);
    for _ in 0..2000 {
        let len = rng.range(1, 64);
        let p = random_payload(&mut rng, len, PLAIN);
        let buf = cstr(&p);
        assert_same("errors#5", &buf);
        assert_eq!(run(&c_tool_basename(), &buf).offset, 0);
        assert_eq!(run(&rs_tool_basename(), &buf).offset, 0);
    }
}

#[test]
fn errors_row_06_only_forward_slash_branch() {
    let mut rng = Rng::new(0x0106);
    for _ in 0..2000 {
        let len = rng.range(1, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        p[rng.below(len)] = SEP_F;
        assert_same("errors#6", &cstr(&p));
    }
}

#[test]
fn errors_row_07_only_backslash_branch() {
    let mut rng = Rng::new(0x0107);
    for _ in 0..2000 {
        let len = rng.range(1, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        p[rng.below(len)] = SEP_B;
        assert_same("errors#7", &cstr(&p));
    }
}

#[test]
fn errors_row_08_both_slash_wins() {
    assert_same("errors#8", &cstr(b"a\\b/c"));
    let mut rng = Rng::new(0x0108);
    for _ in 0..2000 {
        let len = rng.range(3, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let i_b = rng.range(0, len - 2);
        let i_f = rng.range(i_b + 1, len - 1);
        p[i_b] = SEP_B;
        p[i_f] = SEP_F;
        let buf = cstr(&p);
        assert_same("errors#8", &buf);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            (i_f + 1) as isize,
            "'/' should win for {}",
            show(&buf)
        );
    }
}

#[test]
fn errors_row_09_both_backslash_wins() {
    assert_same("errors#9", &cstr(b"a/b\\c"));
    let mut rng = Rng::new(0x0109);
    for _ in 0..2000 {
        let len = rng.range(3, 48);
        let mut p = random_payload(&mut rng, len, PLAIN);
        let i_f = rng.range(0, len - 2);
        let i_b = rng.range(i_f + 1, len - 1);
        p[i_f] = SEP_F;
        p[i_b] = SEP_B;
        let buf = cstr(&p);
        assert_same("errors#9", &buf);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            (i_b + 1) as isize,
            "'\\' should win for {}",
            show(&buf)
        );
    }
}

#[test]
fn errors_row_10_adjacent_separators_pointer_compare() {
    assert_same("errors#10", &cstr(b"a/\\b"));
    assert_same("errors#10", &cstr(b"a\\/b"));
    assert_same("errors#10", &cstr(b"/\\"));
    assert_same("errors#10", &cstr(b"\\/"));
    // Same pair swept across every position of a fixed-length string.
    for len in 2..=24 {
        for i in 0..len - 1 {
            let mut p = vec![b'x'; len];
            p[i] = SEP_F;
            p[i + 1] = SEP_B;
            assert_same("errors#10", &cstr(&p));
            let mut q = vec![b'x'; len];
            q[i] = SEP_B;
            q[i + 1] = SEP_F;
            assert_same("errors#10", &cstr(&q));
        }
    }
}

#[test]
fn errors_row_11_separator_as_first_byte() {
    for s in [&b"/x"[..], &b"\\x"[..], &b"/"[..], &b"\\"[..], &b"/abc"[..], &b"\\abc"[..]] {
        let buf = cstr(s);
        assert_same("errors#11", &buf);
        assert_eq!(run(&c_tool_basename(), &buf).offset, 1);
        assert_eq!(run(&rs_tool_basename(), &buf).offset, 1);
    }
}

#[test]
fn errors_row_12_high_bit_bytes_not_confused_with_separators() {
    // 0xAF and 0xDC are 0x2F ('/') and 0x5C ('\\') with the high bit set;
    // a bad sign/width conversion would match them.
    for &b in &[0x80u8, 0xAF, 0xDC, 0xFF, 0x2F | 0x80, 0x5C | 0x80] {
        assert_same("errors#12", &cstr(&[b]));
        assert_same("errors#12", &cstr(&[b'a', b, b'b']));
        assert_same("errors#12", &cstr(&[b, b, b]));
        let buf = cstr(&[b'a', b, b'b']);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            0,
            "byte {b:#04x} must not be treated as a separator"
        );
    }
    let mut rng = Rng::new(0x0112);
    for _ in 0..3000 {
        let len = rng.range(1, 64);
        let p: Vec<u8> = (0..len)
            .map(|_| {
                let b = rng.nonzero_byte();
                if b == SEP_F || b == SEP_B { b | 0x80 } else { b }
            })
            .collect();
        let buf = cstr(&p);
        assert_same("errors#12", &buf);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            0,
            "no separators present, expected identity for {}",
            show(&buf)
        );
    }
}

#[test]
fn errors_row_13_bytes_past_terminator_ignored() {
    // "a\0/b" -> string is "a", the '/' past the NUL must be invisible.
    let cases: [&[u8]; 6] = [
        b"a\0/b\0",
        b"a\0\\b\0",
        b"\0/////\0",
        b"\0\\\\\\\0",
        b"abc\0/x/y\0",
        b"abc\0\\x\\y\0",
    ];
    for buf in cases {
        assert_same("errors#13", buf);
        let o = run(&c_tool_basename(), buf);
        assert_eq!(o.offset, 0, "must ignore separators past the NUL in {}", show(buf));
        assert_eq!(o.offset, run(&rs_tool_basename(), buf).offset);
    }
}

#[test]
fn errors_row_14_very_long_input_no_length_limit() {
    let mut rng = Rng::new(0x0114);
    for &n in &[1024usize, 4096, 16 * 1024, 64 * 1024] {
        let mut p = random_payload(&mut rng, n, PLAIN);
        p[n - 5] = SEP_F;
        p[n - 3] = SEP_B;
        let buf = cstr(&p);
        assert_same("errors#14", &buf);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            (n - 2) as isize,
            "last '\\' at n-3 should win for length {n}"
        );
    }
}

#[test]
fn errors_row_15_only_last_occurrence_matters() {
    for s in [
        &b"a//b//"[..],
        &b"a//b//c"[..],
        &b"a\\\\b\\\\c"[..],
        &b"//a//b"[..],
        &b"x/y/z/w"[..],
        &b"x\\y\\z\\w"[..],
    ] {
        assert_same("errors#15", &cstr(s));
    }
    let mut rng = Rng::new(0x0115);
    for _ in 0..3000 {
        let len = rng.range(8, 64);
        let mut p = random_payload(&mut rng, len, PLAIN);
        // A run of separators early on, then a single later one that must win.
        let run_at = rng.range(0, len / 2 - 1);
        let run_end = (run_at + 3).min(len / 2); // exclusive
        for k in run_at..run_end {
            p[k] = SEP_F;
        }
        let last = rng.range(run_end, len - 1);
        p[last] = SEP_F;
        let buf = cstr(&p);
        assert_same("errors#15", &buf);
        assert_eq!(
            run(&c_tool_basename(), &buf).offset,
            (last + 1) as isize,
            "only the last separator (index {last}) may count for {}",
            show(&buf)
        );
    }
}

// ===========================================================================
// Phase D — symbol parity, asserted from inside the test suite
// ===========================================================================

#[test]
fn phase_d_symbol_parity() {
    fn defined_globals(so: &PathBuf) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(so)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", so.display());
        let text = String::from_utf8_lossy(&out.stdout);
        let mut v: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let _addr = it.next()?;
                let kind = it.next()?;
                let name = it.next()?;
                // Global text/data only; skip toolchain/section artifacts.
                if !matches!(kind, "T" | "D" | "B" | "R") {
                    return None;
                }
                if name.starts_with("_init")
                    || name.starts_with("_fini")
                    || name.starts_with("__bss_start")
                    || name.starts_with("_edata")
                    || name.starts_with("_end")
                {
                    return None;
                }
                Some(name.to_string())
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = defined_globals(&c_so_path());
    let rs = defined_globals(&rust_so_path());

    assert!(
        c.contains(&"tool_basename".to_string()),
        "C .so must export tool_basename, got {c:?}"
    );

    let missing: Vec<&String> = c.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
}

#[test]
fn phase_d_no_undefined_non_libc_symbols_in_rust() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the Rust cdylib may legitimately import: libc, libgcc unwind,
    // libm, pthread, and the usual toolchain hooks.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__gmon_start__", "__tls_get_addr", "__errno_location",
        "__libc_", "pthread_", "__pthread_", "__assert", "__stack_chk", "__memcpy", "__strlen",
        "__snprintf", "__sprintf", "__vsnprintf", "__fxstat", "__xstat", "__environ",
    ];

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        let bare = name.split('@').next().unwrap_or(name);
        if allowed_prefixes.iter().any(|p| bare.starts_with(p)) {
            continue;
        }
        // A libc/libm/libpthread export resolves in the process image.
        if is_libc_symbol(bare) {
            continue;
        }
        suspicious.push(bare.to_string());
    }

    assert!(
        suspicious.is_empty(),
        "Rust .so has undefined non-libc symbols (would fail to load): {suspicious:?}"
    );
}

/// A symbol counts as libc-provided if a lookup in the global scope of this
/// process resolves it, which is exactly the condition for the `.so` to load.
fn is_libc_symbol(name: &str) -> bool {
    // The Rust .so itself loaded successfully via dlopen (see `libs()`), which
    // proves every one of its undefined symbols resolved at load time. Use that
    // as the authority rather than maintaining a hand-written list.
    let _ = libs();
    let mut probe = name.as_bytes().to_vec();
    probe.push(0);
    let this = libloading::os::unix::Library::this();
    unsafe { this.get::<*const ()>(&probe).is_ok() }
}
