//! Phase D — exported-symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Mechanically re-derives `SYMBOLS.md`: every dynamic symbol the C shared
//! library defines must also be defined by the Rust shared library under the
//! exact same name, and the Rust library must not import any non-libc symbol.

mod common;
use common::*;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(args)
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("failed to run `nm` on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// Defined dynamic symbols (`nm -D --defined-only`, types T/W/D/B).
fn defined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--defined-only"], so)
        .into_iter()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let b = it.next()?;
            // "<addr> <type> <name>"  or  "<type> <name>" for weak/undefined
            let (ty, name) = match it.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            if matches!(ty, "T" | "W" | "D" | "B" | "R" | "V" | "i") {
                Some(name.split('@').next().unwrap().to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Undefined (imported) dynamic symbols.
fn undefined(so: &Path) -> BTreeSet<String> {
    nm(&["-D", "--undefined-only"], so)
        .into_iter()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Symbols that legitimately come from libc / libgcc / the ELF runtime.
fn is_runtime_import(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_ITM_deregisterTMCloneTable",
        "_ITM_registerTMCloneTable",
        "__cxa_finalize",
        "__cxa_thread_atexit_impl",
        "__gmon_start__",
        "__errno_location",
        "__tls_get_addr",
        "__libc_start_main",
        "_init",
        "_fini",
    ];
    if EXACT.contains(&name) {
        return true;
    }
    // libgcc unwinder, glibc & pthread entry points, compiler builtins.
    const PREFIXES: &[&str] = &[
        "_Unwind_",
        "pthread_",
        "__pthread",
        "__libc_",
        "__stack_chk",
        "__memcpy",
        "__rust_",
    ];
    if PREFIXES.iter().any(|p| name.starts_with(p)) {
        return true;
    }
    // Plain libc / libm functions used by either library.
    const LIBC: &[&str] = &[
        "sqrtf", "sqrt", "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free",
        "fstat", "fstat64", "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc",
        "memcmp", "memcpy", "memmove", "memset", "mmap", "mmap64", "munmap", "open",
        "open64", "posix_memalign", "read", "readlink", "realloc", "realpath", "sigaction",
        "sigaltstack", "signal", "strlen", "sysconf", "write", "writev", "syscall",
        "getpid", "raise", "exit", "_exit", "poll", "mprotect", "madvise", "environ",
        "stat", "stat64", "lstat", "lstat64", "statx", "pread64", "prctl", "dlsym",
        "dladdr", "dl_find_object", "nanosleep", "clock_gettime", "sched_yield",
        "sched_getaffinity", "getrandom", "pipe2", "dup", "dup2", "fcntl", "isatty",
        "strerror_r", "abs", "qsort", "bsearch",
    ];
    LIBC.contains(&name)
}

#[test]
fn symbols_every_c_export_is_present_in_rust() {
    let (c_so, rust_so) = so_paths();
    let c_syms = defined(c_so);
    let rust_syms = defined(rust_so);

    assert!(
        !c_syms.is_empty(),
        "nm found no defined symbols in {}",
        c_so.display()
    );

    let missing: Vec<&String> = c_syms.difference(&rust_syms).collect();
    assert!(
        missing.is_empty(),
        "\n{} symbol(s) exported by the C .so are MISSING from the Rust .so:\n  {}\n\
         C .so   = {}\n  Rust .so = {}\n",
        missing.len(),
        missing
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        c_so.display(),
        rust_so.display()
    );

    // Pin the expected surface so a future partial translation is caught.
    const EXPECTED: &[&str] = &[
        "c2AABBtoAABB",
        "c2AABBtoPoint",
        "c2Absv",
        "c2Add",
        "c2CCW90",
        "c2CastRay",
        "c2CircleToPoint",
        "c2Div",
        "c2Dot",
        "c2Len",
        "c2Maxv",
        "c2Minv",
        "c2MulmvT",
        "c2Mulrv",
        "c2MulrvT",
        "c2Mulvs",
        "c2MulxvT",
        "c2Norm",
        "c2RaytoAABB",
        "c2RaytoCapsule",
        "c2RaytoCircle",
        "c2RaytoPoly",
        "c2RotIdentity",
        "c2Skew",
        "c2Sub",
        "c2V",
        "c2xIdentity",
        "poly_ray",
    ];
    for name in EXPECTED {
        assert!(
            c_syms.contains(*name),
            "the C .so no longer exports `{name}` — regenerate SYMBOLS.md"
        );
        assert!(
            rust_syms.contains(*name),
            "the Rust .so does not export `{name}`"
        );
    }
    assert_eq!(
        c_syms.len(),
        EXPECTED.len(),
        "the C .so exports {} symbols but SYMBOLS.md lists {}: {:?}",
        c_syms.len(),
        EXPECTED.len(),
        c_syms
    );
}

#[test]
fn symbols_no_unresolved_non_libc_imports() {
    let (c_so, rust_so) = so_paths();
    for so in [c_so, rust_so] {
        let bad: Vec<String> = undefined(so)
            .into_iter()
            .filter(|s| !is_runtime_import(s))
            .collect();
        assert!(
            bad.is_empty(),
            "{} has unresolved non-libc imports:\n  {}",
            so.display(),
            bad.join("\n  ")
        );
    }
}

/// Every symbol the C `.so` exports must be `dlsym`-able from the Rust `.so`
/// *and* actually callable through the FFI boundary.
#[test]
fn symbols_all_dlsym_able_and_callable() {
    let (c, r) = (c(), rs());
    // Touching every field of `Api` proves each symbol resolved via dlsym.
    for api in [c, r] {
        let a = unsafe { (api.c2V)(1.0, 2.0) };
        assert!(veq(a, v(1.0, 2.0)), "{}: c2V", api.name);
        assert!(feq(unsafe { (api.c2Dot)(a, a) }, 5.0), "{}: c2Dot", api.name);
        assert!(unsafe { (api.c2Len)(v(3.0, 4.0)) } == 5.0, "{}: c2Len", api.name);
        let _ = unsafe { (api.c2Add)(a, a) };
        let _ = unsafe { (api.c2Sub)(a, a) };
        let _ = unsafe { (api.c2Mulvs)(a, 2.0) };
        let _ = unsafe { (api.c2Div)(a, 2.0) };
        let _ = unsafe { (api.c2Norm)(a) };
        let _ = unsafe { (api.c2Minv)(a, a) };
        let _ = unsafe { (api.c2Maxv)(a, a) };
        let _ = unsafe { (api.c2Skew)(a) };
        let _ = unsafe { (api.c2Absv)(a) };
        let _ = unsafe { (api.c2CCW90)(a) };
        let _ = unsafe { (api.c2MulmvT)(C2m { x: a, y: a }, a) };
        let rot = unsafe { (api.c2RotIdentity)() };
        let xf = unsafe { (api.c2xIdentity)() };
        let _ = unsafe { (api.c2Mulrv)(rot, a) };
        let _ = unsafe { (api.c2MulrvT)(rot, a) };
        let _ = unsafe { (api.c2MulxvT)(xf, a) };
        let boxx = C2AABB {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        let _ = unsafe { (api.c2AABBtoAABB)(boxx, boxx) };
        let _ = unsafe { (api.c2AABBtoPoint)(boxx, a) };
        let circle = C2Circle { p: v(0.0, 0.0), r: 1.0 };
        let _ = unsafe { (api.c2CircleToPoint)(circle, a) };
        let rr = C2Ray {
            p: v(-4.0, 0.0),
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let mut out = C2Raycast::default();
        let _ = unsafe { (api.c2RaytoCircle)(rr, circle, &mut out) };
        let _ = unsafe { (api.c2RaytoAABB)(rr, boxx, &mut out) };
        let _ = unsafe {
            (api.c2RaytoCapsule)(
                rr,
                C2Capsule {
                    a: v(0.0, 0.0),
                    b: v(0.0, 10.0),
                    r: 1.0,
                },
                &mut out,
            )
        };
        let mut poly = C2Poly::default();
        poly.count = 4;
        poly.verts[0] = v(1.0, -1.0);
        poly.verts[1] = v(1.0, 1.0);
        poly.verts[2] = v(-1.0, 1.0);
        poly.verts[3] = v(-1.0, -1.0);
        poly.norms[0] = v(1.0, 0.0);
        poly.norms[1] = v(0.0, 1.0);
        poly.norms[2] = v(-1.0, 0.0);
        poly.norms[3] = v(0.0, -1.0);
        let hit = unsafe { (api.c2RaytoPoly)(rr, &poly, std::ptr::null(), &mut out) };
        assert_eq!(hit, 1, "{}: c2RaytoPoly should hit", api.name);
        let hit2 = unsafe {
            (api.c2CastRay)(
                rr,
                (&poly as *const C2Poly) as *const std::ffi::c_void,
                std::ptr::null(),
                C2_TYPE_POLY,
                &mut out,
            )
        };
        assert_eq!(hit2, 1, "{}: c2CastRay(POLY) should hit", api.name);
        let mut o1 = C2Raycast::default();
        let mut o2 = C2Raycast::default();
        let _ = unsafe { (api.poly_ray)(&mut o1, &mut o2) };
    }
}
