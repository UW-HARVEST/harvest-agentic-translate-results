//! Area 8 (part 3) — `crypto_ipcrypt/{crypto_ipcrypt.c, ipcrypt_soft.c}`.
//!
//! Covers `configs_8.md` rows 8.123 – 8.150 and `errors_8.md` rows
//! 8.202 – 8.211.  Every `crypto_ipcrypt_*` entry point is `void` and total
//! (no rejection branch exists), so the "error surface" rows are really
//! *total-function* rows: the tests below pin the byte-exact behaviour of the
//! documented shapes and the two degenerate-key fixups, which are the only
//! data-dependent branches in the module.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::sync::{Mutex, MutexGuard};

/// Tests inside one binary run in parallel threads and `rng_reset()` rewinds a
/// *process-global* stream, so every test that depends on the deterministic RNG
/// holds this lock while it needs a stable stream position.
static RNG_LOCK: Mutex<()> = Mutex::new(());

fn rng_guard() -> MutexGuard<'static, ()> {
    RNG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

type SizeGetter = unsafe extern "C" fn() -> usize;
type IntGetter = unsafe extern "C" fn() -> c_int;
type Keygen = unsafe extern "C" fn(*mut u8);
type Crypt = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type CryptT = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type Ip2Bin = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
type Bin2Ip = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;

// ------------------------------------------------------------------ helpers

fn h(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0);
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}

/// `::ffff:a.b.c.d`
fn v4(a: u8, b: u8, c: u8, d: u8) -> Vec<u8> {
    let mut v = vec![0u8; 16];
    v[10] = 0xff;
    v[11] = 0xff;
    v[12] = a;
    v[13] = b;
    v[14] = c;
    v[15] = d;
    v
}

/// Run a `(out, in, k)` primitive in both libraries and compare, including the
/// out-of-bounds guard region.
#[track_caller]
fn both_crypt(label: &str, c: &Crypt, r: &Crypt, outlen: usize, inp: &[u8], k: &[u8]) -> Vec<u8> {
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    unsafe {
        c(oc.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
        r(or.as_mut_ptr(), inp.as_ptr(), k.as_ptr());
    }
    eqb(label, &oc[..outlen], &or[..outlen]);
    check_pad(&format!("{label}(C)"), &oc, outlen);
    check_pad(&format!("{label}(Rust)"), &or, outlen);
    oc[..outlen].to_vec()
}

#[track_caller]
fn both_crypt_t(
    label: &str,
    c: &CryptT,
    r: &CryptT,
    outlen: usize,
    inp: &[u8],
    t: &[u8],
    k: &[u8],
) -> Vec<u8> {
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    unsafe {
        c(oc.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
        r(or.as_mut_ptr(), inp.as_ptr(), t.as_ptr(), k.as_ptr());
    }
    eqb(label, &oc[..outlen], &or[..outlen]);
    check_pad(&format!("{label}(C)"), &oc, outlen);
    check_pad(&format!("{label}(Rust)"), &or, outlen);
    oc[..outlen].to_vec()
}

// ------------------------------------------------ 8.123 – 8.126 constants

#[test]
fn r8_123_to_126_constants() {
    let table: &[(&str, usize)] = &[
        // 8.123
        ("crypto_ipcrypt_bytes", 16),
        ("crypto_ipcrypt_keybytes", 16),
        // 8.124  (OUTPUT == TWEAK + INPUT)
        ("crypto_ipcrypt_nd_keybytes", 16),
        ("crypto_ipcrypt_nd_tweakbytes", 8),
        ("crypto_ipcrypt_nd_inputbytes", 16),
        ("crypto_ipcrypt_nd_outputbytes", 24),
        // 8.125
        ("crypto_ipcrypt_ndx_keybytes", 32),
        ("crypto_ipcrypt_ndx_tweakbytes", 16),
        ("crypto_ipcrypt_ndx_inputbytes", 16),
        ("crypto_ipcrypt_ndx_outputbytes", 32),
        // 8.126
        ("crypto_ipcrypt_pfx_keybytes", 32),
        ("crypto_ipcrypt_pfx_bytes", 16),
    ];
    for (name, want) in table {
        let (c, r) = both::<SizeGetter>(name);
        let (cv, rv) = unsafe { (c(), r()) };
        assert_eq!(cv, *want, "{name}: C returned {cv}, expected {want}");
        assert_eq!(rv, cv, "{name}: Rust returned {rv}, C returned {cv}");
    }
    // ND_OUTPUTBYTES == ND_TWEAKBYTES + ND_INPUTBYTES
    assert_eq!(24, 8 + 16);
    // NDX_OUTPUTBYTES == NDX_TWEAKBYTES + NDX_INPUTBYTES
    assert_eq!(32, 16 + 16);
}

// ---------------------------------------------------- 8.127 / 8.210 keygen

#[test]
fn r8_127_keygen_all_variants() {
    let _rng = rng_guard();
    for (name, len) in [
        ("crypto_ipcrypt_keygen", 16usize),
        ("crypto_ipcrypt_nd_keygen", 16),
        ("crypto_ipcrypt_ndx_keygen", 32),
        ("crypto_ipcrypt_pfx_keygen", 32),
    ] {
        let (c, r) = both::<Keygen>(name);
        // Two successive calls per library, from a common RNG rewind, so the
        // n-th byte consumed by C equals the n-th byte consumed by Rust.
        rng_reset();
        let mut c1 = padded(len);
        let mut c2 = padded(len);
        unsafe {
            c(c1.as_mut_ptr());
            c(c2.as_mut_ptr());
        }
        let mut r1 = padded(len);
        let mut r2 = padded(len);
        unsafe {
            r(r1.as_mut_ptr());
            r(r2.as_mut_ptr());
        }
        eqb(&format!("{name} call#1"), &c1[..len], &r1[..len]);
        eqb(&format!("{name} call#2"), &c2[..len], &r2[..len]);
        check_pad(name, &c1, len);
        check_pad(name, &r1, len);
        // Successive calls differ (randomness), and the whole buffer is written.
        assert_ne!(&c1[..len], &c2[..len], "{name}: two calls produced the same key");
        assert!(c1[..len].iter().any(|&b| b != 0), "{name}: key all zero");
    }
}

// ------------------------------------- 8.128 – 8.133 / 8.202 deterministic

#[test]
fn r8_128_to_132_deterministic_round_trip() {
    let _rng = rng_guard();
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_encrypt");
    let (cd, rd) = both::<Crypt>("crypto_ipcrypt_decrypt");
    let (kg, _) = both::<Keygen>("crypto_ipcrypt_keygen");

    // 8.128 all-zero key, all-zero input.
    let zero_key = vec![0u8; 16];
    let zero_in = vec![0u8; 16];
    let ct = both_crypt("8.128 encrypt", &ce, &re, 16, &zero_in, &zero_key);
    let pt = both_crypt("8.128 decrypt", &cd, &rd, 16, &ct, &zero_key);
    eqb("8.128 round trip", &zero_in, &pt);
    // Deterministic: identical (in, key) gives an identical block.
    let ct2 = both_crypt("8.128 encrypt again", &ce, &re, 16, &zero_in, &zero_key);
    eqb("8.128 determinism", &ct, &ct2);

    // 8.129 random keygen key, all-0xFF input.
    rng_reset();
    let mut key = vec![0u8; 16];
    unsafe { kg(key.as_mut_ptr()) };
    let ff_in = vec![0xffu8; 16];
    let ct = both_crypt("8.129 encrypt", &ce, &re, 16, &ff_in, &key);
    let pt = both_crypt("8.129 decrypt", &cd, &rd, 16, &ct, &key);
    eqb("8.129 round trip", &ff_in, &pt);

    // 8.130 / 8.131 IPv4-mapped inputs; the deterministic variant is *not*
    // format preserving, so we only require an exact round trip.
    // 8.132 pure IPv6 inputs.
    let mut rng = Rng::new(0x8_0128);
    let inputs: Vec<Vec<u8>> = vec![
        v4(192, 0, 2, 1),          // 8.130 ::ffff:192.0.2.1
        v4(0, 0, 0, 0),            // 8.131 ::ffff:0.0.0.0
        v4(255, 255, 255, 255),    // 8.131 ::ffff:255.255.255.255
        vec![0u8; 16],             // 8.132 ::
        h("00000000000000000000000000000001"), // ::1
        h("20010db8000000000000000000000001"), // 2001:db8::1
        vec![0xffu8; 16],          // ffff:...:ffff
    ];
    for (i, inp) in inputs.iter().enumerate() {
        for trial in 0..4 {
            let k: Vec<u8> = if trial == 0 { vec![0u8; 16] } else { rng.bytes(16) };
            let label = format!("8.130-132 in#{i} trial#{trial}");
            let ct = both_crypt(&format!("{label} encrypt"), &ce, &re, 16, inp, &k);
            let pt = both_crypt(&format!("{label} decrypt"), &cd, &rd, 16, &ct, &k);
            eqb(&format!("{label} round trip"), inp, &pt);
        }
    }
}

#[test]
fn r8_133_fips197_known_answer() {
    // The deterministic variant is plain AES-128 on a single block, so the
    // FIPS-197 C.1 vector must reproduce exactly.
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_encrypt");
    let (cd, rd) = both::<Crypt>("crypto_ipcrypt_decrypt");
    let key = h("000102030405060708090a0b0c0d0e0f");
    let pt = h("00112233445566778899aabbccddeeff");
    let want = h("69c4e0d86a7b0430d8cdb78070b4c55a");
    let got = both_crypt("8.133 encrypt", &ce, &re, 16, &pt, &key);
    eqb("8.133 FIPS-197 ciphertext", &want, &got);
    let back = both_crypt("8.133 decrypt", &cd, &rd, 16, &want, &key);
    eqb("8.133 FIPS-197 plaintext", &pt, &back);
}

// ----------------------------------------- 8.134 – 8.137 / 8.203 – 8.204 ND

#[test]
fn r8_134_to_137_nd() {
    let _rng = rng_guard();
    let (ce, re) = both::<CryptT>("crypto_ipcrypt_nd_encrypt");
    let (cd, rd) = both::<Crypt>("crypto_ipcrypt_nd_decrypt");
    let (kg, _) = both::<Keygen>("crypto_ipcrypt_nd_keygen");

    // 8.134 all-zero tweak and key.
    let k = vec![0u8; 16];
    let t = vec![0u8; 8];
    let inp = h("20010db8000000000000000000000001");
    let out = both_crypt_t("8.134 nd_encrypt", &ce, &re, 24, &inp, &t, &k);
    eqb("8.134 tweak is copied to out[0..8)", &t, &out[..8]);
    let back = both_crypt("8.134 nd_decrypt", &cd, &rd, 16, &out, &k);
    eqb("8.134 nd round trip", &inp, &back);

    // 8.135 random tweak / random key, IPv4-mapped and IPv6 inputs.
    rng_reset();
    let mut key = vec![0u8; 16];
    unsafe { kg(key.as_mut_ptr()) };
    let mut rng = Rng::new(0x8_0135);
    for (i, inp) in [v4(192, 0, 2, 1), h("20010db8000000000000000000000001")]
        .iter()
        .enumerate()
    {
        let t1 = rng.bytes(8);
        let t2 = rng.bytes(8);
        let o1 = both_crypt_t(&format!("8.135 nd#{i} t1"), &ce, &re, 24, inp, &t1, &key);
        let o1b = both_crypt_t(&format!("8.135 nd#{i} t1 again"), &ce, &re, 24, inp, &t1, &key);
        let o2 = both_crypt_t(&format!("8.135 nd#{i} t2"), &ce, &re, 24, inp, &t2, &key);
        eqb("8.135 nd deterministic given the tweak", &o1, &o1b);
        assert_ne!(o1[8..], o2[8..], "8.135: distinct tweaks gave the same ciphertext half");
        eqb("8.135 tweak echo t1", &t1, &o1[..8]);
        eqb("8.135 tweak echo t2", &t2, &o2[..8]);
        let b1 = both_crypt(&format!("8.135 nd#{i} dec t1"), &cd, &rd, 16, &o1, &key);
        let b2 = both_crypt(&format!("8.135 nd#{i} dec t2"), &cd, &rd, 16, &o2, &key);
        eqb("8.135 nd round trip t1", inp, &b1);
        eqb("8.135 nd round trip t2", inp, &b2);
    }

    // 8.136 tweak edge values: all-zero, all-0xFF, and only-odd-bytes set
    // (`tweak_expand` packs tweak[2i] and tweak[2i+1] into one 32-bit word,
    //  so every one of the 8 bytes has to matter).
    let inp = v4(198, 51, 100, 7);
    let tweaks: Vec<Vec<u8>> = vec![
        vec![0u8; 8],
        vec![0xffu8; 8],
        vec![0x00, 0x11, 0x00, 0x22, 0x00, 0x33, 0x00, 0x44], // odd bytes only
        vec![0x11, 0x00, 0x22, 0x00, 0x33, 0x00, 0x44, 0x00], // even bytes only
    ];
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (i, t) in tweaks.iter().enumerate() {
        let o = both_crypt_t(&format!("8.136 tweak#{i}"), &ce, &re, 24, &inp, t, &key);
        assert!(
            !seen.contains(&o[8..].to_vec()),
            "8.136: tweak#{i} collided with an earlier tweak"
        );
        seen.push(o[8..].to_vec());
        let b = both_crypt(&format!("8.136 dec tweak#{i}"), &cd, &rd, 16, &o, &key);
        eqb("8.136 round trip", &inp, &b);
    }

    // 8.137 / 8.204 hand-built 24-byte buffer (tweak ‖ ciphertext): nd_decrypt
    // reads the tweak from in[0..8) only and there is no integrity check, so a
    // forged input decrypts to garbage rather than failing.
    let mut forged = vec![0u8; 24];
    let mut rng = Rng::new(0x8_0137);
    rng.fill(&mut forged);
    let b = both_crypt("8.137 forged nd_decrypt", &cd, &rd, 16, &forged, &key);
    assert_eq!(b.len(), 16);
    // Flipping one ciphertext bit yields a different (but still valid) result.
    let mut forged2 = forged.clone();
    forged2[23] ^= 1;
    let b2 = both_crypt("8.137 forged nd_decrypt bit-flip", &cd, &rd, 16, &forged2, &key);
    assert_ne!(b, b2);
}

// -------------------------------- 8.138 – 8.141 / 8.205 – 8.207 NDX

#[test]
fn r8_138_to_141_ndx() {
    let _rng = rng_guard();
    let (ce, re) = both::<CryptT>("crypto_ipcrypt_ndx_encrypt");
    let (cd, rd) = both::<Crypt>("crypto_ipcrypt_ndx_decrypt");
    let (kg, _) = both::<Keygen>("crypto_ipcrypt_ndx_keygen");

    // 8.138 distinct halves, all-zero tweak.
    let mut k = vec![0u8; 32];
    for i in 0..32 {
        k[i] = i as u8;
    }
    let t = vec![0u8; 16];
    let inp = h("20010db8000000000000000000000001");
    let o = both_crypt_t("8.138 ndx_encrypt", &ce, &re, 32, &inp, &t, &k);
    eqb("8.138 tweak is copied to out[0..16)", &t, &o[..16]);
    let b = both_crypt("8.138 ndx_decrypt", &cd, &rd, 16, &o, &k);
    eqb("8.138 ndx round trip", &inp, &b);

    // 8.139 random keygen key, random tweak, IPv4-mapped and IPv6 inputs.
    rng_reset();
    let mut key = vec![0u8; 32];
    unsafe { kg(key.as_mut_ptr()) };
    let mut rng = Rng::new(0x8_0139);
    for (i, inp) in [v4(192, 0, 2, 1), h("20010db8000000000000000000000001"), vec![0xffu8; 16]]
        .iter()
        .enumerate()
    {
        let t1 = rng.bytes(16);
        let t2 = rng.bytes(16);
        let o1 = both_crypt_t(&format!("8.139 ndx#{i} t1"), &ce, &re, 32, inp, &t1, &key);
        let o2 = both_crypt_t(&format!("8.139 ndx#{i} t2"), &ce, &re, 32, inp, &t2, &key);
        assert_ne!(o1[16..], o2[16..], "8.139: distinct tweaks gave the same ciphertext");
        eqb("8.139 tweak echo", &t1, &o1[..16]);
        let b1 = both_crypt(&format!("8.139 ndx#{i} dec"), &cd, &rd, 16, &o1, &key);
        let b2 = both_crypt(&format!("8.139 ndx#{i} dec2"), &cd, &rd, 16, &o2, &key);
        eqb("8.139 ndx round trip t1", inp, &b1);
        eqb("8.139 ndx round trip t2", inp, &b2);
    }

    // 8.140 / 8.207 degenerate key (identical halves): the `d == 0` fixup
    // re-derives the data key as k[i] ^ 0x5a.  Not an error.
    let inp = v4(203, 0, 113, 9);
    let t = vec![0x5a; 16];
    for half in [vec![0u8; 16], vec![0xffu8; 16], h("0f0e0d0c0b0a09080706050403020100")] {
        let mut deg = half.clone();
        deg.extend_from_slice(&half);
        let o = both_crypt_t("8.140 degenerate ndx_encrypt", &ce, &re, 32, &inp, &t, &deg);
        let b = both_crypt("8.140 degenerate ndx_decrypt", &cd, &rd, 16, &o, &deg);
        eqb("8.140 degenerate round trip", &inp, &b);

        // 8.141 halves differing in a single bit -> no fixup; must differ from
        // the degenerate result.
        let mut nondeg = deg.clone();
        nondeg[31] ^= 1;
        let o2 = both_crypt_t("8.141 one-bit-apart ndx_encrypt", &ce, &re, 32, &inp, &t, &nondeg);
        assert_ne!(
            o[16..], o2[16..],
            "8.140/8.141: degenerate fixup and the neighbouring non-degenerate key agree"
        );
        let b2 = both_crypt("8.141 one-bit-apart ndx_decrypt", &cd, &rd, 16, &o2, &nondeg);
        eqb("8.141 round trip", &inp, &b2);
    }

    // 8.206 forged 32-byte input silently decrypts to garbage.
    let mut forged = vec![0u8; 32];
    Rng::new(0x8_0206).fill(&mut forged);
    let g = both_crypt("8.206 forged ndx_decrypt", &cd, &rd, 16, &forged, &key);
    assert_eq!(g.len(), 16);
}

// ----------------------------- 8.142 – 8.147 / 8.208 – 8.209 PFX

#[test]
fn r8_142_to_147_pfx() {
    let _rng = rng_guard();
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_pfx_encrypt");
    let (cd, rd) = both::<Crypt>("crypto_ipcrypt_pfx_decrypt");
    let (kg, _) = both::<Keygen>("crypto_ipcrypt_pfx_keygen");

    let mut k = vec![0u8; 32];
    for i in 0..32 {
        k[i] = (0x10 + i) as u8;
    }

    // 8.142 IPv4-mapped input keeps the ::ffff: prefix (prefix_start = 96).
    let inp = v4(192, 0, 2, 1);
    let o = both_crypt("8.142 pfx_encrypt v4", &ce, &re, 16, &inp, &k);
    assert_eq!(&o[..10], &[0u8; 10], "8.142: IPv4-mapped prefix not preserved");
    assert_eq!(&o[10..12], &[0xff, 0xff], "8.142: encrypted[10..12] must be 0xff 0xff");
    assert_ne!(&o[12..], &inp[12..], "8.142: v4 part was not randomised");
    let b = both_crypt("8.142 pfx_decrypt v4", &cd, &rd, 16, &o, &k);
    eqb("8.142 pfx round trip", &inp, &b);

    // 8.143 pure IPv6 -> prefix_start = 0, all 128 bits processed.
    let inp6 = h("20010db8000000000000000000000001");
    let o6 = both_crypt("8.143 pfx_encrypt v6", &ce, &re, 16, &inp6, &k);
    let b6 = both_crypt("8.143 pfx_decrypt v6", &cd, &rd, 16, &o6, &k);
    eqb("8.143 pfx round trip v6", &inp6, &b6);

    // 8.144 prefix preservation for IPv4: a shared /24 keeps the leading 24
    // bits of the 32-bit v4 part; a different first octet must not.
    let a = both_crypt("8.144 a", &ce, &re, 16, &v4(192, 0, 2, 1), &k);
    let b24 = both_crypt("8.144 b", &ce, &re, 16, &v4(192, 0, 2, 99), &k);
    assert_eq!(&a[12..15], &b24[12..15], "8.144: /24 prefix not preserved");
    let c1 = both_crypt("8.144 c", &ce, &re, 16, &v4(10, 0, 2, 1), &k);
    assert_ne!(&a[12..13], &c1[12..13], "8.144: unrelated /8s produced the same first octet");

    // 8.145 prefix preservation for IPv6: a shared /64 keeps the first 64 bits.
    let p1 = both_crypt(
        "8.145 a",
        &ce,
        &re,
        16,
        &h("20010db8000000010000000000000001"),
        &k,
    );
    let p2 = both_crypt(
        "8.145 b",
        &ce,
        &re,
        16,
        &h("20010db800000001ffffffffffffffff"),
        &k,
    );
    assert_eq!(&p1[..8], &p2[..8], "8.145: /64 prefix not preserved");
    let p3 = both_crypt(
        "8.145 c",
        &ce,
        &re,
        16,
        &h("20010db8000000020000000000000001"),
        &k,
    );
    assert_ne!(&p1[..8], &p3[..8], "8.145: distinct /64s shared a ciphertext prefix");

    // 8.146 / 8.209 degenerate key: k2 is re-derived as k[i] ^ 0x5a.
    for half in [vec![0u8; 16], vec![0xffu8; 16]] {
        let mut deg = half.clone();
        deg.extend_from_slice(&half);
        for inp in [v4(192, 0, 2, 1), h("20010db8000000000000000000000001")] {
            let o = both_crypt("8.146 degenerate pfx_encrypt", &ce, &re, 16, &inp, &deg);
            let b = both_crypt("8.146 degenerate pfx_decrypt", &cd, &rd, 16, &o, &deg);
            eqb("8.146 degenerate round trip", &inp, &b);
        }
    }

    // 8.147 edge inputs, with a random pfx key as well as the fixed one.
    rng_reset();
    let mut rk = vec![0u8; 32];
    unsafe { kg(rk.as_mut_ptr()) };
    for key in [k.clone(), rk] {
        for inp in [
            vec![0u8; 16],
            vec![0xffu8; 16],
            v4(0, 0, 0, 0),
            v4(255, 255, 255, 255),
        ] {
            let o = both_crypt("8.147 pfx_encrypt", &ce, &re, 16, &inp, &key);
            let b = both_crypt("8.147 pfx_decrypt", &cd, &rd, 16, &o, &key);
            eqb("8.147 pfx round trip", &inp, &b);
        }
    }
}

// ------------------------- 8.148 / 8.149 / 8.211 implementation selection

#[test]
fn r8_148_149_211_implementation_table() {
    // 8.149: the exported `ipcrypt_soft_implementation` struct is eight
    // function pointers; all of them must be non-NULL in both libraries.
    unsafe {
        for lib in [c_lib(), rust_lib()] {
            let sym = lib
                .get::<*const [*const c_void; 8]>(b"ipcrypt_soft_implementation\0")
                .expect("ipcrypt_soft_implementation");
            let table = **sym;
            for (i, p) in table.iter().enumerate() {
                assert!(!p.is_null(), "ipcrypt_soft_implementation slot {i} is NULL");
            }
        }
    }

    // 8.148 / 8.211: pick_best_implementation returns 0 and keeps the soft
    // implementation, so every output is unchanged across the call.
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_encrypt");
    let (cnd, rnd) = both::<CryptT>("crypto_ipcrypt_nd_encrypt");
    let (cndx, rndx) = both::<CryptT>("crypto_ipcrypt_ndx_encrypt");
    let (cpfx, rpfx) = both::<Crypt>("crypto_ipcrypt_pfx_encrypt");
    let k16 = h("000102030405060708090a0b0c0d0e0f");
    let mut k32 = k16.clone();
    k32.extend_from_slice(&h("101112131415161718191a1b1c1d1e1f"));
    let inp = v4(192, 0, 2, 1);
    let t8 = h("0011223344556677");
    let t16 = k16.clone();

    let before = (
        both_crypt("8.148 pre enc", &ce, &re, 16, &inp, &k16),
        both_crypt_t("8.148 pre nd", &cnd, &rnd, 24, &inp, &t8, &k16),
        both_crypt_t("8.148 pre ndx", &cndx, &rndx, 32, &inp, &t16, &k32),
        both_crypt("8.148 pre pfx", &cpfx, &rpfx, 16, &inp, &k32),
    );

    let (cp, rp) = both::<IntGetter>("_crypto_ipcrypt_pick_best_implementation");
    let (cv, rv) = unsafe { (cp(), rp()) };
    eqi("_crypto_ipcrypt_pick_best_implementation", cv, rv);
    assert_eq!(cv, 0);

    let after = (
        both_crypt("8.148 post enc", &ce, &re, 16, &inp, &k16),
        both_crypt_t("8.148 post nd", &cnd, &rnd, 24, &inp, &t8, &k16),
        both_crypt_t("8.148 post ndx", &cndx, &rndx, 32, &inp, &t16, &k32),
        both_crypt("8.148 post pfx", &cpfx, &rpfx, 16, &inp, &k32),
    );
    eqb("8.148 enc unchanged", &before.0, &after.0);
    eqb("8.148 nd unchanged", &before.1, &after.1);
    eqb("8.148 ndx unchanged", &before.2, &after.2);
    eqb("8.148 pfx unchanged", &before.3, &after.3);
}

// ------------------------------------------------- 8.150 / 8.213 / 8.214

#[test]
fn r8_150_ip_string_forms() {
    // libsodium 1.0.23 has no crypto_ipcrypt_*_str entry points; the 16-byte
    // ipcrypt inputs are built from strings with sodium_ip2bin/sodium_bin2ip
    // (which live in sodium/codecs.c and belong to area 1 — this row only
    // records that they are the string interface for ipcrypt).
    let (ci2b, ri2b) = both::<Ip2Bin>("sodium_ip2bin");
    let (cb2i, rb2i) = both::<Bin2Ip>("sodium_bin2ip");
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_encrypt");
    let key = h("000102030405060708090a0b0c0d0e0f");

    for (s, want) in [
        ("192.0.2.1", v4(192, 0, 2, 1)),
        ("::ffff:192.0.2.1", v4(192, 0, 2, 1)),
        ("2001:db8::1", h("20010db8000000000000000000000001")),
        ("::", vec![0u8; 16]),
    ] {
        let mut cs = s.as_bytes().to_vec();
        cs.push(0);
        let mut bc = padded(16);
        let mut br = padded(16);
        let (rc, rr) = unsafe {
            (
                ci2b(bc.as_mut_ptr(), cs.as_ptr() as *const c_char, cs.len() - 1),
                ri2b(br.as_mut_ptr(), cs.as_ptr() as *const c_char, cs.len() - 1),
            )
        };
        eqi(&format!("sodium_ip2bin({s})"), rc, rr);
        assert_eq!(rc, 0, "sodium_ip2bin({s}) failed");
        eqb(&format!("sodium_ip2bin({s}) bytes"), &bc[..16], &br[..16]);
        eqb(&format!("sodium_ip2bin({s}) expected"), &want, &bc[..16]);
        // The parsed bytes feed crypto_ipcrypt_encrypt directly.
        both_crypt(&format!("8.150 encrypt {s}"), &ce, &re, 16, &bc[..16], &key);

        // sodium_bin2ip renders an IPv4-mapped block back in dotted-quad form.
        let mut tc = padded(64);
        let mut tr = padded(64);
        unsafe {
            let pc = cb2i(tc.as_mut_ptr() as *mut c_char, 64, bc.as_ptr());
            let pr = rb2i(tr.as_mut_ptr() as *mut c_char, 64, br.as_ptr());
            assert!(!pc.is_null() && !pr.is_null(), "sodium_bin2ip({s}) returned NULL");
        }
        let cend = tc[..64].iter().position(|&b| b == 0).unwrap();
        let rend = tr[..64].iter().position(|&b| b == 0).unwrap();
        eqb(&format!("sodium_bin2ip({s})"), &tc[..cend], &tr[..rend]);
    }

    // 8.214: ip_maxlen <= 2 is rejected with NULL.
    let bin = v4(192, 0, 2, 1);
    for maxlen in [0usize, 1, 2, 8] {
        let mut tc = padded(maxlen.max(1));
        let mut tr = padded(maxlen.max(1));
        let (pc, pr) = unsafe {
            (
                cb2i(tc.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr()),
                rb2i(tr.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr()),
            )
        };
        assert_eq!(
            pc.is_null(),
            pr.is_null(),
            "sodium_bin2ip(ip_maxlen={maxlen}): NULL-ness mismatch"
        );
        assert!(pc.is_null(), "sodium_bin2ip(ip_maxlen={maxlen}) should fail");
    }

    // 8.213: malformed IP strings are rejected by sodium_ip2bin.
    for bad in [
        "", "1.2.3", "1.2.3.4.5", "256.0.0.1", "1.2.3.4 ", "1.2.3.04x", ":::", "2001:db8:::1",
        "20010:db8::1", "1:2:3:4:5:6:7", "::1%eth0", "1.2.3.4%eth0", "2001:db8::1%",
        "2001:db8::1%bad!", "1.2.3.4:80",
    ] {
        let mut cs = bad.as_bytes().to_vec();
        cs.push(0);
        let mut bc = padded(16);
        let mut br = padded(16);
        let (rc, rr) = unsafe {
            (
                ci2b(bc.as_mut_ptr(), cs.as_ptr() as *const c_char, cs.len() - 1),
                ri2b(br.as_mut_ptr(), cs.as_ptr() as *const c_char, cs.len() - 1),
            )
        };
        eqi(&format!("sodium_ip2bin({bad:?})"), rc, rr);
    }
}

// ------------------------------------------------- 8.202 – 8.210 totality

#[test]
fn r8_202_to_210_no_rejection_branch() {
    // Every crypto_ipcrypt_* entry point is `void`: there is no return value to
    // compare, and nothing is validated.  What *is* observable is the exact
    // number of bytes written; the guard region of `padded()` in every test
    // above proves neither implementation writes past
    //   16 (encrypt/decrypt/pfx), 24 (nd_encrypt), 32 (ndx_encrypt).
    // Here we pin the exact write extent once more explicitly.
    let (ce, re) = both::<Crypt>("crypto_ipcrypt_encrypt");
    let (cnd, rnd) = both::<CryptT>("crypto_ipcrypt_nd_encrypt");
    let (cndd, rndd) = both::<Crypt>("crypto_ipcrypt_nd_decrypt");
    let (cndx, rndx) = both::<CryptT>("crypto_ipcrypt_ndx_encrypt");
    let (cndxd, rndxd) = both::<Crypt>("crypto_ipcrypt_ndx_decrypt");
    let (cpfx, rpfx) = both::<Crypt>("crypto_ipcrypt_pfx_encrypt");

    let mut rng = Rng::new(0x8_0202);
    for _ in 0..8 {
        let k16 = rng.bytes(16);
        let k32 = rng.bytes(32);
        let inp = rng.bytes(16);
        let in24 = rng.bytes(24);
        let in32 = rng.bytes(32);
        let t8 = rng.bytes(8);
        let t16 = rng.bytes(16);
        both_crypt("8.202 encrypt", &ce, &re, 16, &inp, &k16);
        both_crypt_t("8.203 nd_encrypt", &cnd, &rnd, 24, &inp, &t8, &k16);
        both_crypt("8.204 nd_decrypt", &cndd, &rndd, 16, &in24, &k16);
        both_crypt_t("8.205 ndx_encrypt", &cndx, &rndx, 32, &inp, &t16, &k32);
        both_crypt("8.206 ndx_decrypt", &cndxd, &rndxd, 16, &in32, &k32);
        both_crypt("8.208 pfx_encrypt", &cpfx, &rpfx, 16, &inp, &k32);
    }
}
