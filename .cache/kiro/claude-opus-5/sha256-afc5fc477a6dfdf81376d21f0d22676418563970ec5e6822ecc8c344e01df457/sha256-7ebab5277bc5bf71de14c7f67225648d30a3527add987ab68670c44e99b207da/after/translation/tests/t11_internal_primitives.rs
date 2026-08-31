//! Internal (`_sodium_*` / `_crypto_*`) symbols that the C shared library also
//! exports. These are not part of the documented API but they are part of the
//! ABI surface, so they are compared directly here.
//!
//! Layouts assumed (matching the portable build the CMakeLists produces, i.e.
//! no `HAVE_*` macros so `fe25519` is `int32_t[10]`):
//!   fe25519          = 40 bytes
//!   ge25519_p2       = 3 * fe25519 = 120 bytes
//!   ge25519_p3       = 4 * fe25519 = 160 bytes
//!   ge25519_p1p1     = 4 * fe25519 = 160 bytes
//!   SoftAesBlock     = 4 * uint32  =  16 bytes
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong, c_void};

const FE: usize = 40;
const P2: usize = 3 * FE;
const P3: usize = 4 * FE;
const P1P1: usize = 4 * FE;

// ---------------------------------------------------------------------------
// fe25519
// ---------------------------------------------------------------------------

type FnFeFromBytes = unsafe extern "C" fn(*mut c_void, *const c_uchar);
type FnFeToBytes = unsafe extern "C" fn(*mut c_uchar, *const c_void);
type FnFeInvert = unsafe extern "C" fn(*mut c_void, *const c_void);

#[test]
fn internal_fe25519_matches() {
    unsafe {
        let (cfb, rfb): (FnFeFromBytes, FnFeFromBytes) = pair("_sodium_fe25519_frombytes");
        let (ctb, rtb): (FnFeToBytes, FnFeToBytes) = pair("_sodium_fe25519_tobytes");
        let (cin, rin): (FnFeInvert, FnFeInvert) = pair("_sodium_fe25519_invert");

        let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for bit in 0..256 {
            let mut v = vec![0u8; 32];
            v[bit / 8] = 1 << (bit % 8);
            inputs.push(v);
        }
        // p-1, p, p+1 (2^255-19)
        for delta in [-1i32, 0, 1] {
            let mut v = vec![0xffu8; 32];
            v[0] = 0xed;
            v[31] = 0x7f;
            v[0] = (v[0] as i32 + delta) as u8;
            inputs.push(v);
        }
        let mut rng = Rng::new(0x9000);
        for _ in 0..64 {
            inputs.push(rng.vec(32));
        }

        for s in &inputs {
            let mut cf = vec![0xAAu8; FE + 8];
            let mut rf = vec![0xAAu8; FE + 8];
            cfb(cf.as_mut_ptr() as *mut c_void, s.as_ptr());
            rfb(rf.as_mut_ptr() as *mut c_void, s.as_ptr());
            assert_bytes_eq(&format!("fe25519_frombytes({})", hex(s)), &cf, &rf);

            let mut cb = vec![0xAAu8; 32 + 8];
            let mut rb = vec![0xAAu8; 32 + 8];
            ctb(cb.as_mut_ptr(), cf.as_ptr() as *const c_void);
            rtb(rb.as_mut_ptr(), rf.as_ptr() as *const c_void);
            assert_bytes_eq(&format!("fe25519_tobytes({})", hex(s)), &cb, &rb);

            let mut ci = vec![0xAAu8; FE + 8];
            let mut ri = vec![0xAAu8; FE + 8];
            cin(ci.as_mut_ptr() as *mut c_void, cf.as_ptr() as *const c_void);
            rin(ri.as_mut_ptr() as *mut c_void, rf.as_ptr() as *const c_void);
            assert_bytes_eq(&format!("fe25519_invert({})", hex(s)), &ci, &ri);

            // canonical bytes of the inverse too
            let mut cb2 = vec![0xAAu8; 32 + 8];
            let mut rb2 = vec![0xAAu8; 32 + 8];
            ctb(cb2.as_mut_ptr(), ci.as_ptr() as *const c_void);
            rtb(rb2.as_mut_ptr(), ri.as_ptr() as *const c_void);
            assert_bytes_eq(&format!("fe25519_invert bytes({})", hex(s)), &cb2, &rb2);
        }
    }
}

// ---------------------------------------------------------------------------
// sc25519
// ---------------------------------------------------------------------------

type FnScInvert = unsafe extern "C" fn(*mut c_uchar, *const c_uchar);
type FnScReduce = unsafe extern "C" fn(*mut c_uchar);
type FnScMul = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar);
type FnScMulAdd =
    unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar, *const c_uchar);
type FnScIsCanonical = unsafe extern "C" fn(*const c_uchar) -> c_int;

fn scalars32(seed: u64) -> Vec<Vec<u8>> {
    let l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut v: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for delta in [-1i32, 0, 1] {
        let mut s = l.to_vec();
        s[0] = (s[0] as i32 + delta) as u8;
        v.push(s);
    }
    for bit in 0..256 {
        let mut s = vec![0u8; 32];
        s[bit / 8] = 1 << (bit % 8);
        v.push(s);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..24 {
        v.push(rng.vec(32));
    }
    v
}

#[test]
fn internal_sc25519_matches() {
    unsafe {
        let (cinv, rinv): (FnScInvert, FnScInvert) = pair("_sodium_sc25519_invert");
        let (cred, rred): (FnScReduce, FnScReduce) = pair("_sodium_sc25519_reduce");
        let (cmul, rmul): (FnScMul, FnScMul) = pair("_sodium_sc25519_mul");
        let (cma, rma): (FnScMulAdd, FnScMulAdd) = pair("_sodium_sc25519_muladd");
        let (cic, ric): (FnScIsCanonical, FnScIsCanonical) = pair("_sodium_sc25519_is_canonical");

        let cases = scalars32(0x9100);
        for s in &cases {
            assert_eq!(
                cic(s.as_ptr()),
                ric(s.as_ptr()),
                "sc25519_is_canonical({})",
                hex(s)
            );
            let mut co = vec![0xAAu8; 32 + 8];
            let mut ro = vec![0xAAu8; 32 + 8];
            cinv(co.as_mut_ptr(), s.as_ptr());
            rinv(ro.as_mut_ptr(), s.as_ptr());
            assert_bytes_eq(&format!("sc25519_invert({})", hex(s)), &co, &ro);
        }

        // reduce operates in place on 64 bytes
        let mut rng = Rng::new(0x9200);
        let mut big: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
        for bit in 0..512 {
            let mut v = vec![0u8; 64];
            v[bit / 8] = 1 << (bit % 8);
            big.push(v);
        }
        for _ in 0..32 {
            big.push(rng.vec(64));
        }
        for s in &big {
            let mut a = s.clone();
            let mut b = s.clone();
            cred(a.as_mut_ptr());
            rred(b.as_mut_ptr());
            assert_bytes_eq(&format!("sc25519_reduce({})", hex(s)), &a, &b);
        }

        for x in cases.iter().take(24) {
            for y in cases.iter().take(24) {
                let mut co = vec![0xAAu8; 32 + 8];
                let mut ro = vec![0xAAu8; 32 + 8];
                cmul(co.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                rmul(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                assert_bytes_eq(&format!("sc25519_mul({},{})", hex(x), hex(y)), &co, &ro);

                let z = &cases[(x.len() + y[0] as usize) % cases.len()];
                let mut co = vec![0xAAu8; 32 + 8];
                let mut ro = vec![0xAAu8; 32 + 8];
                cma(co.as_mut_ptr(), x.as_ptr(), y.as_ptr(), z.as_ptr());
                rma(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr(), z.as_ptr());
                assert_bytes_eq("sc25519_muladd", &co, &ro);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ge25519 / ristretto255 internals
// ---------------------------------------------------------------------------

type FnGeFromBytes = unsafe extern "C" fn(*mut c_void, *const c_uchar) -> c_int;
type FnGeToBytes = unsafe extern "C" fn(*mut c_uchar, *const c_void);
type FnGePred = unsafe extern "C" fn(*const c_void) -> c_int;
type FnGeIsCanonical = unsafe extern "C" fn(*const c_uchar) -> c_int;
type FnGeUnaryV = unsafe extern "C" fn(*mut c_void);
type FnGeConv = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnGeBin = unsafe extern "C" fn(*mut c_void, *const c_void, *const c_void);
type FnGeScalarmult = unsafe extern "C" fn(*mut c_void, *const c_uchar, *const c_void);
type FnGeScalarmultBase = unsafe extern "C" fn(*mut c_void, *const c_uchar);
type FnGeDoubleScalarmult = unsafe extern "C" fn(
    *mut c_void,
    *const c_uchar,
    *const c_void,
    *const c_uchar,
);
type FnFromBytes32 = unsafe extern "C" fn(*mut c_uchar, *const c_uchar);

/// Valid encoded ed25519 points, obtained via the public API.
fn ed_points() -> Vec<Vec<u8>> {
    unsafe {
        let (crand, _): (unsafe extern "C" fn(*mut c_uchar), _) =
            pair::<unsafe extern "C" fn(*mut c_uchar)>("crypto_core_ed25519_random");
        let mut out = Vec::new();
        for i in 0..12u64 {
            det_reset();
            // advance the stream so successive points differ
            if i > 0 {
                let (cbuf, _): (unsafe extern "C" fn(*mut c_void, usize), _) =
                    pair::<unsafe extern "C" fn(*mut c_void, usize)>("randombytes_buf");
                let mut junk = vec![0u8; i as usize];
                cbuf(junk.as_mut_ptr() as *mut c_void, i as usize);
            }
            let mut p = vec![0u8; 32];
            crand(p.as_mut_ptr());
            out.push(p);
        }
        out
    }
}

#[test]
fn internal_ge25519_matches() {
    unsafe {
        let (cfb, rfb): (FnGeFromBytes, FnGeFromBytes) = pair("_sodium_ge25519_frombytes");
        let (cfbn, rfbn): (FnGeFromBytes, FnGeFromBytes) =
            pair("_sodium_ge25519_frombytes_negate_vartime");
        let (cp3tb, rp3tb): (FnGeToBytes, FnGeToBytes) = pair("_sodium_ge25519_p3_tobytes");
        let (ctb, rtb): (FnGeToBytes, FnGeToBytes) = pair("_sodium_ge25519_tobytes");
        let (cic, ric): (FnGeIsCanonical, FnGeIsCanonical) = pair("_sodium_ge25519_is_canonical");
        let (coc, roc): (FnGePred, FnGePred) = pair("_sodium_ge25519_is_on_curve");
        let (cms, rms): (FnGePred, FnGePred) = pair("_sodium_ge25519_is_on_main_subgroup");
        let (cso, rso): (FnGePred, FnGePred) = pair("_sodium_ge25519_has_small_order");
        let (ccc, rcc): (FnGeUnaryV, FnGeUnaryV) = pair("_sodium_ge25519_clear_cofactor");
        let (cadd, radd): (FnGeBin, FnGeBin) = pair("_sodium_ge25519_p3_add");
        let (csub, rsub): (FnGeBin, FnGeBin) = pair("_sodium_ge25519_p3_sub");
        let (csm, rsm): (FnGeScalarmult, FnGeScalarmult) = pair("_sodium_ge25519_scalarmult");
        let (csmb, rsmb): (FnGeScalarmultBase, FnGeScalarmultBase) =
            pair("_sodium_ge25519_scalarmult_base");
        let (cds, rds): (FnGeDoubleScalarmult, FnGeDoubleScalarmult) =
            pair("_sodium_ge25519_double_scalarmult_vartime");
        let (cp2p3, rp2p3): (FnGeConv, FnGeConv) = pair("_sodium_ge25519_p2_to_p3");
        let (c11p2, r11p2): (FnGeConv, FnGeConv) = pair("_sodium_ge25519_p1p1_to_p2");
        let (c11p3, r11p3): (FnGeConv, FnGeConv) = pair("_sodium_ge25519_p1p1_to_p3");
        let (cfu, rfu): (FnFromBytes32, FnFromBytes32) = pair("_sodium_ge25519_from_uniform");
        let (cfh, rfh): (FnFromBytes32, FnFromBytes32) = pair("_sodium_ge25519_from_hash");

        let mut encs: Vec<Vec<u8>> = ed_points();
        encs.push(vec![0u8; 32]);
        encs.push(vec![0xffu8; 32]);
        for bit in 0..256 {
            let mut v = vec![0u8; 32];
            v[bit / 8] = 1 << (bit % 8);
            encs.push(v);
        }
        let mut rng = Rng::new(0x9300);
        for _ in 0..48 {
            encs.push(rng.vec(32));
        }

        // is_canonical works on encodings
        for s in &encs {
            assert_eq!(
                cic(s.as_ptr()),
                ric(s.as_ptr()),
                "ge25519_is_canonical({})",
                hex(s)
            );
        }

        // frombytes / frombytes_negate_vartime, then the predicates and tobytes
        let mut cp3s: Vec<Vec<u8>> = Vec::new();
        for s in &encs {
            for (name, cf, rf) in [
                ("ge25519_frombytes", cfb, rfb),
                ("ge25519_frombytes_negate_vartime", cfbn, rfbn),
            ] {
                let mut cp = vec![0xAAu8; P3 + 8];
                let mut rp = vec![0xAAu8; P3 + 8];
                let a = cf(cp.as_mut_ptr() as *mut c_void, s.as_ptr());
                let b = rf(rp.as_mut_ptr() as *mut c_void, s.as_ptr());
                assert_eq!(a, b, "{name}({}) return", hex(s));
                assert_bytes_eq(&format!("{name}({})", hex(s)), &cp, &rp);
                if a != 0 {
                    continue;
                }
                for (pname, cpf, rpf) in [
                    ("is_on_curve", coc, roc),
                    ("is_on_main_subgroup", cms, rms),
                    ("has_small_order", cso, rso),
                ] {
                    assert_eq!(
                        cpf(cp.as_ptr() as *const c_void),
                        rpf(rp.as_ptr() as *const c_void),
                        "ge25519_{pname}({})",
                        hex(s)
                    );
                }
                let mut cb = vec![0xAAu8; 32 + 8];
                let mut rb = vec![0xAAu8; 32 + 8];
                cp3tb(cb.as_mut_ptr(), cp.as_ptr() as *const c_void);
                rp3tb(rb.as_mut_ptr(), rp.as_ptr() as *const c_void);
                assert_bytes_eq(&format!("ge25519_p3_tobytes({})", hex(s)), &cb, &rb);

                // p3's first three field elements are a valid p2 encoding
                let mut cb = vec![0xAAu8; 32 + 8];
                let mut rb = vec![0xAAu8; 32 + 8];
                ctb(cb.as_mut_ptr(), cp.as_ptr() as *const c_void);
                rtb(rb.as_mut_ptr(), rp.as_ptr() as *const c_void);
                assert_bytes_eq(&format!("ge25519_tobytes({})", hex(s)), &cb, &rb);

                // clear_cofactor mutates in place
                let mut cc2 = cp.clone();
                let mut rc2 = rp.clone();
                ccc(cc2.as_mut_ptr() as *mut c_void);
                rcc(rc2.as_mut_ptr() as *mut c_void);
                assert_bytes_eq(&format!("ge25519_clear_cofactor({})", hex(s)), &cc2, &rc2);

                if name == "ge25519_frombytes" && cp3s.len() < 10 {
                    cp3s.push(cp[..P3].to_vec());
                }
            }
        }

        // p3 arithmetic
        for p in &cp3s {
            for q in &cp3s {
                for (name, cf, rf) in [("p3_add", cadd, radd), ("p3_sub", csub, rsub)] {
                    let mut co = vec![0xAAu8; P3 + 8];
                    let mut ro = vec![0xAAu8; P3 + 8];
                    cf(
                        co.as_mut_ptr() as *mut c_void,
                        p.as_ptr() as *const c_void,
                        q.as_ptr() as *const c_void,
                    );
                    rf(
                        ro.as_mut_ptr() as *mut c_void,
                        p.as_ptr() as *const c_void,
                        q.as_ptr() as *const c_void,
                    );
                    assert_bytes_eq(&format!("ge25519_{name}"), &co, &ro);
                }
            }
        }

        // scalarmult / scalarmult_base / double_scalarmult_vartime
        let scalars = scalars32(0x9400);
        for a in scalars.iter().take(16) {
            let mut co = vec![0xAAu8; P3 + 8];
            let mut ro = vec![0xAAu8; P3 + 8];
            csmb(co.as_mut_ptr() as *mut c_void, a.as_ptr());
            rsmb(ro.as_mut_ptr() as *mut c_void, a.as_ptr());
            assert_bytes_eq(&format!("ge25519_scalarmult_base({})", hex(a)), &co, &ro);

            for p in cp3s.iter().take(4) {
                let mut co = vec![0xAAu8; P3 + 8];
                let mut ro = vec![0xAAu8; P3 + 8];
                csm(
                    co.as_mut_ptr() as *mut c_void,
                    a.as_ptr(),
                    p.as_ptr() as *const c_void,
                );
                rsm(
                    ro.as_mut_ptr() as *mut c_void,
                    a.as_ptr(),
                    p.as_ptr() as *const c_void,
                );
                assert_bytes_eq(&format!("ge25519_scalarmult({})", hex(a)), &co, &ro);

                for b in scalars.iter().take(4) {
                    let mut co = vec![0xAAu8; P2 + 8];
                    let mut ro = vec![0xAAu8; P2 + 8];
                    cds(
                        co.as_mut_ptr() as *mut c_void,
                        a.as_ptr(),
                        p.as_ptr() as *const c_void,
                        b.as_ptr(),
                    );
                    rds(
                        ro.as_mut_ptr() as *mut c_void,
                        a.as_ptr(),
                        p.as_ptr() as *const c_void,
                        b.as_ptr(),
                    );
                    assert_bytes_eq("ge25519_double_scalarmult_vartime", &co, &ro);
                }
            }
        }

        // representation conversions. p2/p1p1 are plain tuples of field
        // elements, so a p3's raw bytes are a well-formed input for each.
        for p in &cp3s {
            let mut co = vec![0xAAu8; P3 + 8];
            let mut ro = vec![0xAAu8; P3 + 8];
            cp2p3(co.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            rp2p3(ro.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            assert_bytes_eq("ge25519_p2_to_p3", &co, &ro);

            let mut co = vec![0xAAu8; P2 + 8];
            let mut ro = vec![0xAAu8; P2 + 8];
            c11p2(co.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            r11p2(ro.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            assert_bytes_eq("ge25519_p1p1_to_p2", &co, &ro);

            let mut co = vec![0xAAu8; P3 + 8];
            let mut ro = vec![0xAAu8; P3 + 8];
            c11p3(co.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            r11p3(ro.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            assert_bytes_eq("ge25519_p1p1_to_p3", &co, &ro);
            let _ = P1P1;
        }

        // from_uniform (32-byte input) and from_hash (64-byte input)
        let mut u32s: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for bit in 0..256 {
            let mut v = vec![0u8; 32];
            v[bit / 8] = 1 << (bit % 8);
            u32s.push(v);
        }
        for _ in 0..32 {
            u32s.push(rng.vec(32));
        }
        for r in &u32s {
            let mut co = vec![0xAAu8; 32 + 8];
            let mut ro = vec![0xAAu8; 32 + 8];
            cfu(co.as_mut_ptr(), r.as_ptr());
            rfu(ro.as_mut_ptr(), r.as_ptr());
            assert_bytes_eq(&format!("ge25519_from_uniform({})", hex(r)), &co, &ro);
        }
        let mut h64s: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
        for _ in 0..64 {
            h64s.push(rng.vec(64));
        }
        for h in &h64s {
            let mut co = vec![0xAAu8; 32 + 8];
            let mut ro = vec![0xAAu8; 32 + 8];
            cfh(co.as_mut_ptr(), h.as_ptr());
            rfh(ro.as_mut_ptr(), h.as_ptr());
            assert_bytes_eq(&format!("ge25519_from_hash({})", hex(h)), &co, &ro);
        }
    }
}

#[test]
fn internal_ristretto255_matches() {
    unsafe {
        let (cfb, rfb): (FnGeFromBytes, FnGeFromBytes) = pair("_sodium_ristretto255_frombytes");
        let (ctb, rtb): (FnGeToBytes, FnGeToBytes) = pair("_sodium_ristretto255_p3_tobytes");
        let (cfh, rfh): (FnFromBytes32, FnFromBytes32) = pair("_sodium_ristretto255_from_hash");

        let mut encs: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for bit in 0..256 {
            let mut v = vec![0u8; 32];
            v[bit / 8] = 1 << (bit % 8);
            encs.push(v);
        }
        let mut rng = Rng::new(0x9500);
        for _ in 0..64 {
            encs.push(rng.vec(32));
        }
        // valid ristretto encodings
        {
            let (crand, _): (unsafe extern "C" fn(*mut c_uchar), _) = pair::<
                unsafe extern "C" fn(*mut c_uchar),
            >("crypto_core_ristretto255_random");
            for i in 0..8usize {
                det_reset();
                if i > 0 {
                    let (cbuf, _): (unsafe extern "C" fn(*mut c_void, usize), _) =
                        pair::<unsafe extern "C" fn(*mut c_void, usize)>("randombytes_buf");
                    let mut junk = vec![0u8; i];
                    cbuf(junk.as_mut_ptr() as *mut c_void, i);
                }
                let mut p = vec![0u8; 32];
                crand(p.as_mut_ptr());
                encs.push(p);
            }
        }

        for s in &encs {
            let mut cp = vec![0xAAu8; P3 + 8];
            let mut rp = vec![0xAAu8; P3 + 8];
            let a = cfb(cp.as_mut_ptr() as *mut c_void, s.as_ptr());
            let b = rfb(rp.as_mut_ptr() as *mut c_void, s.as_ptr());
            assert_eq!(a, b, "ristretto255_frombytes({}) return", hex(s));
            assert_bytes_eq(&format!("ristretto255_frombytes({})", hex(s)), &cp, &rp);
            if a != 0 {
                continue;
            }
            let mut cb = vec![0xAAu8; 32 + 8];
            let mut rb = vec![0xAAu8; 32 + 8];
            ctb(cb.as_mut_ptr(), cp.as_ptr() as *const c_void);
            rtb(rb.as_mut_ptr(), rp.as_ptr() as *const c_void);
            assert_bytes_eq(&format!("ristretto255_p3_tobytes({})", hex(s)), &cb, &rb);
        }

        let mut h64s: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
        for _ in 0..96 {
            h64s.push(rng.vec(64));
        }
        for h in &h64s {
            let mut co = vec![0xAAu8; 32 + 8];
            let mut ro = vec![0xAAu8; 32 + 8];
            cfh(co.as_mut_ptr(), h.as_ptr());
            rfh(ro.as_mut_ptr(), h.as_ptr());
            assert_bytes_eq(&format!("ristretto255_from_hash({})", hex(h)), &co, &ro);
        }
    }
}

// ---------------------------------------------------------------------------
// softaes
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

type FnAesExpand = unsafe extern "C" fn(*mut SoftAesBlock, *const c_uchar);
type FnAesInvKs = unsafe extern "C" fn(*mut SoftAesBlock);
type FnAesInvMix = unsafe extern "C" fn(SoftAesBlock) -> SoftAesBlock;
type FnAesBlockOp = unsafe extern "C" fn(SoftAesBlock, SoftAesBlock) -> SoftAesBlock;

fn as_bytes(b: &[SoftAesBlock]) -> Vec<u8> {
    let mut out = Vec::new();
    for x in b {
        out.extend_from_slice(&x.w0.to_le_bytes());
        out.extend_from_slice(&x.w1.to_le_bytes());
        out.extend_from_slice(&x.w2.to_le_bytes());
        out.extend_from_slice(&x.w3.to_le_bytes());
    }
    out
}

#[test]
fn internal_softaes_matches() {
    unsafe {
        let (cek128, rek128): (FnAesExpand, FnAesExpand) = pair("_sodium_softaes_expand_key128");
        let (cek256, rek256): (FnAesExpand, FnAesExpand) = pair("_sodium_softaes_expand_key256");
        let (cik128, rik128): (FnAesInvKs, FnAesInvKs) =
            pair("_sodium_softaes_invert_key_schedule128");
        let (cik256, rik256): (FnAesInvKs, FnAesInvKs) =
            pair("_sodium_softaes_invert_key_schedule256");
        let (cimc, rimc): (FnAesInvMix, FnAesInvMix) = pair("_sodium_softaes_inv_mix_columns");
        let (cenc, renc): (FnAesBlockOp, FnAesBlockOp) = pair("_sodium_softaes_block_encrypt");
        let (cdec, rdec): (FnAesBlockOp, FnAesBlockOp) = pair("_sodium_softaes_block_decrypt");
        let (cencl, rencl): (FnAesBlockOp, FnAesBlockOp) =
            pair("_sodium_softaes_block_encryptlast");
        let (cdecl, rdecl): (FnAesBlockOp, FnAesBlockOp) =
            pair("_sodium_softaes_block_decryptlast");

        let zero = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };
        let mut rng = Rng::new(0x9600);
        let mut blocks: Vec<SoftAesBlock> = vec![
            zero,
            SoftAesBlock {
                w0: 0xffff_ffff,
                w1: 0xffff_ffff,
                w2: 0xffff_ffff,
                w3: 0xffff_ffff,
            },
            SoftAesBlock { w0: 1, w1: 0, w2: 0, w3: 0 },
            SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0x8000_0000 },
        ];
        for _ in 0..48 {
            blocks.push(SoftAesBlock {
                w0: rng.next_u64() as u32,
                w1: rng.next_u64() as u32,
                w2: rng.next_u64() as u32,
                w3: rng.next_u64() as u32,
            });
        }

        for b in &blocks {
            assert_eq!(cimc(*b), rimc(*b), "softaes_inv_mix_columns({b:?})");
            for rk in blocks.iter().take(8) {
                assert_eq!(cenc(*b, *rk), renc(*b, *rk), "softaes_block_encrypt");
                assert_eq!(cdec(*b, *rk), rdec(*b, *rk), "softaes_block_decrypt");
                assert_eq!(cencl(*b, *rk), rencl(*b, *rk), "softaes_block_encryptlast");
                assert_eq!(cdecl(*b, *rk), rdecl(*b, *rk), "softaes_block_decryptlast");
            }
        }

        // key schedules
        let mut keys16: Vec<Vec<u8>> = vec![vec![0u8; 16], vec![0xffu8; 16]];
        let mut keys32: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
        for _ in 0..16 {
            keys16.push(rng.vec(16));
            keys32.push(rng.vec(32));
        }
        for k in &keys16 {
            let mut ck = vec![zero; 11];
            let mut rk = vec![zero; 11];
            cek128(ck.as_mut_ptr(), k.as_ptr());
            rek128(rk.as_mut_ptr(), k.as_ptr());
            assert_bytes_eq(
                &format!("softaes_expand_key128({})", hex(k)),
                &as_bytes(&ck),
                &as_bytes(&rk),
            );
            let mut ck2 = ck.clone();
            let mut rk2 = rk.clone();
            cik128(ck2.as_mut_ptr());
            rik128(rk2.as_mut_ptr());
            assert_bytes_eq(
                &format!("softaes_invert_key_schedule128({})", hex(k)),
                &as_bytes(&ck2),
                &as_bytes(&rk2),
            );
        }
        for k in &keys32 {
            let mut ck = vec![zero; 15];
            let mut rk = vec![zero; 15];
            cek256(ck.as_mut_ptr(), k.as_ptr());
            rek256(rk.as_mut_ptr(), k.as_ptr());
            assert_bytes_eq(
                &format!("softaes_expand_key256({})", hex(k)),
                &as_bytes(&ck),
                &as_bytes(&rk),
            );
            let mut ck2 = ck.clone();
            let mut rk2 = rk.clone();
            cik256(ck2.as_mut_ptr());
            rik256(rk2.as_mut_ptr());
            assert_bytes_eq(
                &format!("softaes_invert_key_schedule256({})", hex(k)),
                &as_bytes(&ck2),
                &as_bytes(&rk2),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// blake2b internals
// ---------------------------------------------------------------------------

const BLAKE2B_STATE: usize = 8 * 8 + 2 * 8 + 2 * 8 + 256 + 8 + 8; // 360, padded to 360
const BLAKE2B_PARAM: usize = 64;

type FnB2Init = unsafe extern "C" fn(*mut c_void, u8) -> c_int;
type FnB2InitSp = unsafe extern "C" fn(*mut c_void, u8, *const c_void, *const c_void) -> c_int;
type FnB2InitKey = unsafe extern "C" fn(*mut c_void, u8, *const c_void, u8) -> c_int;
type FnB2InitKeySp =
    unsafe extern "C" fn(*mut c_void, u8, *const c_void, u8, *const c_void, *const c_void) -> c_int;
type FnB2InitParam = unsafe extern "C" fn(*mut c_void, *const c_void) -> c_int;
type FnB2Update = unsafe extern "C" fn(*mut c_void, *const c_uchar, u64) -> c_int;
type FnB2Final = unsafe extern "C" fn(*mut c_void, *mut c_uchar, u8) -> c_int;
type FnB2 = unsafe extern "C" fn(*mut c_uchar, *const c_void, *const c_void, u8, usize, u8) -> c_int;
type FnB2Sp = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_void,
    *const c_void,
    u8,
    usize,
    u8,
    *const c_void,
    *const c_void,
) -> c_int;
type FnB2Compress = unsafe extern "C" fn(*mut c_void, *const c_uchar) -> c_int;
type FnB2Long = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;

#[test]
fn internal_blake2b_matches() {
    unsafe {
        let (ci, ri): (FnB2Init, FnB2Init) = pair("_sodium_blake2b_init");
        let (cisp, risp): (FnB2InitSp, FnB2InitSp) = pair("_sodium_blake2b_init_salt_personal");
        let (cik, rik): (FnB2InitKey, FnB2InitKey) = pair("_sodium_blake2b_init_key");
        let (ciksp, riksp): (FnB2InitKeySp, FnB2InitKeySp) =
            pair("_sodium_blake2b_init_key_salt_personal");
        let (cip, rip): (FnB2InitParam, FnB2InitParam) = pair("_sodium_blake2b_init_param");
        let (cu, ru): (FnB2Update, FnB2Update) = pair("_sodium_blake2b_update");
        let (cf, rf): (FnB2Final, FnB2Final) = pair("_sodium_blake2b_final");
        let (c1, r1): (FnB2, FnB2) = pair("_sodium_blake2b");
        let (c1sp, r1sp): (FnB2Sp, FnB2Sp) = pair("_sodium_blake2b_salt_personal");
        let (cc, rc): (FnB2Compress, FnB2Compress) = pair("_sodium_blake2b_compress_ref");
        let (cl, rl): (FnB2Long, FnB2Long) = pair("_sodium_blake2b_long");

        let mut rng = Rng::new(0x9700);
        let msg = rng.vec(1001);
        let key = rng.vec(64);
        let salt = rng.vec(16);
        let pers = rng.vec(16);

        // The internal blake2b entry points call sodium_misuse() (abort) for
        // outlen == 0 or outlen > 64 and for keylen == 0 or keylen > 64, so the
        // sweeps below stay inside the valid ranges.
        for &outlen in &[1u8, 16, 32, 64] {
            // init / update / final
            for &inlen in &[0u64, 1, 127, 128, 129, 256, 1000] {
                let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let a = ci(cst.as_mut_ptr() as *mut c_void, outlen);
                let b = ri(rst.as_mut_ptr() as *mut c_void, outlen);
                assert_eq!(a, b, "blake2b_init({outlen}) return");
                assert_bytes_eq(
                    &format!("blake2b_init({outlen}) state"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                if a != 0 {
                    continue;
                }
                let a = cu(cst.as_mut_ptr() as *mut c_void, msg.as_ptr(), inlen);
                let b = ru(rst.as_mut_ptr() as *mut c_void, msg.as_ptr(), inlen);
                assert_eq!(a, b, "blake2b_update return");
                assert_bytes_eq(
                    &format!("blake2b_update({inlen}) state"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                let mut co = vec![0xAAu8; 72];
                let mut ro = vec![0xAAu8; 72];
                let a = cf(cst.as_mut_ptr() as *mut c_void, co.as_mut_ptr(), outlen);
                let b = rf(rst.as_mut_ptr() as *mut c_void, ro.as_mut_ptr(), outlen);
                assert_eq!(a, b, "blake2b_final return");
                assert_bytes_eq(&format!("blake2b_final({outlen},{inlen})"), &co, &ro);
                assert_bytes_eq("blake2b state after final", cst.as_slice(), rst.as_slice());
            }

            // keyed and salt/personal inits
            for &keylen in &[1u8, 32, 64] {
                let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let a = cik(
                    cst.as_mut_ptr() as *mut c_void,
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                );
                let b = rik(
                    rst.as_mut_ptr() as *mut c_void,
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                );
                assert_eq!(a, b, "blake2b_init_key({outlen},{keylen}) return");
                assert_bytes_eq(
                    &format!("blake2b_init_key({outlen},{keylen}) state"),
                    cst.as_slice(),
                    rst.as_slice(),
                );

                let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
                let a = ciksp(
                    cst.as_mut_ptr() as *mut c_void,
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
                let b = riksp(
                    rst.as_mut_ptr() as *mut c_void,
                    outlen,
                    key.as_ptr() as *const c_void,
                    keylen,
                    salt.as_ptr() as *const c_void,
                    pers.as_ptr() as *const c_void,
                );
                assert_eq!(a, b, "blake2b_init_key_salt_personal return");
                assert_bytes_eq(
                    "blake2b_init_key_salt_personal state",
                    cst.as_slice(),
                    rst.as_slice(),
                );
            }

            let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            let a = cisp(
                cst.as_mut_ptr() as *mut c_void,
                outlen,
                salt.as_ptr() as *const c_void,
                pers.as_ptr() as *const c_void,
            );
            let b = risp(
                rst.as_mut_ptr() as *mut c_void,
                outlen,
                salt.as_ptr() as *const c_void,
                pers.as_ptr() as *const c_void,
            );
            assert_eq!(a, b, "blake2b_init_salt_personal return");
            assert_bytes_eq(
                "blake2b_init_salt_personal state",
                cst.as_slice(),
                rst.as_slice(),
            );

            // one-shot forms
            for &inlen in &[0usize, 1, 128, 500] {
                for &keylen in &[1u8, 32, 64] {
                    let kptr = key.as_ptr() as *const c_void;
                    let mut co = vec![0xAAu8; 72];
                    let mut ro = vec![0xAAu8; 72];
                    let a = c1(
                        co.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kptr,
                        outlen,
                        inlen,
                        keylen,
                    );
                    let b = r1(
                        ro.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kptr,
                        outlen,
                        inlen,
                        keylen,
                    );
                    assert_eq!(a, b, "blake2b({outlen},{inlen},{keylen}) return");
                    assert_bytes_eq(&format!("blake2b({outlen},{inlen},{keylen})"), &co, &ro);

                    let mut co = vec![0xAAu8; 72];
                    let mut ro = vec![0xAAu8; 72];
                    let a = c1sp(
                        co.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kptr,
                        outlen,
                        inlen,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    );
                    let b = r1sp(
                        ro.as_mut_ptr(),
                        msg.as_ptr() as *const c_void,
                        kptr,
                        outlen,
                        inlen,
                        keylen,
                        salt.as_ptr() as *const c_void,
                        pers.as_ptr() as *const c_void,
                    );
                    assert_eq!(a, b, "blake2b_salt_personal return");
                    assert_bytes_eq("blake2b_salt_personal", &co, &ro);
                }
            }
        }

        // init_param with a hand-built parameter block
        let mut params: Vec<Vec<u8>> = Vec::new();
        for dl in [1u8, 32, 64] {
            for kl in [0u8, 32] {
                let mut p = vec![0u8; BLAKE2B_PARAM];
                p[0] = dl;
                p[1] = kl;
                p[2] = 1; // fanout
                p[3] = 1; // depth
                p[32..48].copy_from_slice(&salt);
                p[48..64].copy_from_slice(&pers);
                params.push(p);
            }
        }
        params.push(vec![0u8; BLAKE2B_PARAM]);
        params.push(vec![0xffu8; BLAKE2B_PARAM]);
        for p in &params {
            let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            let a = cip(cst.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            let b = rip(rst.as_mut_ptr() as *mut c_void, p.as_ptr() as *const c_void);
            assert_eq!(a, b, "blake2b_init_param return");
            assert_bytes_eq("blake2b_init_param state", cst.as_slice(), rst.as_slice());
        }

        // compress_ref on an initialised state with a full 128-byte block
        for &inlen in &[0u64, 128] {
            let mut cst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            let mut rst = AlignedBuf::new(BLAKE2B_STATE + 64, 0xA5);
            ci(cst.as_mut_ptr() as *mut c_void, 32);
            ri(rst.as_mut_ptr() as *mut c_void, 32);
            cu(cst.as_mut_ptr() as *mut c_void, msg.as_ptr(), inlen);
            ru(rst.as_mut_ptr() as *mut c_void, msg.as_ptr(), inlen);
            let block = rng.vec(128);
            let a = cc(cst.as_mut_ptr() as *mut c_void, block.as_ptr());
            let b = rc(rst.as_mut_ptr() as *mut c_void, block.as_ptr());
            assert_eq!(a, b, "blake2b_compress_ref return");
            assert_bytes_eq(
                &format!("blake2b_compress_ref state (after {inlen} bytes)"),
                cst.as_slice(),
                rst.as_slice(),
            );
        }

        // blake2b_long (used by argon2)
        for &outlen in &[1usize, 16, 32, 64, 65, 100, 128, 1024] {
            for &inlen in &[0usize, 1, 64, 128, 500] {
                let mut co = vec![0xAAu8; outlen + 8];
                let mut ro = vec![0xAAu8; outlen + 8];
                let a = cl(
                    co.as_mut_ptr() as *mut c_void,
                    outlen,
                    msg.as_ptr() as *const c_void,
                    inlen,
                );
                let b = rl(
                    ro.as_mut_ptr() as *mut c_void,
                    outlen,
                    msg.as_ptr() as *const c_void,
                    inlen,
                );
                assert_eq!(a, b, "blake2b_long({outlen},{inlen}) return");
                assert_bytes_eq(&format!("blake2b_long({outlen},{inlen})"), &co, &ro);
            }
        }

        cmp_int("_sodium_blake2b_pick_best_implementation");
    }
}

// ---------------------------------------------------------------------------
// keccak1600 / shake / turboshake reference implementations
// ---------------------------------------------------------------------------

#[test]
fn internal_keccak_and_xof_refs_match() {
    unsafe {
        // keccak1600 ref: identical signatures to the public wrappers
        type FnKInit = unsafe extern "C" fn(*mut c_void);
        type FnKXor = unsafe extern "C" fn(*mut c_void, *const c_uchar, usize, usize);
        type FnKExtract = unsafe extern "C" fn(*const c_void, *mut c_uchar, usize, usize);
        type FnKPermute = unsafe extern "C" fn(*mut c_void);

        let (ci, ri): (FnKInit, FnKInit) = pair("_sodium_keccak1600_ref_init");
        let (cx, rx): (FnKXor, FnKXor) = pair("_sodium_keccak1600_ref_xor_bytes");
        let (ce, re): (FnKExtract, FnKExtract) = pair("_sodium_keccak1600_ref_extract_bytes");
        let (c24, r24): (FnKPermute, FnKPermute) = pair("_sodium_keccak1600_ref_permute_24");
        let (c12, r12): (FnKPermute, FnKPermute) = pair("_sodium_keccak1600_ref_permute_12");

        let mut cst = AlignedBuf::new(256, 0xA5);
        let mut rst = AlignedBuf::new(256, 0xA5);
        ci(cst.as_mut_ptr() as *mut c_void);
        ri(rst.as_mut_ptr() as *mut c_void);
        assert_bytes_eq("keccak1600_ref_init", cst.as_slice(), rst.as_slice());

        let mut rng = Rng::new(0x9800);
        for round in 0..30 {
            let off = (rng.byte() as usize) % 200;
            let len = (rng.byte() as usize) % (200 - off).max(1);
            let data = rng.vec(len.max(1));
            cx(cst.as_mut_ptr() as *mut c_void, data.as_ptr(), off, len);
            rx(rst.as_mut_ptr() as *mut c_void, data.as_ptr(), off, len);
            assert_bytes_eq(
                &format!("keccak1600_ref_xor_bytes round {round}"),
                cst.as_slice(),
                rst.as_slice(),
            );
            if round % 2 == 0 {
                c24(cst.as_mut_ptr() as *mut c_void);
                r24(rst.as_mut_ptr() as *mut c_void);
            } else {
                c12(cst.as_mut_ptr() as *mut c_void);
                r12(rst.as_mut_ptr() as *mut c_void);
            }
            assert_bytes_eq(
                &format!("keccak1600_ref permute round {round}"),
                cst.as_slice(),
                rst.as_slice(),
            );
            let mut cb = vec![0x55u8; 208];
            let mut rb = vec![0x55u8; 208];
            ce(cst.as_ptr() as *const c_void, cb.as_mut_ptr(), 0, 200);
            re(rst.as_ptr() as *const c_void, rb.as_mut_ptr(), 0, 200);
            assert_bytes_eq(
                &format!("keccak1600_ref_extract_bytes round {round}"),
                &cb,
                &rb,
            );
        }

        // shake / turboshake reference entry points
        type FnXof = unsafe extern "C" fn(*mut c_uchar, usize, *const c_uchar, c_ulonglong) -> c_int;
        type FnXofInit = unsafe extern "C" fn(*mut c_void) -> c_int;
        type FnXofInitDomain = unsafe extern "C" fn(*mut c_void, c_uchar) -> c_int;
        type FnXofUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, c_ulonglong) -> c_int;
        type FnXofSqueeze = unsafe extern "C" fn(*mut c_void, *mut c_uchar, usize) -> c_int;

        let msg = rng.vec(1001);
        for name in [
            "_sodium_shake128_ref",
            "_sodium_shake256_ref",
            "_sodium_turboshake128_ref",
            "_sodium_turboshake256_ref",
        ] {
            let (c1, r1): (FnXof, FnXof) = pair(name);
            let (cinit, rinit): (FnXofInit, FnXofInit) = pair(&format!("{name}_init"));
            let (cidom, ridom): (FnXofInitDomain, FnXofInitDomain) =
                pair(&format!("{name}_init_with_domain"));
            let (cup, rup): (FnXofUpdate, FnXofUpdate) = pair(&format!("{name}_update"));
            let (csq, rsq): (FnXofSqueeze, FnXofSqueeze) = pair(&format!("{name}_squeeze"));

            for &outlen in &[0usize, 1, 31, 32, 33, 135, 136, 137, 167, 168, 169, 500] {
                for &inlen in &[0usize, 1, 135, 136, 137, 168, 500, 1000] {
                    let mut co = vec![0xAAu8; outlen + 8];
                    let mut ro = vec![0xAAu8; outlen + 8];
                    let a = c1(co.as_mut_ptr(), outlen, msg.as_ptr(), inlen as c_ulonglong);
                    let b = r1(ro.as_mut_ptr(), outlen, msg.as_ptr(), inlen as c_ulonglong);
                    assert_eq!(a, b, "{name}({outlen},{inlen}) return");
                    assert_bytes_eq(&format!("{name}({outlen},{inlen})"), &co, &ro);
                }
            }

            for dom in [0u16, 1, 0x1f, 0x7f, 0x80, 0xff, 0x100 - 1] {
                let dom = (dom & 0xff) as u8;
                let mut cst = AlignedBuf::new(256, 0xA5);
                let mut rst = AlignedBuf::new(256, 0xA5);
                let a = cidom(cst.as_mut_ptr() as *mut c_void, dom);
                let b = ridom(rst.as_mut_ptr() as *mut c_void, dom);
                assert_eq!(a, b, "{name}_init_with_domain({dom}) return");
                assert_bytes_eq(
                    &format!("{name}_init_with_domain({dom}) state"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                if a != 0 {
                    continue;
                }
                cup(cst.as_mut_ptr() as *mut c_void, msg.as_ptr(), 300);
                rup(rst.as_mut_ptr() as *mut c_void, msg.as_ptr(), 300);
                assert_bytes_eq(
                    &format!("{name} state after update"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
                for sq in [1usize, 0, 100, 200, 337] {
                    let mut cb = vec![0xAAu8; sq + 8];
                    let mut rb = vec![0xAAu8; sq + 8];
                    let a = csq(cst.as_mut_ptr() as *mut c_void, cb.as_mut_ptr(), sq);
                    let b = rsq(rst.as_mut_ptr() as *mut c_void, rb.as_mut_ptr(), sq);
                    assert_eq!(a, b, "{name}_squeeze({sq}) return");
                    assert_bytes_eq(&format!("{name}_squeeze({sq})"), &cb, &rb);
                    assert_bytes_eq(
                        &format!("{name} state after squeeze({sq})"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                }
            }

            // plain init path
            let mut cst = AlignedBuf::new(256, 0xA5);
            let mut rst = AlignedBuf::new(256, 0xA5);
            let a = cinit(cst.as_mut_ptr() as *mut c_void);
            let b = rinit(rst.as_mut_ptr() as *mut c_void);
            assert_eq!(a, b, "{name}_init return");
            assert_bytes_eq(&format!("{name}_init state"), cst.as_slice(), rst.as_slice());
        }
    }
}

// ---------------------------------------------------------------------------
// pick_best_implementation entry points and the runtime feature block
// ---------------------------------------------------------------------------

#[test]
fn internal_pick_best_implementation_matches() {
    for f in [
        "_crypto_aead_aegis128l_pick_best_implementation",
        "_crypto_aead_aegis256_pick_best_implementation",
        "_crypto_generichash_blake2b_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
        "_crypto_onetimeauth_poly1305_pick_best_implementation",
        "_crypto_pwhash_argon2_pick_best_implementation",
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_sodium_blake2b_pick_best_implementation",
        "_sodium_runtime_get_cpu_features",
        "_sodium_alloc_init",
    ] {
        cmp_int(f);
    }
}
