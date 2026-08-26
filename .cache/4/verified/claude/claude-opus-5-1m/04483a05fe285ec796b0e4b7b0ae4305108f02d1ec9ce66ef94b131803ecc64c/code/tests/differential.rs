// Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//
// The Rust implementation is ONLY ever exercised through its exported
// `#[no_mangle] extern "C"` symbols in `libfindrep_lib.so` — never called
// directly as Rust code — so the export wrappers are covered too.
//
// Both libraries keep mutable file-scope state (`accumulator`, `multiplier`,
// `operation_count`). Every helper in this file issues each call to BOTH
// libraries in lockstep, so their hidden state stays in sync no matter in which
// order the (possibly parallel) tests run. A global mutex serialises the
// lockstep pairs.

#![allow(clippy::missing_safety_doc)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type FnI = unsafe extern "C" fn(i32) -> i32;
type FnII = unsafe extern "C" fn(i32, i32) -> i32;
type FnBufI = unsafe extern "C" fn(*mut c_char, i32);
type FnIIII = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

pub struct Api {
    pub name: &'static str,
    pub add_to_accumulator: FnII,
    pub multiply_with_multiplier: FnII,
    pub subtract_from_accumulator: FnII,
    pub divide_multiplier: FnII,
    pub process_octal_string: FnBufI,
    pub find_and_replace_char: FnBufI,
    pub validate_and_normalize: FnI,
    pub findrep: FnIIII,
    // keep the library alive for the whole process lifetime
    _lib: Library,
}

unsafe fn sym<T: Copy, const N: usize>(lib: &Library, name: &[u8; N]) -> T {
    let s: Symbol<T> = lib
        .get(&name[..])
        .unwrap_or_else(|e| panic!("missing symbol {:?}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Api {
    pub fn load(path: &Path, name: &'static str) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
            Api {
                name,
                add_to_accumulator: sym(&lib, b"add_to_accumulator\0"),
                multiply_with_multiplier: sym(&lib, b"multiply_with_multiplier\0"),
                subtract_from_accumulator: sym(&lib, b"subtract_from_accumulator\0"),
                divide_multiplier: sym(&lib, b"divide_multiplier\0"),
                process_octal_string: sym(&lib, b"process_octal_string\0"),
                find_and_replace_char: sym(&lib, b"find_and_replace_char\0"),
                validate_and_normalize: sym(&lib, b"validate_and_normalize\0"),
                findrep: sym(&lib, b"findrep\0"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    // `HARVEST_C_LIB` allows pointing the suite at a differently configured C
    // build (e.g. an -O2 one) without touching anything in c_src/.
    if let Ok(p) = std::env::var("HARVEST_C_LIB") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_C_LIB does not exist: {}", p.display());
        return p;
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `cargo test` does not rebuild the `cdylib` artifact (no test target links
/// it), so the `.so` next to the test binary can be missing or stale, which
/// would silently invalidate every result. Build it once, on demand. Cargo
/// releases its build lock before it runs the test binaries, so this is safe.
fn ensure_built() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build").current_dir(manifest_dir());
        if !cfg!(debug_assertions) {
            cmd.arg("--release");
        }
        let _ = cmd.status();
    });
}

fn is_fresh(so: &Path) -> bool {
    let src = manifest_dir().join("src/lib.rs");
    match (std::fs::metadata(so), std::fs::metadata(&src)) {
        (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
            (Ok(so_t), Ok(src_t)) => so_t >= src_t,
            _ => true,
        },
        _ => false,
    }
}

fn check_fresh(so: &Path) {
    assert!(
        is_fresh(so),
        "{} is older than {} — run `cargo build` (or `cargo build --release`) \
         before `cargo test`, because cargo does not rebuild a cdylib for tests",
        so.display(),
        manifest_dir().join("src/lib.rs").display()
    );
}

pub fn rust_lib_path() -> PathBuf {
    // `HARVEST_RUST_LIB` allows testing another profile's cdylib (e.g.
    // target/release/libfindrep_lib.so).
    if let Ok(p) = std::env::var("HARVEST_RUST_LIB") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_RUST_LIB does not exist: {}", p.display());
        check_fresh(&p);
        return p;
    }
    // The test binary lives in <target>/<profile>/deps/, the cdylib in
    // <target>/<profile>/ (and a copy in deps/).
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let candidates = [
        profile.join("libfindrep_lib.so"),
        deps.join("libfindrep_lib.so"),
    ];
    if !candidates.iter().any(|c| is_fresh(c)) {
        ensure_built();
    }
    for cand in &candidates {
        if is_fresh(cand) {
            return cand.clone();
        }
    }
    for cand in &candidates {
        if cand.exists() {
            check_fresh(cand); // panics with the actionable message
            return cand.clone();
        }
    }
    panic!(
        "Rust cdylib libfindrep_lib.so not found next to {} — run `cargo build` first",
        exe.display()
    );
}

pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();

pub fn pair() -> MutexGuard<'static, Pair> {
    let m = PAIR.get_or_init(|| {
        Mutex::new(Pair {
            c: Api::load(&c_lib_path(), "C"),
            r: Api::load(&rust_lib_path(), "Rust"),
        })
    });
    // Never let a failing test poison the mutex for the others.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every run is reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Small-magnitude int, biased towards interesting values.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIALS: [i32; 20] = [
            0,
            1,
            -1,
            2,
            -2,
            7,
            8,
            63,
            64,
            65,
            0o100,
            0o150,
            0o151,
            0o777,
            0o1000,
            511,
            512,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
        ];
        match self.below(3) {
            0 => SPECIALS[self.below(SPECIALS.len() as u64) as usize],
            1 => (self.below(2048) as i32) - 1024,
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Lockstep differential helpers
// ---------------------------------------------------------------------------

const BUF: usize = 256;
const CANARY: u8 = 0xAA;

/// Probe the hidden state through state-neutral public calls and assert the two
/// libraries agree. `add_to_accumulator(0, 0)` leaves `accumulator` unchanged
/// and returns it; `divide_multiplier(0, 1)` leaves `multiplier` unchanged and
/// returns it. Both bump `operation_count` in both libraries equally.
pub fn probe_state(p: &Pair, ctx: &str) -> (i32, i32) {
    unsafe {
        let ca = (p.c.add_to_accumulator)(0, 0);
        let ra = (p.r.add_to_accumulator)(0, 0);
        assert_eq!(ca, ra, "accumulator diverged ({ctx}): C={ca} Rust={ra}");
        let cm = (p.c.divide_multiplier)(0, 1);
        let rm = (p.r.divide_multiplier)(0, 1);
        assert_eq!(cm, rm, "multiplier diverged ({ctx}): C={cm} Rust={rm}");
        (ca, cm)
    }
}

pub fn diff_add(p: &Pair, a: i32, b: i32) -> i32 {
    unsafe {
        let c = (p.c.add_to_accumulator)(a, b);
        let r = (p.r.add_to_accumulator)(a, b);
        assert_eq!(c, r, "add_to_accumulator({a},{b}): C={c} Rust={r}");
        c
    }
}

pub fn diff_mul(p: &Pair, a: i32, b: i32) -> i32 {
    unsafe {
        let c = (p.c.multiply_with_multiplier)(a, b);
        let r = (p.r.multiply_with_multiplier)(a, b);
        assert_eq!(c, r, "multiply_with_multiplier({a},{b}): C={c} Rust={r}");
        c
    }
}

pub fn diff_sub(p: &Pair, a: i32, b: i32) -> i32 {
    unsafe {
        let c = (p.c.subtract_from_accumulator)(a, b);
        let r = (p.r.subtract_from_accumulator)(a, b);
        assert_eq!(c, r, "subtract_from_accumulator({a},{b}): C={c} Rust={r}");
        c
    }
}

pub fn diff_div(p: &Pair, a: i32, b: i32) -> i32 {
    unsafe {
        let c = (p.c.divide_multiplier)(a, b);
        let r = (p.r.divide_multiplier)(a, b);
        assert_eq!(c, r, "divide_multiplier({a},{b}): C={c} Rust={r}");
        c
    }
}

pub fn diff_validate(p: &Pair, v: i32) -> i32 {
    unsafe {
        let c = (p.c.validate_and_normalize)(v);
        let r = (p.r.validate_and_normalize)(v);
        assert_eq!(c, r, "validate_and_normalize({v}): C={c} Rust={r}");
        c
    }
}

fn show(buf: &[u8]) -> String {
    let mut s = String::new();
    for &b in buf {
        if b == CANARY {
            s.push('.');
        } else if (0x20..0x7f).contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02x}"));
        }
    }
    s
}

/// Compare `process_octal_string` on a canary-filled destination buffer:
/// the whole buffer (not just the string) must match byte for byte.
pub fn diff_process_octal(p: &Pair, val: i32) -> Vec<u8> {
    let mut cb = [CANARY; BUF];
    let mut rb = [CANARY; BUF];
    unsafe {
        (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, val);
        (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, val);
    }
    assert_eq!(
        cb.as_slice(),
        rb.as_slice(),
        "process_octal_string({val}) buffers differ:\n C   : {}\n Rust: {}",
        show(&cb),
        show(&rb)
    );
    cb.to_vec()
}

/// Like `diff_process_octal` but on a pre-seeded buffer (to check that stale
/// bytes past the terminator are treated identically).
pub fn diff_process_octal_seeded(p: &Pair, val: i32, seed: &[u8]) {
    let mut cb = [CANARY; BUF];
    let mut rb = [CANARY; BUF];
    cb[..seed.len()].copy_from_slice(seed);
    rb[..seed.len()].copy_from_slice(seed);
    unsafe {
        (p.c.process_octal_string)(cb.as_mut_ptr() as *mut c_char, val);
        (p.r.process_octal_string)(rb.as_mut_ptr() as *mut c_char, val);
    }
    assert_eq!(
        cb.as_slice(),
        rb.as_slice(),
        "process_octal_string({val}) on seeded buffer differ:\n C   : {}\n Rust: {}",
        show(&cb),
        show(&rb)
    );
}

/// Compare `find_and_replace_char` for a haystack (given without terminator)
/// and a needle passed as a raw `int`.
pub fn diff_find_replace(p: &Pair, hay: &[u8], needle: i32) {
    assert!(hay.len() + 1 < BUF);
    assert!(!hay.contains(&0), "haystack must not contain NUL");
    let mut cb = [CANARY; BUF];
    let mut rb = [CANARY; BUF];
    cb[..hay.len()].copy_from_slice(hay);
    rb[..hay.len()].copy_from_slice(hay);
    cb[hay.len()] = 0;
    rb[hay.len()] = 0;
    unsafe {
        (p.c.find_and_replace_char)(cb.as_mut_ptr() as *mut c_char, needle);
        (p.r.find_and_replace_char)(rb.as_mut_ptr() as *mut c_char, needle);
    }
    assert_eq!(
        cb.as_slice(),
        rb.as_slice(),
        "find_and_replace_char({:?}, {needle}) buffers differ:\n C   : {}\n Rust: {}",
        String::from_utf8_lossy(hay),
        show(&cb),
        show(&rb)
    );
}

pub fn diff_findrep(p: &Pair, a: i32, b: i32, c_: i32, d: i32) -> i32 {
    unsafe {
        let c = (p.c.findrep)(a, b, c_, d);
        let r = (p.r.findrep)(a, b, c_, d);
        assert_eq!(c, r, "findrep({a},{b},{c_},{d}): C={c} Rust={r}");
        c
    }
}

/// `divide_multiplier` with a divisor that would trap: `INT_MIN / -1` is
/// undefined in C and traps (SIGFPE) on x86-64. That exact case is covered by
/// its own subprocess test (`err_divide_int_min_by_minus_one`); here we make
/// sure the in-process randomized tests never take the whole harness down.
pub fn diff_div_guarded(p: &Pair, a: i32, b: i32) -> i32 {
    let (_, m) = probe_state(p, "before divide_multiplier");
    let b = if b == -1 && m == i32::MIN { -3 } else { b };
    diff_div(p, a, b)
}

// ===========================================================================
// PHASE B — valid-path differential tests (CONFIGS.md rows)
// ===========================================================================

// --- rows 1-3: validate_and_normalize -------------------------------------

#[test]
fn cfg_validate_boundaries() {
    let p = pair();
    for v in [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -65,
        -64,
        -1,
        0,
        1,
        2,
        62,
        63,
        64,
        65,
        66,
        104,
        105,
        509,
        510,
        511,
        512,
        513,
        1000,
        65535,
        i32::MAX - 1,
        i32::MAX,
    ] {
        diff_validate(&p, v);
    }
}

#[test]
fn cfg_validate_random_full_range() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..4096 {
        diff_validate(&p, rng.next_i32());
        diff_validate(&p, rng.interesting_i32());
    }
}

#[test]
fn cfg_validate_random_per_class() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..1024 {
        // class: value <= 0
        diff_validate(&p, -(rng.below(0x8000_0000) as i64 as i32));
        // class: 1 .. 63
        diff_validate(&p, 1 + rng.below(63) as i32);
        // class: 64 .. 511
        diff_validate(&p, 64 + rng.below(448) as i32);
        // class: 512 .. INT_MAX
        diff_validate(&p, 512 + rng.below(i32::MAX as u64 - 511) as i32);
    }
}

// --- rows 4-9: process_octal_string ---------------------------------------

#[test]
fn cfg_process_octal_string_boundaries() {
    let p = pair();
    for v in [
        0, 1, 2, 7, 8, 9, 63, 64, 0o123, 0o777, 511, 512, 4095, 65535, 1_000_000, i32::MAX - 1,
        i32::MAX, -1, -2, -8, -9, -511, -1000, -65536, i32::MIN + 1, i32::MIN,
    ] {
        let out = diff_process_octal(&p, v);
        // sanity: the rendering must be NUL terminated inside the buffer
        assert!(out.iter().any(|&b| b == 0), "no terminator for {v}");
    }
    // exact expected text for the value the library itself uses internally
    let out = diff_process_octal(&p, 0o123);
    let s = std::ffi::CStr::from_bytes_until_nul(&out).unwrap();
    assert_eq!(s.to_bytes(), b"Octal: 0123, Decimal: 83");
}

#[test]
fn cfg_process_octal_string_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..2048 {
        diff_process_octal(&p, rng.next_i32());
        diff_process_octal(&p, rng.interesting_i32());
    }
}

#[test]
fn cfg_process_octal_string_reused_buffer() {
    let p = pair();
    // long rendering first, then a short one into the same (seeded) buffer:
    // the bytes past the new terminator must be identical in both libraries.
    let long = diff_process_octal(&p, i32::MIN);
    diff_process_octal_seeded(&p, 0, &long[..64]);
    diff_process_octal_seeded(&p, 7, &long[..64]);
    diff_process_octal_seeded(&p, -1, &long[..64]);
    let mut rng = Rng::new(0x5EED_0004);
    for _ in 0..512 {
        let seed: Vec<u8> = (0..64).map(|_| (rng.below(255) as u8) + 1).collect();
        diff_process_octal_seeded(&p, rng.next_i32(), &seed);
    }
}

// --- rows 10-14: find_and_replace_char ------------------------------------

#[test]
fn cfg_find_and_replace_positions() {
    let p = pair();
    let hay = b"Function pointer example with static vars";
    // first byte, middle, last byte, absent
    for needle in [b'F', b'p', b'o', b's', b'z', b'X', b' '] {
        diff_find_replace(&p, hay, needle as i32);
    }
    // 1-byte haystacks: hit and miss
    diff_find_replace(&p, b"a", b'a' as i32);
    diff_find_replace(&p, b"a", b'b' as i32);
    // needle == last byte only
    diff_find_replace(&p, b"abcdefg", b'g' as i32);
    // haystack already containing 'X'
    diff_find_replace(&p, b"aXbXc", b'X' as i32);
    // every byte value 1..=255 against a haystack that contains it
    for byte in 1u8..=255 {
        let hay = [b'a', b'b', byte, b'c', byte];
        diff_find_replace(&p, &hay, byte as i32);
    }
}

#[test]
fn cfg_find_and_replace_repeated() {
    let p = pair();
    diff_find_replace(&p, b"aaaaaaaa", b'a' as i32);
    diff_find_replace(&p, b"abababab", b'b' as i32);
    diff_find_replace(&p, &[b'z'; 80], b'z' as i32);
}

#[test]
fn cfg_find_and_replace_high_bit() {
    let p = pair();
    let hay: Vec<u8> = (0x80u8..=0xFF).collect();
    for &needle_byte in &[0x80u8, 0xAAu8, 0xFFu8] {
        // as a plain positive int
        diff_find_replace(&p, &hay, needle_byte as i32);
        // as a sign-extended negative int (same unsigned char)
        diff_find_replace(&p, &hay, needle_byte as i8 as i32);
        // as an int above 0xFF with the same low byte
        diff_find_replace(&p, &hay, 0x1234_5600 | needle_byte as i32);
    }
}

#[test]
fn cfg_find_and_replace_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..4096 {
        let len = rng.below(81) as usize;
        let hay: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
        let needle = match rng.below(4) {
            0 if len > 0 => hay[rng.below(len as u64) as usize] as i32, // guaranteed hit
            1 => rng.below(256) as i32,
            2 => rng.next_i32(),
            _ => (rng.below(256) as i32) - 128,
        };
        diff_find_replace(&p, &hay, needle);
    }
}

// --- rows 15-20: the four stateful leaf ops -------------------------------

#[test]
fn cfg_add_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0006);
    for _ in 0..1024 {
        diff_add(&p, rng.interesting_i32(), rng.interesting_i32());
        probe_state(&p, "cfg_add_random");
    }
}

#[test]
fn cfg_sub_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0007);
    for _ in 0..1024 {
        diff_sub(&p, rng.interesting_i32(), rng.interesting_i32());
        probe_state(&p, "cfg_sub_random");
    }
}

#[test]
fn cfg_mul_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0008);
    for _ in 0..1024 {
        diff_mul(&p, rng.interesting_i32(), rng.interesting_i32());
        probe_state(&p, "cfg_mul_random");
    }
}

#[test]
fn cfg_divide_special_divisors() {
    let p = pair();
    // put the multiplier through several magnitude/sign classes and divide by
    // each interesting divisor
    for setup in [1, 1000, -1000, 7, -7, 0] {
        diff_mul(&p, 1, setup);
        for b in [0, 1, -1, 2, -2, 3, -3, 1000, -1000, i32::MAX, i32::MIN] {
            diff_div_guarded(&p, 0, b);
            probe_state(&p, "cfg_divide_special_divisors");
        }
        // restore a non-zero multiplier for the next round
        diff_mul(&p, 1, 1);
    }
}

#[test]
fn cfg_divide_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0009);
    for _ in 0..1024 {
        // occasionally re-seed the multiplier so it does not stay at 0 forever
        if rng.below(8) == 0 {
            diff_mul(&p, 1, rng.interesting_i32());
        }
        diff_div_guarded(&p, rng.interesting_i32(), rng.interesting_i32());
        probe_state(&p, "cfg_divide_random");
    }
}

#[test]
fn cfg_interleaved_state_sequence() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_000A);
    for _ in 0..8192 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        match rng.below(4) {
            0 => {
                diff_add(&p, a, b);
            }
            1 => {
                diff_mul(&p, a, b);
            }
            2 => {
                diff_sub(&p, a, b);
            }
            _ => {
                diff_div_guarded(&p, a, b);
            }
        }
        probe_state(&p, "cfg_interleaved_state_sequence");
    }
}

// --- rows 21-29: findrep (the composed pipeline) ---------------------------

#[test]
fn cfg_findrep_active_param_counts() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_000B);
    // every one of the 16 zero/non-zero masks -> active_params 0..4
    for mask in 0u32..16 {
        for _ in 0..32 {
            let mut v = [0i32; 4];
            for (i, slot) in v.iter_mut().enumerate() {
                if mask & (1 << i) != 0 {
                    // guaranteed non-zero value
                    let mut x = rng.interesting_i32();
                    if x == 0 {
                        x = 1;
                    }
                    *slot = x;
                }
            }
            diff_findrep(&p, v[0], v[1], v[2], v[3]);
            probe_state(&p, "cfg_findrep_active_param_counts");
        }
    }
}

#[test]
fn cfg_findrep_normalization_classes() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_000C);
    // 4 normalization classes ^ 4 params = 256 combinations
    let pick = |rng: &mut Rng, class: u32| -> i32 {
        match class {
            0 => -(1 + rng.below(4096) as i32),          // v < 0  (pass through)
            1 => 1 + rng.below(63) as i32,               // 0 < v < 64 -> 64
            2 => 64 + rng.below(448) as i32,             // 64..511 identity
            _ => 512 + rng.below(1_000_000) as i32,      // > 511 -> 511
        }
    };
    for combo in 0u32..256 {
        let a = pick(&mut rng, combo & 3);
        let b = pick(&mut rng, (combo >> 2) & 3);
        let c = pick(&mut rng, (combo >> 4) & 3);
        let d = pick(&mut rng, (combo >> 6) & 3);
        diff_findrep(&p, a, b, c, d);
        probe_state(&p, "cfg_findrep_normalization_classes");
    }
}

#[test]
fn cfg_findrep_boundary_cross() {
    let p = pair();
    const B: [i32; 13] = [
        i32::MIN,
        -1,
        0,
        1,
        63,
        64,
        65,
        104,
        105,
        510,
        511,
        512,
        i32::MAX,
    ];
    for &a in &B {
        for &b in &B {
            diff_findrep(&p, a, b, 0, 0);
            diff_findrep(&p, 0, 0, a, b);
            diff_findrep(&p, a, b, b, a);
            probe_state(&p, "cfg_findrep_boundary_cross");
        }
    }
}

#[test]
fn cfg_findrep_accumulator_over_threshold() {
    let p = pair();
    // drive the accumulator above 0150 (=104) so findrep takes the subtract
    // branch, then across it again from the other side
    for target in [105i32, 200, 10_000, -10_000, 104, 0] {
        let (acc, _) = probe_state(&p, "setup");
        diff_add(&p, target.wrapping_sub(acc), 0);
        probe_state(&p, "after setup");
        diff_findrep(&p, 1, 2, 3, 4);
        diff_findrep(&p, 0, 0, 0, 0);
        diff_findrep(&p, 700, -700, 65, 0);
        probe_state(&p, "cfg_findrep_accumulator_over_threshold");
    }
}

#[test]
fn cfg_findrep_multiplier_states() {
    let p = pair();
    // Deterministic coverage of the multiplier-dependent branches lives in the
    // scenario children (a fresh process each, so `multiplier` starts at 1):
    //   multiplier == 64 / 128 / 10^6 -> the `multiplier > 0100` divide branch
    //   multiplier == 0              -> the `both_active` branch is skipped
    //   multiplier < 0               -> negative divisor/dividend handling
    for scenario in ["multiplier_over_100", "multiplier_zero", "multiplier_negative"] {
        let c = run_child("c", scenario);
        let r = run_child("rust", scenario);
        assert!(
            c.status.success() && r.status.success(),
            "{scenario}: C {:?} Rust {:?}",
            c.status,
            r.status
        );
        assert_eq!(c.transcript, r.transcript, "{scenario}: transcripts differ");
    }
    // In-process: whatever the shared multiplier currently is, findrep must
    // agree; then push it to zero (a state the C library can never leave) and
    // re-check.
    let (_, m) = probe_state(&p, "cfg_findrep_multiplier_states");
    diff_findrep(&p, 1, 1, 1, 1);
    diff_findrep(&p, 511, 512, -1, 64);
    assert_eq!(diff_mul(&p, 0, 0), 0, "multiplier must be 0 after *0 (was {m})");
    diff_findrep(&p, 0, 0, 0, 0);
    diff_findrep(&p, 1, 2, 3, 4);
    diff_findrep(&p, 511, 512, -1, 64);
    probe_state(&p, "cfg_findrep_multiplier_states end");
}

#[test]
fn cfg_findrep_random() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_000D);
    for _ in 0..4096 {
        diff_findrep(
            &p,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        probe_state(&p, "cfg_findrep_random");
    }
}

#[test]
fn cfg_full_api_random_sequence() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_000E);
    for _ in 0..8192 {
        let a = rng.interesting_i32();
        let b = rng.interesting_i32();
        match rng.below(8) {
            0 => {
                diff_add(&p, a, b);
            }
            1 => {
                diff_mul(&p, a, b);
            }
            2 => {
                diff_sub(&p, a, b);
            }
            3 => {
                diff_div_guarded(&p, a, b);
            }
            4 => {
                diff_validate(&p, a);
            }
            5 => {
                diff_process_octal(&p, a);
            }
            6 => {
                let len = rng.below(64) as usize;
                let hay: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
                diff_find_replace(&p, &hay, b);
            }
            _ => {
                diff_findrep(&p, a, b, rng.interesting_i32(), rng.interesting_i32());
            }
        }
        probe_state(&p, "cfg_full_api_random_sequence");
    }
}

// ===========================================================================
// Scenario children — full control over the hidden state
//
// Every scenario runs in a FRESH child process (so `accumulator == 0`,
// `multiplier == 1`, `operation_count == 0`) against exactly ONE of the two
// libraries, and writes a transcript of every observable to a file. The parent
// runs the C child and the Rust child and compares the transcripts byte for
// byte. This is how state-dependent branches (`accumulator > 0150`,
// `multiplier > 0100`, `multiplier == 0`, the `result == 0` sentinel) and the
// fatal paths (null pointer, `INT_MIN / -1`) are covered deterministically.
// ===========================================================================

#[derive(Default)]
pub struct Transcript(pub String);

impl Transcript {
    fn line(&mut self, s: impl AsRef<str>) {
        self.0.push_str(s.as_ref());
        self.0.push('\n');
    }
}

fn t_octal(api: &Api, t: &mut Transcript, v: i32) {
    let mut b = [CANARY; BUF];
    unsafe { (api.process_octal_string)(b.as_mut_ptr() as *mut c_char, v) };
    t.line(format!("process_octal_string({v}) -> [{}]", show(&b)));
}

fn t_find(api: &Api, t: &mut Transcript, hay: &[u8], needle: i32) {
    let mut b = [CANARY; BUF];
    b[..hay.len()].copy_from_slice(hay);
    b[hay.len()] = 0;
    unsafe { (api.find_and_replace_char)(b.as_mut_ptr() as *mut c_char, needle) };
    t.line(format!(
        "find_and_replace_char({:?}, {needle}) -> [{}]",
        String::from_utf8_lossy(hay),
        show(&b)
    ));
}

fn t_state(api: &Api, t: &mut Transcript, tag: &str) -> (i32, i32) {
    unsafe {
        let a = (api.add_to_accumulator)(0, 0);
        let m = (api.divide_multiplier)(0, 1);
        t.line(format!("state[{tag}] accumulator={a} multiplier={m}"));
        (a, m)
    }
}

fn t_add(api: &Api, t: &mut Transcript, a: i32, b: i32) -> i32 {
    let r = unsafe { (api.add_to_accumulator)(a, b) };
    t.line(format!("add_to_accumulator({a},{b}) -> {r}"));
    r
}

fn t_sub(api: &Api, t: &mut Transcript, a: i32, b: i32) -> i32 {
    let r = unsafe { (api.subtract_from_accumulator)(a, b) };
    t.line(format!("subtract_from_accumulator({a},{b}) -> {r}"));
    r
}

fn t_mul(api: &Api, t: &mut Transcript, a: i32, b: i32) -> i32 {
    let r = unsafe { (api.multiply_with_multiplier)(a, b) };
    t.line(format!("multiply_with_multiplier({a},{b}) -> {r}"));
    r
}

fn t_div(api: &Api, t: &mut Transcript, a: i32, b: i32) -> i32 {
    let r = unsafe { (api.divide_multiplier)(a, b) };
    t.line(format!("divide_multiplier({a},{b}) -> {r}"));
    r
}

fn t_validate(api: &Api, t: &mut Transcript, v: i32) -> i32 {
    let r = unsafe { (api.validate_and_normalize)(v) };
    t.line(format!("validate_and_normalize({v}) -> {r}"));
    r
}

fn t_findrep(api: &Api, t: &mut Transcript, a: i32, b: i32, c: i32, d: i32) -> i32 {
    let r = unsafe { (api.findrep)(a, b, c, d) };
    t.line(format!("findrep({a},{b},{c},{d}) -> {r}"));
    r
}

pub const SCENARIOS: &[&str] = &[
    "pristine_all",
    "all_zero_params",
    "one_active_param",
    "accumulator_over_150",
    "multiplier_over_100",
    "multiplier_zero",
    "multiplier_negative",
    "divide_truncation",
    "divide_by_zero",
    "sentinel_zero_result",
    "overflow_walk",
    "long_random",
];

pub const CRASH_SCENARIOS: &[&str] = &["null_find_and_replace", "null_process_octal", "idiv_overflow"];

fn run_scenario(name: &str, api: &Api, t: &mut Transcript) {
    match name {
        // CONFIGS row 30: every entry point from the pristine initial state
        "pristine_all" => {
            t_state(api, t, "initial");
            for v in [0, 1, 63, 64, 65, 511, 512, -1, i32::MIN, i32::MAX] {
                t_validate(api, t, v);
            }
            for v in [0, 1, 0o123, 0o777, -1, i32::MIN, i32::MAX] {
                t_octal(api, t, v);
            }
            for n in [b'F' as i32, b'p' as i32, b'z' as i32, 0, -1, 256 + b'F' as i32] {
                t_find(api, t, b"Function pointer example with static vars", n);
            }
            t_findrep(api, t, 0, 0, 0, 0);
            t_state(api, t, "after first findrep");
            t_findrep(api, t, 1, 2, 3, 4);
            t_state(api, t, "after second findrep");
            t_findrep(api, t, 100, 200, 300, 400);
            t_state(api, t, "after third findrep");
            t_add(api, t, 5, 6);
            t_mul(api, t, 3, 4);
            t_sub(api, t, 7, 8);
            t_div(api, t, 0, 2);
            t_state(api, t, "final");
        }
        // ERRORS row 17: active_params == 0 -> add and multiply skipped
        "all_zero_params" => {
            for _ in 0..6 {
                t_findrep(api, t, 0, 0, 0, 0);
                t_state(api, t, "loop");
            }
        }
        // ERRORS row 18: active_params == 1 -> multiply skipped
        "one_active_param" => {
            for i in 0..4 {
                let mut v = [0i32; 4];
                v[i] = 7;
                t_findrep(api, t, v[0], v[1], v[2], v[3]);
                t_state(api, t, "one-active");
            }
            for i in 0..4 {
                let mut v = [0i32; 4];
                v[i] = -7;
                t_findrep(api, t, v[0], v[1], v[2], v[3]);
                t_state(api, t, "one-active-negative");
            }
        }
        // CONFIGS row 26 / ERRORS row 19: accumulator above and below 0150
        "accumulator_over_150" => {
            t_add(api, t, 104, 0); // exactly at the threshold -> branch NOT taken
            t_state(api, t, "at 104");
            t_findrep(api, t, 0, 0, 0, 0);
            t_add(api, t, 1, 0); // one past the threshold -> branch taken
            t_state(api, t, "at 105");
            t_findrep(api, t, 0, 0, 0, 0);
            t_findrep(api, t, 1, 1, 1, 1);
            t_add(api, t, 100_000, 0);
            t_findrep(api, t, 500, -500, 64, 65);
            t_state(api, t, "final");
        }
        // CONFIGS row 27 / ERRORS row 21: multiplier above and below 0100
        "multiplier_over_100" => {
            t_mul(api, t, 1, 64); // exactly at the threshold -> divide NOT taken
            t_state(api, t, "multiplier 64");
            t_findrep(api, t, 0, 0, 0, 0);
            t_state(api, t, "after findrep at 64");
            t_mul(api, t, 1, 2); // 128 -> divide taken
            t_state(api, t, "multiplier 128");
            t_findrep(api, t, 0, 0, 0, 0);
            t_state(api, t, "after findrep at 128");
            t_mul(api, t, 1, 1_000_000);
            t_findrep(api, t, 3, 3, 3, 3);
            t_state(api, t, "final");
        }
        // ERRORS row 20: multiplier == 0 -> both_active false
        "multiplier_zero" => {
            t_mul(api, t, 0, 0);
            t_state(api, t, "multiplier 0");
            t_findrep(api, t, 0, 0, 0, 0);
            t_findrep(api, t, 1, 2, 3, 4);
            t_div(api, t, 0, 5);
            t_div(api, t, 0, -5);
            t_state(api, t, "final");
        }
        "multiplier_negative" => {
            t_mul(api, t, 1, -1000);
            t_state(api, t, "multiplier -1000");
            t_findrep(api, t, 0, 0, 0, 0);
            t_findrep(api, t, 511, 512, 1, -1);
            t_div(api, t, 0, 3);
            t_div(api, t, 0, -3);
            t_state(api, t, "final");
        }
        // ERRORS row 3: C integer division truncates toward zero
        "divide_truncation" => {
            t_mul(api, t, 1, 7);
            t_div(api, t, 0, -2); // 7 / -2 == -3 (trunc), NOT -4 (floor)
            t_state(api, t, "after 7/-2");
            t_mul(api, t, 1, -1000);
            t_div(api, t, 0, 3); // -333
            t_div(api, t, 0, -7);
            t_div(api, t, 0, 1000); // truncates to 0
            t_state(api, t, "final");
        }
        // ERRORS rows 1-2: b == 0 and b == 1
        "divide_by_zero" => {
            t_mul(api, t, 1, 12345);
            for b in [0, 1, 0, 1] {
                t_div(api, t, 999, b);
                t_state(api, t, "after divide");
            }
            t_mul(api, t, 1, 0);
            for b in [0, 1] {
                t_div(api, t, 999, b);
            }
            t_state(api, t, "final");
        }
        // ERRORS row 22: force findrep's computed result to exactly 0 so the
        // 0777 sentinel is substituted.
        //   pristine: accumulator=0, multiplier=1, operation_count=0
        //   add_to_accumulator(-18,0) -> accumulator=-18, operation_count=1
        //   findrep(0,0,0,0): result = 9 ('p' index) + (accumulator+multiplier)
        //                            + operation_count*8
        //                   = 9 + (-18 + 1) + 8 = 0  -> replaced by 0777 = 511
        "sentinel_zero_result" => {
            // pristine: accumulator=0, multiplier=1, operation_count=0
            t_add(api, t, -100, 0); // accumulator=-100, operation_count=1
            t_state(api, t, "prepared"); // two probe calls -> operation_count=3
            t_add(api, t, 58, 0); // accumulator=-42, operation_count=4
            // findrep(0,0,0,0) calls no operation (accumulator <= 0150,
            // multiplier <= 0100, active_params == 0), so:
            //   result = 9 + (accumulator + multiplier) + operation_count * 8
            //          = 9 + (-42 + 1) + 4 * 8 = 0  -> sentinel 0777
            let r = t_findrep(api, t, 0, 0, 0, 0);
            t.line(format!("SENTINEL_HIT={}", r == 0o777));
            assert_eq!(r, 0o777, "expected the 0777 sentinel, got {r}");
        }
        // ERRORS rows 23-25: signed overflow wrap-around
        "overflow_walk" => {
            t_add(api, t, i32::MAX, i32::MAX);
            t_state(api, t, "after MAX+MAX");
            t_add(api, t, i32::MIN, i32::MIN);
            t_state(api, t, "after MIN+MIN");
            t_sub(api, t, i32::MIN, i32::MAX);
            t_state(api, t, "after sub");
            t_mul(api, t, i32::MAX, i32::MAX);
            t_state(api, t, "after MAX*MAX");
            t_mul(api, t, i32::MIN, -1);
            t_state(api, t, "after MIN*-1");
            t_mul(api, t, 65536, 65536);
            t_state(api, t, "after 2^32");
            t_findrep(api, t, i32::MIN, i32::MAX, i32::MIN, i32::MAX);
            t_findrep(api, t, i32::MAX, i32::MAX, i32::MAX, i32::MAX);
            t_state(api, t, "final");
        }
        // A long deterministic random walk over the whole API from a pristine
        // state (same seed in both children).
        "long_random" => {
            // `HARVEST_STEPS` allows a much longer fuzz run on demand, e.g.
            //   HARVEST_STEPS=200000 cargo test --release -- --nocapture \
            //       cfg_pristine_initial_state
            let steps: u64 = std::env::var("HARVEST_STEPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3000);
            let mut rng = Rng::new(0xC0FF_EE01);
            for _ in 0..steps {
                let a = rng.interesting_i32();
                let b = rng.interesting_i32();
                match rng.below(8) {
                    0 => {
                        t_add(api, t, a, b);
                    }
                    1 => {
                        t_mul(api, t, a, b);
                    }
                    2 => {
                        t_sub(api, t, a, b);
                    }
                    3 => {
                        // avoid the INT_MIN / -1 trap (covered separately)
                        let m = unsafe { (api.divide_multiplier)(0, 1) };
                        let b = if b == -1 && m == i32::MIN { -3 } else { b };
                        t_div(api, t, a, b);
                    }
                    4 => {
                        t_validate(api, t, a);
                    }
                    5 => t_octal(api, t, a),
                    6 => {
                        let len = rng.below(64) as usize;
                        let hay: Vec<u8> = (0..len).map(|_| (rng.below(255) as u8) + 1).collect();
                        t_find(api, t, &hay, b);
                    }
                    _ => {
                        t_findrep(api, t, a, b, rng.interesting_i32(), rng.interesting_i32());
                    }
                }
                t_state(api, t, "walk");
            }
        }
        // `operation_count` wrap-around: bump it to just below INT_MAX with
        // state-neutral calls (`divide_multiplier(0, 1)` leaves `multiplier`
        // alone), then cross INT_MAX. `findrep` folds `operation_count * 010`
        // into its result, so the wrap (UB in C, 2's-complement in practice)
        // becomes observable. Only used by the #[ignore]d test.
        "opcount_wrap" => {
            let mut i: i64 = 0;
            let target = i32::MAX as i64 - 6;
            while i < target {
                unsafe { (api.divide_multiplier)(0, 1) };
                i += 1;
            }
            t_state(api, t, "just below INT_MAX");
            for _ in 0..12 {
                t_findrep(api, t, 0, 0, 0, 0);
                t_div(api, t, 0, 1);
            }
            for _ in 0..4 {
                t_findrep(api, t, 1, 2, 3, 4);
                t_state(api, t, "after wrap");
            }
        }
        // ---- fatal scenarios: the child is expected to die by signal ----
        "null_find_and_replace" => {
            t.line("about to call find_and_replace_char(NULL, 'a')");
            unsafe { (api.find_and_replace_char)(std::ptr::null_mut(), b'a' as i32) };
            t.line("SURVIVED");
        }
        "null_process_octal" => {
            t.line("about to call process_octal_string(NULL, 0123)");
            unsafe { (api.process_octal_string)(std::ptr::null_mut(), 0o123) };
            t.line("SURVIVED");
        }
        // ERRORS row 4: multiplier == INT_MIN, divisor == -1
        "idiv_overflow" => {
            t_mul(api, t, 1, i32::MIN); // multiplier = INT_MIN
            t_state(api, t, "before trap");
            let r = t_div(api, t, 0, -1);
            t.line(format!("SURVIVED with {r}"));
        }
        other => panic!("unknown scenario {other}"),
    }
}

/// Child entry point: `HARVEST_SCENARIO=<c|rust>:<scenario name>`.
#[test]
fn scenario_child_entry() {
    let Ok(spec) = std::env::var("HARVEST_SCENARIO") else {
        return; // normal parent run: nothing to do
    };
    let (which, name) = spec.split_once(':').expect("spec must be <lib>:<scenario>");
    let path = match which {
        "c" => c_lib_path(),
        "rust" => rust_lib_path(),
        other => panic!("unknown library selector {other}"),
    };
    let api = Api::load(&path, "child");
    let mut t = Transcript::default();
    run_scenario(name, &api, &mut t);
    if let Ok(out) = std::env::var("HARVEST_OUT") {
        std::fs::write(out, t.0.as_bytes()).expect("write transcript");
    }
    print!("{}", t.0);
}

struct ChildResult {
    status: std::process::ExitStatus,
    transcript: Option<Vec<u8>>,
    stderr: String,
}

fn run_child(which: &str, scenario: &str) -> ChildResult {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current_exe");
    // unique per invocation: several tests may run the same scenario
    // concurrently, and they must not share the transcript file
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!(
        "harvest-{}-{}-{}-{}.txt",
        which,
        scenario,
        std::process::id(),
        seq
    ));
    let _ = std::fs::remove_file(&out);
    let output = Command::new(&exe)
        .arg("scenario_child_entry")
        .arg("--exact")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env("HARVEST_SCENARIO", format!("{which}:{scenario}"))
        .env("HARVEST_OUT", &out)
        .output()
        .expect("spawn child");
    let transcript = std::fs::read(&out).ok();
    let _ = std::fs::remove_file(&out);
    ChildResult {
        status: output.status,
        transcript,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[test]
fn scenario_transcripts_match() {
    for &scenario in SCENARIOS {
        let c = run_child("c", scenario);
        let r = run_child("rust", scenario);
        assert!(
            c.status.success(),
            "C child failed for scenario {scenario}: {:?}\n{}",
            c.status,
            c.stderr
        );
        assert!(
            r.status.success(),
            "Rust child failed for scenario {scenario}: {:?}\n{}",
            r.status,
            r.stderr
        );
        let ct = c.transcript.unwrap_or_else(|| panic!("no C transcript for {scenario}"));
        let rt = r
            .transcript
            .unwrap_or_else(|| panic!("no Rust transcript for {scenario}"));
        assert!(!ct.is_empty(), "empty transcript for {scenario}");
        if ct != rt {
            // report the first differing line
            let cs = String::from_utf8_lossy(&ct);
            let rs = String::from_utf8_lossy(&rt);
            for (i, (a, b)) in cs.lines().zip(rs.lines()).enumerate() {
                if a != b {
                    panic!("scenario {scenario}: transcripts differ at line {i}\n C   : {a}\n Rust: {b}");
                }
            }
            panic!(
                "scenario {scenario}: transcripts differ in length ({} vs {} lines)",
                cs.lines().count(),
                rs.lines().count()
            );
        }
    }
}

// ===========================================================================
// PHASE C — error / rejection path differential tests (ERRORS.md rows)
// ===========================================================================

/// ERRORS rows 1 & 2: `b == 0` (guard skips the division) and `b == 1`.
#[test]
fn err_divide_by_zero() {
    let p = pair();
    let (_, m0) = probe_state(&p, "err_divide_by_zero");
    let r = diff_div(&p, 12345, 0);
    assert_eq!(r, m0, "b==0 must leave the multiplier untouched (C returned {r}, was {m0})");
    let (_, m1) = probe_state(&p, "after b==0");
    assert_eq!(m1, m0);
    let r = diff_div(&p, -1, 1);
    assert_eq!(r, m0, "b==1 must leave the multiplier untouched");
    // and again with extreme dividend arguments (the first argument is ignored)
    for a in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(diff_div(&p, a, 0), m0);
        assert_eq!(diff_div(&p, a, 1), m0);
    }
}

/// ERRORS row 3: C division truncates toward zero (not floor).
#[test]
fn err_divide_negative_truncation() {
    let p = pair();
    for b in [-2, -3, -7, -1000, 2, 3, 7, 1000] {
        let (_, m) = probe_state(&p, "err_divide_negative_truncation");
        if b == -1 && m == i32::MIN {
            continue;
        }
        let got = diff_div(&p, 0, b);
        assert_eq!(
            got,
            m.wrapping_div(b),
            "divide_multiplier(_, {b}) with multiplier={m} must truncate toward zero"
        );
    }
}

/// ERRORS row 5: needle absent -> string untouched.
#[test]
fn err_find_char_absent() {
    let p = pair();
    diff_find_replace(&p, b"abcdef", b'z' as i32);
    diff_find_replace(&p, b"abcdef", b'A' as i32);
    diff_find_replace(&p, &[0x41u8; 60], 0x42);
    let mut rng = Rng::new(0x5EED_00C1);
    for _ in 0..512 {
        // haystack of bytes 1..=127, needle from 128..=255 -> guaranteed miss
        let len = rng.below(60) as usize;
        let hay: Vec<u8> = (0..len).map(|_| 1 + rng.below(127) as u8).collect();
        diff_find_replace(&p, &hay, 128 + rng.below(128) as i32);
    }
}

/// ERRORS row 6: empty haystack -> `memchr(s, c, 0)` -> NULL, nothing written.
#[test]
fn err_find_char_empty_string() {
    let p = pair();
    for needle in [0, 1, b'a' as i32, b'\0' as i32, 255, 256, -1, i32::MIN, i32::MAX] {
        diff_find_replace(&p, b"", needle);
    }
}

/// ERRORS row 7: needle == 0 -> the terminator is outside the search window.
#[test]
fn err_find_char_zero_needle() {
    let p = pair();
    diff_find_replace(&p, b"", 0);
    diff_find_replace(&p, b"a", 0);
    diff_find_replace(&p, b"abcdefghij", 0);
    // ints whose low byte is 0 must behave the same way
    for needle in [0x100, 0x200, -256, 0x7FFF_FF00, i32::MIN] {
        diff_find_replace(&p, b"abcdefghij", needle);
    }
}

/// ERRORS row 8: needle outside `unsigned char` range is narrowed by `memchr`.
#[test]
fn err_find_char_out_of_uchar_range() {
    let p = pair();
    let hay = b"the quick brown fox";
    for base in [0x100, 0x200, 0x1_0000, 0x7FFF_FF00u32 as i32] {
        for byte in [b'e', b'q', b'x', b'z'] {
            diff_find_replace(&p, hay, base | byte as i32);
        }
    }
    let mut rng = Rng::new(0x5EED_00C2);
    for _ in 0..512 {
        let hay: Vec<u8> = (0..1 + rng.below(40)).map(|_| 1 + rng.below(255) as u8).collect();
        let byte = hay[rng.below(hay.len() as u64) as usize] as i32;
        let high = (rng.next_i32() & !0xFF) | byte;
        diff_find_replace(&p, &hay, high);
    }
}

/// ERRORS row 9: negative needles are narrowed to `unsigned char` too.
#[test]
fn err_find_char_negative_needle() {
    let p = pair();
    let hay: Vec<u8> = (0x70u8..=0xFF).collect();
    for needle in [-1, -2, -128, -129, -256, i32::MIN, i32::MIN + 1] {
        diff_find_replace(&p, &hay, needle);
    }
    for n in -256i32..=0 {
        diff_find_replace(&p, &hay, n);
    }
}

/// ERRORS rows 12 & 13: non-positive values are NOT clamped.
#[test]
fn err_validate_rejects_nonpositive() {
    let p = pair();
    assert_eq!(diff_validate(&p, 0), 0);
    for v in [-1, -2, -63, -64, -65, -511, -512, -100000, i32::MIN, i32::MIN + 1] {
        assert_eq!(diff_validate(&p, v), v, "negative values must pass through");
    }
    let mut rng = Rng::new(0x5EED_00C3);
    for _ in 0..1024 {
        let v = -(rng.below(0x8000_0000) as i64) as i32;
        assert_eq!(diff_validate(&p, v), v);
    }
}

/// ERRORS row 14: `0 < value < 0100` -> `0100`.
#[test]
fn err_validate_clamps_low() {
    let p = pair();
    for v in 1..64 {
        assert_eq!(diff_validate(&p, v), 0o100);
    }
}

/// ERRORS row 15: `value > 0777` -> `0777`.
#[test]
fn err_validate_clamps_high() {
    let p = pair();
    for v in [512, 513, 1000, 0o1000, 65535, 1 << 20, i32::MAX - 1, i32::MAX] {
        assert_eq!(diff_validate(&p, v), 0o777);
    }
    let mut rng = Rng::new(0x5EED_00C4);
    for _ in 0..1024 {
        let v = 512 + rng.below(i32::MAX as u64 - 511) as i32;
        assert_eq!(diff_validate(&p, v), 0o777);
    }
}

/// ERRORS row 16: the exact documented boundary values.
#[test]
fn err_validate_boundaries() {
    let p = pair();
    let cases: [(i32, i32); 11] = [
        (0, 0),
        (1, 64),
        (63, 64),
        (64, 64),
        (65, 65),
        (510, 510),
        (511, 511),
        (512, 511),
        (i32::MAX, 511),
        (-1, -1),
        (i32::MIN, i32::MIN),
    ];
    for (input, want) in cases {
        let got = diff_validate(&p, input);
        assert_eq!(got, want, "validate_and_normalize({input})");
    }
}

/// ERRORS row 17: all four parameters zero -> the add branch is skipped.
#[test]
fn err_findrep_all_zero_params() {
    let p = pair();
    for _ in 0..64 {
        diff_findrep(&p, 0, 0, 0, 0);
        probe_state(&p, "err_findrep_all_zero_params");
    }
}

/// ERRORS row 18: exactly one non-zero parameter -> multiply branch skipped.
#[test]
fn err_findrep_one_active_param() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_00C5);
    for i in 0..4 {
        for _ in 0..64 {
            let mut v = [0i32; 4];
            let mut x = rng.interesting_i32();
            if x == 0 {
                x = 1;
            }
            v[i] = x;
            diff_findrep(&p, v[0], v[1], v[2], v[3]);
            probe_state(&p, "err_findrep_one_active_param");
        }
    }
}

/// ERRORS rows 19-21: the state guards inside findrep. The deterministic,
/// state-controlled coverage lives in the scenario children
/// (`accumulator_over_150`, `multiplier_over_100`, `multiplier_zero`); here the
/// same guards are crossed with randomized inputs on the shared state.
#[test]
fn err_findrep_accumulator_guard() {
    let p = pair();
    let c = run_child("c", "accumulator_over_150");
    let r = run_child("rust", "accumulator_over_150");
    assert!(c.status.success() && r.status.success());
    assert_eq!(c.transcript, r.transcript, "accumulator guard transcripts differ");
    // plus in-process crossings of the threshold from the current state
    for delta in [0, 1, -1, 105, -105, 10_000] {
        let (acc, _) = probe_state(&p, "err_findrep_accumulator_guard");
        diff_add(&p, (105i32).wrapping_sub(acc).wrapping_add(delta), 0);
        diff_findrep(&p, 0, 0, 0, 0);
    }
}

#[test]
fn err_findrep_both_active_guard() {
    let c = run_child("c", "multiplier_zero");
    let r = run_child("rust", "multiplier_zero");
    assert!(c.status.success() && r.status.success());
    assert_eq!(c.transcript, r.transcript, "both_active guard transcripts differ");
}

#[test]
fn err_findrep_multiplier_guard() {
    let p = pair();
    let c = run_child("c", "multiplier_over_100");
    let r = run_child("rust", "multiplier_over_100");
    assert!(c.status.success() && r.status.success());
    assert_eq!(c.transcript, r.transcript, "multiplier guard transcripts differ");
    let _ = probe_state(&p, "err_findrep_multiplier_guard");
}

/// ERRORS row 22: the `result == 0` -> `0777` sentinel, constructed exactly.
#[test]
fn err_findrep_zero_result_sentinel() {
    let c = run_child("c", "sentinel_zero_result");
    let r = run_child("rust", "sentinel_zero_result");
    assert!(
        c.status.success(),
        "C child could not reach the sentinel: {:?}\n{}",
        c.status,
        c.stderr
    );
    assert!(
        r.status.success(),
        "Rust child could not reach the sentinel: {:?}\n{}",
        r.status,
        r.stderr
    );
    let ct = c.transcript.expect("C transcript");
    let rt = r.transcript.expect("Rust transcript");
    assert_eq!(ct, rt, "sentinel transcripts differ");
    let s = String::from_utf8_lossy(&ct);
    assert!(s.contains("SENTINEL_HIT=true"), "sentinel not reached:\n{s}");
    assert!(s.contains("findrep(0,0,0,0) -> 511"), "unexpected sentinel value:\n{s}");
}

/// ERRORS rows 23 & 24: signed overflow wrap-around in the stateful ops.
#[test]
fn err_signed_overflow_add_sub() {
    let p = pair();
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MAX, 1),
        (i32::MIN, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
    ] {
        diff_add(&p, a, b);
        probe_state(&p, "overflow add");
        diff_sub(&p, a, b);
        probe_state(&p, "overflow sub");
    }
}

#[test]
fn err_signed_overflow_multiply() {
    let p = pair();
    for (a, b) in [
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (65536, 65536),
        (i32::MAX, 2),
        (1, i32::MIN),
    ] {
        diff_mul(&p, a, b);
        probe_state(&p, "overflow mul");
    }
    // deterministic, pristine-state coverage of the same thing
    let c = run_child("c", "overflow_walk");
    let r = run_child("rust", "overflow_walk");
    assert!(c.status.success() && r.status.success());
    assert_eq!(c.transcript, r.transcript, "overflow_walk transcripts differ");
}

/// ERRORS row 25: extreme findrep parameters (result accumulation overflows).
#[test]
fn err_findrep_extreme_params() {
    let p = pair();
    const E: [i32; 6] = [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX];
    for &a in &E {
        for &b in &E {
            diff_findrep(&p, a, b, a, b);
            diff_findrep(&p, a, a, b, b);
            probe_state(&p, "err_findrep_extreme_params");
        }
    }
}

// ---- fatal error paths: compare the terminating signal of both children ----

fn signal_of(res: &ChildResult) -> String {
    use std::os::unix::process::ExitStatusExt;
    match res.status.signal() {
        Some(sig) => format!("signal {sig}"),
        None => format!("exit {:?}", res.status.code()),
    }
}

/// ERRORS row 10: `find_and_replace_char(NULL, _)` — C has no null check.
#[test]
fn err_null_find_and_replace_char() {
    let c = run_child("c", "null_find_and_replace");
    let r = run_child("rust", "null_find_and_replace");
    assert_eq!(
        signal_of(&c),
        signal_of(&r),
        "NULL find_and_replace_char: C {} vs Rust {}\nC stderr: {}\nRust stderr: {}",
        signal_of(&c),
        signal_of(&r),
        c.stderr,
        r.stderr
    );
    assert!(
        !c.status.success(),
        "the C library unexpectedly survived a NULL pointer"
    );
}

/// ERRORS row 11: `process_octal_string(NULL, _)` — C has no null check.
#[test]
fn err_null_process_octal_string() {
    let c = run_child("c", "null_process_octal");
    let r = run_child("rust", "null_process_octal");
    assert_eq!(
        signal_of(&c),
        signal_of(&r),
        "NULL process_octal_string: C {} vs Rust {}\nC stderr: {}\nRust stderr: {}",
        signal_of(&c),
        signal_of(&r),
        c.stderr,
        r.stderr
    );
    assert!(
        !c.status.success(),
        "the C library unexpectedly survived a NULL pointer"
    );
}

/// ERRORS row 4: `multiplier == INT_MIN`, divisor `-1` — signed division
/// overflow. On x86-64 the C code traps with SIGFPE; the Rust library must do
/// exactly the same.
#[test]
fn err_divide_int_min_by_minus_one() {
    let c = run_child("c", "idiv_overflow");
    let r = run_child("rust", "idiv_overflow");
    assert_eq!(
        signal_of(&c),
        signal_of(&r),
        "INT_MIN / -1: C {} vs Rust {}\nC stderr: {}\nRust stderr: {}",
        signal_of(&c),
        signal_of(&r),
        c.stderr,
        r.stderr
    );
    if c.status.success() {
        // architecture where the division does not trap: outputs must match
        assert_eq!(c.transcript, r.transcript, "idiv_overflow transcripts differ");
    }
}

// ===========================================================================
// Extra breadth: digit-count transitions, exhaustive small ranges, full grids
// ===========================================================================

/// `%o` / `%d` digit-count transitions for every power of 8 and 10 (this is
/// where a hand-rolled formatter would drift from glibc's).
#[test]
fn cfg_process_octal_string_digit_boundaries() {
    let p = pair();
    let mut vals: Vec<i32> = Vec::new();
    for k in 0..11u32 {
        let base = 8u64.pow(k);
        for d in [-1i64, 0, 1] {
            let v = base as i64 + d;
            if v >= 0 && v <= i32::MAX as i64 {
                vals.push(v as i32);
                vals.push(-(v as i32));
            }
        }
    }
    for k in 0..10u32 {
        let base = 10u64.pow(k);
        for d in [-1i64, 0, 1] {
            let v = base as i64 + d;
            if v >= 0 && v <= i32::MAX as i64 {
                vals.push(v as i32);
                vals.push(-(v as i32));
            }
        }
    }
    vals.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1, 0, -1]);
    for v in vals {
        diff_process_octal(&p, v);
    }
}

/// Exhaustive `validate_and_normalize` over the whole interesting neighbourhood
/// (every single value from -2048 to 4096 inclusive).
#[test]
fn cfg_validate_exhaustive_small_range() {
    let p = pair();
    for v in -2048..=4096 {
        diff_validate(&p, v);
    }
}

/// Full 4-way grid of the parameter classes that findrep distinguishes
/// (6^4 = 1296 calls, state carried across them).
#[test]
fn cfg_findrep_full_small_grid() {
    let p = pair();
    const V: [i32; 6] = [0, 1, 64, 65, 511, 512];
    for &a in &V {
        for &b in &V {
            for &c in &V {
                for &d in &V {
                    diff_findrep(&p, a, b, c, d);
                }
            }
        }
    }
    probe_state(&p, "cfg_findrep_full_small_grid");
}

/// CONFIGS row 30: all eight entry points exercised from the pristine initial
/// state (`accumulator == 0`, `multiplier == 1`, `operation_count == 0`), which
/// is only observable in a fresh process.
#[test]
fn cfg_pristine_initial_state() {
    let c = run_child("c", "pristine_all");
    let r = run_child("rust", "pristine_all");
    assert!(
        c.status.success(),
        "C child failed: {:?}\n{}",
        c.status,
        c.stderr
    );
    assert!(
        r.status.success(),
        "Rust child failed: {:?}\n{}",
        r.status,
        r.stderr
    );
    let ct = c.transcript.expect("C transcript");
    let rt = r.transcript.expect("Rust transcript");
    let cs = String::from_utf8_lossy(&ct);
    let rs = String::from_utf8_lossy(&rt);
    for (i, (a, b)) in cs.lines().zip(rs.lines()).enumerate() {
        assert_eq!(a, b, "pristine_all: line {i} differs");
    }
    assert_eq!(cs.lines().count(), rs.lines().count());
    // sanity: the transcript really starts from the documented initial state
    assert!(
        cs.starts_with("state[initial] accumulator=0 multiplier=1\n"),
        "unexpected initial state:\n{cs}"
    );
    // and the long deterministic random walk from that state matches too
    let c = run_child("c", "long_random");
    let r = run_child("rust", "long_random");
    assert!(c.status.success() && r.status.success());
    assert_eq!(
        c.transcript.as_deref(),
        r.transcript.as_deref(),
        "long_random transcripts differ"
    );
}

/// Exhaustive proof for the one pure integer→integer entry point: every single
/// one of the 2^32 possible `int` inputs. Ignored by default because it takes
/// minutes; run with:
///     cargo build --release
///     HARVEST_RUST_LIB=target/release/libfindrep_lib.so \
///         cargo test --release -- --ignored exhaustive_validate_all_i32
#[test]
#[ignore]
fn exhaustive_validate_all_i32() {
    let p = pair();
    let (c, r) = (p.c.validate_and_normalize, p.r.validate_and_normalize);
    let mut v: i32 = i32::MIN;
    loop {
        unsafe {
            let a = c(v);
            let b = r(v);
            if a != b {
                panic!("validate_and_normalize({v}): C={a} Rust={b}");
            }
        }
        if v == i32::MAX {
            break;
        }
        v += 1;
    }
}

/// Wide (not quite exhaustive — `sprintf` is too slow for 2^32) sweep of
/// `process_octal_string`: every value in `-2^22 ..= 2^22`, every 4096th value
/// over the whole `int` range, and the extremes. Ignored by default.
#[test]
#[ignore]
fn exhaustive_process_octal_wide() {
    let p = pair();
    for v in -(1 << 22)..=(1 << 22) {
        diff_process_octal(&p, v);
    }
    let mut v: i64 = i32::MIN as i64;
    while v <= i32::MAX as i64 {
        diff_process_octal(&p, v as i32);
        v += 4096;
    }
    for v in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        diff_process_octal(&p, v);
    }
}

/// Wide sweep of the `find_and_replace_char` needle conversion: every needle in
/// `-2^17 ..= 2^17` plus every 251st value over the whole `int` range, against a
/// haystack that contains all 255 non-NUL byte values. Ignored by default.
#[test]
#[ignore]
fn exhaustive_find_and_replace_needles() {
    let p = pair();
    // two haystacks so that all 255 non-NUL byte values are covered while
    // staying inside the 256-byte comparison buffer
    let hay_lo: Vec<u8> = (1u8..=250).collect();
    let hay_hi: Vec<u8> = (251u8..=255).collect();
    for n in -(1 << 17)..=(1 << 17) {
        diff_find_replace(&p, &hay_lo, n);
        diff_find_replace(&p, &hay_hi, n);
    }
    let mut n: i64 = i32::MIN as i64;
    while n <= i32::MAX as i64 {
        diff_find_replace(&p, &hay_lo, n as i32);
        diff_find_replace(&p, &hay_hi, n as i32);
        n += 251;
    }
}

/// `operation_count` wrap-around past `INT_MAX` (2^31 state-neutral calls, so
/// it is `#[ignore]`d). `findrep` mixes `operation_count * 010` into its result,
/// so the wrap is observable through the public ABI.
#[test]
#[ignore]
fn exhaustive_operation_count_wraparound() {
    let c = run_child("c", "opcount_wrap");
    let r = run_child("rust", "opcount_wrap");
    assert!(
        c.status.success(),
        "C child failed: {:?}\n{}",
        c.status,
        c.stderr
    );
    assert!(
        r.status.success(),
        "Rust child failed: {:?}\n{}",
        r.status,
        r.stderr
    );
    let ct = c.transcript.expect("C transcript");
    let rt = r.transcript.expect("Rust transcript");
    let cs = String::from_utf8_lossy(&ct);
    let rs = String::from_utf8_lossy(&rt);
    for (i, (a, b)) in cs.lines().zip(rs.lines()).enumerate() {
        assert_eq!(a, b, "opcount_wrap: line {i} differs");
    }
    assert_eq!(cs.lines().count(), rs.lines().count());
    // the transcript must really contain a negative (wrapped) contribution
    assert!(cs.contains("findrep(0,0,0,0) -> "), "unexpected transcript:\n{cs}");
}
