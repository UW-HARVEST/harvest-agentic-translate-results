//! Area 2, part 2 — `crypto_core/salsa`, `hsalsa20`, `hchacha20`, `keccak1600`.
//!
//! Covers `configs_2.md` rows 2.11 - 2.41.  None of these functions has a
//! rejection branch (see the "total-function note" in `errors_2.md`), so the
//! error surface here is only "no out-of-range behaviour / no OOB write".
mod common;
use common::*;
use std::ffi::{c_int, c_void};

type Core4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;
type Getter = unsafe extern "C" fn() -> usize;

// ---------------------------------------------------------------- shared bits

fn getters(pairs: &[(&str, usize)]) {
    for (name, want) in pairs {
        assert!(has(name), "{name} must be exported by both libraries");
        let (c, r) = both::<Getter>(name);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
        assert_eq!(vc, *want, "{name}: C returned {vc}, header says {want}");
    }
}

/// Drive a `crypto_core_*` primitive on both libraries and compare the output
/// (with out-of-bounds guard bytes) for one `(in, k, c)` triple.
#[track_caller]
fn core_case(
    c: &Core4,
    r: &Core4,
    outlen: usize,
    input: &[u8],
    key: &[u8],
    konst: Option<&[u8]>,
    label: &str,
) -> Vec<u8> {
    let kp = konst.map(|x| x.as_ptr()).unwrap_or(std::ptr::null());
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    let rc = unsafe { c(oc.as_mut_ptr(), input.as_ptr(), key.as_ptr(), kp) };
    let rr = unsafe { r(or.as_mut_ptr(), input.as_ptr(), key.as_ptr(), kp) };
    eqi(&format!("{label} ret"), rc, rr);
    assert_eq!(rc, 0, "{label}: C must always return 0");
    eqb(&format!("{label} out"), &oc[..outlen], &or[..outlen]);
    check_pad(&format!("{label}(C)"), &oc, outlen);
    check_pad(&format!("{label}(Rust)"), &or, outlen);
    oc[..outlen].to_vec()
}

/// A generic sweep over the `(in, k, c)` shape used by all four core functions.
fn core_family(name: &str, outlen: usize, inlen: usize, keylen: usize, clen: usize, seed: u64) {
    assert!(has(name), "{name} must be exported by both libraries");
    let (c, r) = both::<Core4>(name);
    let mut rng = Rng::new(seed);

    // c == NULL (built-in sigma) and c != NULL (custom constant), all-zero,
    // all-0xff and random inputs.
    let specials: [Vec<u8>; 3] = [vec![0u8; 64], vec![0xffu8; 64], vec![0x80u8; 64]];
    for sk in &specials {
        for si in &specials {
            for kc in [None, Some(vec![0u8; clen]), Some(vec![0xffu8; clen])] {
                core_case(&c, &r, outlen, &si[..inlen], &sk[..keylen], kc.as_deref(), name);
            }
        }
    }
    for _ in 0..1500 {
        let input = rng.bytes(inlen);
        let key = rng.bytes(keylen);
        // c == NULL
        core_case(&c, &r, outlen, &input, &key, None, name);
        // c != NULL, random constant
        let k = rng.bytes(clen);
        core_case(&c, &r, outlen, &input, &key, Some(&k), name);
    }
    // single-bit sweeps: flip every bit of the key, of `in`, and of `c`
    let base_in = rng.bytes(inlen);
    let base_key = rng.bytes(keylen);
    let base_c = rng.bytes(clen);
    for i in 0..keylen * 8 {
        let mut k = base_key.clone();
        k[i / 8] ^= 1 << (i % 8);
        core_case(&c, &r, outlen, &base_in, &k, None, name);
        core_case(&c, &r, outlen, &base_in, &k, Some(&base_c), name);
    }
    for i in 0..inlen * 8 {
        let mut v = base_in.clone();
        v[i / 8] ^= 1 << (i % 8);
        core_case(&c, &r, outlen, &v, &base_key, None, name);
    }
    for i in 0..clen * 8 {
        let mut v = base_c.clone();
        v[i / 8] ^= 1 << (i % 8);
        core_case(&c, &r, outlen, &base_in, &base_key, Some(&v), name);
    }
}

// -------------------------------------------------------------------- salsa20

/// Rows 2.11 - 2.13 (`salsa20`), 2.14/2.15 (`salsa2012`), 2.16/2.17 (`salsa208`).
#[test]
fn salsa20_all_configs() {
    core_family("crypto_core_salsa20", 64, 16, 32, 16, 0x2_0011);
}

#[test]
fn salsa2012_all_configs() {
    core_family("crypto_core_salsa2012", 64, 16, 32, 16, 0x2_0014);
}

#[test]
fn salsa208_all_configs() {
    core_family("crypto_core_salsa208", 64, 16, 32, 16, 0x2_0016);
}

/// Row 2.13 / 2.22 / 2.26 — the canonical all-zero vectors, checked explicitly.
#[test]
fn salsa_family_zero_vectors() {
    for name in [
        "crypto_core_salsa20",
        "crypto_core_salsa2012",
        "crypto_core_salsa208",
    ] {
        let (c, r) = both::<Core4>(name);
        let z16 = [0u8; 16];
        let z32 = [0u8; 32];
        core_case(&c, &r, 64, &z16, &z32, None, name);
        core_case(&c, &r, 64, &z16, &z32, Some(&z16), name);
    }
}

/// Row 2.18 — the only difference between the three families is the round
/// count, so identical inputs must give three *different* outputs.
#[test]
fn salsa_round_counts_differ() {
    let (c20, r20) = both::<Core4>("crypto_core_salsa20");
    let (c12, r12) = both::<Core4>("crypto_core_salsa2012");
    let (c8, r8) = both::<Core4>("crypto_core_salsa208");
    let mut rng = Rng::new(0x2_0018);
    for _ in 0..300 {
        let input = rng.bytes(16);
        let key = rng.bytes(32);
        let konst = rng.bytes(16);
        for kc in [None, Some(&konst[..])] {
            let o20 = core_case(&c20, &r20, 64, &input, &key, kc, "salsa20");
            let o12 = core_case(&c12, &r12, 64, &input, &key, kc, "salsa2012");
            let o8 = core_case(&c8, &r8, 64, &input, &key, kc, "salsa208");
            assert_ne!(o20, o12, "salsa20 and salsa2012 must differ");
            assert_ne!(o20, o8, "salsa20 and salsa208 must differ");
            assert_ne!(o12, o8, "salsa2012 and salsa208 must differ");
        }
    }
}

/// Row 2.19 — the twelve salsa constant getters.
#[test]
fn salsa_getters() {
    getters(&[
        ("crypto_core_salsa20_outputbytes", 64),
        ("crypto_core_salsa20_inputbytes", 16),
        ("crypto_core_salsa20_keybytes", 32),
        ("crypto_core_salsa20_constbytes", 16),
        ("crypto_core_salsa2012_outputbytes", 64),
        ("crypto_core_salsa2012_inputbytes", 16),
        ("crypto_core_salsa2012_keybytes", 32),
        ("crypto_core_salsa2012_constbytes", 16),
        ("crypto_core_salsa208_outputbytes", 64),
        ("crypto_core_salsa208_inputbytes", 16),
        ("crypto_core_salsa208_keybytes", 32),
        ("crypto_core_salsa208_constbytes", 16),
    ]);
}

// ------------------------------------------------------------------- hsalsa20

/// Rows 2.20 - 2.22.
#[test]
fn hsalsa20_all_configs() {
    core_family("crypto_core_hsalsa20", 32, 16, 32, 16, 0x2_0020);
}

/// Row 2.23.
#[test]
fn hsalsa20_getters() {
    getters(&[
        ("crypto_core_hsalsa20_outputbytes", 32),
        ("crypto_core_hsalsa20_inputbytes", 16),
        ("crypto_core_hsalsa20_keybytes", 32),
        ("crypto_core_hsalsa20_constbytes", 16),
    ]);
}

// ------------------------------------------------------------------ hchacha20

/// Rows 2.24 - 2.26.
#[test]
fn hchacha20_all_configs() {
    core_family("crypto_core_hchacha20", 32, 16, 32, 16, 0x2_0024);
}

/// Row 2.27.
#[test]
fn hchacha20_getters() {
    getters(&[
        ("crypto_core_hchacha20_outputbytes", 32),
        ("crypto_core_hchacha20_inputbytes", 16),
        ("crypto_core_hchacha20_keybytes", 32),
        ("crypto_core_hchacha20_constbytes", 16),
    ]);
}

/// hsalsa20 and hchacha20 both produce 32 bytes from the same shapes but are
/// different permutations, so they must never agree.
#[test]
fn hsalsa20_and_hchacha20_differ() {
    let (cs, rs) = both::<Core4>("crypto_core_hsalsa20");
    let (cc, rc) = both::<Core4>("crypto_core_hchacha20");
    let mut rng = Rng::new(0x2_0026);
    for _ in 0..300 {
        let input = rng.bytes(16);
        let key = rng.bytes(32);
        let a = core_case(&cs, &rs, 32, &input, &key, None, "hsalsa20");
        let b = core_case(&cc, &rc, 32, &input, &key, None, "hchacha20");
        assert_ne!(a, b);
    }
}

// ----------------------------------------------------------------- keccak1600

const KECCAK_STATEBYTES: usize = 200;
const KECCAK_STRUCTBYTES: usize = 224;

#[repr(C, align(16))]
struct KState([u8; KECCAK_STRUCTBYTES + PAD]);

impl KState {
    fn new() -> Box<KState> {
        let mut s = Box::new(KState([0u8; KECCAK_STRUCTBYTES + PAD]));
        for (i, b) in s.0[KECCAK_STRUCTBYTES..].iter_mut().enumerate() {
            *b = 0xA5u8.wrapping_add(i as u8);
        }
        s
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    fn body(&self) -> &[u8] {
        &self.0[..KECCAK_STRUCTBYTES]
    }
}

type KInit = unsafe extern "C" fn(*mut c_void);
type KPermute = unsafe extern "C" fn(*mut c_void);
type KXor = unsafe extern "C" fn(*mut c_void, *const u8, usize, usize);
type KExtract = unsafe extern "C" fn(*const c_void, *mut u8, usize, usize);

struct Keccak {
    init: (libloading::Symbol<'static, KInit>, libloading::Symbol<'static, KInit>),
    p24: (libloading::Symbol<'static, KPermute>, libloading::Symbol<'static, KPermute>),
    p12: (libloading::Symbol<'static, KPermute>, libloading::Symbol<'static, KPermute>),
    xor: (libloading::Symbol<'static, KXor>, libloading::Symbol<'static, KXor>),
    ext: (libloading::Symbol<'static, KExtract>, libloading::Symbol<'static, KExtract>),
}

impl Keccak {
    /// `private` selects the internal `_sodium_keccak1600_ref_*` entry points
    /// instead of the public `crypto_core_keccak1600_*` wrappers.
    fn new(private: bool) -> Self {
        let p = |s: &str, q: &str| -> String {
            if private { format!("_sodium_keccak1600_ref_{s}") } else { format!("crypto_core_keccak1600_{q}") }
        };
        Keccak {
            init: both(&p("init", "init")),
            p24: both(&p("permute_24", "permute_24")),
            p12: both(&p("permute_12", "permute_12")),
            xor: both(&p("xor_bytes", "xor_bytes")),
            ext: both(&p("extract_bytes", "extract_bytes")),
        }
    }
}

/// One differential keccak scenario: a program of operations replayed against
/// both libraries, comparing the whole 224-byte state after each step.
#[derive(Clone, Debug)]
enum Op {
    Init,
    Permute24,
    Permute12,
    Xor(usize, usize, u64), // offset, length, data seed
    Extract(usize, usize),  // offset, length
}

fn run_keccak_program(k: &Keccak, ops: &[Op], label: &str) {
    let mut sc = KState::new();
    let mut sr = KState::new();
    for (step, op) in ops.iter().enumerate() {
        let tag = format!("{label} step{step} {op:?}");
        match *op {
            Op::Init => unsafe {
                (k.init.0)(sc.ptr());
                (k.init.1)(sr.ptr());
            },
            Op::Permute24 => unsafe {
                (k.p24.0)(sc.ptr());
                (k.p24.1)(sr.ptr());
            },
            Op::Permute12 => unsafe {
                (k.p12.0)(sc.ptr());
                (k.p12.1)(sr.ptr());
            },
            Op::Xor(off, len, seed) => {
                let data = Rng::new(seed).bytes(len.max(1));
                unsafe {
                    (k.xor.0)(sc.ptr(), data.as_ptr(), off, len);
                    (k.xor.1)(sr.ptr(), data.as_ptr(), off, len);
                }
            }
            Op::Extract(off, len) => {
                let mut oc = padded(len);
                let mut or = padded(len);
                unsafe {
                    (k.ext.0)(sc.ptr() as *const c_void, oc.as_mut_ptr(), off, len);
                    (k.ext.1)(sr.ptr() as *const c_void, or.as_mut_ptr(), off, len);
                }
                eqb(&format!("{tag} extracted"), &oc[..len], &or[..len]);
                check_pad(&format!("{tag} extract(C)"), &oc, len);
                check_pad(&format!("{tag} extract(Rust)"), &or, len);
                // extract must agree with the state we already compared
                assert_eq!(&oc[..len], &sc.body()[off..off + len], "{tag}: C extract != state");
            }
        }
        eqb(&format!("{tag} state"), sc.body(), sr.body());
        check_pad(&format!("{tag} guard(C)"), &sc.0, KECCAK_STRUCTBYTES);
        check_pad(&format!("{tag} guard(Rust)"), &sr.0, KECCAK_STRUCTBYTES);
    }
}

/// Row 2.28 — `crypto_core_keccak1600_statebytes()`.
#[test]
fn keccak1600_statebytes() {
    getters(&[("crypto_core_keccak1600_statebytes", KECCAK_STRUCTBYTES)]);
    // `init` must only clear the first 200 bytes; the remaining 24 padding
    // bytes of the 224-byte struct are left alone by both implementations.
    let k = Keccak::new(false);
    let mut sc = KState::new();
    let mut sr = KState::new();
    for i in 0..KECCAK_STRUCTBYTES {
        sc.0[i] = 0x3c;
        sr.0[i] = 0x3c;
    }
    unsafe {
        (k.init.0)(sc.ptr());
        (k.init.1)(sr.ptr());
    }
    eqb("keccak1600_init partial clear", sc.body(), sr.body());
    assert!(sc.body()[..KECCAK_STATEBYTES].iter().all(|&b| b == 0));
    assert!(sc.body()[KECCAK_STATEBYTES..].iter().all(|&b| b == 0x3c));
}

/// Rows 2.29 - 2.33 — permutations from the zero state and from non-trivial
/// states, applied repeatedly.
#[test]
fn keccak1600_permutations() {
    for private in [false, true] {
        let k = Keccak::new(private);
        let tag = if private { "ref" } else { "public" };

        // 2.29 / 2.30: all-zero state -> permute_24 / permute_12
        run_keccak_program(&k, &[Op::Init, Op::Permute24, Op::Extract(0, KECCAK_STATEBYTES)], &format!("{tag} zero+p24"));
        run_keccak_program(&k, &[Op::Init, Op::Permute12, Op::Extract(0, KECCAK_STATEBYTES)], &format!("{tag} zero+p12"));

        // 2.31: permute_24 applied repeatedly, state carried across
        let mut ops = vec![Op::Init];
        for _ in 0..8 {
            ops.push(Op::Permute24);
            ops.push(Op::Extract(0, KECCAK_STATEBYTES));
        }
        run_keccak_program(&k, &ops, &format!("{tag} p24 x8"));
        let mut ops = vec![Op::Init];
        for _ in 0..8 {
            ops.push(Op::Permute12);
            ops.push(Op::Extract(0, KECCAK_STATEBYTES));
        }
        run_keccak_program(&k, &ops, &format!("{tag} p12 x8"));

        // 2.32 / 2.33: init -> xor a rate-sized block -> permute
        for rate in [72usize, 104, 136, 144, 168, KECCAK_STATEBYTES] {
            run_keccak_program(
                &k,
                &[Op::Init, Op::Xor(0, rate, 0xabc0 + rate as u64), Op::Permute24, Op::Extract(0, KECCAK_STATEBYTES)],
                &format!("{tag} rate{rate}+p24"),
            );
            run_keccak_program(
                &k,
                &[Op::Init, Op::Xor(0, rate, 0xdef0 + rate as u64), Op::Permute12, Op::Extract(0, KECCAK_STATEBYTES)],
                &format!("{tag} rate{rate}+p12"),
            );
        }
    }
}

/// The 12-round permutation must use a different round-constant window than
/// the 24-round one (row 2.30), so their outputs cannot coincide.
#[test]
fn keccak1600_p12_differs_from_p24() {
    let k = Keccak::new(false);
    let mut a = KState::new();
    let mut b = KState::new();
    unsafe {
        (k.init.0)(a.ptr());
        (k.init.0)(b.ptr());
        (k.p24.0)(a.ptr());
        (k.p12.0)(b.ptr());
    }
    assert_ne!(a.body(), b.body(), "permute_24 and permute_12 must differ");
}

/// Rows 2.34 - 2.38 — every `xor_bytes` loop combination.
#[test]
fn keccak1600_xor_bytes_shapes() {
    for private in [false, true] {
        let k = Keccak::new(private);
        let tag = if private { "ref" } else { "public" };
        let mut cases: Vec<Op> = Vec::new();

        // 2.34: offset 0, length a multiple of 8 (only the 8-byte loop runs)
        for len in [0usize, 8, 16, 72, 104, 136, 144, 168, 192, KECCAK_STATEBYTES] {
            cases.push(Op::Xor(0, len, 0x100 + len as u64));
        }
        // 2.35: unaligned offset with a <8 tail (all three loops run)
        for off in 1..8usize {
            for len in [1usize, 7, 8, 9, 15, 16, 23, 100, 137] {
                if off + len <= KECCAK_STATEBYTES {
                    cases.push(Op::Xor(off, len, 0x200 + (off * 1000 + len) as u64));
                }
            }
        }
        // 2.36: offset 0, 0 < length < 8 (only the trailing loop runs)
        for len in 1..8usize {
            cases.push(Op::Xor(0, len, 0x300 + len as u64));
        }
        // 2.37: length == 0 for every offset (complete no-op)
        for off in 0..=KECCAK_STATEBYTES {
            cases.push(Op::Xor(off, 0, 0x400));
        }
        // 2.38: offset + length == 200 exactly
        for off in 0..=KECCAK_STATEBYTES {
            cases.push(Op::Xor(off, KECCAK_STATEBYTES - off, 0x500 + off as u64));
        }
        // each case on a fresh zero state, and again on a permuted state
        for op in &cases {
            run_keccak_program(&k, &[Op::Init, op.clone(), Op::Extract(0, KECCAK_STATEBYTES)], &format!("{tag} xor-fresh"));
        }
        // and all of them chained onto one evolving state
        let mut ops = vec![Op::Init, Op::Xor(0, 136, 9), Op::Permute24];
        ops.extend(cases.iter().cloned());
        ops.push(Op::Extract(0, KECCAK_STATEBYTES));
        run_keccak_program(&k, &ops, &format!("{tag} xor-chained"));
    }
}

/// Rows 2.39 - 2.41 — every `extract_bytes` shape.
#[test]
fn keccak1600_extract_bytes_shapes() {
    for private in [false, true] {
        let k = Keccak::new(private);
        let tag = if private { "ref" } else { "public" };
        let mut ops = vec![Op::Init, Op::Xor(0, 168, 0x777), Op::Permute24];
        // 2.39: full state
        ops.push(Op::Extract(0, KECCAK_STATEBYTES));
        // 2.41: zero-length extract at every offset
        for off in 0..=KECCAK_STATEBYTES {
            ops.push(Op::Extract(off, 0));
        }
        // 2.40: partial extracts at every offset
        for off in 0..KECCAK_STATEBYTES {
            for len in [1usize, 5, 8, 32, 33] {
                if off + len <= KECCAK_STATEBYTES {
                    ops.push(Op::Extract(off, len));
                }
            }
            ops.push(Op::Extract(off, KECCAK_STATEBYTES - off));
        }
        run_keccak_program(&k, &ops, &format!("{tag} extract"));
    }
}

/// A long randomized program mixing all five operations.
#[test]
fn keccak1600_random_programs() {
    for private in [false, true] {
        let k = Keccak::new(private);
        let mut rng = Rng::new(0x2_0034 + private as u64);
        for prog in 0..40 {
            let mut ops = vec![Op::Init];
            for _ in 0..40 {
                ops.push(match rng.below(5) {
                    0 => Op::Permute24,
                    1 => Op::Permute12,
                    2 | 3 => {
                        let off = rng.below(KECCAK_STATEBYTES + 1);
                        let len = rng.below(KECCAK_STATEBYTES - off + 1);
                        Op::Xor(off, len, rng.next_u64() | 1)
                    }
                    _ => {
                        let off = rng.below(KECCAK_STATEBYTES + 1);
                        let len = rng.below(KECCAK_STATEBYTES - off + 1);
                        Op::Extract(off, len)
                    }
                });
            }
            ops.push(Op::Extract(0, KECCAK_STATEBYTES));
            run_keccak_program(&k, &ops, &format!("prog{prog} private={private}"));
        }
    }
}

/// The public `crypto_core_keccak1600_*` wrappers must be exactly the
/// `keccak1600_ref_*` implementations (the build defines no `__ARM_FEATURE_SHA3`).
#[test]
fn keccak1600_public_equals_ref() {
    let pubk = Keccak::new(false);
    let refk = Keccak::new(true);
    let mut rng = Rng::new(0x2_0041);
    for _ in 0..20 {
        let mut a = KState::new();
        let mut b = KState::new();
        unsafe {
            (pubk.init.0)(a.ptr());
            (refk.init.0)(b.ptr());
        }
        for _ in 0..10 {
            let off = rng.below(KECCAK_STATEBYTES + 1);
            let len = rng.below(KECCAK_STATEBYTES - off + 1);
            let data = rng.bytes(len.max(1));
            unsafe {
                (pubk.xor.0)(a.ptr(), data.as_ptr(), off, len);
                (refk.xor.0)(b.ptr(), data.as_ptr(), off, len);
                if rng.below(2) == 0 {
                    (pubk.p24.0)(a.ptr());
                    (refk.p24.0)(b.ptr());
                } else {
                    (pubk.p12.0)(a.ptr());
                    (refk.p12.0)(b.ptr());
                }
            }
        }
        assert_eq!(a.body(), b.body(), "public keccak1600 != keccak1600_ref");
    }
}
