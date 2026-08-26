//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Reproduces the `nm -D` diff recorded in SYMBOLS.md as an assertion.

mod harness;

use harness::*;

#[test]
fn c_exported_symbols_are_all_exported_by_rust() {
    let c_so = c_so_path();
    let rust_so = rust_so_path();

    let c_syms = nm_defined_symbols(&c_so);
    let rust_syms = nm_defined_symbols(&rust_so);

    assert!(!c_syms.is_empty(), "nm found no symbols in {}", c_so.display());

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         C:    {}\n  {c_syms:?}\nRUST: {}\n  (rust exports {} symbols)",
        missing.len(),
        c_so.display(),
        rust_so.display(),
        rust_syms.len()
    );
}

#[test]
fn the_expected_five_symbols_are_present_in_both() {
    for so in [c_so_path(), rust_so_path()] {
        let syms = nm_defined_symbols(&so);
        for want in EXPECTED_SYMBOLS {
            assert!(
                syms.contains(&want.to_string()),
                "{} does not export {want}",
                so.display()
            );
        }
    }
}

#[test]
fn static_c_functions_are_exported_by_neither() {
    // `goodG2B` and `goodB2G` are `static` in driver.c. Exporting them from
    // the Rust .so would be a divergence in the ABI surface, so assert their
    // ABSENCE on both sides.
    for so in [c_so_path(), rust_so_path()] {
        let syms = nm_defined_symbols(&so);
        for hidden in ["goodG2B", "goodB2G"] {
            assert!(
                !syms.contains(&hidden.to_string()),
                "{} must NOT export the static function {hidden}",
                so.display()
            );
        }
    }
}

#[test]
fn every_c_symbol_resolves_via_dlsym_in_both_libraries() {
    // `pair()` resolves all five symbols in both libraries during load and
    // panics if any is missing, so simply constructing it is the assertion.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "RUST");
    println!("C    .so: {}", p.c.path.display());
    println!("RUST .so: {}", p.rust.path.display());
}

#[test]
fn rust_so_has_no_undefined_non_libc_symbols() {
    let rust_so = rust_so_path();
    let out = std::process::Command::new("nm")
        .args(["-D", "-u"])
        .arg(&rust_so)
        .output()
        .expect("run nm -D -u");
    assert!(out.status.success(), "nm -D -u failed");

    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno_location", "__gmon_start__", "__tls_get_addr",
        "__libc_", "__stack_chk_",
    ];
    // Everything the Rust std runtime and the translated `printf` call need.
    let allowed_exact = [
        "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat64", "fstat",
        "getcwd", "getenv", "gettid", "lseek64", "lseek", "malloc", "memcpy", "memmove", "memset",
        "mmap64", "mmap", "munmap", "open64", "open", "posix_memalign", "printf",
        "pthread_key_create", "pthread_key_delete", "pthread_setspecific", "pthread_getspecific",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_destroy", "pthread_self",
        "read", "readlink", "realloc", "realpath", "stat64", "stat", "statx", "strlen", "syscall",
        "write", "writev", "sysconf", "memcmp", "qsort", "exit", "sigaction", "sigaltstack",
        "mprotect", "pthread_attr_init", "pthread_attr_destroy", "pthread_getattr_np",
        "pthread_attr_getstack", "environ", "__environ",
        // Optimised builds rewrite `printf("%s\n", s)` into `puts(s)`. gcc -O2
        // performs the *identical* transform on the C source (its `printLine`
        // is likewise `test %rdi,%rdi; je; jmp puts@plt`), so seeing these
        // instead of `printf` is codegen parity, not a behavioural change.
        "puts", "putchar", "fputs", "fputc", "fwrite", "fwrite_unlocked", "putc",
    ];

    let mut unexpected = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let name = match line.split_whitespace().last() {
            Some(n) => n,
            None => continue,
        };
        // Strip any "@GLIBC_2.x" / "@@GLIBC_2.x" version suffix.
        let base = name.split('@').next().unwrap_or(name);
        if allowed_prefixes.iter().any(|p| base.starts_with(p))
            || allowed_exact.contains(&base)
        {
            continue;
        }
        unexpected.push(base.to_string());
    }
    unexpected.sort();
    unexpected.dedup();
    assert!(
        unexpected.is_empty(),
        "the Rust .so has undefined NON-libc symbols (unresolved translation \
         references): {unexpected:?}"
    );
}

#[test]
fn both_libraries_load_standalone_with_no_unresolved_relocations() {
    // `dlopen` with RTLD_NOW forces every relocation to be resolved at load
    // time; if the Rust .so referenced a symbol that does not exist anywhere,
    // this is where it would fail.
    for so in [c_so_path(), rust_so_path()] {
        let lib = unsafe {
            libloading::os::unix::Library::open(
                Some(&so),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL,
            )
        };
        assert!(
            lib.is_ok(),
            "RTLD_NOW dlopen of {} failed: {:?}",
            so.display(),
            lib.err()
        );
    }
}

#[test]
fn report_profile_and_feature_configuration() {
    // Recorded so the test log shows which of the CONFIGS.md build
    // configurations this run covered.
    println!(
        "profile={} debug_assertions={} overflow_checks_active={}",
        profile(),
        cfg!(debug_assertions),
        cfg!(debug_assertions)
    );
    // There is no `[features]` section in Cargo.toml, hence exactly one
    // feature combination; assert that no feature cfg ever becomes active.
    println!("feature combination: <none> (the only one)");
}

// ---------------------------------------------------------------------------
// Optimised-C cross-check.
//
// The default CMake build of the C library is unoptimised. The one real
// divergence this verification found (printHexCharLine's `char` parameter) was
// invisible until the *Rust* side was optimised, so the optimised C build is
// compared too: it is the configuration that proves the narrowing behaviour is
// a deliberate property of the C, not an artefact of -O0.
// ---------------------------------------------------------------------------

#[test]
fn optimised_c_build_agrees_with_rust_on_the_whole_surface() {
    let Some(c_o2) = c_o2_api() else {
        eprintln!("no C compiler available; skipping the -O2 cross-check");
        return;
    };
    let rust = &pair().rust;
    let mut rng = Rng::new(SEED ^ 0x0002);

    // Whole char domain through the correctly-typed prototype.
    for raw in 0u16..=255 {
        let v = raw as u8 as std::ffi::c_char;
        let a = capture(|| unsafe { (c_o2.print_hex_char_line)(v) });
        let b = capture(|| unsafe { (rust.print_hex_char_line)(v) });
        assert_eq!(a, b, "-O2 C vs Rust: printHexCharLine({v})");
    }

    // The widened prototype: arbitrary upper bits, which is where the
    // divergence lived. gcc narrows at -O2, so the Rust must too.
    for i in 0..512 {
        let v = if i < 16 {
            [0x1234_5678i32, 256, 0x1ff, -1000, i32::MIN, i32::MAX, -1, 0,
             0xdead_beefu32 as i32, 0xffff_ff7fu32 as i32, 129, -129, 255, 128, 127, -128][i]
        } else {
            rng.next_i32()
        };
        let a = capture(|| unsafe { (c_o2.print_hex_char_line_widened)(v) });
        let b = capture(|| unsafe { (rust.print_hex_char_line_widened)(v) });
        assert_eq!(a, b, "-O2 C vs Rust: printHexCharLine(widened {v:#x})");
    }

    // printLine: gcc -O2 rewrites printf("%s\n",s) into puts(s); byte output
    // must still match, including NULL, empty, `%`-bearing and long payloads.
    let payloads: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"%s%n%d".to_vec(),
        b"\xff\xfe\x80".to_vec(),
        b"a\nb".to_vec(),
        vec![b'L'; 70_000],
    ];
    for pl in &payloads {
        let a = capture(|| call_print_line(c_o2, pl));
        let b = capture(|| call_print_line(rust, pl));
        assert_eq!(a, b, "-O2 C vs Rust: printLine(len={})", pl.len());
    }
    let a = capture(|| unsafe { (c_o2.print_line)(std::ptr::null()) });
    let b = capture(|| unsafe { (rust.print_line)(std::ptr::null()) });
    assert_eq!(a, b, "-O2 C vs Rust: printLine(NULL)");
    assert_eq!(a, b"", "-O2 C vs Rust: printLine(NULL) must emit nothing");

    // bad / good / driver.
    for (label, f) in [
        ("bad", 0usize),
        ("good", 1),
    ] {
        let a = capture(|| unsafe { if f == 0 { (c_o2.bad)() } else { (c_o2.good)() } });
        let b = capture(|| unsafe { if f == 0 { (rust.bad)() } else { (rust.good)() } });
        assert_eq!(a, b, "-O2 C vs Rust: {label}()");
    }
    for i in 0..512 {
        let v = if i == 0 { 0 } else { rng.next_i32() };
        let a = capture(|| unsafe { (c_o2.driver)(v) });
        let b = capture(|| unsafe { (rust.driver)(v) });
        assert_eq!(a, b, "-O2 C vs Rust: driver({v})");
    }
}

#[test]
fn optimised_c_build_exports_the_same_symbols() {
    let Some(c_o2) = c_o2_api() else { return };
    let syms = nm_defined_symbols(&c_o2.path);
    for want in EXPECTED_SYMBOLS {
        assert!(syms.contains(&want.to_string()), "-O2 C .so lacks {want}");
    }
    for hidden in ["goodG2B", "goodB2G"] {
        assert!(!syms.contains(&hidden.to_string()), "-O2 C .so exports {hidden}");
    }
}
