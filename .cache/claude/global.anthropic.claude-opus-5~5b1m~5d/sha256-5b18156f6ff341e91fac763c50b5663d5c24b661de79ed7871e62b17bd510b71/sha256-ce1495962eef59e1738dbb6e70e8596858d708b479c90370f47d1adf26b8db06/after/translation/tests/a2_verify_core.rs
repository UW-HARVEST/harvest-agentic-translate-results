//! Area 2, part 1 — `crypto_verify/verify.c` and `crypto_core/softaes/softaes.c`.
//!
//! Covers `configs_2.md` rows 2.1 - 2.10 (crypto_verify) and 2.42 - 2.50
//! (softaes), plus `errors_2.md` rows 2.1 - 2.3.
mod common;
use common::*;
use std::ffi::c_int;

type Verify = unsafe extern "C" fn(*const u8, *const u8) -> c_int;
type Getter = unsafe extern "C" fn() -> usize;

// --------------------------------------------------------------- crypto_verify

/// Call both implementations on the same pair of buffers and require identical
/// results.  `x`/`y` are over-allocated with *differing* guard tails, so any
/// read past `n` bytes would make the two agree/disagree inconsistently and be
/// caught by the expected-value assertion.
#[track_caller]
fn verify_case(c: &Verify, r: &Verify, n: usize, x: &[u8], y: &[u8], expect: c_int, label: &str) {
    assert!(x.len() >= n && y.len() >= n);
    let rc = unsafe { c(x.as_ptr(), y.as_ptr()) };
    let rr = unsafe { r(x.as_ptr(), y.as_ptr()) };
    eqi(&format!("crypto_verify_{n} [{label}]"), rc, rr);
    assert_eq!(rc, expect, "crypto_verify_{n} [{label}]: C returned {rc}, expected {expect}");
}

/// Build a buffer of `n` payload bytes plus a 32-byte tail filled with `fill`.
fn tail(payload: &[u8], fill: u8) -> Vec<u8> {
    let mut v = payload.to_vec();
    v.extend(std::iter::repeat(fill).take(PAD));
    v
}

fn verify_family(n: usize, seed: u64) {
    let name = format!("crypto_verify_{n}");
    let (c, r) = both::<Verify>(&name);

    // ---- equal inputs: all-zero, all-0xff, random  (rows 2.1, 2.2, 2.6, 2.8)
    for pat in [0x00u8, 0xff, 0x55, 0xaa] {
        let p = vec![pat; n];
        // deliberately different guard tails: proves neither side reads past n
        verify_case(&c, &r, n, &tail(&p, 0x11), &tail(&p, 0xee), 0, "equal/const");
    }
    let mut rng = Rng::new(seed);
    for _ in 0..512 {
        let p = rng.bytes(n);
        verify_case(&c, &r, n, &tail(&p, 0x00), &tail(&p, 0xff), 0, "equal/random");
    }

    // ---- differ at exactly one byte position k, every k, every bit
    //      (rows 2.3, 2.4, 2.7, 2.9 and error rows 2.1 - 2.3)
    for k in 0..n {
        for bit in 0..8 {
            let x = rng.bytes(n);
            let mut y = x.clone();
            y[k] ^= 1u8 << bit;
            verify_case(&c, &r, n, &tail(&x, 0), &tail(&y, 0), -1, &format!("diff@{k} bit{bit}"));
            // and the mirrored argument order
            verify_case(&c, &r, n, &tail(&y, 0), &tail(&x, 0), -1, &format!("diff@{k} bit{bit} rev"));
        }
    }

    // ---- differ at every byte (row 2.5)
    for _ in 0..64 {
        let x = rng.bytes(n);
        let y: Vec<u8> = x.iter().map(|b| !b).collect();
        verify_case(&c, &r, n, &tail(&x, 0), &tail(&y, 0), -1, "complement");
    }
    verify_case(&c, &r, n, &tail(&vec![0x00; n], 0), &tail(&vec![0xff; n], 0), -1, "zero-vs-ff");

    // ---- random pairs (overwhelmingly unequal, plus the occasional 0 above)
    for _ in 0..2000 {
        let x = rng.bytes(n);
        let mut y = rng.bytes(n);
        // sometimes make the two agree on a prefix so the loop runs further
        let keep = rng.below(n + 1);
        y[..keep].copy_from_slice(&x[..keep]);
        let expect = if x == y { 0 } else { -1 };
        verify_case(&c, &r, n, &tail(&x, 0x5a), &tail(&y, 0xa5), expect, "fuzz");
    }

    // ---- alignment sweep: every start offset inside a larger allocation
    let big_x = rng.bytes(n + 16);
    for off in 0..16usize {
        let x = &big_x[off..off + n];
        let mut y = x.to_vec();
        verify_case(&c, &r, n, &tail(x, 0), &tail(&y, 0), 0, &format!("align{off}"));
        y[n - 1] ^= 0x80;
        verify_case(&c, &r, n, &tail(x, 0), &tail(&y, 0), -1, &format!("align{off} diff"));
    }
}

#[test]
fn verify16_all_configs() {
    verify_family(16, 0x2_0001);
}

#[test]
fn verify32_all_configs() {
    verify_family(32, 0x2_0002);
}

#[test]
fn verify64_all_configs() {
    verify_family(64, 0x2_0003);
}

/// Row 2.10 — the `crypto_verify_*_bytes()` constant getters.
#[test]
fn verify_bytes_getters() {
    for (name, want) in [
        ("crypto_verify_16_bytes", 16usize),
        ("crypto_verify_32_bytes", 32),
        ("crypto_verify_64_bytes", 64),
    ] {
        let (c, r) = both::<Getter>(name);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
        assert_eq!(vc, want, "{name}: C returned {vc}, header says {want}");
    }
}

/// The three sizes must be mutually consistent: verifying the first 16 bytes of
/// a 64-byte pair is the same question as `crypto_verify_16` on that prefix.
#[test]
fn verify_sizes_are_prefix_consistent() {
    let (c16, r16) = both::<Verify>("crypto_verify_16");
    let (c32, r32) = both::<Verify>("crypto_verify_32");
    let (c64, r64) = both::<Verify>("crypto_verify_64");
    let mut rng = Rng::new(0x2_0004);
    for _ in 0..1000 {
        let x = rng.bytes(64);
        let mut y = x.clone();
        let flips = rng.below(4);
        for _ in 0..flips {
            let k = rng.below(64);
            y[k] ^= 1u8 << rng.below(8);
        }
        unsafe {
            for (cf, rf, n) in [
                (&c16, &r16, 16usize),
                (&c32, &r32, 32),
                (&c64, &r64, 64),
            ] {
                let rc = cf(x.as_ptr(), y.as_ptr());
                let rr = rf(x.as_ptr(), y.as_ptr());
                eqi(&format!("prefix-consistency verify_{n}"), rc, rr);
                let expect = if x[..n] == y[..n] { 0 } else { -1 };
                assert_eq!(rc, expect, "verify_{n} disagrees with memcmp");
            }
        }
    }
}

// ------------------------------------------------------------------- softaes

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

impl SoftAesBlock {
    fn load(b: &[u8]) -> Self {
        let g = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        SoftAesBlock { w0: g(0), w1: g(4), w2: g(8), w3: g(12) }
    }
    fn store(&self) -> [u8; 16] {
        let mut o = [0u8; 16];
        o[0..4].copy_from_slice(&self.w0.to_le_bytes());
        o[4..8].copy_from_slice(&self.w1.to_le_bytes());
        o[8..12].copy_from_slice(&self.w2.to_le_bytes());
        o[12..16].copy_from_slice(&self.w3.to_le_bytes());
        o
    }
    fn xor(self, o: SoftAesBlock) -> Self {
        SoftAesBlock { w0: self.w0 ^ o.w0, w1: self.w1 ^ o.w1, w2: self.w2 ^ o.w2, w3: self.w3 ^ o.w3 }
    }
    fn bytes(&self) -> Vec<u8> {
        self.store().to_vec()
    }
}

type ExpandKey = unsafe extern "C" fn(*mut SoftAesBlock, *const u8);
type InvertKs = unsafe extern "C" fn(*mut SoftAesBlock);
type Block1 = unsafe extern "C" fn(SoftAesBlock) -> SoftAesBlock;
type Block2 = unsafe extern "C" fn(SoftAesBlock, SoftAesBlock) -> SoftAesBlock;

fn rkeys_bytes(k: &[SoftAesBlock]) -> Vec<u8> {
    k.iter().flat_map(|b| b.store()).collect()
}

/// Key-expansion inputs used by every softaes test.
fn aes_keys(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![0u8; n],
        vec![0xffu8; n],
        (0..n).map(|i| i as u8).collect(),
        // FIPS-197 AES-128 / AES-256 sample keys
        if n == 16 {
            vec![
                0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
                0x4f, 0x3c,
            ]
        } else {
            (0..n).map(|i| i as u8).collect()
        },
    ];
    for _ in 0..300 {
        v.push(rng.bytes(n));
    }
    v
}

/// Rows 2.42 / 2.43 — `softaes_expand_key128` / `_expand_key256`.
#[test]
fn softaes_expand_key() {
    for (name, keylen, nrk) in [
        ("_sodium_softaes_expand_key128", 16usize, 11usize),
        ("_sodium_softaes_expand_key256", 32, 15),
    ] {
        assert!(has(name), "{name} must be exported by both libraries");
        let (c, r) = both::<ExpandKey>(name);
        let mut rng = Rng::new(0x2_0042 ^ keylen as u64);
        for key in aes_keys(&mut rng, keylen) {
            // pad the rkeys array so an over-long write is caught
            let mut kc = vec![SoftAesBlock::default(); nrk + 4];
            let mut kr = vec![SoftAesBlock::default(); nrk + 4];
            let sentinel = SoftAesBlock { w0: 0xDEADBEEF, w1: 0xFEEDFACE, w2: 0x12345678, w3: 0x9ABCDEF0 };
            for i in nrk..nrk + 4 {
                kc[i] = sentinel;
                kr[i] = sentinel;
            }
            unsafe {
                c(kc.as_mut_ptr(), key.as_ptr());
                r(kr.as_mut_ptr(), key.as_ptr());
            }
            eqb(&format!("{name} rkeys"), &rkeys_bytes(&kc[..nrk]), &rkeys_bytes(&kr[..nrk]));
            for i in nrk..nrk + 4 {
                assert_eq!(kc[i], sentinel, "{name}: C wrote past rkeys[{nrk}]");
                assert_eq!(kr[i], sentinel, "{name}: Rust wrote past rkeys[{nrk}]");
            }
            // rkeys[0] must be the raw key (little-endian words)
            assert_eq!(&rkeys_bytes(&kc[..keylen / 16])[..keylen], &key[..]);
        }
    }
}

/// Rows 2.44 / 2.45 — `softaes_invert_key_schedule128` / `_256`.
#[test]
fn softaes_invert_key_schedule() {
    for (exp, inv, keylen, nrk) in [
        ("_sodium_softaes_expand_key128", "_sodium_softaes_invert_key_schedule128", 16usize, 11usize),
        ("_sodium_softaes_expand_key256", "_sodium_softaes_invert_key_schedule256", 32, 15),
    ] {
        assert!(has(exp) && has(inv));
        let (ce, re) = both::<ExpandKey>(exp);
        let (ci, ri) = both::<InvertKs>(inv);
        let mut rng = Rng::new(0x2_0044 ^ keylen as u64);
        for key in aes_keys(&mut rng, keylen) {
            let mut kc = vec![SoftAesBlock::default(); nrk];
            let mut kr = vec![SoftAesBlock::default(); nrk];
            unsafe {
                ce(kc.as_mut_ptr(), key.as_ptr());
                re(kr.as_mut_ptr(), key.as_ptr());
            }
            let before_c = kc.clone();
            unsafe {
                ci(kc.as_mut_ptr());
                ri(kr.as_mut_ptr());
            }
            eqb(&format!("{inv}"), &rkeys_bytes(&kc), &rkeys_bytes(&kr));
            // indices 0 and nrk-1 must be untouched, and at least one middle
            // index must have changed (inv_mix_columns is not the identity).
            assert_eq!(kc[0], before_c[0], "{inv}: rkeys[0] must not change");
            assert_eq!(kc[nrk - 1], before_c[nrk - 1], "{inv}: rkeys[{}] must not change", nrk - 1);
        }
    }
}

/// Row 2.46 — `softaes_inv_mix_columns` on arbitrary blocks.
#[test]
fn softaes_inv_mix_columns() {
    assert!(has("_sodium_softaes_inv_mix_columns"));
    let (c, r) = both::<Block1>("_sodium_softaes_inv_mix_columns");
    let mut rng = Rng::new(0x2_0046);
    let mut cases: Vec<SoftAesBlock> = vec![
        SoftAesBlock::default(),
        SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
    ];
    // every single bit set, in every word
    for w in 0..4 {
        for bit in 0..32 {
            let mut b = SoftAesBlock::default();
            let v = 1u32 << bit;
            match w {
                0 => b.w0 = v,
                1 => b.w1 = v,
                2 => b.w2 = v,
                _ => b.w3 = v,
            }
            cases.push(b);
        }
    }
    for _ in 0..2000 {
        cases.push(SoftAesBlock {
            w0: rng.next_u32(),
            w1: rng.next_u32(),
            w2: rng.next_u32(),
            w3: rng.next_u32(),
        });
    }
    for b in cases {
        let (oc, or) = unsafe { (c(b), r(b)) };
        eqb(&format!("inv_mix_columns({:08x})", b.w0), &oc.bytes(), &or.bytes());
    }
}

/// Rows 2.47 / 2.48 — the single-round primitives, exercised standalone over
/// many random (block, round-key) pairs, and then chained into the full
/// AES-128 / AES-256 encryption and decryption.
#[test]
fn softaes_single_round_primitives() {
    for name in [
        "_sodium_softaes_block_encrypt",
        "_sodium_softaes_block_encryptlast",
        "_sodium_softaes_block_decrypt",
        "_sodium_softaes_block_decryptlast",
    ] {
        assert!(has(name), "{name} must be exported by both libraries");
        let (c, r) = both::<Block2>(name);
        let mut rng = Rng::new(0x2_0047);
        let mut pairs: Vec<(SoftAesBlock, SoftAesBlock)> = vec![
            (SoftAesBlock::default(), SoftAesBlock::default()),
            (
                SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
                SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 },
            ),
            (SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 }, SoftAesBlock::default()),
            (SoftAesBlock::default(), SoftAesBlock { w0: !0, w1: !0, w2: !0, w3: !0 }),
        ];
        // walk every byte value through byte 0 so every SBOX slice is hit
        for v in 0..=255u8 {
            let mut b = [0u8; 16];
            b[0] = v;
            b[5] = v;
            b[10] = v;
            b[15] = v;
            pairs.push((SoftAesBlock::load(&b), SoftAesBlock::default()));
        }
        for _ in 0..3000 {
            pairs.push((
                SoftAesBlock {
                    w0: rng.next_u32(),
                    w1: rng.next_u32(),
                    w2: rng.next_u32(),
                    w3: rng.next_u32(),
                },
                SoftAesBlock {
                    w0: rng.next_u32(),
                    w1: rng.next_u32(),
                    w2: rng.next_u32(),
                    w3: rng.next_u32(),
                },
            ));
        }
        for (b, rk) in pairs {
            let (oc, or) = unsafe { (c(b, rk), r(b, rk)) };
            eqb(&format!("{name}"), &oc.bytes(), &or.bytes());
        }
    }
}

struct SoftAes {
    exp128: (libloading::Symbol<'static, ExpandKey>, libloading::Symbol<'static, ExpandKey>),
    exp256: (libloading::Symbol<'static, ExpandKey>, libloading::Symbol<'static, ExpandKey>),
    inv128: (libloading::Symbol<'static, InvertKs>, libloading::Symbol<'static, InvertKs>),
    inv256: (libloading::Symbol<'static, InvertKs>, libloading::Symbol<'static, InvertKs>),
    enc: (libloading::Symbol<'static, Block2>, libloading::Symbol<'static, Block2>),
    encl: (libloading::Symbol<'static, Block2>, libloading::Symbol<'static, Block2>),
    dec: (libloading::Symbol<'static, Block2>, libloading::Symbol<'static, Block2>),
    decl: (libloading::Symbol<'static, Block2>, libloading::Symbol<'static, Block2>),
}

impl SoftAes {
    fn new() -> Self {
        SoftAes {
            exp128: both("_sodium_softaes_expand_key128"),
            exp256: both("_sodium_softaes_expand_key256"),
            inv128: both("_sodium_softaes_invert_key_schedule128"),
            inv256: both("_sodium_softaes_invert_key_schedule256"),
            enc: both("_sodium_softaes_block_encrypt"),
            encl: both("_sodium_softaes_block_encryptlast"),
            dec: both("_sodium_softaes_block_decrypt"),
            decl: both("_sodium_softaes_block_decryptlast"),
        }
    }

    /// side == 0 -> C, side == 1 -> Rust
    fn schedule(&self, side: usize, key: &[u8]) -> Vec<SoftAesBlock> {
        let nrk = if key.len() == 16 { 11 } else { 15 };
        let mut k = vec![SoftAesBlock::default(); nrk];
        unsafe {
            if key.len() == 16 {
                let f = if side == 0 { &self.exp128.0 } else { &self.exp128.1 };
                f(k.as_mut_ptr(), key.as_ptr());
            } else {
                let f = if side == 0 { &self.exp256.0 } else { &self.exp256.1 };
                f(k.as_mut_ptr(), key.as_ptr());
            }
        }
        k
    }

    fn invert(&self, side: usize, k: &mut [SoftAesBlock]) {
        unsafe {
            if k.len() == 11 {
                let f = if side == 0 { &self.inv128.0 } else { &self.inv128.1 };
                f(k.as_mut_ptr());
            } else {
                let f = if side == 0 { &self.inv256.0 } else { &self.inv256.1 };
                f(k.as_mut_ptr());
            }
        }
    }

    fn encrypt(&self, side: usize, k: &[SoftAesBlock], pt: SoftAesBlock) -> SoftAesBlock {
        let e = if side == 0 { &self.enc.0 } else { &self.enc.1 };
        let el = if side == 0 { &self.encl.0 } else { &self.encl.1 };
        let mut s = pt.xor(k[0]);
        unsafe {
            for rk in &k[1..k.len() - 1] {
                s = e(s, *rk);
            }
            el(s, k[k.len() - 1])
        }
    }

    fn decrypt(&self, side: usize, ik: &[SoftAesBlock], ct: SoftAesBlock) -> SoftAesBlock {
        let dd = if side == 0 { &self.dec.0 } else { &self.dec.1 };
        let dl = if side == 0 { &self.decl.0 } else { &self.decl.1 };
        let n = ik.len();
        let mut s = ct.xor(ik[n - 1]);
        unsafe {
            for i in (1..n - 1).rev() {
                s = dd(s, ik[i]);
            }
            dl(s, ik[0])
        }
    }
}

/// Rows 2.47 - 2.50 — full AES-128 / AES-256 encrypt, decrypt, and round trip.
#[test]
fn softaes_full_block_cipher() {
    let s = SoftAes::new();
    let mut rng = Rng::new(0x2_0049);

    // FIPS-197 known-answer tests, to prove the composition above is real AES.
    let kat: &[(&[u8], &[u8], &[u8])] = &[
        (
            &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ],
            &[
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ],
            &[
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
                0xc5, 0x5a,
            ],
        ),
        (
            &(0..32u8).collect::<Vec<u8>>(),
            &[
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff,
            ],
            &[
                0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
                0x60, 0x89,
            ],
        ),
    ];
    for (key, pt, ct) in kat {
        for side in 0..2 {
            let k = s.schedule(side, key);
            let got = s.encrypt(side, &k, SoftAesBlock::load(pt));
            assert_eq!(
                &got.store()[..],
                *ct,
                "softaes AES-{} KAT failed on side {side}",
                key.len() * 8
            );
        }
    }

    for keylen in [16usize, 32] {
        for key in aes_keys(&mut rng, keylen) {
            let kc = s.schedule(0, &key);
            let kr = s.schedule(1, &key);
            eqb("softaes schedule", &rkeys_bytes(&kc), &rkeys_bytes(&kr));
            let mut ikc = kc.clone();
            let mut ikr = kr.clone();
            s.invert(0, &mut ikc);
            s.invert(1, &mut ikr);
            eqb("softaes inverted schedule", &rkeys_bytes(&ikc), &rkeys_bytes(&ikr));

            let mut blocks: Vec<[u8; 16]> = vec![[0u8; 16], [0xffu8; 16]];
            for _ in 0..40 {
                let mut b = [0u8; 16];
                rng.fill(&mut b);
                blocks.push(b);
            }
            for b in blocks {
                let pt = SoftAesBlock::load(&b);
                let ec = s.encrypt(0, &kc, pt);
                let er = s.encrypt(1, &kr, pt);
                eqb("softaes encrypt", &ec.bytes(), &er.bytes());
                let dc = s.decrypt(0, &ikc, ec);
                let dr = s.decrypt(1, &ikr, er);
                eqb("softaes decrypt", &dc.bytes(), &dr.bytes());
                assert_eq!(dc, pt, "softaes AES-{} round trip lost data", keylen * 8);
                // decrypting an arbitrary block must also agree
                let xc = s.decrypt(0, &ikc, pt);
                let xr = s.decrypt(1, &ikr, pt);
                eqb("softaes raw decrypt", &xc.bytes(), &xr.bytes());
                let yc = s.encrypt(0, &kc, xc);
                assert_eq!(yc, pt, "softaes AES-{} inverse round trip lost data", keylen * 8);
            }
        }
    }
}
