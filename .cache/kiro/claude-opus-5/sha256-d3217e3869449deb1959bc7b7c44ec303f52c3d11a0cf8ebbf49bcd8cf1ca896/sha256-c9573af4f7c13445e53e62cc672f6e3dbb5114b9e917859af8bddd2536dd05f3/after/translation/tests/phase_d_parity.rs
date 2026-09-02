// Phase D — symbol parity and feature-matrix guards, enforced as tests rather
// than as a one-off shell check, so they cannot silently rot.
mod common;

use common::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// Global text symbols (`nm -D`, capital type letters only) exported by a `.so`.
fn exported_symbols(so: &PathBuf) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(
        out.status.success(),
        "nm -D failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let a = parts.next()?;
            let b = parts.next()?;
            // "<addr> <type> <name>" or "<type> <name>" for undefined-address syms.
            let (ty, name) = match parts.next() {
                Some(n) => (b, n),
                None => (a, b),
            };
            // Uppercase type letter == global/external linkage.
            let t = ty.chars().next()?;
            if t.is_ascii_uppercase() { Some(name.to_string()) } else { None }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// The seven functions with external linkage in `c_src/src/lib.c`.
const EXPECTED: [&str; 7] = [
    "apply_multiplier",
    "classify_mode",
    "convert_negative_overflow",
    "convert_time_factor",
    "get_modified_time",
    "hash_time_value",
    "modeselect",
];

#[test]
fn d1_every_c_symbol_is_exported_by_rust() {
    let c_so = c_so_path();
    let rs_so = rust_so_path();

    let c_syms = exported_symbols(&c_so);
    let rs_syms = exported_symbols(&rs_so);

    // The C `.so` also exports the usual glibc-injected bookkeeping symbols
    // (_init, _fini, __bss_start, _edata, _end). Those are linker artifacts, not
    // library API, so compare against the functions the C source actually
    // defines with external linkage.
    let linker_artifacts = ["_init", "_fini", "__bss_start", "_edata", "_end"];
    let c_api: Vec<&String> = c_syms
        .iter()
        .filter(|s| !linker_artifacts.contains(&s.as_str()))
        .collect();

    // First: the C `.so` really does export exactly the seven we claim.
    let c_api_names: Vec<&str> = c_api.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        c_api_names, EXPECTED,
        "the C .so's API surface is not what SYMBOLS.md records"
    );

    // Second: nothing in that surface is missing from the Rust `.so`.
    let missing: Vec<&&String> = c_api.iter().filter(|s| !rs_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {missing:?}\n\
         Per Phase A: add the #[no_mangle] wrapper if the impl exists, or translate \
         the missing C source if a whole module was skipped.",
        missing.len()
    );
}

#[test]
fn d2_every_symbol_is_dlsym_resolvable_and_callable() {
    // `nm` showing a name is weaker than the symbol actually resolving and
    // running. `pair()` dlsym's all seven from BOTH libraries, and each is
    // invoked here, so a stub that only exists in the symbol table would still
    // have to produce the C's answer.
    let p = pair();
    let s = cstr(b"extreme");
    // SAFETY: NUL-terminated buffer; plain scalar C ABI calls otherwise.
    unsafe {
        eq_int(
            "D2",
            "classify_mode",
            (p.c.classify_mode)(s.as_ptr() as *const std::ffi::c_char),
            (p.rs.classify_mode)(s.as_ptr() as *const std::ffi::c_char),
        );
        eq_int("D2", "apply_multiplier", (p.c.apply_multiplier)(0xA0, 2), (p.rs.apply_multiplier)(0xA0, 2));
        eq_int(
            "D2",
            "convert_time_factor",
            (p.c.convert_time_factor)(1e-4),
            (p.rs.convert_time_factor)(1e-4),
        );
        eq_int(
            "D2",
            "convert_negative_overflow",
            (p.c.convert_negative_overflow)(1e-7),
            (p.rs.convert_negative_overflow)(1e-7),
        );
        eq_time(
            "D2",
            "get_modified_time",
            (p.c.get_modified_time)(3, 4),
            (p.rs.get_modified_time)(3, 4),
        );
        eq_int("D2", "hash_time_value", (p.c.hash_time_value)(12345), (p.rs.hash_time_value)(12345));
    }
    let (rc, oc) = capture_forked_i32(|| unsafe { (p.c.modeselect)(1, 2, 3, 4) });
    let (rr, or) = capture_forked_i32(|| unsafe { (p.rs.modeselect)(1, 2, 3, 4) });
    eq_bytes("D2", "modeselect", &oc, &or);
    eq_int("D2", "modeselect", rc, rr);
}

#[test]
fn d3_rust_so_has_no_unresolved_non_libc_symbols() {
    let rs_so = rust_so_path();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(&rs_so)
        .output()
        .expect("run nm -D --undefined-only");
    assert!(out.status.success(), "nm failed");
    let text = String::from_utf8_lossy(&out.stdout);
    let undef: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|n| !n.is_empty())
        .collect();

    // Everything imported must come from libc / the ELF runtime. Anything else
    // would mean the cdylib depends on a Rust crate that was not linked in.
    let allowed_prefixes = [
        "_ITM_", "__cxa_", "__gmon_", "__tls_", "_Unwind_", "__libc_", "__rust_",
    ];
    let allowed_exact = [
        "printf", "time", "memcpy", "memset", "memmove", "memcmp", "bcmp", "strlen",
        "abort", "malloc", "free", "realloc", "calloc", "posix_memalign", "write",
        "writev", "dl_iterate_phdr", "pthread_mutex_lock", "pthread_mutex_unlock",
        "pthread_mutex_trylock", "pthread_getspecific", "pthread_setspecific",
        "pthread_key_create", "pthread_key_delete", "pthread_self", "getenv",
        "sysconf", "syscall", "open64", "close", "read", "poll", "mmap", "munmap",
        "mprotect", "sigaltstack", "sigaction", "signal", "raise", "gettid",
        "__errno_location", "strerror_r", "unlink", "readlink", "stat64", "fstat64",
        "lseek64", "getcwd", "environ", "__stack_chk_fail", "_exit", "exit",
    ];
    let unexpected: Vec<&&str> = undef
        .iter()
        .filter(|n| {
            !allowed_exact.contains(n) && !allowed_prefixes.iter().any(|p| n.starts_with(p))
        })
        .collect();
    // Report rather than hard-fail on unknown libc names, but fail on anything
    // that looks like an untranslated project symbol.
    let project_like: Vec<&&&str> = unexpected
        .iter()
        .filter(|n| EXPECTED.contains(&n.trim_start_matches('_')))
        .collect();
    assert!(
        project_like.is_empty(),
        "Rust .so imports project symbols it should define itself: {project_like:?}"
    );
    if !unexpected.is_empty() {
        eprintln!("note: additional imported symbols (assumed libc/runtime): {unexpected:?}");
    }
}

#[test]
fn d4_feature_matrix_is_single_default_configuration() {
    // The verification matrix in SYMBOLS.md claims there is exactly ONE build
    // configuration. If a [features] table is ever added to Cargo.toml, this
    // test fails so the matrix gets extended rather than quietly under-covering.
    let manifest = std::fs::read_to_string(root().join("translation/Cargo.toml"))
        .expect("read translation/Cargo.toml");
    let has_features = manifest
        .lines()
        .map(|l| l.trim())
        .any(|l| l == "[features]" || l.starts_with("[features."));
    assert!(
        !has_features,
        "Cargo.toml now declares [features]; Phases B-C must be re-run for every \
         feature combination and SYMBOLS.md / CONFIGS.md updated accordingly"
    );

    // Same for the C side: no #ifdef means no compile-time variants to cover.
    let c_src = std::fs::read_to_string(root().join("c_src/src/lib.c")).expect("read lib.c");
    let ifdefs: Vec<&str> = c_src
        .lines()
        .map(|l| l.trim())
        .filter(|l| {
            l.starts_with("#ifdef") || l.starts_with("#ifndef") || l.starts_with("#if ")
        })
        .collect();
    assert!(
        ifdefs.is_empty(),
        "c_src/src/lib.c has conditional compilation that CONFIGS.md does not cover: {ifdefs:?}"
    );
}

#[test]
fn d5_c_source_is_fully_covered_by_the_rust_translation() {
    // Completeness check: every non-static function DEFINED in the C source must
    // appear as a #[no_mangle] export in the Rust source. Catches the "a whole
    // module was never translated" failure mode mechanically.
    let c_src = std::fs::read_to_string(root().join("c_src/src/lib.c")).expect("read lib.c");
    let rs_src =
        std::fs::read_to_string(root().join("translation/src/lib.rs")).expect("read lib.rs");

    // Definitions look like `<type> name(args) {` at column 0 in this file.
    let mut defined: Vec<String> = Vec::new();
    for line in c_src.lines() {
        let l = line.trim_end_matches(['\r', '\n']);
        if l.starts_with(' ') || l.starts_with('\t') || l.starts_with("//") || l.starts_with('#') {
            continue;
        }
        if !l.ends_with('{') || !l.contains('(') {
            continue;
        }
        if l.starts_with("static ") {
            continue; // internal linkage: not part of the ABI
        }
        let before_paren = &l[..l.find('(').unwrap()];
        if let Some(name) = before_paren.split_whitespace().last() {
            let name = name.trim_start_matches('*');
            if !name.is_empty() {
                defined.push(name.to_string());
            }
        }
    }
    defined.sort();
    defined.dedup();

    assert_eq!(
        defined,
        EXPECTED.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "the set of externally-linked functions defined in lib.c changed; \
         re-derive SYMBOLS.md"
    );

    for name in &defined {
        assert!(
            rs_src.contains(&format!("fn {name}(")),
            "C defines `{name}` but translation/src/lib.rs has no `fn {name}(`: \
             that C source was never translated"
        );
        assert!(
            !rs_src.contains(&format!("fn {name}() -> ! {{ unimplemented!()")),
            "`{name}` looks like a stub"
        );
    }

    // No stubbing anywhere in the translation.
    for bad in ["unimplemented!(", "todo!(", "unreachable!(\"stub"] {
        assert!(
            !rs_src.contains(bad),
            "translation/src/lib.rs contains `{bad}`: stubs are not an acceptable \
             way to make a symbol appear in nm -D"
        );
    }
}
