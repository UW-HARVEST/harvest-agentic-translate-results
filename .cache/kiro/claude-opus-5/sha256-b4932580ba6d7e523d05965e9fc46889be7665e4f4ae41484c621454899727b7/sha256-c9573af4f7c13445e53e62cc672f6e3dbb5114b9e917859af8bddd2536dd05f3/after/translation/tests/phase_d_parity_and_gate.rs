//! Phase D — symbol parity, harness sensitivity, and the completion gate.
//!
//! * Re-derives the `nm -D` symbol diff at test time so `SYMBOLS.md` cannot go
//!   stale, and asserts the diff is empty.
//! * Proves the differential harness is actually *sensitive*: a plausible but
//!   wrong implementation (round-to-nearest instead of truncate-toward-zero,
//!   or premultiplying the alpha channel too) is detected. Without this a
//!   passing suite could just mean both libraries were doing nothing.

mod support;

use std::process::Command;

use support::{load_pair, Rng};

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Keep global text/data/bss/rodata definitions; drop the
            // loader/runtime bookkeeping symbols that are not part of the
            // library's API surface.
            match kind {
                "T" | "D" | "B" | "R" | "G" | "S" | "W" | "V" => Some(name.to_string()),
                _ => None,
            }
        })
        .filter(|n| {
            !n.starts_with("_ITM_")
                && !n.starts_with("__cxa_")
                && !n.starts_with("_fini")
                && !n.starts_with("_init")
                && *n != "__gmon_start__"
                && *n != "_edata"
                && *n != "_end"
                && *n != "__bss_start"
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn symbol_parity_c_subset_of_rust() {
    let (c, r) = load_pair();
    let c_syms = nm_defined(&c.path);
    let r_syms = nm_defined(&r.path);

    assert!(
        c_syms.contains(&"premultiply".to_string()),
        "sanity: the C .so must export `premultiply`; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {r_syms:?}"
    );
}

#[test]
fn rust_so_has_no_unresolved_library_symbols() {
    let (_c, r) = load_pair();
    let out = Command::new("nm")
        .args(["-D", "--undefined-only", "--format=posix"])
        .arg(&r.path)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);

    // Everything the Rust .so imports must be libc / language-runtime, never a
    // library symbol the translation failed to provide.
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__tls_get_addr", "__errno_location", "__gmon_start__",
        "pthread_", "std", "_dl", "dl_iterate_phdr",
    ];
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "free", "fstat", "fstat64", "getcwd", "getenv",
        "gettid", "lseek", "lseek64", "malloc", "memcpy", "memmove", "memset", "mmap", "mmap64",
        "munmap", "open", "open64", "posix_memalign", "read", "readlink", "realloc", "realpath",
        "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "memcmp", "sysconf",
        "__libc_start_main", "qsort", "getauxval", "sigaltstack", "sigaction", "mprotect",
        "pipe2", "poll", "nanosleep", "__assert_fail", "raise", "signal", "exit",
    ];

    let mut suspicious = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let raw = match it.next() {
            Some(s) => s,
            None => continue,
        };
        // strip the @GLIBC_x.y / @@VER version suffix
        let name = raw.split('@').next().unwrap_or(raw);
        if allowed_prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        if allowed_exact.contains(&name) {
            continue;
        }
        suspicious.push(name.to_string());
    }
    assert!(
        suspicious.is_empty(),
        "Rust .so imports non-libc symbols (would indicate an untranslated \
         module): {suspicious:?}"
    );
}

// --------------------------------------------------------------------------
// Harness sensitivity: the differential comparison must be able to FAIL.
// --------------------------------------------------------------------------

/// Faithful model of the C expression, used only to check the harness.
fn model_trunc(c: u8, a: u8) -> u8 {
    let af = f32::from(a) / 255.0f32;
    let cf = f32::from(c) / 255.0f32;
    ((cf * af) * 255.0f32) as i32 as u8
}

/// A plausible-but-wrong variant: round to nearest instead of truncating.
fn model_round(c: u8, a: u8) -> u8 {
    let af = f32::from(a) / 255.0f32;
    let cf = f32::from(c) / 255.0f32;
    ((cf * af) * 255.0f32).round() as i32 as u8
}

#[test]
fn harness_is_sensitive_to_rounding_mode() {
    let (c, r) = load_pair();

    // Full (channel, alpha) sweep, straight through both .so exports.
    let mut input = vec![0u8; 256 * 256 * 4];
    for a in 0usize..256 {
        for v in 0usize..256 {
            let i = (a * 256 + v) * 4;
            input[i] = v as u8;
            input[i + 1] = v as u8;
            input[i + 2] = v as u8;
            input[i + 3] = a as u8;
        }
    }
    let mut cb = input.clone();
    let mut rb = input.clone();
    c.call_bytes(256, 256, &mut cb);
    r.call_bytes(256, 256, &mut rb);
    assert_eq!(cb, rb, "C/Rust divergence on the exhaustive sweep");

    let mut trunc_mismatch = 0usize;
    let mut round_mismatch = 0usize;
    for a in 0usize..256 {
        for v in 0usize..256 {
            let i = (a * 256 + v) * 4;
            if cb[i] != model_trunc(v as u8, a as u8) {
                trunc_mismatch += 1;
            }
            if cb[i] != model_round(v as u8, a as u8) {
                round_mismatch += 1;
            }
        }
    }
    assert_eq!(
        trunc_mismatch, 0,
        "the C library does NOT match truncate-toward-zero in {trunc_mismatch} cases"
    );
    assert!(
        round_mismatch > 1_000,
        "harness insensitive: round-to-nearest differs from the real C output in \
         only {round_mismatch} of 65536 cases, so these inputs would not catch a \
         rounding bug"
    );
}

#[test]
fn harness_is_sensitive_to_alpha_being_written() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0xD00D_D00D);
    // If a buggy translation also premultiplied alpha, the alpha byte would
    // change for a large fraction of random pixels. Verify that the inputs used
    // throughout the suite really do have that property, i.e. the "alpha
    // untouched" assertions are load-bearing.
    let mut input = vec![0u8; 4096 * 4];
    rng.fill_bytes(&mut input);
    let mut cb = input.clone();
    let mut rb = input.clone();
    c.call_bytes(4096, 1, &mut cb);
    r.call_bytes(4096, 1, &mut rb);
    assert_eq!(cb, rb);

    let would_change = (0..4096)
        .filter(|&k| {
            let a = input[k * 4 + 3];
            model_trunc(a, a) != a
        })
        .count();
    assert!(
        would_change > 3_000,
        "harness insensitive: only {would_change}/4096 pixels would reveal an \
         alpha-premultiplying bug"
    );

    // And the RGB channels really are being modified (guards against "both
    // libraries did nothing").
    let changed_rgb = (0..4096)
        .filter(|&k| input[k * 4..k * 4 + 3] != cb[k * 4..k * 4 + 3])
        .count();
    assert!(
        changed_rgb > 3_000,
        "only {changed_rgb}/4096 pixels changed — the libraries may be no-ops"
    );
}
