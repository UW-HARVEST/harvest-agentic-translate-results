//! Phase C/D — generic FFI-boundary tests that every C API has, plus an
//! in-test symbol-parity gate.
//!
//! Covers the "Additional generic FFI-boundary boundaries" section of
//! ERRORS.md: out-of-range enum values crossing the FFI, zero and oversized
//! lengths, NULL optional out-parameters, and one step past every documented
//! range. Also re-checks that the Rust `.so` exports every symbol the C `.so`
//! exports (the Phase D gate) from inside the test process.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_ulonglong};

type Sz = usize;

// =========================================================== symbol parity

/// Phase D gate, enforced from inside the test binary: every symbol the C `.so`
/// exports must be resolvable in the Rust `.so` under the exact same name.
#[test]
fn phase_d_symbol_parity() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "../c_src/build/libsodium.so"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run nm on the C .so");
    assert!(out.status.success(), "nm failed: {:?}", out);
    let text = String::from_utf8_lossy(&out.stdout);
    let mut c_syms: Vec<String> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 && matches!(f[1], "T" | "D" | "B" | "R") {
            c_syms.push(f[2].to_string());
        }
    }
    c_syms.sort();
    c_syms.dedup();
    assert!(
        c_syms.len() > 800,
        "expected ~878 exported C symbols, got {}",
        c_syms.len()
    );

    let l = libs();
    let mut missing: Vec<String> = Vec::new();
    for s in &c_syms {
        let mut z = s.clone().into_bytes();
        z.push(0);
        if unsafe { l.rs.get::<*const ()>(&z) }.is_err() {
            missing.push(s.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} of {} C symbols: {:?}",
        missing.len(),
        c_syms.len(),
        missing
    );
    eprintln!("symbol parity: {} C symbols, 0 missing from Rust", c_syms.len());
}

/// Every zero-argument `size_t`/`int`/`const char*` getter in the library must
/// return the same value. This sweeps the whole exported surface cheaply.
#[test]
fn phase_d_all_getters_agree() {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "../c_src/build/libsodium.so"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<String> = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 && f[1] == "T" {
            names.push(f[2].to_string());
        }
    }
    // Name-suffix heuristics for the *_bytes()-style constant getters, which is
    // the only class of symbol we can call blind with no arguments.
    let size_suffixes = [
        "_bytes", "_bytes_min", "_bytes_max", "_keybytes", "_keybytes_min", "_keybytes_max",
        "_noncebytes", "_macbytes", "_abytes", "_nsecbytes", "_npubbytes", "_statebytes",
        "_saltbytes", "_personalbytes", "_seedbytes", "_publickeybytes", "_secretkeybytes",
        "_beforenmbytes", "_zerobytes", "_boxzerobytes", "_sealbytes", "_headerbytes",
        "_sessionkeybytes", "_scalarbytes", "_uniformbytes", "_hashbytes",
        "_nonreducedscalarbytes", "_messagebytes_max", "_strbytes", "_passwd_min", "_passwd_max",
        "_opslimit_min", "_opslimit_max", "_memlimit_min", "_memlimit_max",
        "_opslimit_interactive", "_memlimit_interactive", "_opslimit_moderate",
        "_memlimit_moderate", "_opslimit_sensitive", "_memlimit_sensitive", "_contextbytes",
        "_blockbytes", "_ciphertextbytes", "_sharedsecretbytes", "_tweakbytes", "_inputbytes",
        "_outputbytes", "_constbytes", "_rate",
    ];
    let mut n_size = 0usize;
    let mut n_str = 0usize;
    let mut n_int = 0usize;
    for name in &names {
        if name.starts_with('_') {
            continue; // internal helpers with unknown signatures
        }
        if name.ends_with("_primitive") || name.ends_with("_strprefix") || name == "sodium_version_string" {
            let (c, r) = pair::<unsafe extern "C" fn() -> *const c_char>(name);
            unsafe {
                let a = std::ffi::CStr::from_ptr(c()).to_bytes().to_vec();
                let b = std::ffi::CStr::from_ptr(r()).to_bytes().to_vec();
                same_bytes(name, &a, &b);
            }
            n_str += 1;
            continue;
        }
        if name.starts_with("crypto_pwhash_alg_")
            || name.starts_with("sodium_library_version")
            || name == "sodium_library_minimal"
            || name.starts_with("sodium_runtime_has_")
            || name == "crypto_aead_aes256gcm_is_available"
        {
            let (c, r) = pair::<unsafe extern "C" fn() -> c_int>(name);
            unsafe { assert_eq!(c(), r(), "{name}") };
            n_int += 1;
            continue;
        }
        if name.ends_with("_domain_standard") {
            let (c, r) = pair::<unsafe extern "C" fn() -> u8>(name);
            unsafe { assert_eq!(c(), r(), "{name}") };
            n_int += 1;
            continue;
        }
        if name.starts_with("crypto_secretstream_xchacha20poly1305_tag_") {
            let (c, r) = pair::<unsafe extern "C" fn() -> u8>(name);
            unsafe { assert_eq!(c(), r(), "{name}") };
            n_int += 1;
            continue;
        }
        // These match the *_bytes naming pattern but take arguments.
        const NOT_GETTERS: &[&str] = &[
            "crypto_core_keccak1600_extract_bytes",
            "crypto_core_keccak1600_xor_bytes",
        ];
        if NOT_GETTERS.contains(&name.as_str()) {
            continue;
        }
        if size_suffixes.iter().any(|s| name.ends_with(s)) {
            let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(name);
            unsafe { assert_eq!(c(), r(), "{name}") };
            n_size += 1;
            continue;
        }
    }
    eprintln!("getters compared: {n_size} size_t, {n_int} int/uchar, {n_str} string");
    assert!(n_size > 150, "expected >150 size_t getters, compared {n_size}");
}

// ================================= out-of-range secretstream tag across FFI

/// A C `unsigned char tag` accepts any of 256 values; only 0..=3 are named.
/// The C ORs the tag into the first plaintext block, so values 4..=255 are
/// valid inputs with defined (if undocumented) behaviour. Rust must match,
/// including which of them trigger the auto-rekey (any tag with bit 1 set).
#[test]
fn secretstream_out_of_range_tags() {
    let p = "crypto_secretstream_xchacha20poly1305";
    let gg = |s: &str| -> usize {
        let (c, r) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_{s}"));
        unsafe {
            assert_eq!(c(), r());
            c()
        }
    };
    let statebytes = gg("statebytes");
    let abytes = gg("abytes");
    let headerbytes = gg("headerbytes");
    let keybytes = gg("keybytes");

    let (ipc, _) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int>(&format!(
        "{p}_init_push"
    ));
    let (iplc, iplr) = pair::<unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int>(
        &format!("{p}_init_pull"),
    );
    let (pushc, pushr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *mut c_ulonglong,
            *const u8,
            c_ulonglong,
            *const u8,
            c_ulonglong,
            u8,
        ) -> c_int,
    >(&format!("{p}_push"));
    let (pullc, pullr) = pair::<
        unsafe extern "C" fn(
            *mut u8,
            *mut u8,
            *mut c_ulonglong,
            *mut u8,
            *const u8,
            c_ulonglong,
            *const u8,
            c_ulonglong,
        ) -> c_int,
    >(&format!("{p}_pull"));

    let mut rng = Rng::seeded();
    let k = rng.bytes(keybytes);
    let mut header = buf(headerbytes);
    let mut tmp = buf(statebytes);
    unsafe {
        ipc(tmp.as_mut_ptr(), header.as_mut_ptr(), k.as_ptr());
    }

    for tag in 0u16..=255 {
        let tag = tag as u8;
        for &ml in &[0usize, 1, 17, 64, 200] {
            let m = rng.bytes(ml);
            let ad = rng.bytes(16);
            let mut sa = buf(statebytes);
            let mut sb = buf(statebytes);
            unsafe {
                iplc(sa.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                iplr(sb.as_mut_ptr(), header.as_ptr(), k.as_ptr());
            }
            let mut ca = buf(ml + abytes + 8);
            let mut cb = buf(ml + abytes + 8);
            let mut la: c_ulonglong = 0xDEAD;
            let mut lb: c_ulonglong = 0xDEAD;
            unsafe {
                same_ret(
                    &format!("push tag={tag}"),
                    pushc(sa.as_mut_ptr(), ca.as_mut_ptr(), &mut la, m.as_ptr(), ml as c_ulonglong, ad.as_ptr(), 16, tag),
                    pushr(sb.as_mut_ptr(), cb.as_mut_ptr(), &mut lb, m.as_ptr(), ml as c_ulonglong, ad.as_ptr(), 16, tag),
                );
            }
            same_bytes(&format!("push tag={tag} ml={ml} ct"), &ca, &cb);
            same_bytes(&format!("push tag={tag} ml={ml} state"), &sa, &sb);
            assert_eq!(la, lb, "push tag={tag} clen");

            // and the pull side must recover the same out-of-range tag byte
            let mut sa = buf(statebytes);
            let mut sb = buf(statebytes);
            unsafe {
                iplc(sa.as_mut_ptr(), header.as_ptr(), k.as_ptr());
                iplr(sb.as_mut_ptr(), header.as_ptr(), k.as_ptr());
            }
            let mut ma = buf(ml + 8);
            let mut mb = buf(ml + 8);
            let mut ta: u8 = 0xEE;
            let mut tb: u8 = 0xEE;
            let mut xa: c_ulonglong = 0xDEAD;
            let mut xb: c_ulonglong = 0xDEAD;
            unsafe {
                same_ret(
                    &format!("pull tag={tag}"),
                    pullc(sa.as_mut_ptr(), ma.as_mut_ptr(), &mut xa, &mut ta, ca.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16),
                    pullr(sb.as_mut_ptr(), mb.as_mut_ptr(), &mut xb, &mut tb, cb.as_ptr(), (ml + abytes) as c_ulonglong, ad.as_ptr(), 16),
                );
            }
            same_bytes(&format!("pull tag={tag} ml={ml} pt"), &ma, &mb);
            same_bytes(&format!("pull tag={tag} ml={ml} state"), &sa, &sb);
            assert_eq!(ta, tb, "pull tag={tag} recovered tag");
            assert_eq!(xa, xb, "pull tag={tag} mlen");
            assert_eq!(ta, tag, "pull must recover the exact tag byte");
        }
    }
}

// ==================== one step past every documented length boundary

/// Sweep `len - 1`, `len`, `len + 1` around every advertised constant for the
/// length-taking functions where an out-of-range length is a *defined* input.
#[test]
fn boundary_lengths_one_step_past() {
    let mut rng = Rng::seeded();

    // generichash: outlen / keylen at MIN-1, MIN, MAX, MAX+1
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong, *const u8, Sz) -> c_int,
    >("crypto_generichash");
    let m = rng.bytes(64);
    let key = rng.bytes(128);
    let mut out = buf(256);
    for ol in 0usize..=66 {
        for kl in [0usize, 15, 16, 63, 64, 65, 66] {
            let kp = if kl == 0 { std::ptr::null() } else { key.as_ptr() };
            set_errno(0);
            let a = unsafe { c(out.as_mut_ptr(), ol, m.as_ptr(), 64, kp, kl) };
            let ea = errno();
            let mut out2 = buf(256);
            set_errno(0);
            let b = unsafe { r(out2.as_mut_ptr(), ol, m.as_ptr(), 64, kp, kl) };
            let eb = errno();
            assert_eq!(a, b, "generichash ol={ol} kl={kl}: return differs");
            assert_eq!(ea, eb, "generichash ol={ol} kl={kl}: errno differs");
            if a == 0 {
                same_bytes(&format!("generichash ol={ol} kl={kl}"), &out[..ol], &out2[..ol]);
            }
        }
    }

    // crypto_kdf_blake2b_derive_from_key: subkey_len 0..=70
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut u8, Sz, u64, *const c_char, *const u8) -> c_int,
    >("crypto_kdf_derive_from_key");
    let k32 = rng.bytes(32);
    let ctx = b"context\0";
    for sl in 0usize..=70 {
        let mut a = buf(128);
        let mut b = buf(128);
        set_errno(0);
        let x = unsafe { c(a.as_mut_ptr(), sl, 7, ctx.as_ptr() as *const c_char, k32.as_ptr()) };
        let ea = errno();
        set_errno(0);
        let y = unsafe { r(b.as_mut_ptr(), sl, 7, ctx.as_ptr() as *const c_char, k32.as_ptr()) };
        let eb = errno();
        assert_eq!(x, y, "kdf subkey_len={sl}: return differs");
        assert_eq!(ea, eb, "kdf subkey_len={sl}: errno differs");
        same_bytes(&format!("kdf subkey_len={sl}"), &a, &b);
    }

    // hkdf expand: out_len around BYTES_MAX
    for (name, maxv) in [
        ("crypto_kdf_hkdf_sha256_expand", 0xffusize * 32),
        ("crypto_kdf_hkdf_sha512_expand", 0xff * 64),
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, *const c_char, Sz, *const u8) -> c_int,
        >(name);
        let prk = rng.bytes(64);
        for ol in [0usize, 1, maxv - 1, maxv, maxv + 1, maxv + 2] {
            let mut a = buf(maxv + 8);
            let mut b = buf(maxv + 8);
            set_errno(0);
            let x = unsafe { c(a.as_mut_ptr(), ol, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()) };
            let ea = errno();
            set_errno(0);
            let y = unsafe { r(b.as_mut_ptr(), ol, ctx.as_ptr() as *const c_char, 7, prk.as_ptr()) };
            let eb = errno();
            assert_eq!(x, y, "{name} out_len={ol}: return differs");
            assert_eq!(ea, eb, "{name} out_len={ol}: errno differs");
            same_bytes(&format!("{name} out_len={ol}"), &a, &b);
        }
    }

    // sodium_pad / sodium_unpad: one step past max_buflen at every blocksize
    let (pc, pr) = pair::<unsafe extern "C" fn(*mut Sz, *mut u8, Sz, Sz, Sz) -> c_int>("sodium_pad");
    let (uc, ur) =
        pair::<unsafe extern "C" fn(*mut Sz, *const u8, Sz, Sz) -> c_int>("sodium_unpad");
    for bs in 0usize..=20 {
        for ul in 0usize..=20 {
            for maxl in 0usize..=44 {
                let mut a = vec![CANARY; 64];
                let mut b = vec![CANARY; 64];
                let mut la: Sz = 0xDEAD;
                let mut lb: Sz = 0xDEAD;
                set_errno(0);
                let x = unsafe { pc(&mut la, a.as_mut_ptr(), ul, bs, maxl) };
                let ea = errno();
                set_errno(0);
                let y = unsafe { pr(&mut lb, b.as_mut_ptr(), ul, bs, maxl) };
                let eb = errno();
                assert_eq!(x, y, "sodium_pad bs={bs} ul={ul} maxl={maxl}: return differs");
                assert_eq!(ea, eb, "sodium_pad bs={bs} ul={ul} maxl={maxl}: errno differs");
                assert_eq!(la, lb, "sodium_pad bs={bs} ul={ul} maxl={maxl}: len differs");
                same_bytes(&format!("sodium_pad bs={bs} ul={ul} maxl={maxl}"), &a, &b);
            }
            for plen in 0usize..=44 {
                let a = vec![0x80u8; 64];
                let mut la: Sz = 0xDEAD;
                let mut lb: Sz = 0xDEAD;
                set_errno(0);
                let x = unsafe { uc(&mut la, a.as_ptr(), plen, bs) };
                let ea = errno();
                set_errno(0);
                let y = unsafe { ur(&mut lb, a.as_ptr(), plen, bs) };
                let eb = errno();
                assert_eq!(x, y, "sodium_unpad bs={bs} plen={plen}: return differs");
                assert_eq!(ea, eb, "sodium_unpad bs={bs} plen={plen}: errno differs");
                assert_eq!(la, lb, "sodium_unpad bs={bs} plen={plen}: len differs");
            }
        }
    }

    // xof squeeze: 0 and lengths straddling the rate
    for prefix in [
        "crypto_xof_shake128",
        "crypto_xof_shake256",
        "crypto_xof_turboshake128",
        "crypto_xof_turboshake256",
    ] {
        let sb = {
            let (c, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_statebytes"));
            unsafe { c() }
        };
        let rate = {
            let (c, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{prefix}_blockbytes"));
            unsafe { c() }
        };
        let (oc, or) = pair::<
            unsafe extern "C" fn(*mut u8, Sz, *const u8, c_ulonglong) -> c_int,
        >(prefix);
        let (ic, ir) = pair::<unsafe extern "C" fn(*mut u8) -> c_int>(&format!("{prefix}_init"));
        let (qc, qr) = pair::<unsafe extern "C" fn(*mut u8, *mut u8, Sz) -> c_int>(&format!(
            "{prefix}_squeeze"
        ));
        let cap = 4 * rate + 8;
        for ol in [0usize, 1, rate - 1, rate, rate + 1, 2 * rate - 1, 2 * rate, 2 * rate + 1, 3 * rate] {
            let mut a = buf(cap);
            let mut b = buf(cap);
            unsafe {
                same_ret(
                    prefix,
                    oc(a.as_mut_ptr(), ol, m.as_ptr(), 64),
                    or(b.as_mut_ptr(), ol, m.as_ptr(), 64),
                );
            }
            same_bytes(&format!("{prefix} one-shot ol={ol}"), &a, &b);
            let mut sa = buf(sb);
            let mut sbb = buf(sb);
            let mut ta = buf(cap);
            let mut tb = buf(cap);
            unsafe {
                ic(sa.as_mut_ptr());
                ir(sbb.as_mut_ptr());
                same_ret(
                    &format!("{prefix}_squeeze ol={ol}"),
                    qc(sa.as_mut_ptr(), ta.as_mut_ptr(), ol),
                    qr(sbb.as_mut_ptr(), tb.as_mut_ptr(), ol),
                );
            }
            same_bytes(&format!("{prefix} empty-absorb squeeze ol={ol}"), &ta, &tb);
        }
    }
}

// =============================== zero-length inputs everywhere

#[test]
fn zero_length_inputs_everywhere() {
    let mut rng = Rng::seeded();
    // every stream: len 0 with NULL output must behave identically
    for (name, noncebytes) in [
        ("crypto_stream", 24usize),
        ("crypto_stream_salsa20", 8),
        ("crypto_stream_salsa2012", 8),
        ("crypto_stream_salsa208", 8),
        ("crypto_stream_xsalsa20", 24),
        ("crypto_stream_chacha20", 8),
        ("crypto_stream_chacha20_ietf", 12),
        ("crypto_stream_xchacha20", 24),
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, c_ulonglong, *const u8, *const u8) -> c_int,
        >(name);
        let k = rng.bytes(32);
        let n = rng.bytes(noncebytes);
        unsafe {
            same_ret(
                &format!("{name} len=0"),
                c(std::ptr::null_mut(), 0, n.as_ptr(), k.as_ptr()),
                r(std::ptr::null_mut(), 0, n.as_ptr(), k.as_ptr()),
            );
        }
        let (xc, xr) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8, *const u8) -> c_int,
        >(&format!("{name}_xor"));
        unsafe {
            same_ret(
                &format!("{name}_xor len=0"),
                xc(std::ptr::null_mut(), std::ptr::null(), 0, n.as_ptr(), k.as_ptr()),
                xr(std::ptr::null_mut(), std::ptr::null(), 0, n.as_ptr(), k.as_ptr()),
            );
        }
    }
    // hashes with NULL input and inlen 0
    for (name, ol) in [
        ("crypto_hash_sha256", 32usize),
        ("crypto_hash_sha512", 64),
        ("crypto_hash", 64),
        ("crypto_hash_sha3256", 32),
        ("crypto_hash_sha3512", 64),
    ] {
        let (c, r) =
            pair::<unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong) -> c_int>(name);
        let mut a = buf(ol);
        let mut b = buf(ol);
        unsafe {
            same_ret(
                &format!("{name} NULL/0"),
                c(a.as_mut_ptr(), std::ptr::null(), 0),
                r(b.as_mut_ptr(), std::ptr::null(), 0),
            );
        }
        same_bytes(&format!("{name} NULL/0"), &a, &b);
    }
    // shorthash / onetimeauth with NULL input
    for (name, ol, kl) in [
        ("crypto_shorthash", 8usize, 16usize),
        ("crypto_shorthash_siphashx24", 16, 16),
        ("crypto_onetimeauth", 16, 32),
    ] {
        let (c, r) = pair::<
            unsafe extern "C" fn(*mut u8, *const u8, c_ulonglong, *const u8) -> c_int,
        >(name);
        let k = rng.bytes(kl);
        let mut a = buf(ol);
        let mut b = buf(ol);
        unsafe {
            same_ret(
                &format!("{name} NULL/0"),
                c(a.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
                r(b.as_mut_ptr(), std::ptr::null(), 0, k.as_ptr()),
            );
        }
        same_bytes(&format!("{name} NULL/0"), &a, &b);
    }
    // sodium_* with len 0 and NULL pointers
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, Sz) -> c_int>("sodium_is_zero");
    unsafe {
        same_ret("is_zero NULL/0", c(std::ptr::null(), 0), r(std::ptr::null(), 0));
    }
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int>("sodium_memcmp");
    unsafe {
        same_ret(
            "memcmp NULL/0",
            c(std::ptr::null(), std::ptr::null(), 0),
            r(std::ptr::null(), std::ptr::null(), 0),
        );
    }
    let (c, r) = pair::<unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int>("sodium_compare");
    unsafe {
        same_ret(
            "compare NULL/0",
            c(std::ptr::null(), std::ptr::null(), 0),
            r(std::ptr::null(), std::ptr::null(), 0),
        );
    }
    for name in ["sodium_increment"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, Sz)>(name);
        unsafe {
            c(std::ptr::null_mut(), 0);
            r(std::ptr::null_mut(), 0);
        }
    }
    for name in ["sodium_add", "sodium_sub"] {
        let (c, r) = pair::<unsafe extern "C" fn(*mut u8, *const u8, Sz)>(name);
        unsafe {
            c(std::ptr::null_mut(), std::ptr::null(), 0);
            r(std::ptr::null_mut(), std::ptr::null(), 0);
        }
    }
    let (c, r) = pair::<unsafe extern "C" fn(*mut u8, Sz)>("sodium_memzero");
    unsafe {
        c(std::ptr::null_mut(), 0);
        r(std::ptr::null_mut(), 0);
    }
    // sodium_malloc(0) is legal and must agree on NULL-ness
    let (mc, mr) = pair::<unsafe extern "C" fn(Sz) -> *mut u8>("sodium_malloc");
    let (fc, fr) = pair::<unsafe extern "C" fn(*mut u8)>("sodium_free");
    unsafe {
        let p = mc(0);
        let q = mr(0);
        assert_eq!(p.is_null(), q.is_null(), "sodium_malloc(0) NULL-ness");
        fc(p);
        fr(q);
    }
    let (ac, ar) = pair::<unsafe extern "C" fn(Sz, Sz) -> *mut u8>("sodium_allocarray");
    unsafe {
        let p = ac(0, 0);
        let q = ar(0, 0);
        assert_eq!(p.is_null(), q.is_null(), "sodium_allocarray(0,0) NULL-ness");
        fc(p);
        fr(q);
    }
    // sodium_bin2hex / bin2base64 with bin_len 0
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz) -> *mut c_char,
    >("sodium_bin2hex");
    let mut a = buf(8);
    let mut b = buf(8);
    unsafe {
        c(a.as_mut_ptr() as *mut c_char, 1, std::ptr::null(), 0);
        r(b.as_mut_ptr() as *mut c_char, 1, std::ptr::null(), 0);
    }
    same_bytes("bin2hex 0-length", &a, &b);
    let (c, r) = pair::<
        unsafe extern "C" fn(*mut c_char, Sz, *const u8, Sz, c_int) -> *mut c_char,
    >("sodium_bin2base64");
    for v in [1, 3, 5, 7] {
        let mut a = buf(8);
        let mut b = buf(8);
        unsafe {
            c(a.as_mut_ptr() as *mut c_char, 1, std::ptr::null(), 0, v);
            r(b.as_mut_ptr() as *mut c_char, 1, std::ptr::null(), 0, v);
        }
        same_bytes(&format!("bin2base64 0-length v={v}"), &a, &b);
    }
    // AEAD / secretbox with mlen 0 and NULL m
    for p in [
        "crypto_aead_chacha20poly1305",
        "crypto_aead_chacha20poly1305_ietf",
        "crypto_aead_xchacha20poly1305_ietf",
        "crypto_aead_aegis128l",
        "crypto_aead_aegis256",
    ] {
        let gg = |s: &str| -> usize {
            let (c, _) = pair::<unsafe extern "C" fn() -> Sz>(&format!("{p}_{s}"));
            unsafe { c() }
        };
        let (ec, er) = pair::<
            unsafe extern "C" fn(
                *mut u8,
                *mut c_ulonglong,
                *const u8,
                c_ulonglong,
                *const u8,
                c_ulonglong,
                *const u8,
                *const u8,
                *const u8,
            ) -> c_int,
        >(&format!("{p}_encrypt"));
        let k = rng.bytes(gg("keybytes"));
        let np = rng.bytes(gg("npubbytes"));
        let ab = gg("abytes");
        let mut a = buf(ab + 8);
        let mut b = buf(ab + 8);
        unsafe {
            same_ret(
                &format!("{p}_encrypt NULL m, mlen 0"),
                ec(a.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null(), 0, std::ptr::null(), 0, std::ptr::null(), np.as_ptr(), k.as_ptr()),
                er(b.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null(), 0, std::ptr::null(), 0, std::ptr::null(), np.as_ptr(), k.as_ptr()),
            );
        }
        same_bytes(&format!("{p}_encrypt NULL m, mlen 0"), &a, &b);
    }
}
