//! Phase B (continued) — the G1 `CONFIGS.md` rows that `t01_g1_utils.rs` does
//! not reach: process-startup ordering, the misuse handler, the critical
//! section, the AMD64-asm-sized `sodium_increment/add/sub` widths, the secure
//! allocator's mprotect sequences, and the deterministic `sodium_bin2ip`
//! zero-run shapes.
//!
//! ROW MAP for CONFIGS.md rows 1-163 (`t01` = `t01_g1_utils.rs`,
//! `t02` = `t02_g1_errors.rs`, `t13` = this file):
//!
//! | rows | covered by |
//! |------|------------|
//! | 1, 23 | `t13::g1c_row1_23_first_init_and_pre_init_runtime_getters` (child proc) |
//! | 2 | `t01::g1_sodium_init_idempotent` |
//! | 3, 4 | `t13::g1c_rows3_4_misuse_handler` |
//! | 5 | `t13::g1c_row5_crit_enter_leave_balanced` |
//! | 6-10 | `t00::version_strings_match` |
//! | 11-22, 31 | `t01::g1_runtime_has_all` (all 12 getters, both libs) |
//! | 24-30 | `t01::g1_crypto_verify_16_32_64` |
//! | 32-34 | `t01::g1_memzero_and_stackzero` |
//! | 35-45 | `t01::g1_compare_is_zero_increment_add_sub` |
//! | 46-48, 52-54, 58-61, 63, 64 | `t01::g1_compare_is_zero_increment_add_sub` (len 0..40 sweep) |
//! | 49-51, 55-57, 62 | `t13::g1c_rows49_64_asm_width_arithmetic` (widths 8/12/24/64) |
//! | 65, 66 | `t13::g1c_rows65_66_mlock_alignment` |
//! | 67-78 | `t01::g1_sodium_malloc_family` |
//! | 79-82 | `t13::g1c_rows79_82_mprotect_sequences` |
//! | 83-87 | `t01::g1_bin2hex_all_lengths` |
//! | 88-98 | `t01::g1_hex2bin_valid_and_ignore`, `g1_hex2bin_null_bin_len_and_null_hex_end`, `g1_hex2bin_random_strings` |
//! | 99-102 | `t01::g1_base64_encoded_len_all_variants` |
//! | 103 | `t13::g1c_row103_encoded_len_matches_macro` |
//! | 104-113 | `t01::g1_bin2base64_all_variants_all_lengths` |
//! | 113, 125 | `t13::g1c_rows113_125_full_alphabet_and_A_sentinel` |
//! | 114-124, 126 | `t01::g1_base642bin_roundtrip_all_variants`, `g1_base642bin_random_strings` |
//! | 127-135, 137-141, 143 | `t01::g1_pad_unpad_matrix`, `g1_unpad_arbitrary_buffers` |
//! | 136 | `t13::g1c_row136_pad_null_out_param` |
//! | 142 | `t13::g1c_row142_unpad_data_byte_equal_to_barrier` |
//! | 144-154 | `t01::g1_ip2bin_bin2ip` |
//! | 146, 147 | `t13::g1c_rows146_147_ip_len_shorter_and_longer` |
//! | 155-163 | `t13::g1c_rows155_163_bin2ip_zero_runs` |
mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// ===========================================================================
// child-process cases (things that must happen before/around sodium_init)
// ===========================================================================
#[test]
fn zz_abort_child() {
    let Some((case, is_c)) = child_case() else {
        return;
    };
    // NOTE: `libs()` calls sodium_init on BOTH libraries, so the pre-init cases
    // must load the library directly instead of going through the harness.
    unsafe {
        match case.as_str() {
            // --- CONFIGS rows 1 + 23 ------------------------------------
            // A freshly loaded library: every runtime_has_* must be readable
            // before sodium_init(), and the FIRST sodium_init() must return 0
            // while later ones return 1.
            "first_init_and_pre_init_getters" => {
                let path = if is_c {
                    std::env::var(C_ENV).unwrap()
                } else {
                    std::env::var(R_ENV).unwrap()
                };
                let l = libloading::Library::new(path).unwrap();
                let mut acc: u32 = 0;
                for n in [
                    "sodium_runtime_has_neon",
                    "sodium_runtime_has_armcrypto",
                    "sodium_runtime_has_sse2",
                    "sodium_runtime_has_sse3",
                    "sodium_runtime_has_ssse3",
                    "sodium_runtime_has_sse41",
                    "sodium_runtime_has_avx",
                    "sodium_runtime_has_avx2",
                    "sodium_runtime_has_avx512f",
                    "sodium_runtime_has_pclmul",
                    "sodium_runtime_has_aesni",
                    "sodium_runtime_has_rdrand",
                ] {
                    let f: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                        l.get(format!("{n}\0").as_bytes()).unwrap();
                    acc = acc * 2 + (f() != 0) as u32;
                }
                let init: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    l.get(b"sodium_init\0").unwrap();
                let a = init();
                let b = init();
                let c = init();
                // encode everything into the exit status
                std::process::exit((((acc & 0x0f) << 3) | ((a as u32 & 3) << 2) | ((b as u32 & 1) << 1) | (c as u32 & 1)) as i32);
            }
            // --- CONFIGS rows 3 + 4 -------------------------------------
            // The misuse handler must be installed, replaced and cleared, and
            // must actually be invoked (before abort()) on a misuse.
            "misuse_handler_lifecycle" => {
                let path = if is_c {
                    std::env::var(C_ENV).unwrap()
                } else {
                    std::env::var(R_ENV).unwrap()
                };
                let l = libloading::Library::new(path).unwrap();
                let set: libloading::Symbol<
                    unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int,
                > = l.get(b"sodium_set_misuse_handler\0").unwrap();
                let init: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    l.get(b"sodium_init\0").unwrap();
                // row 3: handler NULL, then sodium_init
                let r0 = set(None);
                let ri = init();
                // row 4: install, replace, clear
                extern "C" fn h1() {}
                extern "C" fn h2() {}
                let r1 = set(Some(h1));
                let r2 = set(Some(h2));
                let r3 = set(None);
                std::process::exit(
                    ((r0 & 1) * 16 + (ri & 1) * 8 + (r1 & 1) * 4 + (r2 & 1) * 2 + (r3 & 1)) as i32,
                );
            }
            // The installed handler runs first, then abort() happens anyway.
            "misuse_handler_invoked" => {
                let path = if is_c {
                    std::env::var(C_ENV).unwrap()
                } else {
                    std::env::var(R_ENV).unwrap()
                };
                let l = libloading::Library::new(path).unwrap();
                let init: libloading::Symbol<unsafe extern "C" fn() -> c_int> =
                    l.get(b"sodium_init\0").unwrap();
                init();
                let set: libloading::Symbol<
                    unsafe extern "C" fn(Option<extern "C" fn()>) -> c_int,
                > = l.get(b"sodium_set_misuse_handler\0").unwrap();
                extern "C" fn handler() {
                    // Proves the handler ran: exit 77 instead of aborting.
                    std::process::exit(77);
                }
                assert_eq!(set(Some(handler)), 0);
                let mis: libloading::Symbol<unsafe extern "C" fn()> =
                    l.get(b"sodium_misuse\0").unwrap();
                mis();
                std::process::exit(0);
            }
            other => panic!("unknown case {other}"),
        }
    }
}

/// CONFIGS.md rows 1, 23.
#[test]
fn g1c_row1_23_first_init_and_pre_init_runtime_getters() {
    let t = diff_abort_case("first_init_and_pre_init_getters");
    assert!(
        t.signal.is_none(),
        "pre-init runtime getters + first sodium_init must not crash: {t:?}"
    );
}

/// CONFIGS.md rows 3, 4.
#[test]
fn g1c_rows3_4_misuse_handler() {
    let t = diff_abort_case("misuse_handler_lifecycle");
    assert!(t.signal.is_none(), "handler lifecycle must not crash: {t:?}");
    assert_eq!(t.code, Some(0), "all five calls must return 0: {t:?}");
    // The handler must actually be called before abort() in BOTH libraries.
    let t = diff_abort_case("misuse_handler_invoked");
    assert_eq!(
        t.code,
        Some(77),
        "the misuse handler must run before abort(): {t:?}"
    );
}

/// CONFIGS.md row 5 — a balanced `sodium_crit_enter` / `sodium_crit_leave` pair.
#[test]
fn g1c_row5_crit_enter_leave_balanced() {
    unsafe {
        let (ce, re) = pair::<unsafe extern "C" fn() -> c_int>("sodium_crit_enter");
        let (cl, rl) = pair::<unsafe extern "C" fn() -> c_int>("sodium_crit_leave");
        // Repeat a few times: the lock counter must return to 0 each round.
        for i in 0..4 {
            set_errno(0);
            let a = ce();
            let ae = errno();
            set_errno(0);
            let b = cl();
            let be = errno();
            set_errno(0);
            let c = re();
            let ce_ = errno();
            set_errno(0);
            let d = rl();
            let de = errno();
            eq_i32(&format!("row5 crit_enter round {i}"), a, c);
            eq_i32(&format!("row5 crit_leave round {i}"), b, d);
            assert_eq!(ae, ce_, "row5 crit_enter errno round {i}");
            assert_eq!(be, de, "row5 crit_leave errno round {i}");
            assert_eq!(a, 0, "row5: balanced crit_enter must return 0");
            assert_eq!(b, 0, "row5: balanced crit_leave must return 0");
        }
    }
}

/// CONFIGS.md rows 49-51, 55-57, 62-64 — the byte widths that select the
/// `HAVE_AMD64_ASM` fast paths in `sodium_increment` (8/12/24),
/// `sodium_add` (8/12/24) and `sodium_sub` (64), plus aliased operands.
#[test]
fn g1c_rows49_64_asm_width_arithmetic() {
    unsafe {
        let (cinc, rinc) = pair::<unsafe extern "C" fn(*mut u8, usize)>("sodium_increment");
        let (cadd, radd) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_add");
        let (csub, rsub) = pair::<unsafe extern "C" fn(*mut u8, *const u8, usize)>("sodium_sub");
        let mut rng = Rng::new(0x515F_A5D_9001);
        for len in [1usize, 7, 8, 9, 11, 12, 13, 15, 16, 23, 24, 25, 31, 32, 63, 64, 65, 96, 128] {
            // structured operands that force carries/borrows at every internal
            // 8-byte boundary, plus random ones
            let mut shapes: Vec<(Vec<u8>, Vec<u8>)> = vec![
                (vec![0x00; len], vec![0x00; len]),
                (vec![0xff; len], vec![0x00; len]),
                (vec![0xff; len], vec![0x01; len]),
                (vec![0x00; len], vec![0xff; len]),
                (vec![0xff; len], vec![0xff; len]),
            ];
            for boundary in (8..len).step_by(8) {
                // all-ff below `boundary`, zero above: +1 carries exactly across it
                let mut a = vec![0u8; len];
                for x in a[..boundary].iter_mut() {
                    *x = 0xff;
                }
                let mut b = vec![0u8; len];
                b[0] = 1;
                shapes.push((a.clone(), b.clone()));
                // and the mirror case for sub (borrow across the boundary)
                let mut a2 = vec![0u8; len];
                a2[boundary] = 1;
                shapes.push((a2, b));
            }
            for _ in 0..30 {
                shapes.push((rng.bytes(len), rng.bytes(len)));
            }
            for (a, b) in shapes {
                let ctx = format!("len={len} a={} b={}", hex(&a), hex(&b));
                let mut x = a.clone();
                let mut y = a.clone();
                cinc(x.as_mut_ptr(), len);
                rinc(y.as_mut_ptr(), len);
                eq_bytes(&format!("rows49-51 sodium_increment {ctx}"), &x, &y);
                let mut x = a.clone();
                let mut y = a.clone();
                cadd(x.as_mut_ptr(), b.as_ptr(), len);
                radd(y.as_mut_ptr(), b.as_ptr(), len);
                eq_bytes(&format!("rows55-57 sodium_add {ctx}"), &x, &y);
                let mut x = a.clone();
                let mut y = a.clone();
                csub(x.as_mut_ptr(), b.as_ptr(), len);
                rsub(y.as_mut_ptr(), b.as_ptr(), len);
                eq_bytes(&format!("row62 sodium_sub {ctx}"), &x, &y);
                // rows 59 / 64: aliased pointers (a == b)
                let mut x = a.clone();
                let mut y = a.clone();
                let p = x.as_mut_ptr();
                cadd(p, p, len);
                let q = y.as_mut_ptr();
                radd(q, q, len);
                eq_bytes(&format!("row59 sodium_add aliased {ctx}"), &x, &y);
                let mut x = a.clone();
                let mut y = a.clone();
                let p = x.as_mut_ptr();
                csub(p, p, len);
                let q = y.as_mut_ptr();
                rsub(q, q, len);
                eq_bytes(&format!("row64 sodium_sub aliased {ctx}"), &x, &y);
                assert!(
                    x.iter().all(|&v| v == 0),
                    "row64: a - a must be all zero, got {}",
                    hex(&x)
                );
            }
        }
    }
}

/// CONFIGS.md rows 65, 66 — `sodium_mlock`/`sodium_munlock` on a page-aligned
/// region and on an unaligned heap address.
///
/// This build has no `HAVE_MLOCK`, so both return -1/ENOSYS; the test asserts
/// C/Rust agreement rather than success, so it stays correct either way.
/// Note `sodium_munlock()` zeroes the range before doing anything else, which
/// is itself observable and compared.
#[test]
fn g1c_rows65_66_mlock_alignment() {
    unsafe {
        let mut res: Vec<Vec<(String, c_int, i32, String)>> = Vec::new();
        for is_c in [true, false] {
            let l = libs();
            let h = if is_c { l.c } else { l.rs };
            let mlock: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
                h.get(b"sodium_mlock\0").unwrap();
            let munlock: libloading::Symbol<unsafe extern "C" fn(*mut c_void, usize) -> c_int> =
                h.get(b"sodium_munlock\0").unwrap();
            let mut rec = Vec::new();
            // row 65: page-aligned region, len == page size
            let page = 4096usize;
            let mut backing = vec![0xA7u8; page * 3];
            let base = backing.as_mut_ptr();
            let aligned = base.add(base.align_offset(page));
            set_errno(0);
            let r = mlock(aligned as *mut c_void, page);
            rec.push(("mlock(aligned,4096)".into(), r, errno(), String::new()));
            set_errno(0);
            let r = munlock(aligned as *mut c_void, page);
            let zeroed = std::slice::from_raw_parts(aligned, page)
                .iter()
                .all(|&b| b == 0);
            rec.push((
                "munlock(aligned,4096)".into(),
                r,
                errno(),
                format!("zeroed={zeroed}"),
            ));
            // row 66: unaligned heap address, small len
            let mut buf = vec![0x5Cu8; 512];
            let un = buf.as_mut_ptr().add(3);
            set_errno(0);
            let r = mlock(un as *mut c_void, 64);
            rec.push(("mlock(unaligned,64)".into(), r, errno(), String::new()));
            set_errno(0);
            let r = munlock(un as *mut c_void, 64);
            rec.push((
                "munlock(unaligned,64)".into(),
                r,
                errno(),
                hex(&buf[..80]),
            ));
            res.push(rec);
        }
        assert_eq!(
            res[0], res[1],
            "rows 65/66: mlock/munlock return value, errno or zeroing differs"
        );
    }
}

/// CONFIGS.md rows 79-82 — mprotect state sequences over a `sodium_malloc`
/// region, including freeing a region that was left read-only and calling
/// `noaccess` twice.
#[test]
fn g1c_rows79_82_mprotect_sequences() {
    unsafe {
        let mut res: Vec<Vec<(String, c_int, i32)>> = Vec::new();
        for is_c in [true, false] {
            let l = libs();
            let h = if is_c { l.c } else { l.rs };
            macro_rules! g {
                ($t:ty, $n:literal) => {{
                    let s: libloading::Symbol<$t> = h.get(concat!($n, "\0").as_bytes()).unwrap();
                    s
                }};
            }
            let malloc = g!(unsafe extern "C" fn(usize) -> *mut c_void, "sodium_malloc");
            let free = g!(unsafe extern "C" fn(*mut c_void), "sodium_free");
            let noaccess = g!(
                unsafe extern "C" fn(*mut c_void) -> c_int,
                "sodium_mprotect_noaccess"
            );
            let readonly = g!(
                unsafe extern "C" fn(*mut c_void) -> c_int,
                "sodium_mprotect_readonly"
            );
            let readwrite = g!(
                unsafe extern "C" fn(*mut c_void) -> c_int,
                "sodium_mprotect_readwrite"
            );
            let mut rec = Vec::new();
            for size in [1usize, 64, 4096, 12289] {
                // row 79: readwrite -> write -> readonly -> readwrite -> write
                let p = malloc(size);
                assert!(!p.is_null());
                set_errno(0);
                rec.push((format!("rw({size})"), readwrite(p), errno()));
                ptr::write_bytes(p as *mut u8, 0x11, size);
                set_errno(0);
                rec.push((format!("ro({size})"), readonly(p), errno()));
                set_errno(0);
                rec.push((format!("rw2({size})"), readwrite(p), errno()));
                ptr::write_bytes(p as *mut u8, 0x22, size);
                free(p);
                // row 80: noaccess -> readwrite -> free
                let p = malloc(size);
                set_errno(0);
                rec.push((format!("na({size})"), noaccess(p), errno()));
                set_errno(0);
                rec.push((format!("rw3({size})"), readwrite(p), errno()));
                free(p);
                // row 81: left read-only at free() time
                let p = malloc(size);
                set_errno(0);
                rec.push((format!("ro_then_free({size})"), readonly(p), errno()));
                free(p);
                // row 82: noaccess twice in a row
                let p = malloc(size);
                set_errno(0);
                rec.push((format!("na_a({size})"), noaccess(p), errno()));
                set_errno(0);
                rec.push((format!("na_b({size})"), noaccess(p), errno()));
                set_errno(0);
                rec.push((format!("rw4({size})"), readwrite(p), errno()));
                free(p);
            }
            res.push(rec);
        }
        assert_eq!(
            res[0], res[1],
            "rows 79-82: mprotect sequence return values or errno differ"
        );
    }
}

/// CONFIGS.md row 103 — `sodium_base64_encoded_len()` must equal the
/// `sodium_base64_ENCODED_LEN` macro for every (bin_len, variant).
#[test]
fn g1c_row103_encoded_len_matches_macro() {
    // Verbatim transcription of the macro in include/sodium/utils.h:79-88:
    //
    //   #define sodium_base64_ENCODED_LEN(BIN_LEN, VARIANT)                  \
    //     ((BIN_LEN) / 3U > (SIZE_MAX - 5) / 4U                              \
    //          ? (size_t)SIZE_MAX                                            \
    //          : (((BIN_LEN) / 3U) * 4U +                                    \
    //             ((((BIN_LEN) - ((BIN_LEN) / 3U) * 3U) |                     \
    //               (((BIN_LEN) - ((BIN_LEN) / 3U) * 3U) >> 1)) &             \
    //              1U) *                                                     \
    //                 (4U - ((0U - (((VARIANT) & 2U) >> 1)) &                 \
    //                        (3U - ((BIN_LEN) - ((BIN_LEN) / 3U) * 3U)))) +   \
    //             1U))
    fn macro_len(bin_len: usize, v: c_int) -> usize {
        if bin_len / 3 > (usize::MAX - 5) / 4 {
            return usize::MAX;
        }
        let rem = bin_len - (bin_len / 3) * 3;
        // `0U - ((VARIANT & 2) >> 1)` is unsigned-int arithmetic: 0 for the
        // padded variants (bit 1 clear) and 0xffffffff for the NO_PADDING ones.
        let no_pad_mask: u32 = 0u32.wrapping_sub(((v as u32) & 2) >> 1);
        let sub = (no_pad_mask & (3u32 - rem as u32)) as usize;
        (bin_len / 3) * 4 + (((rem | (rem >> 1)) & 1) * (4 - sub)) + 1
    }
    unsafe {
        let (c, r) = pair::<unsafe extern "C" fn(usize, c_int) -> usize>(
            "sodium_base64_encoded_len",
        );
        for v in [1i32, 3, 5, 7] {
            for n in 0..=400usize {
                let cv = c(n, v);
                let rv = r(n, v);
                assert_eq!(cv, rv, "row103 encoded_len({n},{v}) C/Rust");
                assert_eq!(
                    cv,
                    macro_len(n, v),
                    "row103 encoded_len({n},{v}) vs sodium_base64_ENCODED_LEN macro"
                );
            }
            for n in [1000usize, 4095, 4096, 65535, 1 << 20] {
                assert_eq!(c(n, v), r(n, v));
                assert_eq!(c(n, v), macro_len(n, v));
            }
        }
    }
}

/// CONFIGS.md rows 113, 125 — every one of the 64 alphabet symbols must be
/// produced (standard `+`/`/` and urlsafe `-`/`_`), and the `'A'` (value 0)
/// sentinel in `b64_char_to_byte` / `b64_urlsafe_char_to_byte` must round-trip.
#[test]
fn g1c_rows113_125_full_alphabet_and_A_sentinel() {
    type E = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize, c_int) -> *mut c_char;
    type D = unsafe extern "C" fn(
        *mut u8,
        usize,
        *const c_char,
        usize,
        *const c_char,
        *mut usize,
        *mut *const c_char,
        c_int,
    ) -> c_int;
    unsafe {
        let (ce, re) = pair::<E>("sodium_bin2base64");
        let (cd, rd) = pair::<D>("sodium_base642bin");
        for v in [1i32, 3, 5, 7] {
            // 48 bytes = 64 base64 symbols, driven so that index i appears at
            // position i for i in 0..64.
            let mut bin = vec![0u8; 48];
            for i in 0..64usize {
                let bitpos = i * 6;
                let val = i as u32;
                for b in 0..6 {
                    if (val >> (5 - b)) & 1 != 0 {
                        let p = bitpos + b;
                        bin[p / 8] |= 0x80 >> (p % 8);
                    }
                }
            }
            let mut cb = vec![0u8; 128];
            let mut rb = vec![0u8; 128];
            let cp = ce(cb.as_mut_ptr() as *mut c_char, 128, bin.as_ptr(), 48, v);
            let rp = re(rb.as_mut_ptr() as *mut c_char, 128, bin.as_ptr(), 48, v);
            assert!(!cp.is_null() && !rp.is_null());
            eq_bytes(&format!("row113 full alphabet v={v}"), &cb, &rb);
            let s = &cb[..cb.iter().position(|&x| x == 0).unwrap()];
            let mut seen = std::collections::BTreeSet::new();
            for &ch in s {
                seen.insert(ch);
            }
            assert_eq!(
                seen.len(),
                64,
                "row113 v={v}: expected all 64 symbols, got {} in {:?}",
                seen.len(),
                String::from_utf8_lossy(s)
            );
            // decode it back
            let mut co = vec![0u8; 64];
            let mut ro = vec![0u8; 64];
            let mut cl = 0usize;
            let mut rl = 0usize;
            let a = cd(
                co.as_mut_ptr(),
                64,
                s.as_ptr() as *const c_char,
                s.len(),
                ptr::null(),
                &mut cl,
                ptr::null_mut(),
                v,
            );
            let b = rd(
                ro.as_mut_ptr(),
                64,
                s.as_ptr() as *const c_char,
                s.len(),
                ptr::null(),
                &mut rl,
                ptr::null_mut(),
                v,
            );
            eq_i32(&format!("row113 decode v={v}"), a, b);
            assert_eq!(cl, rl);
            eq_bytes(&format!("row113 decode v={v}"), &co, &ro);
            assert_eq!(&co[..cl], &bin[..], "row113 v={v}: round-trip");

            // row 125: 'A' == value 0, in every position of a group
            for n in 1..=12usize {
                let bin0 = vec![0u8; n];
                let mut cb = vec![0u8; 64];
                let mut rb = vec![0u8; 64];
                ce(cb.as_mut_ptr() as *mut c_char, 64, bin0.as_ptr(), n, v);
                re(rb.as_mut_ptr() as *mut c_char, 64, bin0.as_ptr(), n, v);
                eq_bytes(&format!("row125 zeros n={n} v={v}"), &cb, &rb);
                let s = &cb[..cb.iter().position(|&x| x == 0).unwrap()];
                let mut co = vec![0u8; 32];
                let mut ro = vec![0u8; 32];
                let mut cl = 0usize;
                let mut rl = 0usize;
                let a = cd(
                    co.as_mut_ptr(),
                    32,
                    s.as_ptr() as *const c_char,
                    s.len(),
                    ptr::null(),
                    &mut cl,
                    ptr::null_mut(),
                    v,
                );
                let b = rd(
                    ro.as_mut_ptr(),
                    32,
                    s.as_ptr() as *const c_char,
                    s.len(),
                    ptr::null(),
                    &mut rl,
                    ptr::null_mut(),
                    v,
                );
                eq_i32(&format!("row125 decode zeros n={n} v={v}"), a, b);
                assert_eq!(cl, rl);
                eq_bytes(&format!("row125 decode zeros n={n} v={v}"), &co, &ro);
            }
        }
    }
}

/// CONFIGS.md row 136 — `sodium_pad` with `padded_buflen_p == NULL`.
#[test]
fn g1c_row136_pad_null_out_param() {
    type P = unsafe extern "C" fn(*mut usize, *mut u8, usize, usize, usize) -> c_int;
    unsafe {
        let (cp, rp) = pair::<P>("sodium_pad");
        let mut rng = Rng::new(0x136);
        for bs in [1usize, 2, 16, 17, 64] {
            for unpadded in 0..=40usize {
                let data = rng.bytes(unpadded);
                let maxb = unpadded + 2 * bs + 1;
                let mut cb = vec![0x99u8; maxb + 8];
                let mut rb = cb.clone();
                cb[..unpadded].copy_from_slice(&data);
                rb[..unpadded].copy_from_slice(&data);
                let a = cp(ptr::null_mut(), cb.as_mut_ptr(), unpadded, bs, maxb);
                let b = rp(ptr::null_mut(), rb.as_mut_ptr(), unpadded, bs, maxb);
                let ctx = format!("row136 pad NULL out unpadded={unpadded} bs={bs}");
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &cb, &rb);
            }
        }
    }
}

/// CONFIGS.md row 142 — a data byte equal to the 0x80 barrier immediately
/// before the real barrier: the LAST barrier from the end must win.
#[test]
fn g1c_row142_unpad_data_byte_equal_to_barrier() {
    type U = unsafe extern "C" fn(*mut usize, *const u8, usize, usize) -> c_int;
    unsafe {
        let (cu, ru) = pair::<U>("sodium_unpad");
        for bs in [1usize, 2, 4, 8, 16, 17, 32] {
            for total in [bs, bs * 2, bs * 3] {
                for barrier in 0..total {
                    let mut buf = vec![0u8; total];
                    buf[barrier] = 0x80;
                    // fill everything before the barrier with 0x80 too
                    for x in buf[..barrier].iter_mut() {
                        *x = 0x80;
                    }
                    let mut cl = usize::MAX;
                    let mut rl = usize::MAX;
                    let a = cu(&mut cl, buf.as_ptr(), total, bs);
                    let b = ru(&mut rl, buf.as_ptr(), total, bs);
                    let ctx =
                        format!("row142 unpad bs={bs} total={total} barrier={barrier}");
                    eq_i32(&ctx, a, b);
                    assert_eq!(cl, rl, "{ctx}: unpadded_buflen differs");
                }
            }
        }
    }
}

/// CONFIGS.md rows 146, 147 — `ip_len_` longer than the NUL-terminated string
/// (scan must stop at the embedded NUL) and shorter than it (explicit length
/// bounds the parse).
#[test]
fn g1c_rows146_147_ip_len_shorter_and_longer() {
    type I = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    unsafe {
        let (ci, ri) = pair::<I>("sodium_ip2bin");
        let cases: &[&[u8]] = &[
            b"1.2.3.4\0\0\0\0garbage\0",
            b"255.255.255.255\0xxxx\0",
            b"::1\0yyyy\0",
            b"2001:db8::1\0zzz\0",
            b"fe80::1%eth0\0qqq\0",
            b"1.2.3.45\0",
            b"::ffff:1.2.3.4\0abc\0",
        ];
        for s in cases {
            let strlen = s.iter().position(|&x| x == 0).unwrap();
            // every length from 0 up to the whole buffer, i.e. both shorter
            // (row 147) and longer than the string (row 146)
            for pass_len in 0..s.len() {
                let mut ca = [0xEEu8; 16];
                let mut ra = [0xEEu8; 16];
                set_errno(0);
                let a = ci(ca.as_mut_ptr(), s.as_ptr() as *const c_char, pass_len);
                let ae = errno();
                set_errno(0);
                let b = ri(ra.as_mut_ptr(), s.as_ptr() as *const c_char, pass_len);
                let be = errno();
                let ctx = format!(
                    "rows146/147 ip2bin {:?} strlen={strlen} pass_len={pass_len}",
                    String::from_utf8_lossy(&s[..strlen])
                );
                eq_i32(&ctx, a, b);
                assert_eq!(ae, be, "{ctx}: errno differs");
                eq_bytes(&ctx, &ca, &ra);
            }
        }
    }
}

/// CONFIGS.md rows 155-163 — `sodium_bin2ip` zero-run compression: no run, a
/// single zero word (no compression), exactly two, several runs of different
/// lengths, ties, runs at the start / middle / end, the all-zero address, the
/// longest possible output, `ip_maxlen == 3`, and the full round-trip.
#[test]
fn g1c_rows155_163_bin2ip_zero_runs() {
    type B = unsafe extern "C" fn(*mut c_char, usize, *const u8) -> *mut c_char;
    type I = unsafe extern "C" fn(*mut u8, *const c_char, usize) -> c_int;
    unsafe {
        let (cb, rb) = pair::<B>("sodium_bin2ip");
        let (ci, ri) = pair::<I>("sodium_ip2bin");
        // Enumerate EVERY subset of the 8 words being zero (256 shapes). This
        // covers rows 156-161 exhaustively: no run, single-word runs, all run
        // lengths, all positions, ties, and the all-zero address.
        for mask in 0u32..256 {
            let mut words = [0u16; 8];
            for i in 0..8 {
                words[i] = if mask & (1 << i) != 0 {
                    0
                } else {
                    (0x1000 + i as u16 * 0x1111) | 1
                };
            }
            let mut bin = [0u8; 16];
            for i in 0..8 {
                bin[i * 2] = (words[i] >> 8) as u8;
                bin[i * 2 + 1] = (words[i] & 0xff) as u8;
            }
            for maxlen in [0usize, 1, 2, 3, 4, 8, 16, 32, 38, 39, 40, 46, 64] {
                let mut co = vec![0x33u8; maxlen + 8];
                let mut ro = vec![0x33u8; maxlen + 8];
                let cp = cb(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let rp = rb(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let ctx = format!(
                    "rows155-162 bin2ip mask={mask:#04x} bin={} maxlen={maxlen}",
                    hex(&bin)
                );
                assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness differs");
                eq_bytes(&ctx, &co, &ro);
            }
            // row 161: the no-zero-word case must fit in 39 chars + NUL
            if mask == 0 {
                let mut o = vec![0u8; 64];
                let p = cb(o.as_mut_ptr() as *mut c_char, 64, bin.as_ptr());
                assert!(!p.is_null());
                let n = o.iter().position(|&x| x == 0).unwrap();
                assert!(n <= 39, "row161: output {n} chars > 39");
            }
            // row 163: round-trip
            let mut txt = vec![0u8; 64];
            if !cb(txt.as_mut_ptr() as *mut c_char, 64, bin.as_ptr()).is_null() {
                let tlen = txt.iter().position(|&x| x == 0).unwrap();
                let mut ca = [0u8; 16];
                let mut ra = [0u8; 16];
                let a = ci(ca.as_mut_ptr(), txt.as_ptr() as *const c_char, tlen);
                let b = ri(ra.as_mut_ptr(), txt.as_ptr() as *const c_char, tlen);
                let ctx = format!(
                    "row163 roundtrip {}",
                    String::from_utf8_lossy(&txt[..tlen])
                );
                eq_i32(&ctx, a, b);
                eq_bytes(&ctx, &ca, &ra);
                if a == 0 {
                    eq_bytes(&format!("{ctx} identity"), &bin, &ca);
                }
            }
        }
        // row 155: IPv4-mapped inputs, ip_maxlen exactly len+1 and larger
        for quad in [
            [0u8, 0, 0, 0],
            [1, 2, 3, 4],
            [255, 255, 255, 255],
            [10, 0, 0, 1],
            [192, 168, 100, 200],
        ] {
            let mut bin = [0u8; 16];
            bin[10] = 0xff;
            bin[11] = 0xff;
            bin[12..].copy_from_slice(&quad);
            for maxlen in 0..=24usize {
                let mut co = vec![0x33u8; maxlen + 8];
                let mut ro = vec![0x33u8; maxlen + 8];
                let cp = cb(co.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let rp = rb(ro.as_mut_ptr() as *mut c_char, maxlen, bin.as_ptr());
                let ctx = format!("row155 ipv4-mapped {quad:?} maxlen={maxlen}");
                assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness differs");
                eq_bytes(&ctx, &co, &ro);
            }
        }
    }
}
