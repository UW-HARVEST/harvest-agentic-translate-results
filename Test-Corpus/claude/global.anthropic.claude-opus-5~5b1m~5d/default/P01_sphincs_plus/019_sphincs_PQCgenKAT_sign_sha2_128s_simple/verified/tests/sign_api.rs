//! Phase B rows 39-53: FORS, Merkle and the full `crypto_sign*` API, driven end
//! to end through both `.so`s (including the cross-checks where one library
//! signs and the other verifies).

mod common;
use common::*;
use libloading::Symbol;

// void fors_sign(u8 *sig, u8 *pk, const u8 *m, const spx_ctx *ctx,
//                const u32 fors_addr[8]);
type FForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
// void fors_pk_from_sig(u8 *pk, const u8 *sig, const u8 *m, const spx_ctx *ctx,
//                       const u32 fors_addr[8]);
type FForsPk = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
// void merkle_sign(u8 *sig, u8 *root, const spx_ctx *ctx,
//                  u32 wots_addr[8], u32 tree_addr[8], u32 idx_leaf);
type FMerkleSign =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
// void merkle_gen_root(u8 *root, const spx_ctx *ctx);
type FMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

type FSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FSignature =
    unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FSignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;

const MERKLE_SIG_BYTES: usize = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

// ---------------------------------------------------------------------------
// Rows 39-42 -- FORS
// ---------------------------------------------------------------------------
fn fors_sign_pair(
    m: &[u8],
    fors_addr: &[u32; 8],
    cctx: &[u8],
    rctx: &[u8],
) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let (c, r) = both!("SPX_fors_sign", FForsSign);
    let mut csig = vec![0xA5u8; SPX_FORS_BYTES + 32];
    let mut rsig = vec![0xA5u8; SPX_FORS_BYTES + 32];
    let mut cpk = vec![0xA5u8; SPX_N + 32];
    let mut rpk = vec![0xA5u8; SPX_N + 32];
    unsafe {
        c(
            csig.as_mut_ptr(),
            cpk.as_mut_ptr(),
            m.as_ptr(),
            cctx.as_ptr(),
            fors_addr.as_ptr(),
        );
        r(
            rsig.as_mut_ptr(),
            rpk.as_mut_ptr(),
            m.as_ptr(),
            rctx.as_ptr(),
            fors_addr.as_ptr(),
        );
    }
    (csig, rsig, cpk, rpk)
}

#[test]
fn row39_fors_sign_random() {
    let mut rng = Rng::new(RNG_SEED ^ 39);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for _ in 0..N_ITER_SLOW + 1 {
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let addr = rng.addr();
        let (cs, rs, cp, rp) = fors_sign_pair(&m, &addr, &cctx, &rctx);
        eq_bytes("fors_sign sig", &cs, &rs);
        eq_bytes("fors_sign pk", &cp, &rp);
    }
}

#[test]
fn row40_fors_sign_message_extremes() {
    let mut rng = Rng::new(RNG_SEED ^ 40);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for fill in [0x00u8, 0xFF] {
        let m = vec![fill; SPX_FORS_MSG_BYTES];
        for addr in [[0u32; 8], [u32::MAX; 8]] {
            let (cs, rs, cp, rp) = fors_sign_pair(&m, &addr, &cctx, &rctx);
            eq_bytes(&format!("fors_sign sig (m=all {fill:#02x})"), &cs, &rs);
            eq_bytes(&format!("fors_sign pk (m=all {fill:#02x})"), &cp, &rp);
        }
    }
}

#[test]
fn row41_fors_pk_from_sig() {
    let (c, r) = both!("SPX_fors_pk_from_sig", FForsPk);
    let mut rng = Rng::new(RNG_SEED ^ 41);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    // (a) a genuine signature, (b) purely random bytes of the right length.
    let m = rng.bytes(SPX_FORS_MSG_BYTES);
    let addr = rng.addr();
    let (real_sig, _, _, _) = fors_sign_pair(&m, &addr, &cctx, &rctx);
    let mut sigs = vec![real_sig[..SPX_FORS_BYTES].to_vec()];
    for _ in 0..N_ITER_SLOW {
        sigs.push(rng.bytes(SPX_FORS_BYTES));
    }
    for sig in &sigs {
        for mm in [m.clone(), vec![0x00; SPX_FORS_MSG_BYTES], vec![0xFF; SPX_FORS_MSG_BYTES]] {
            let mut cp = vec![0xA5u8; SPX_N + 32];
            let mut rp = vec![0xA5u8; SPX_N + 32];
            unsafe {
                c(
                    cp.as_mut_ptr(),
                    sig.as_ptr(),
                    mm.as_ptr(),
                    cctx.as_ptr(),
                    addr.as_ptr(),
                );
                r(
                    rp.as_mut_ptr(),
                    sig.as_ptr(),
                    mm.as_ptr(),
                    rctx.as_ptr(),
                    addr.as_ptr(),
                );
            }
            eq_bytes("fors_pk_from_sig pk", &cp, &rp);
        }
    }
}

#[test]
fn row42_fors_sign_then_pk_from_sig() {
    let (cpk_f, rpk_f) = both!("SPX_fors_pk_from_sig", FForsPk);
    let mut rng = Rng::new(RNG_SEED ^ 42);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for _ in 0..N_ITER_SLOW {
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let addr = rng.addr();
        let (csig, rsig, cpk, rpk) = fors_sign_pair(&m, &addr, &cctx, &rctx);
        eq_bytes("fors round-trip sig", &csig, &rsig);

        // Recover from the C signature using BOTH libraries, and vice versa.
        for (label, sig) in [("C-sig", &csig), ("Rust-sig", &rsig)] {
            let mut co = vec![0u8; SPX_N];
            let mut ro = vec![0u8; SPX_N];
            unsafe {
                cpk_f(
                    co.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    cctx.as_ptr(),
                    addr.as_ptr(),
                );
                rpk_f(
                    ro.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    rctx.as_ptr(),
                    addr.as_ptr(),
                );
            }
            eq_bytes(&format!("fors recover ({label})"), &co, &ro);
            eq_bytes(
                &format!("fors recovered pk == signing pk ({label}) [C]"),
                &co,
                &cpk[..SPX_N],
            );
            eq_bytes(
                &format!("fors recovered pk == signing pk ({label}) [Rust]"),
                &ro,
                &rpk[..SPX_N],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 43-45 -- Merkle
// ---------------------------------------------------------------------------
fn merkle_sign_case(idx_leaf: u32, seed: u64) {
    let (c, r) = both!("SPX_merkle_sign", FMerkleSign);
    let mut rng = Rng::new(seed);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for _ in 0..N_ITER_SLOW {
        let wots_addr = rng.addr();
        let tree_addr = rng.addr();
        let root_in = rng.bytes(SPX_N);

        let mut csig = vec![0xA5u8; MERKLE_SIG_BYTES + 32];
        let mut rsig = vec![0xA5u8; MERKLE_SIG_BYTES + 32];
        let mut croot = root_in.clone();
        let mut rroot = root_in.clone();
        let mut cwa = wots_addr;
        let mut rwa = wots_addr;
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        unsafe {
            c(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                cctx.as_ptr(),
                cwa.as_mut_ptr(),
                cta.as_mut_ptr(),
                idx_leaf,
            );
            r(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                rctx.as_ptr(),
                rwa.as_mut_ptr(),
                rta.as_mut_ptr(),
                idx_leaf,
            );
        }
        eq_bytes(&format!("merkle_sign(idx_leaf={idx_leaf}) sig"), &csig, &rsig);
        eq_bytes(
            &format!("merkle_sign(idx_leaf={idx_leaf}) root"),
            &croot,
            &rroot,
        );
        eq(
            &format!("merkle_sign(idx_leaf={idx_leaf}) wots_addr"),
            cwa,
            rwa,
        );
        eq(
            &format!("merkle_sign(idx_leaf={idx_leaf}) tree_addr"),
            cta,
            rta,
        );
    }
}

#[test]
fn row43_merkle_sign() {
    let th = SPX_TREE_HEIGHT as u32;
    for idx in [0u32, 1, (1u32 << th) - 1, (1u32 << th) / 2] {
        merkle_sign_case(idx, RNG_SEED ^ 43 ^ (idx as u64));
    }
}

#[test]
fn row44_merkle_sign_gen_root_sentinel() {
    // idx_leaf = ~0u -- the value merkle_gen_root passes, so no leaf ever
    // matches wots_sign_leaf and no WOTS signature is emitted.
    merkle_sign_case(u32::MAX, RNG_SEED ^ 44);
}

#[test]
fn row45_merkle_gen_root() {
    let (c, r) = both!("SPX_merkle_gen_root", FMerkleGenRoot);
    let mut rng = Rng::new(RNG_SEED ^ 45);
    for iter in 0..N_ITER_SLOW + 2 {
        let (ps, ss) = match iter {
            0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cctx, rctx) = init_ctx_pair(&ps, &ss);
        let mut cr = vec![0xA5u8; SPX_N + 32];
        let mut rr = vec![0xA5u8; SPX_N + 32];
        unsafe {
            c(cr.as_mut_ptr(), cctx.as_ptr());
            r(rr.as_mut_ptr(), rctx.as_ptr());
        }
        eq_bytes(&format!("merkle_gen_root (pub_seed={})", hex(&ps)), &cr, &rr);
    }
}

// ---------------------------------------------------------------------------
// Rows 46-47 -- key generation
// ---------------------------------------------------------------------------

/// Generate one keypair with a given seed in both libraries; asserts equality
/// and returns the (pk, sk) pair.
pub fn keypair_from_seed(seed: &[u8]) -> (Vec<u8>, Vec<u8>) {
    assert_eq!(seed.len(), CRYPTO_SEEDBYTES);
    let (c, r) = both!("crypto_sign_seed_keypair", FSeedKeypair);
    let mut cpk = vec![0xA5u8; SPX_PK_BYTES + 32];
    let mut rpk = vec![0xA5u8; SPX_PK_BYTES + 32];
    let mut csk = vec![0xA5u8; SPX_SK_BYTES + 32];
    let mut rsk = vec![0xA5u8; SPX_SK_BYTES + 32];
    let (crc, rrc) = unsafe {
        (
            c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
            r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
        )
    };
    eq("crypto_sign_seed_keypair return", crc, rrc);
    eq("crypto_sign_seed_keypair return == 0", crc, 0);
    eq_bytes("crypto_sign_seed_keypair pk", &cpk, &rpk);
    eq_bytes("crypto_sign_seed_keypair sk", &csk, &rsk);
    (
        cpk[..SPX_PK_BYTES].to_vec(),
        csk[..SPX_SK_BYTES].to_vec(),
    )
}

#[test]
fn row46_crypto_sign_seed_keypair() {
    let _g = drbg_guard();
    let mut rng = Rng::new(RNG_SEED ^ 46);
    for iter in 0..N_ITER_SLOW + 2 {
        let seed = match iter {
            0 => vec![0x00u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        keypair_from_seed(&seed);
    }
}

#[test]
fn row47_crypto_sign_keypair_via_drbg() {
    let _g = drbg_guard();
    // Both libraries own an independent DRBG_ctx; seeding both with the same
    // entropy makes crypto_sign_keypair (which draws its seed from
    // randombytes) fully deterministic and comparable.
    seed_both_drbgs(&kat_entropy());
    let (c, r) = both!("crypto_sign_keypair", FKeypair);
    for i in 0..3 {
        let mut cpk = vec![0xA5u8; SPX_PK_BYTES + 32];
        let mut rpk = vec![0xA5u8; SPX_PK_BYTES + 32];
        let mut csk = vec![0xA5u8; SPX_SK_BYTES + 32];
        let mut rsk = vec![0xA5u8; SPX_SK_BYTES + 32];
        let (crc, rrc) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr()),
            )
        };
        eq(&format!("crypto_sign_keypair[{i}] return"), crc, rrc);
        eq_bytes(&format!("crypto_sign_keypair[{i}] pk"), &cpk, &rpk);
        eq_bytes(&format!("crypto_sign_keypair[{i}] sk"), &csk, &rsk);
        // And the DRBG must have advanced identically.
        let (cd, rd) = both_data!("DRBG_ctx", [u8; 52]);
        eq_bytes(
            &format!("DRBG_ctx after crypto_sign_keypair[{i}]"),
            unsafe { &*cd },
            unsafe { &*rd },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 48-50 -- detached signature + verify
// ---------------------------------------------------------------------------

/// Produce a detached signature in both libraries over the same message and
/// assert byte equality. Both DRBGs must already be seeded identically because
/// `crypto_sign_signature` draws `optrand` from `randombytes`.
pub fn signature_pair(sk: &[u8], m: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = both!("crypto_sign_signature", FSignature);
    let mut csig = vec![0xA5u8; SPX_BYTES + 32];
    let mut rsig = vec![0xA5u8; SPX_BYTES + 32];
    let mut cl: usize = usize::MAX;
    let mut rl: usize = usize::MAX;
    let (crc, rrc) = unsafe {
        (
            c(
                csig.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                m.len(),
                sk.as_ptr(),
            ),
            r(
                rsig.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                m.len(),
                sk.as_ptr(),
            ),
        )
    };
    eq("crypto_sign_signature return", crc, rrc);
    eq("crypto_sign_signature return == 0", crc, 0);
    eq("crypto_sign_signature *siglen", cl, rl);
    eq("crypto_sign_signature *siglen == SPX_BYTES", cl, SPX_BYTES);
    eq_bytes("crypto_sign_signature sig", &csig, &rsig);
    (
        csig[..SPX_BYTES].to_vec(),
        rsig[..SPX_BYTES].to_vec(),
    )
}

/// One shared keypair for the (expensive) sign/verify rows. Key generation is a
/// full top-subtree Merkle build, so on the `*s` parameter sets it costs
/// seconds. The DRBG guard already serialises these tests, so sharing the
/// keypair changes nothing observable -- row 46 still covers keygen itself with
/// all-00 / all-FF / random seeds.
fn shared_keypair() -> &'static (Vec<u8>, Vec<u8>) {
    use std::sync::OnceLock;
    static KP: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    KP.get_or_init(|| {
        let mut rng = Rng::new(RNG_SEED ^ 0xA11CE);
        keypair_from_seed(&rng.bytes(CRYPTO_SEEDBYTES))
    })
}

#[test]
fn row48_crypto_sign_signature() {
    let _g = drbg_guard();
    let mut rng = Rng::new(RNG_SEED ^ 48);
    let (_pk, sk) = shared_keypair().clone();
    // The mlen axis itself is covered exhaustively (24 lengths straddling every
    // backend's block/rate boundary) at the level where the C actually branches
    // on it -- `SPX_gen_message_random` / `SPX_hash_message`, rows 25-27. These
    // whole-signature rows only need to confirm the composed pipeline, and a
    // single `*s` signature costs seconds, so they use a small set of lengths
    // (empty, non-block-aligned, and a rate multiple).
    for &mlen in &[0usize, 33, 136] {
        seed_both_drbgs(&kat_entropy());
        let m = rng.bytes(mlen.max(1));
        signature_pair(&sk, &m[..mlen]);
    }
}

#[test]
fn row49_crypto_sign_verify_accepts() {
    let _g = drbg_guard();
    let (cv, rv) = both!("crypto_sign_verify", FVerify);
    let mut rng = Rng::new(RNG_SEED ^ 49);
    let (pk, sk) = shared_keypair().clone();
    for &mlen in &[0usize, 33] {
        seed_both_drbgs(&kat_entropy());
        let m = rng.bytes(mlen.max(1));
        let m = &m[..mlen];
        let (csig, _rsig) = signature_pair(&sk, m);
        let (crc, rrc) = unsafe {
            (
                cv(csig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()),
                rv(csig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()),
            )
        };
        eq(&format!("verify(mlen={mlen}) return"), crc, rrc);
        eq(&format!("verify(mlen={mlen}) accepted"), crc, 0);
    }
}

#[test]
fn row50_cross_verify() {
    let _g = drbg_guard();
    // C signs -> Rust verifies, and Rust signs -> C verifies.
    let (cv, rv) = both!("crypto_sign_verify", FVerify);
    let mut rng = Rng::new(RNG_SEED ^ 50);
    let (pk, sk) = shared_keypair().clone();
    for &mlen in &[0usize, 33] {
        seed_both_drbgs(&kat_entropy());
        let m = rng.bytes(mlen.max(1));
        let m = &m[..mlen];
        let (csig, rsig) = signature_pair(&sk, m);
        // The two signatures are already asserted equal, but verify both ways
        // anyway -- this is the property an external consumer relies on.
        let a = unsafe { rv(csig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
        let b = unsafe { cv(rsig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
        eq(&format!("Rust verifies C sig (mlen={mlen})"), a, 0);
        eq(&format!("C verifies Rust sig (mlen={mlen})"), b, 0);
    }
}

// ---------------------------------------------------------------------------
// Rows 51-53 -- the attached-signature API
// ---------------------------------------------------------------------------

pub fn sign_pair(sk: &[u8], m: &[u8]) -> (Vec<u8>, u64) {
    let (c, r) = both!("crypto_sign", FSign);
    let n = SPX_BYTES + m.len();
    let mut csm = vec![0xA5u8; n + 32];
    let mut rsm = vec![0xA5u8; n + 32];
    let mut cl: u64 = u64::MAX;
    let mut rl: u64 = u64::MAX;
    let (crc, rrc) = unsafe {
        (
            c(
                csm.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                m.len() as u64,
                sk.as_ptr(),
            ),
            r(
                rsm.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                m.len() as u64,
                sk.as_ptr(),
            ),
        )
    };
    eq("crypto_sign return", crc, rrc);
    eq("crypto_sign return == 0", crc, 0);
    eq("crypto_sign *smlen", cl, rl);
    eq("crypto_sign *smlen value", cl, n as u64);
    eq_bytes("crypto_sign sm", &csm, &rsm);
    (csm[..n].to_vec(), cl)
}

#[test]
fn row51_crypto_sign() {
    let _g = drbg_guard();
    let mut rng = Rng::new(RNG_SEED ^ 51);
    let (_pk, sk) = shared_keypair().clone();
    for &mlen in &[0usize, 33] {
        seed_both_drbgs(&kat_entropy());
        let m = rng.bytes(mlen.max(1));
        sign_pair(&sk, &m[..mlen]);
    }
}

#[test]
fn row52_crypto_sign_open() {
    let _g = drbg_guard();
    let (c, r) = both!("crypto_sign_open", FSignOpen);
    let mut rng = Rng::new(RNG_SEED ^ 52);
    let (pk, sk) = shared_keypair().clone();
    for &mlen in &[0usize, 33] {
        seed_both_drbgs(&kat_entropy());
        let m = rng.bytes(mlen.max(1));
        let m = &m[..mlen];
        let (sm, smlen) = sign_pair(&sk, m);
        let mut cm = vec![0xA5u8; sm.len() + 32];
        let mut rm = vec![0xA5u8; sm.len() + 32];
        let mut cl: u64 = u64::MAX;
        let mut rl: u64 = u64::MAX;
        let (crc, rrc) = unsafe {
            (
                c(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()),
                r(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()),
            )
        };
        eq(&format!("crypto_sign_open(mlen={mlen}) return"), crc, rrc);
        eq(&format!("crypto_sign_open(mlen={mlen}) accepted"), crc, 0);
        eq(&format!("crypto_sign_open(mlen={mlen}) *mlen"), cl, rl);
        eq(
            &format!("crypto_sign_open(mlen={mlen}) *mlen value"),
            cl,
            mlen as u64,
        );
        eq_bytes(&format!("crypto_sign_open(mlen={mlen}) m"), &cm, &rm);
        eq_bytes(
            &format!("crypto_sign_open(mlen={mlen}) recovered message"),
            &cm[..mlen],
            m,
        );
    }
}

#[test]
fn row53_sign_open_roundtrip_crossed() {
    let _g = drbg_guard();
    let (co, ro) = both!("crypto_sign_open", FSignOpen);
    let (cs, rs) = both!("crypto_sign", FSign);
    let mut rng = Rng::new(RNG_SEED ^ 53);
    let (pk, sk) = shared_keypair().clone();
    for &mlen in &[0usize, 33] {
        let m = rng.bytes(mlen.max(1));
        let m = &m[..mlen];
        let n = SPX_BYTES + mlen;

        // Sign once with each library (DRBGs re-seeded so optrand matches).
        seed_both_drbgs(&kat_entropy());
        let mut csm = vec![0u8; n];
        let mut rsm = vec![0u8; n];
        let mut cl = 0u64;
        let mut rl = 0u64;
        unsafe {
            cs(
                csm.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
            rs(
                rsm.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen as u64,
                sk.as_ptr(),
            );
        }
        eq_bytes("crossed: sm equal", &csm, &rsm);

        // Open each with the *other* library.
        for (label, sm, len, f) in [
            ("C-signed opened by Rust", &csm, cl, &ro),
            ("Rust-signed opened by C", &rsm, rl, &co),
        ] {
            let mut out = vec![0u8; n];
            let mut ol = 0u64;
            let rc = unsafe { f(out.as_mut_ptr(), &mut ol, sm.as_ptr(), len, pk.as_ptr()) };
            eq(&format!("{label}: return"), rc, 0);
            eq(&format!("{label}: *mlen"), ol, mlen as u64);
            eq_bytes(&format!("{label}: message"), &out[..mlen], m);
        }
    }
}
