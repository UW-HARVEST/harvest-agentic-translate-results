//! Phase D — symbol parity and build-configuration independence.
//!
//! These tests re-derive the `SYMBOLS.md` claims mechanically at test time so
//! the artifact cannot drift away from reality.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// Global (uppercase-type) defined symbols of a shared object, via `nm -D`.
fn defined_globals(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm` — binutils must be installed");
    assert!(
        out.status.success(),
        "nm -D --defined-only {} failed: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (a, b, c) = (it.next(), it.next(), it.next());
            match (a, b, c) {
                // "<addr> <type> <name>"
                (Some(_), Some(t), Some(name)) if t.len() == 1 => {
                    Some((t.to_string(), name.to_string()))
                }
                // "<type> <name>" (undefined/absolute forms)
                (Some(t), Some(name), None) if t.len() == 1 => {
                    Some((t.to_string(), name.to_string()))
                }
                _ => None,
            }
        })
        // keep only true globals: uppercase nm type letters
        .filter(|(t, _)| t.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
        .map(|(_, name)| name)
        .collect()
}

fn undefined_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(so)
        .output()
        .expect("failed to run `nm`");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| s != "U" && s != "w" && s != "U*")
        .collect()
}

/// Symbols that are part of any Rust `cdylib`'s own bookkeeping and are not
/// expected to appear in the C `.so`.
fn is_rust_runtime_symbol(name: &str) -> bool {
    name.starts_with("_ZN")
        || name.starts_with("rust_")
        || name.starts_with("__rust")
        || name.starts_with("_R")
        || name.contains("17h") // legacy mangled hash
}

/// Symbols the linker adds to every ELF shared object.
fn is_elf_boilerplate(name: &str) -> bool {
    matches!(
        name,
        "_init"
            | "_fini"
            | "__bss_start"
            | "_edata"
            | "_end"
            | "__gmon_start__"
            | "_ITM_deregisterTMCloneTable"
            | "_ITM_registerTMCloneTable"
            | "__cxa_finalize"
            | "__gxx_personality_v0"
    )
}

/// Every global symbol exported by the C `.so` must also be exported by the
/// Rust `.so` under the exact same name. The diff must be empty.
#[test]
fn phase_d_symbol_parity_c_to_rust() {
    let c_so = c_so_path();
    let r_so = rust_so_path();
    let c_syms = defined_globals(&c_so);
    let r_syms = defined_globals(&r_so);

    let c_api: BTreeSet<_> = c_syms
        .iter()
        .filter(|s| !is_elf_boilerplate(s))
        .cloned()
        .collect();
    assert!(
        !c_api.is_empty(),
        "no API symbols found in the C .so ({}) — is it built?",
        c_so.display()
    );

    let missing: Vec<_> = c_api.difference(&r_syms).cloned().collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C .so:    {}\n\
         Rust .so: {}\n\
         C globals:    {c_api:?}\n\
         Rust globals: {r_syms:?}",
        c_so.display(),
        r_so.display()
    );

    // The whole documented API is exactly one function.
    assert!(
        c_api.contains("to_barycentric"),
        "C .so must export to_barycentric, found {c_api:?}"
    );
    assert!(r_syms.contains("to_barycentric"));

    // `static` C helpers must NOT leak into the Rust ABI either.
    for hidden in ["lm_v2", "lm_sub2", "lm_dot2"] {
        assert!(
            !c_syms.contains(hidden),
            "{hidden} is static in C and must not be global"
        );
        assert!(
            !r_syms.contains(hidden),
            "{hidden} must not be exported from the Rust .so"
        );
    }
}

/// The Rust `.so` must not depend on any library-internal symbol that is not
/// provided by libc / the platform runtime.
///
/// Checked two ways:
///  1. `ldd -r` must report no unresolved symbol at all (this is the mechanical,
///     allowlist-free proof that nothing library-internal is missing);
///  2. every undefined symbol must be attributable to glibc / libgcc / the Rust
///     std runtime rather than to an untranslated `c_src` module.
#[test]
fn phase_d_no_unresolved_internal_symbols() {
    let r_so = rust_so_path();

    // (1) nothing at all is unresolved at load time.
    let out = Command::new("ldd")
        .arg("-r")
        .arg(&r_so)
        .output()
        .expect("failed to run `ldd`");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let unresolved: Vec<&str> = combined
        .lines()
        .filter(|l| l.to_lowercase().contains("undefined symbol"))
        .collect();
    assert!(
        unresolved.is_empty(),
        "`ldd -r {}` reports unresolved symbols:\n{}",
        r_so.display(),
        unresolved.join("\n")
    );

    // (2) every import is libc / libgcc / Rust-std provenance. `nm` prints
    // versioned names (`memcpy@GLIBC_2.14`), so strip the version tag first.
    let undef = undefined_symbols(&r_so);
    let suspicious: Vec<String> = undef
        .iter()
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !is_rust_runtime_symbol(s) && !is_elf_boilerplate(s))
        .filter(|s| !s.starts_with("__")) // __errno_location, __tls_get_addr, ...
        .filter(|s| !s.starts_with("_Unwind_")) // libgcc unwinder
        .filter(|s| !s.starts_with("pthread_"))
        .filter(|s| !LIBC_IMPORTS.contains(&s.as_str()))
        .collect();
    assert!(
        suspicious.is_empty(),
        "undefined symbols in the Rust .so that are not libc/libgcc/Rust-std: \
         {suspicious:?}\nall undefined: {undef:?}"
    );

    // The C library itself imports nothing beyond the ELF boilerplate, so the
    // Rust side importing only libc means no `c_src` code went missing.
    let c_undef = undefined_symbols(&c_so_path());
    let c_api_undef: Vec<_> = c_undef
        .iter()
        .map(|s| s.split('@').next().unwrap_or(s).to_string())
        .filter(|s| !is_elf_boilerplate(s) && !s.starts_with("__"))
        .collect();
    assert!(
        c_api_undef.is_empty(),
        "unexpected imports in the C .so: {c_api_undef:?}"
    );
}

/// glibc functions the Rust standard library pulls in for a `cdylib`.
const LIBC_IMPORTS: &[&str] = &[
    "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64", "getcwd",
    "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset",
    "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign", "read", "readlink", "realloc",
    "realpath", "stat", "stat64", "statx", "strlen", "syscall", "write", "writev", "sysconf",
    "sqrt", "sqrtf", "fmod", "fmodf", "getrandom", "poll", "sigaction", "sigaltstack",
];

/// The Rust `.so` must export the C API with C linkage and the SysV ABI: this
/// verifies the name is unmangled and the symbol is directly `dlsym`-able and
/// callable with the C prototype (which is what the whole differential suite
/// relies on).
#[test]
fn phase_d_exported_symbol_is_callable_via_dlsym() {
    let d = Dual::load();
    // a well-conditioned reference triangle with an exactly representable answer
    let p1 = Vec2::new(0.0, 0.0);
    let p2 = Vec2::new(4.0, 0.0);
    let p3 = Vec2::new(0.0, 4.0);
    let p = Vec2::new(1.0, 2.0);
    let c = d.call_c(p1, p2, p3, p);
    let r = d.call_rust(p1, p2, p3, p);
    assert_eq!(c.bits(), r.bits(), "C={c:?} rust={r:?}");
    // sanity: v0 = p3 - p1 = (0,4) so u is the p3 weight (0.5), v the p2 (0.25)
    assert_eq!(c.x, 0.5);
    assert_eq!(c.y, 0.25);
}

/// The bit-exactness must not depend on the Rust optimisation level: the same
/// results must come out of the debug and the release `cdylib`.
///
/// Skipped (with a clear message) when only one profile has been built.
#[test]
fn phase_d_debug_and_release_agree() {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut sos: Vec<std::path::PathBuf> = Vec::new();
    for profile in ["debug", "release"] {
        let dir = target.join(profile);
        if !dir.is_dir() {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && p.extension().map(|x| x == "so").unwrap_or(false) {
                    sos.push(p);
                }
            }
        }
    }
    if sos.len() < 2 {
        eprintln!(
            "only {} Rust .so profile(s) built; run `cargo build && cargo build --release` \
             to cross-check optimisation levels",
            sos.len()
        );
        return;
    }
    let c_so = c_so_path();
    // SAFETY: all paths are shared objects we built; the signature matches the
    // C prototype exactly.
    unsafe {
        let c_lib = libloading::Library::new(&c_so).unwrap();
        let c_fn: libloading::Symbol<ToBarycentric> = c_lib.get(b"to_barycentric\0").unwrap();
        let mut rng = Rng::new(SEED ^ 0xD1);
        let mut libs = Vec::new();
        for so in &sos {
            let l = libloading::Library::new(so).unwrap();
            libs.push(l);
        }
        let fns: Vec<ToBarycentric> = libs
            .iter()
            .map(|l| {
                let s: libloading::Symbol<ToBarycentric> = l.get(b"to_barycentric\0").unwrap();
                *s
            })
            .collect();
        for _ in 0..100_000 {
            let a = rng.vec2_any();
            let b = rng.vec2_any();
            let cc = rng.vec2_any();
            let p = rng.vec2_any();
            let expect = (*c_fn)(a, b, cc, p);
            for (f, so) in fns.iter().zip(sos.iter()) {
                let got = f(a, b, cc, p);
                assert_eq!(
                    expect.bits(),
                    got.bits(),
                    "{} diverged from C for \
                     ({:#010x},{:#010x})({:#010x},{:#010x})({:#010x},{:#010x})({:#010x},{:#010x})",
                    so.display(),
                    a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits(),
                    cc.x.to_bits(), cc.y.to_bits(), p.x.to_bits(), p.y.to_bits(),
                );
            }
        }
    }
}
