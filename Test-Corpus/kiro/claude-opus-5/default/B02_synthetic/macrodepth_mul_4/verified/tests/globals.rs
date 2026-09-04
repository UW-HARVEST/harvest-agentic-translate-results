//! Phase B — rows 22..24 of `CONFIGS.md`: the two exported `.data` globals.
//!
//! These rows mutate process-global state inside the loaded libraries, so they
//! live in their own test binary and run as a single sequential test to avoid
//! racing with (or leaking into) any other row.

mod common;

use std::ffi::{c_char, c_int};

use common::*;

#[test]
fn r22_r23_r24_globals() {
    let (c, r) = (c_impl(), rust_impl());

    let c_gop = c.g_op();
    let r_gop = r.g_op();
    let c_name = c.g_op_name();
    let r_name = r.g_op_name();

    // Row 23/24 store into these objects. `int (*G_OP)(int,int)` and
    // `const char *G_OP_NAME` are non-`const` in C, so gcc places them in
    // writable `.data`. Check the mapping first so a regression to an immutable
    // Rust `static` (which LLVM marks constant and the linker puts in
    // `.data.rel.ro`, mapped read-only by RELRO) reports this message instead of
    // dying with SIGSEGV.
    for (label, addr) in [
        ("C G_OP", c_gop as usize),
        ("Rust G_OP", r_gop as usize),
        ("C G_OP_NAME", c_name as usize),
        ("Rust G_OP_NAME", r_name as usize),
    ] {
        assert!(
            is_writable(addr),
            "[{}] {label} at {addr:#x} is not in a writable mapping; the C .so \
             keeps both globals in .data, so the translation must too \
             (use `static mut`, not `static`)",
            config_label()
        );
    }

    let c_gop0 = unsafe { *c_gop };
    let r_gop0 = unsafe { *r_gop };
    let c_name0 = unsafe { *c_name };
    let r_name0 = unsafe { *r_name };

    /* ---- row 22: read G_OP and call through it ---- */
    {
        let op_sym = format!("op_{OP}");
        let (c_op, r_op) = (c.binop(&op_sym), r.binop(&op_sym));
        for (a, b) in all_pairs() {
            let (cv, rv) = unsafe { (c_gop0(a, b), r_gop0(a, b)) };
            assert_eq!(cv, rv, "[{}] G_OP({a}, {b}): C={cv} Rust={rv}", config_label());
            let (co, ro) = unsafe { (c_op(a, b), r_op(a, b)) };
            assert_eq!(cv, co, "[{}] C G_OP != op_{OP} at ({a},{b})", config_label());
            assert_eq!(rv, ro, "[{}] Rust G_OP != op_{OP} at ({a},{b})", config_label());
        }
    }

    /* ---- row 24 (read half): G_OP_NAME == STR(OP) ---- */
    {
        let cn = c.g_op_name_bytes();
        let rn = r.g_op_name_bytes();
        assert_eq!(
            cn,
            rn,
            "[{}] G_OP_NAME differs: C={:?} Rust={:?}",
            config_label(),
            String::from_utf8_lossy(&cn),
            String::from_utf8_lossy(&rn)
        );
        assert_eq!(cn, OP.as_bytes(), "[{}] C G_OP_NAME is not STR(OP)", config_label());
    }

    /* ---- row 23: G_OP is writable, and helpers ignore it ---- */
    {
        // Baselines captured before any mutation.
        let (c_hc, r_hc) = (c.binop("helper_call"), r.binop("helper_call"));
        let (c_hp, r_hp) = (c.binop("helper_ptr"), r.binop("helper_ptr"));
        let probes: Vec<(c_int, c_int)> = random_pairs(SEED ^ 0x77, 64);
        let c_hc_before: Vec<c_int> = probes.iter().map(|&(a, b)| unsafe { c_hc(a, b) }).collect();
        let r_hc_before: Vec<c_int> = probes.iter().map(|&(a, b)| unsafe { r_hc(a, b) }).collect();
        let c_hp_before: Vec<c_int> = probes.iter().map(|&(a, b)| unsafe { c_hp(a, b) }).collect();
        let r_hp_before: Vec<c_int> = probes.iter().map(|&(a, b)| unsafe { r_hp(a, b) }).collect();

        for target in ["op_add", "op_sub", "op_mul"] {
            let (c_t, r_t) = (c.binop(target), r.binop(target));
            // A store into the exported `.data` object. This is valid against
            // the C `.so` (`int (*G_OP)(int,int)` is not `const`), so it must be
            // valid against the Rust `.so` too.
            unsafe {
                *c_gop = *c_t;
                *r_gop = *r_t;
            }
            let (c_now, r_now) = unsafe { (*c_gop, *r_gop) };
            for (a, b) in all_pairs() {
                let (cv, rv) = unsafe { (c_now(a, b), r_now(a, b)) };
                assert_eq!(
                    cv, rv,
                    "[{}] after G_OP = {target}: G_OP({a}, {b}): C={cv} Rust={rv}",
                    config_label()
                );
                let (ce, re) = unsafe { (c_t(a, b), r_t(a, b)) };
                assert_eq!(cv, ce, "[{}] C G_OP != {target} after store", config_label());
                assert_eq!(rv, re, "[{}] Rust G_OP != {target} after store", config_label());
            }

            // `helper_call`/`helper_ptr` use `OP_FN(OP)` directly, never `G_OP`,
            // so overwriting the global must not change them.
            for (i, &(a, b)) in probes.iter().enumerate() {
                let (cv, rv) = unsafe { (c_hc(a, b), r_hc(a, b)) };
                assert_eq!(cv, rv, "[{}] helper_call({a},{b}) after G_OP={target}", config_label());
                assert_eq!(cv, c_hc_before[i], "[{}] C helper_call changed by G_OP store", config_label());
                assert_eq!(rv, r_hc_before[i], "[{}] Rust helper_call changed by G_OP store", config_label());

                let (cv, rv) = unsafe { (c_hp(a, b), r_hp(a, b)) };
                assert_eq!(cv, rv, "[{}] helper_ptr({a},{b}) after G_OP={target}", config_label());
                assert_eq!(cv, c_hp_before[i], "[{}] C helper_ptr changed by G_OP store", config_label());
                assert_eq!(rv, r_hp_before[i], "[{}] Rust helper_ptr changed by G_OP store", config_label());
            }
        }
    }

    /* ---- row 24 (write half): G_OP_NAME is writable ---- */
    {
        let replacement: &[u8] = b"REPOINTED\0";
        unsafe {
            *c_name = replacement.as_ptr() as *const c_char;
            *r_name = replacement.as_ptr() as *const c_char;
        }
        let cn = c.g_op_name_bytes();
        let rn = r.g_op_name_bytes();
        assert_eq!(cn, rn, "[{}] G_OP_NAME after store", config_label());
        assert_eq!(cn, b"REPOINTED", "[{}] C G_OP_NAME store did not take", config_label());
    }

    /* ---- restore, and confirm the restore is observable ---- */
    unsafe {
        *c_gop = c_gop0;
        *r_gop = r_gop0;
        *c_name = c_name0;
        *r_name = r_name0;
    }
    assert_eq!(c.g_op_name_bytes(), OP.as_bytes());
    assert_eq!(r.g_op_name_bytes(), OP.as_bytes());
}

/// Is `addr` inside a writable mapping of this process? Parsed from
/// `/proc/self/maps` so the check itself cannot fault.
fn is_writable(addr: usize) -> bool {
    let maps = match std::fs::read_to_string("/proc/self/maps") {
        Ok(m) => m,
        // No procfs: fall back to "assume writable" rather than failing for an
        // unrelated reason. The store then either works or crashes, as before.
        Err(_) => return true,
    };
    for line in maps.lines() {
        let mut it = line.split_whitespace();
        let range = match it.next() {
            Some(r) => r,
            None => continue,
        };
        let perms = it.next().unwrap_or("");
        let (lo, hi) = match range.split_once('-') {
            Some((a, b)) => (
                usize::from_str_radix(a, 16).unwrap_or(0),
                usize::from_str_radix(b, 16).unwrap_or(0),
            ),
            None => continue,
        };
        if addr >= lo && addr < hi {
            return perms.contains('w');
        }
    }
    false
}
