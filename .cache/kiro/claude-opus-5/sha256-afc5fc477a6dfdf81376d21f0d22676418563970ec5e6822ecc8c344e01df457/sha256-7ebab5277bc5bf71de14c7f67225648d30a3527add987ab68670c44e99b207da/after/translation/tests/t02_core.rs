//! crypto_core layer: salsa20/salsa2012/salsa208 cores, hsalsa20, hchacha20,
//! keccak1600 permutation, ed25519 group/scalar arithmetic and ristretto255.
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_void};

type FnCore4 = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    *const c_uchar,
    *const c_uchar,
) -> c_int;

// ---------------------------------------------------------------------------
// salsa20 / salsa2012 / salsa208 / hsalsa20 / hchacha20 cores
// ---------------------------------------------------------------------------

fn core_case(name: &str, outbytes: usize, inbytes: usize, keybytes: usize, constbytes: usize) {
    for s in ["outputbytes", "inputbytes", "keybytes", "constbytes"] {
        cmp_size(&format!("{name}_{s}"));
    }
    unsafe {
        let (c, r): (FnCore4, FnCore4) = pair(name);
        let mut rng = Rng::new(0x100 + name.len() as u64);
        for iter in 0..64 {
            let inb = rng.vec(inbytes);
            let k = rng.vec(keybytes);
            let cst = rng.vec(constbytes);
            let use_const = iter % 3 != 0;
            let cp = if use_const { cst.as_ptr() } else { std::ptr::null() };
            let mut co = vec![0xAAu8; outbytes + 8];
            let mut ro = vec![0xAAu8; outbytes + 8];
            let cr = c(co.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp);
            let rr = r(ro.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp);
            assert_eq!(cr, rr, "{name} return iter {iter}");
            assert_bytes_eq(&format!("{name} iter {iter} const={use_const}"), &co, &ro);
        }
        // edge inputs
        let zeros_in = vec![0u8; inbytes];
        let ones_in = vec![0xffu8; inbytes];
        let zeros_k = vec![0u8; keybytes];
        let ones_k = vec![0xffu8; keybytes];
        let zeros_c = vec![0u8; constbytes];
        let ones_c = vec![0xffu8; constbytes];
        for (inb, k, cst) in [
            (&zeros_in, &zeros_k, &zeros_c),
            (&ones_in, &ones_k, &ones_c),
            (&zeros_in, &ones_k, &zeros_c),
            (&ones_in, &zeros_k, &ones_c),
        ] {
            for cp in [cst.as_ptr(), std::ptr::null()] {
                let mut co = vec![0xAAu8; outbytes + 8];
                let mut ro = vec![0xAAu8; outbytes + 8];
                let cr = c(co.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp);
                let rr = r(ro.as_mut_ptr(), inb.as_ptr(), k.as_ptr(), cp);
                assert_eq!(cr, rr, "{name} edge return");
                assert_bytes_eq(&format!("{name} edge"), &co, &ro);
            }
        }
    }
}

#[test]
fn crypto_core_salsa_family() {
    core_case("crypto_core_salsa20", 64, 16, 32, 16);
    core_case("crypto_core_salsa2012", 64, 16, 32, 16);
    core_case("crypto_core_salsa208", 64, 16, 32, 16);
    core_case("crypto_core_hsalsa20", 32, 16, 32, 16);
    core_case("crypto_core_hchacha20", 32, 16, 32, 16);
}

// ---------------------------------------------------------------------------
// keccak1600
// ---------------------------------------------------------------------------

type FnKInit = unsafe extern "C" fn(*mut c_void);
type FnKXor = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize, usize);
type FnKExtract = unsafe extern "C" fn(*const c_void, *mut c_uchar, usize, usize);
type FnKPermute = unsafe extern "C" fn(*mut c_void);

#[test]
fn crypto_core_keccak1600_matches() {
    cmp_size("crypto_core_keccak1600_statebytes");
    unsafe {
        let (csb, _): (FnSize, FnSize) = pair("crypto_core_keccak1600_statebytes");
        let sb = csb();
        assert_eq!(sb, 224, "unexpected keccak state size");
        let (cinit, rinit): (FnKInit, FnKInit) = pair("crypto_core_keccak1600_init");
        let (cxor, rxor): (FnKXor, FnKXor) = pair("crypto_core_keccak1600_xor_bytes");
        let (cext, rext): (FnKExtract, FnKExtract) =
            pair("crypto_core_keccak1600_extract_bytes");
        let (cp24, rp24): (FnKPermute, FnKPermute) = pair("crypto_core_keccak1600_permute_24");
        let (cp12, rp12): (FnKPermute, FnKPermute) = pair("crypto_core_keccak1600_permute_12");

        // 16-byte aligned state buffers (CRYPTO_ALIGN(16))
        #[repr(align(16))]
        struct St([u8; 256]);

        let mut cst = St([0xAAu8; 256]);
        let mut rst = St([0xAAu8; 256]);
        cinit(cst.0.as_mut_ptr() as *mut c_void);
        rinit(rst.0.as_mut_ptr() as *mut c_void);
        assert_bytes_eq("keccak init", &cst.0, &rst.0);

        let mut rng = Rng::new(0x200);
        for round in 0..40 {
            let off = (rng.byte() as usize) % 200;
            let len = (rng.byte() as usize) % (200 - off).max(1);
            let data = rng.vec(len.max(1));
            cxor(cst.0.as_mut_ptr() as *mut c_void, data.as_ptr(), off, len);
            rxor(rst.0.as_mut_ptr() as *mut c_void, data.as_ptr(), off, len);
            assert_bytes_eq(&format!("keccak xor_bytes round {round}"), &cst.0, &rst.0);

            if round % 2 == 0 {
                cp24(cst.0.as_mut_ptr() as *mut c_void);
                rp24(rst.0.as_mut_ptr() as *mut c_void);
                assert_bytes_eq(&format!("keccak permute_24 round {round}"), &cst.0, &rst.0);
            } else {
                cp12(cst.0.as_mut_ptr() as *mut c_void);
                rp12(rst.0.as_mut_ptr() as *mut c_void);
                assert_bytes_eq(&format!("keccak permute_12 round {round}"), &cst.0, &rst.0);
            }

            let eoff = (rng.byte() as usize) % 200;
            let elen = (rng.byte() as usize) % (200 - eoff).max(1);
            let mut cb = vec![0x55u8; elen + 8];
            let mut rb = vec![0x55u8; elen + 8];
            cext(cst.0.as_ptr() as *const c_void, cb.as_mut_ptr(), eoff, elen);
            rext(rst.0.as_ptr() as *const c_void, rb.as_mut_ptr(), eoff, elen);
            assert_bytes_eq(&format!("keccak extract round {round}"), &cb, &rb);
        }

        // deterministic sequence: xor 200 bytes then permute repeatedly
        let mut cst = St([0u8; 256]);
        let mut rst = St([0u8; 256]);
        cinit(cst.0.as_mut_ptr() as *mut c_void);
        rinit(rst.0.as_mut_ptr() as *mut c_void);
        let block: Vec<u8> = (0..200u32).map(|i| (i & 0xff) as u8).collect();
        for i in 0..10 {
            cxor(cst.0.as_mut_ptr() as *mut c_void, block.as_ptr(), 0, 200);
            rxor(rst.0.as_mut_ptr() as *mut c_void, block.as_ptr(), 0, 200);
            cp24(cst.0.as_mut_ptr() as *mut c_void);
            rp24(rst.0.as_mut_ptr() as *mut c_void);
            assert_bytes_eq(&format!("keccak full block iter {i}"), &cst.0, &rst.0);
        }
        // zero-length operations
        cxor(cst.0.as_mut_ptr() as *mut c_void, block.as_ptr(), 0, 0);
        rxor(rst.0.as_mut_ptr() as *mut c_void, block.as_ptr(), 0, 0);
        assert_bytes_eq("keccak xor zero len", &cst.0, &rst.0);
        let mut cb = [0x55u8; 8];
        let mut rb = [0x55u8; 8];
        cext(cst.0.as_ptr() as *const c_void, cb.as_mut_ptr(), 0, 0);
        rext(rst.0.as_ptr() as *const c_void, rb.as_mut_ptr(), 0, 0);
        assert_bytes_eq("keccak extract zero len", &cb, &rb);
    }
}

// ---------------------------------------------------------------------------
// ed25519 / ristretto255 group and scalar arithmetic
// ---------------------------------------------------------------------------

type FnUnary1 = unsafe extern "C" fn(*const c_uchar) -> c_int;
type FnBin = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;
type FnBinV = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar);
type FnUnV = unsafe extern "C" fn(*mut c_uchar, *const c_uchar);
type FnUnI = unsafe extern "C" fn(*mut c_uchar, *const c_uchar) -> c_int;
type FnRandom = unsafe extern "C" fn(*mut c_uchar);
type FnFromString = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    usize,
    *const c_uchar,
    usize,
    c_int,
) -> c_int;

/// Generate a set of valid curve points using the C library's own
/// `_random`/`_from_string` so that group operations get meaningful inputs.
fn valid_points(prefix: &str, n: usize) -> Vec<Vec<u8>> {
    unsafe {
        let (crand, _): (FnRandom, FnRandom) = pair(&format!("{prefix}_random"));
        let (cbytes, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let sz = cbytes();
        let mut out = Vec::new();
        for i in 0..n {
            det_reset();
            // advance the deterministic stream so points differ
            let mut junk = vec![0u8; i];
            if i > 0 {
                let (cbuf, _): (unsafe extern "C" fn(*mut c_void, usize), _) =
                    pair::<unsafe extern "C" fn(*mut c_void, usize)>("randombytes_buf");
                cbuf(junk.as_mut_ptr() as *mut c_void, i);
            }
            let mut p = vec![0u8; sz];
            crand(p.as_mut_ptr());
            out.push(p);
        }
        out
    }
}

fn scalar_cases(sz: usize, n: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![0u8; sz],
        vec![0xffu8; sz],
        {
            let mut s = vec![0u8; sz];
            s[0] = 1;
            s
        },
        {
            // L - 1 (ed25519 group order minus one), little-endian
            let mut s = vec![0u8; sz];
            let l: [u8; 32] = [
                0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde,
                0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
            ];
            for (i, b) in l.iter().enumerate() {
                if i < sz {
                    s[i] = *b;
                }
            }
            if sz > 0 {
                s[0] = s[0].wrapping_sub(1);
            }
            s
        },
        {
            // exactly L
            let mut s = vec![0u8; sz];
            let l: [u8; 32] = [
                0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde,
                0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
            ];
            for (i, b) in l.iter().enumerate() {
                if i < sz {
                    s[i] = *b;
                }
            }
            s
        },
    ];
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        v.push(rng.vec(sz));
    }
    // low-order-ish patterns
    for bit in 0..(sz * 8).min(256) {
        let mut s = vec![0u8; sz];
        s[bit / 8] = 1 << (bit % 8);
        v.push(s);
    }
    v
}

fn scalar_suite(prefix: &str) {
    unsafe {
        cmp_size(&format!("{prefix}_scalarbytes"));
        cmp_size(&format!("{prefix}_nonreducedscalarbytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_scalarbytes"));
        let (cnsb, _): (FnSize, FnSize) = pair(&format!("{prefix}_nonreducedscalarbytes"));
        let sz = csb();
        let nsz = cnsb();

        let cases = scalar_cases(sz, 32, 0x300 + prefix.len() as u64);

        // is_canonical
        let (cic, ric): (FnUnary1, FnUnary1) =
            pair(&format!("{prefix}_scalar_is_canonical"));
        for s in &cases {
            assert_eq!(
                cic(s.as_ptr()),
                ric(s.as_ptr()),
                "{prefix}_scalar_is_canonical({})",
                hex(s)
            );
        }

        // negate / complement / invert
        for op in ["negate", "complement"] {
            let (c, r): (FnUnV, FnUnV) = pair(&format!("{prefix}_scalar_{op}"));
            for s in &cases {
                let mut co = vec![0xAAu8; sz + 8];
                let mut ro = vec![0xAAu8; sz + 8];
                c(co.as_mut_ptr(), s.as_ptr());
                r(ro.as_mut_ptr(), s.as_ptr());
                assert_bytes_eq(
                    &format!("{prefix}_scalar_{op}({})", hex(s)),
                    &co,
                    &ro,
                );
            }
        }
        {
            let (c, r): (FnUnI, FnUnI) = pair(&format!("{prefix}_scalar_invert"));
            for s in &cases {
                let mut co = vec![0xAAu8; sz + 8];
                let mut ro = vec![0xAAu8; sz + 8];
                let cr = c(co.as_mut_ptr(), s.as_ptr());
                let rr = r(ro.as_mut_ptr(), s.as_ptr());
                assert_eq!(cr, rr, "{prefix}_scalar_invert({}) return", hex(s));
                assert_bytes_eq(&format!("{prefix}_scalar_invert({})", hex(s)), &co, &ro);
            }
        }

        // add / sub / mul
        for op in ["add", "sub", "mul"] {
            let (c, r): (FnBinV, FnBinV) = pair(&format!("{prefix}_scalar_{op}"));
            for x in cases.iter().take(24) {
                for y in cases.iter().take(24) {
                    let mut co = vec![0xAAu8; sz + 8];
                    let mut ro = vec![0xAAu8; sz + 8];
                    c(co.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    r(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    assert_bytes_eq(
                        &format!("{prefix}_scalar_{op}({},{})", hex(x), hex(y)),
                        &co,
                        &ro,
                    );
                }
            }
        }

        // reduce (takes nonreducedscalarbytes input)
        {
            let (c, r): (FnUnV, FnUnV) = pair(&format!("{prefix}_scalar_reduce"));
            let big = scalar_cases(nsz, 40, 0x400 + prefix.len() as u64);
            for s in &big {
                let mut co = vec![0xAAu8; sz + 8];
                let mut ro = vec![0xAAu8; sz + 8];
                c(co.as_mut_ptr(), s.as_ptr());
                r(ro.as_mut_ptr(), s.as_ptr());
                assert_bytes_eq(&format!("{prefix}_scalar_reduce({})", hex(s)), &co, &ro);
            }
        }

        // scalar_random with the shared deterministic RNG
        {
            let (c, r): (FnRandom, FnRandom) = pair(&format!("{prefix}_scalar_random"));
            for i in 0..8 {
                let mut co = vec![0xAAu8; sz + 8];
                let mut ro = vec![0xAAu8; sz + 8];
                det_reset();
                c(co.as_mut_ptr());
                det_reset();
                r(ro.as_mut_ptr());
                assert_bytes_eq(&format!("{prefix}_scalar_random iter {i}"), &co, &ro);
            }
        }

        // scalar_from_string (hash-to-scalar), both hash algorithms
        {
            let (c, r): (FnFromString, FnFromString) =
                pair(&format!("{prefix}_scalar_from_string"));
            let mut rng = Rng::new(0x500);
            for alg in [0i32, 1, 2, 3] {
                for msglen in [0usize, 1, 32, 100] {
                    for ctxlen in [0usize, 1, 16] {
                        let msg = rng.vec(msglen.max(1));
                        let ctx = rng.vec(ctxlen.max(1));
                        let mut co = vec![0xAAu8; sz + 8];
                        let mut ro = vec![0xAAu8; sz + 8];
                        let cr = c(
                            co.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let rr = r(
                            ro.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let tag = format!(
                            "{prefix}_scalar_from_string(alg={alg},ctx={ctxlen},msg={msglen})"
                        );
                        assert_eq!(cr, rr, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }
    }
}

fn point_suite(prefix: &str) {
    unsafe {
        cmp_size(&format!("{prefix}_bytes"));
        cmp_size(&format!("{prefix}_hashbytes"));
        let (cbytes, _): (FnSize, FnSize) = pair(&format!("{prefix}_bytes"));
        let sz = cbytes();

        // is_valid_point over random and structured inputs
        let (civ, riv): (FnUnary1, FnUnary1) = pair(&format!("{prefix}_is_valid_point"));
        let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; sz], vec![0xffu8; sz], {
            let mut v = vec![0u8; sz];
            v[0] = 1;
            v
        }];
        let mut rng = Rng::new(0x600 + prefix.len() as u64);
        for _ in 0..200 {
            inputs.push(rng.vec(sz));
        }
        for bit in 0..(sz * 8) {
            let mut v = vec![0u8; sz];
            v[bit / 8] = 1 << (bit % 8);
            inputs.push(v);
        }
        let good = valid_points(prefix, 24);
        inputs.extend(good.iter().cloned());
        // valid points with a flipped high bit
        for p in &good {
            let mut v = p.clone();
            v[sz - 1] ^= 0x80;
            inputs.push(v);
            let mut v = p.clone();
            v[0] ^= 0x01;
            inputs.push(v);
        }
        for p in &inputs {
            assert_eq!(
                civ(p.as_ptr()),
                riv(p.as_ptr()),
                "{prefix}_is_valid_point({})",
                hex(p)
            );
        }

        // add / sub over valid and invalid points
        for op in ["add", "sub"] {
            let (c, r): (FnBin, FnBin) = pair(&format!("{prefix}_{op}"));
            for p in inputs.iter().take(40) {
                for q in good.iter().take(8) {
                    let mut co = vec![0xAAu8; sz + 8];
                    let mut ro = vec![0xAAu8; sz + 8];
                    let cr = c(co.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    let rr = r(ro.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    let tag = format!("{prefix}_{op}({},{})", hex(p), hex(q));
                    assert_eq!(cr, rr, "{tag} return");
                    assert_bytes_eq(&tag, &co, &ro);
                }
            }
            // all-valid pairs, including p == q
            for p in &good {
                for q in &good {
                    let mut co = vec![0xAAu8; sz + 8];
                    let mut ro = vec![0xAAu8; sz + 8];
                    let cr = c(co.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    let rr = r(ro.as_mut_ptr(), p.as_ptr(), q.as_ptr());
                    assert_eq!(cr, rr, "{prefix}_{op} valid return");
                    assert_bytes_eq(&format!("{prefix}_{op} valid"), &co, &ro);
                }
            }
        }

        // random point
        {
            let (c, r): (FnRandom, FnRandom) = pair(&format!("{prefix}_random"));
            for i in 0..8 {
                let mut co = vec![0xAAu8; sz + 8];
                let mut ro = vec![0xAAu8; sz + 8];
                det_reset();
                c(co.as_mut_ptr());
                det_reset();
                r(ro.as_mut_ptr());
                assert_bytes_eq(&format!("{prefix}_random iter {i}"), &co, &ro);
            }
        }

        // from_string (and from_string_nu for ed25519)
        let mut names = vec![format!("{prefix}_from_string")];
        if has(&format!("{prefix}_from_string_nu")) {
            names.push(format!("{prefix}_from_string_nu"));
        }
        for name in names {
            let (c, r): (FnFromString, FnFromString) = pair(&name);
            let mut rng = Rng::new(0x700);
            for alg in [0i32, 1, 2, 3] {
                for msglen in [0usize, 1, 5, 32, 64, 100] {
                    for ctxlen in [0usize, 1, 16] {
                        let msg = rng.vec(msglen.max(1));
                        let ctx = rng.vec(ctxlen.max(1));
                        let mut co = vec![0xAAu8; sz + 8];
                        let mut ro = vec![0xAAu8; sz + 8];
                        let cr = c(
                            co.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let rr = r(
                            ro.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctxlen,
                            msg.as_ptr(),
                            msglen,
                            alg,
                        );
                        let tag = format!("{name}(alg={alg},ctx={ctxlen},msg={msglen})");
                        assert_eq!(cr, rr, "{tag} return");
                        assert_bytes_eq(&tag, &co, &ro);
                    }
                }
            }
        }
    }
}

#[test]
fn crypto_core_ed25519_scalars() {
    scalar_suite("crypto_core_ed25519");
}

#[test]
fn crypto_core_ed25519_points() {
    cmp_size("crypto_core_ed25519_uniformbytes");
    point_suite("crypto_core_ed25519");
}

#[test]
fn crypto_core_ristretto255_scalars() {
    scalar_suite("crypto_core_ristretto255");
}

#[test]
fn crypto_core_ristretto255_points() {
    point_suite("crypto_core_ristretto255");
}

#[test]
fn crypto_core_ristretto255_from_hash_matches() {
    unsafe {
        let (chb, _): (FnSize, FnSize) = pair("crypto_core_ristretto255_hashbytes");
        let (cb, _): (FnSize, FnSize) = pair("crypto_core_ristretto255_bytes");
        let hb = chb();
        let sz = cb();
        let (c, r): (FnUnI, FnUnI) = pair("crypto_core_ristretto255_from_hash");
        let mut rng = Rng::new(0x800);
        let mut cases: Vec<Vec<u8>> = vec![vec![0u8; hb], vec![0xffu8; hb]];
        for _ in 0..128 {
            cases.push(rng.vec(hb));
        }
        for bit in 0..(hb * 8) {
            let mut v = vec![0u8; hb];
            v[bit / 8] = 1 << (bit % 8);
            cases.push(v);
        }
        for h in &cases {
            let mut co = vec![0xAAu8; sz + 8];
            let mut ro = vec![0xAAu8; sz + 8];
            let cr = c(co.as_mut_ptr(), h.as_ptr());
            let rr = r(ro.as_mut_ptr(), h.as_ptr());
            let tag = format!("ristretto255_from_hash({})", hex(h));
            assert_eq!(cr, rr, "{tag} return");
            assert_bytes_eq(&tag, &co, &ro);
        }
    }
}
