//! Phase A / Phase D — symbol parity and link-configuration parity.
//!
//! These are executable versions of `SYMBOLS.md`: the symbol diff must be empty,
//! and the Rust `.so` must have no unresolved non-libc symbols.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only` -> the set of exported symbol names.
fn exported(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Skip the linker/loader boilerplate every shared object carries.
            const BOILERPLATE: &[&str] = &[
                "_init",
                "_fini",
                "__bss_start",
                "_edata",
                "_end",
                "__cxa_finalize",
                "_ITM_registerTMCloneTable",
                "_ITM_deregisterTMCloneTable",
                "__gmon_start__",
                "__odr_asan_gen_",
            ];
            if BOILERPLATE.contains(&name) {
                return None;
            }
            // Only global/weak text & data definitions are part of the API.
            if !matches!(kind, "T" | "t" | "D" | "B" | "W" | "R" | "V" | "G") {
                return None;
            }
            if kind == "t" {
                return None; // local
            }
            Some(name.to_string())
        })
        .collect()
}

/// SYMBOLS.md checklist item 1: every C export must exist in the Rust `.so`.
#[test]
fn symbol_parity_c_subset_of_rust() {
    let c = exported(&c_so_path());
    let rust = exported(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C library — is it built?"
    );

    let missing: Vec<&String> = c.difference(&rust).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {rust:?}"
    );

    // The four symbols of driver.c must actually be there (guards against the
    // filter above accidentally emptying both sets).
    for want in ["printLine", "bad", "good", "driver"] {
        assert!(c.contains(want), "C .so is missing {want}");
        assert!(rust.contains(want), "Rust .so is missing {want}");
    }
}

/// The C library exports exactly these four; report anything extra the Rust
/// leaks so the surface stays comparable.
#[test]
fn symbol_surface_is_exactly_the_four_functions() {
    let c = exported(&c_so_path());
    let expected: BTreeSet<String> = ["printLine", "bad", "good", "driver"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(c, expected, "the C export surface changed unexpectedly");

    let rust = exported(&rust_so_path());
    let extra: Vec<&String> = rust.difference(&c).collect();
    // A Rust cdylib legitimately exports a few `rust_*` runtime hooks; anything
    // that looks like part of driver.c's API, though, would be a mistake.
    let suspicious: Vec<&&String> = extra
        .iter()
        .filter(|n| !n.starts_with("rust_") && !n.starts_with("_R") && !n.starts_with("__rust"))
        .collect();
    assert!(
        suspicious.is_empty(),
        "Rust .so exports unexpected non-runtime symbols: {suspicious:?}"
    );
}

/// SYMBOLS.md checklist item: 0 unresolved non-libc symbols.
#[test]
fn rust_so_has_no_unresolved_symbols() {
    let out = Command::new("ldd")
        .arg("-r")
        .arg(rust_so_path())
        .output()
        .expect("run ldd -r");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("undefined symbol") || l.contains("not found"))
        .collect();
    assert!(bad.is_empty(), "Rust .so has unresolved symbols:\n{}", bad.join("\n"));
}

/// The C library imports `puts` (gcc's lowering of `printf("%s\n", …)`); the
/// Rust translation must import the same routine, otherwise the two would differ
/// in stdio behaviour and in the stack residue they leave behind.
#[test]
fn both_import_the_same_stdio_routine() {
    fn undefined(path: &std::path::Path) -> BTreeSet<String> {
        let out = Command::new("nm")
            .args(["-D", "--undefined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| {
                // strip the `@GLIBC_x.y.z` version suffix
                l.split_whitespace()
                    .last()
                    .map(|s| s.split('@').next().unwrap_or(s).to_string())
            })
            .collect()
    }
    let c = undefined(&c_so_path());
    let rust = undefined(&rust_so_path());
    assert!(c.contains("puts"), "unexpected: C .so does not import puts ({c:?})");
    assert!(
        rust.contains("puts"),
        "Rust .so must call the same libc routine the C does (puts), imports: {rust:?}"
    );
}

/// Phase D regression guard: the Rust `.so` must be linked the same way the C one
/// is — lazy PLT binding. With `-z now` the dynamic linker's resolver never runs,
/// which changes the stale stack bytes that `bad()`'s indeterminate read
/// observes, and the two libraries then print different output.
/// See `build.rs`.
#[test]
fn link_configuration_matches_c() {
    fn bind_now(path: &std::path::Path) -> bool {
        let out = Command::new("readelf")
            .args(["-d", path.to_str().unwrap()])
            .output()
            .expect("run readelf -d");
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines()
            .any(|l| l.contains("BIND_NOW") || (l.contains("FLAGS_1") && l.contains("NOW")))
    }
    let c_now = bind_now(&c_so_path());
    let rust_now = bind_now(&rust_so_path());
    assert!(!c_now, "unexpected: reference C .so uses BIND_NOW");
    assert!(
        !rust_now,
        "the Rust .so is linked with -z now but the C .so uses lazy binding; \
         build.rs must pass -Wl,-z,lazy (this changes bad()'s observable output)"
    );
}

/// Guard: the suite captures stdout by redirecting the process-global fd 1, so it
/// must not run concurrently with other tests (libtest's own progress lines would
/// end up inside a capture). `.cargo/config.toml` sets `RUST_TEST_THREADS=1`;
/// this test makes a mis-invocation fail with an explanation instead of a
/// confusing byte mismatch.
#[test]
fn test_harness_runs_single_threaded() {
    let n = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        n, "1",
        "this suite must run single-threaded (RUST_TEST_THREADS=1, or \
         `cargo test -- --test-threads=1`) because it redirects the \
         process-global stdout file descriptor; got RUST_TEST_THREADS={n:?}"
    );
}

/// All four symbols must be reachable via `dlsym` from an external caller, which
/// is what the rest of the suite relies on.
#[test]
fn all_symbols_resolve_via_dlsym() {
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
}

// ---------------------------------------------------------------------------
// Frame-layout invariant
// ---------------------------------------------------------------------------

/// Normalise one objdump instruction line to something address-independent:
/// keep the mnemonic, replace absolute addresses by the symbol objdump names,
/// and collapse `%rip`-relative operands (the literal's address differs by
/// construction).
fn normalize_insn(text: &str) -> String {
    // Drop objdump's trailing `# addr <sym>` comment.
    let s = text.split('#').next().unwrap_or("").trim();

    // `call 1040 <puts@plt>` / `je 1158 <printLine+0x1f>` -> `call <puts>`
    if let (Some(lt), Some(gt)) = (s.find('<'), s.rfind('>')) {
        if lt < gt {
            let mnemonic = s[..lt].split_whitespace().next().unwrap_or("").to_string();
            let sym = s[lt + 1..gt]
                .replace("@plt", "")
                .replace("@@Base", "")
                // rustc name-mangles the private literal; the C one is anonymous.
                .replace("_fini", "LITERAL");
            let sym = if sym.contains("STRING_LITERAL") {
                "LITERAL".to_string()
            } else {
                sym
            };
            return format!("{mnemonic} <{sym}>");
        }
    }

    // `lea 0xe7f(%rip),%rax` -> `lea RIPREL,%rax`
    if let Some(pos) = s.find("(%rip)") {
        let start = s[..pos].rfind(|c: char| c == ' ' || c == ',').map_or(0, |i| i + 1);
        let mut out = String::new();
        out.push_str(&s[..start]);
        out.push_str("RIPREL");
        out.push_str(&s[pos + "(%rip)".len()..]);
        return out.split_whitespace().collect::<Vec<_>>().join(" ");
    }

    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract the normalised instruction sequence of one exported function.
fn disasm(path: &std::path::Path, symbol: &str) -> Vec<String> {
    let out = Command::new("objdump")
        .args(["-d", "--no-show-raw-insn", path.to_str().unwrap()])
        .output()
        .expect("run objdump");
    assert!(out.status.success(), "objdump failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines().skip_while(|l| !l.ends_with(&format!("<{symbol}>:")));
    assert!(lines.next().is_some(), "{symbol} not found in {}", path.display());

    let mut insns = Vec::new();
    for l in lines {
        let Some((_addr, rest)) = l.split_once(":\t") else {
            if l.trim().is_empty() {
                break;
            }
            continue;
        };
        let n = normalize_insn(rest);
        if n.is_empty() {
            continue;
        }
        let done = n.split_whitespace().next() == Some("ret");
        insns.push(n);
        if done {
            break;
        }
    }
    assert!(!insns.is_empty(), "no instructions decoded for {symbol}");
    insns
}

/// Phase D invariant: the four functions must have the *same stack frame layout*
/// as the C ones.
///
/// This is not cosmetic. `bad()` reproduces the original's CWE-457 read of an
/// uninitialised local, so its output is decided by which stack address it loads
/// and by what the neighbouring frames spilled there. `good()`'s spill of the
/// string literal, `driver()`'s frame size, and `printLine()`'s parameter spill
/// all feed into that. Comparing the normalised instruction streams pins the
/// whole set of layout decisions at once, and fails loudly if a future change to
/// the translation (or a different codegen setting) perturbs them.
#[test]
fn asm_frame_layout_matches_c() {
    if !cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        eprintln!("skipping: layout-exact translation only applies to x86_64-linux");
        return;
    }
    for sym in ["printLine", "bad", "good", "driver"] {
        let c = disasm(&c_so_path(), sym);
        let rust = disasm(&rust_so_path(), sym);
        assert_eq!(
            c,
            rust,
            "instruction stream of `{sym}` differs between the C and the Rust .so\n\
             C:    {c:#?}\n\
             Rust: {rust:#?}"
        );
    }
}
